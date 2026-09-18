//! Transaction recalculation, mirroring
//! `saleor/payment/tests/test_transaction_item_calculations.py` semantics.

use rust_decimal::Decimal;
use rustygod_core::payments::*;
use rustygod_core::payments::{recalculate, CalcEvent};

fn ev(t: &str, amount: &str, psp: &str) -> CalcEvent {
    CalcEvent {
        event_type: t.into(),
        psp_reference: Some(psp.into()),
        amount: amount.parse().unwrap(),
        include_in_calculations: true,
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
fn success_with_failure_moves_nothing() {
    let b = recalculate(&[
        ev("charge_request", "50.00", "psp1"),
        ev("charge_success", "50.00", "psp1"),
        ev("charge_failure", "50.00", "psp1"),
    ]);
    assert_eq!(b.charged, dec("0"));
    assert_eq!(b.charge_pending, dec("0"));
}

#[test]
fn adjustment_overwrites_then_success_still_adds() {
    // Exact Django order: adjustment overwrites first, then the success
    // amount is still added by _recalculate_base_amounts.
    let b = recalculate(&[
        ev("authorization_success", "100.00", "psp1"),
        ev("authorization_adjustment", "60.00", "psp1"),
    ]);
    assert_eq!(b.authorized, dec("160.00"));
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
