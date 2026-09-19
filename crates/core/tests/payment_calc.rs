//! Transaction recalculation, mirroring
//! `saleor/payment/tests/test_transaction_item_calculations.py` semantics,
//! including the time-sensitive rules (adjustment cutoff, success/failure
//! timestamp race, psp-less handling).

use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use rustygod_core::payments::*;
use rustygod_core::payments::{recalculate, CalcEvent};

fn at(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0).unwrap()
}

fn ev(t: &str, amount: &str, psp: &str) -> CalcEvent {
    ev_at(t, amount, Some(psp), 1000, 1)
}

fn ev_at(t: &str, amount: &str, psp: Option<&str>, secs: i64, id: i32) -> CalcEvent {
    CalcEvent {
        event_type: t.into(),
        psp_reference: psp.map(|s| s.into()),
        amount: amount.parse().unwrap(),
        include_in_calculations: true,
        created_at: at(secs),
        id,
    }
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

#[test]
fn authorize_success_lands_in_authorized() {
    let b = recalculate(&[
        ev("authorization_request", "100.00", "psp1"),
        ev("authorization_success", "100.00", "psp1"),
    ]);
    assert_eq!(b.authorized, dec("100.00"));
    assert_eq!(b.authorize_pending, dec("0"));
}

#[test]
fn lone_request_lands_in_pending() {
    let b = recalculate(&[ev("charge_request", "50.00", "psp1")]);
    assert_eq!(b.charge_pending, dec("50.00"));
    assert_eq!(b.charged, dec("0"));
    // Previous bucket (authorized) moves down.
    assert_eq!(b.authorized, dec("-50.00"));
}

#[test]
fn success_with_newer_failure_moves_nothing() {
    let b = recalculate(&[
        ev_at("charge_request", "50.00", Some("psp1"), 1000, 1),
        ev_at("charge_success", "50.00", Some("psp1"), 1001, 2),
        ev_at("charge_failure", "50.00", Some("psp1"), 1002, 3),
    ]);
    assert_eq!(b.charged, dec("0"));
    assert_eq!(b.charge_pending, dec("0"));
}

#[test]
fn success_newer_than_failure_counts() {
    // Retry with the same psp after a failure: Django compares timestamps
    // (`_should_increse_amount`), the newer success wins.
    let b = recalculate(&[
        ev_at("charge_request", "50.00", Some("psp1"), 1000, 1),
        ev_at("charge_failure", "50.00", Some("psp1"), 1001, 2),
        ev_at("charge_success", "50.00", Some("psp1"), 1002, 3),
    ]);
    assert_eq!(b.charged, dec("50.00"));
    assert_eq!(b.authorized, dec("-50.00"));
}

#[test]
fn adjustment_cuts_older_authorize_events() {
    // Django `_get_authorize_events`: everything older than the newest
    // adjustment is skipped; the adjustment overwrites.
    let b = recalculate(&[
        ev_at("authorization_request", "100.00", Some("psp1"), 1000, 1),
        ev_at("authorization_success", "100.00", Some("psp1"), 1001, 2),
        ev_at("authorization_adjustment", "60.00", Some("psp2"), 1002, 3),
    ]);
    assert_eq!(b.authorized, dec("60.00"));
    assert_eq!(b.authorize_pending, dec("0"));
}

#[test]
fn adjustment_then_new_auth_adds_on_top() {
    // Events NEWER than the adjustment survive: base 60 + new success 40.
    let b = recalculate(&[
        ev_at("authorization_success", "100.00", Some("psp1"), 1000, 1),
        ev_at("authorization_adjustment", "60.00", Some("psp1"), 1001, 2),
        ev_at("authorization_success", "40.00", Some("psp2"), 1002, 3),
    ]);
    assert_eq!(b.authorized, dec("100.00"));
}

#[test]
fn pspless_success_adds_without_previous_move() {
    // App-managed amounts (`_handle_events_without_psp_reference`): no
    // previous-bucket move, unlike grouped PSP events.
    let b = recalculate(&[ev_at("charge_success", "50.00", None, 1000, 1)]);
    assert_eq!(b.charged, dec("50.00"));
    assert_eq!(b.authorized, dec("0"));
}

#[test]
fn pspless_adjustment_assigns_first_groups_add_after() {
    // Order matters (Django's too): without_psp handling runs BEFORE the
    // grouped recalcs, so a psp-less adjustment assigns 25 and the grouped
    // success still adds 100 on top. PSP-less adjustments never cut.
    let b = recalculate(&[
        ev_at("authorization_success", "100.00", Some("psp1"), 1000, 1),
        ev_at("authorization_adjustment", "25.00", None, 1001, 2),
    ]);
    assert_eq!(b.authorized, dec("125.00"));
}

#[test]
fn charge_back_subtracts_charged() {
    let b = recalculate(&[
        ev("charge_success", "100.00", "psp1"),
        ev("charge_back", "20.00", "psp1"),
    ]);
    assert_eq!(b.charged, dec("80.00"));
}

#[test]
fn refund_reverse_moves_back_to_charged() {
    let b = recalculate(&[
        ev("charge_success", "100.00", "psp1"),
        ev("refund_success", "40.00", "psp1"),
        ev("refund_reverse", "40.00", "psp1"),
    ]);
    assert_eq!(b.charged, dec("100.00"));
    assert_eq!(b.refunded, dec("0"));
}

#[test]
fn excluded_events_do_not_count() {
    let b = recalculate(&[CalcEvent {
        event_type: "charge_success".into(),
        psp_reference: Some("psp1".into()),
        amount: dec("100.00"),
        include_in_calculations: false,
        created_at: at(1000),
        id: 1,
    }]);
    assert_eq!(b.charged, dec("0"));
    assert!(!has_money_movement(&b));
}

#[test]
fn coverage_status_thresholds() {
    assert_eq!(coverage_status(dec("0"), dec("100")), "none");
    assert_eq!(coverage_status(dec("50"), dec("100")), "partial");
    assert_eq!(coverage_status(dec("100"), dec("100")), "full");
    assert_eq!(coverage_status(dec("120"), dec("100")), "full");
}
