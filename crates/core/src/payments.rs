//! Transaction amount recalculation, exact port of
//! `saleor/payment/transaction_item_calculations.py`.
//!
//! Rules (all Django-verbatim, including the non-obvious ones):
//! - Events group by action family (authorize/charge/refund/cancel) ×
//!   psp_reference; only events with `include_in_calculations` count.
//! - `*_request` with no success/failure yet → pending bucket += amount
//!   (and the previous bucket -= amount for charge/refund/cancel).
//! - `*_success` counts unless a failure for the same psp is NEWER
//!   (`_should_increse_amount`: timestamps decide, not mere presence).
//!   Same role twice on one psp → last (chronological) wins, like Django's
//!   dict-overwrite in `_initilize_action_map`.
//! - `authorization_adjustment` is a CUTOFF: every authorize-family event
//!   older than the newest adjustment is skipped, and the adjustment
//!   overwrites `authorized` (`_get_authorize_events`).
//! - Events WITHOUT psp_reference (app-managed, `transactionCreate` style)
//!   bypass grouping: successes add with NO previous-bucket move, an
//!   adjustment assigns (`_handle_events_without_psp_reference`).
//! - `charge_back` subtracts `charged`; `refund_reverse` adds `charged`
//!   and subtracts `refunded`.
//! - `*_action_required` / `info` are customer-action records: no bucket
//!   role, never move money.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Family {
    Authorize,
    Charge,
    Refund,
    Cancel,
    Other,
}

/// Failure event of the same family (Django's
/// `get_failed_type_based_on_event`): where a rejected write leaves its
/// audit trail. None for record-only/unknown types.
pub fn failure_event_of(event_type: &str) -> Option<&'static str> {
    match family_of(event_type).0 {
        Family::Authorize => Some("authorization_failure"),
        Family::Charge => Some("charge_failure"),
        Family::Refund => Some("refund_failure"),
        Family::Cancel => Some("cancel_failure"),
        Family::Other => None,
    }
}

pub fn family_of(event_type: &str) -> (Family, &'static str) {
    match event_type {
        "authorization_request" => (Family::Authorize, "request"),
        "authorization_success" => (Family::Authorize, "success"),
        "authorization_failure" => (Family::Authorize, "failure"),
        "authorization_adjustment" => (Family::Authorize, "adjustment"),
        "charge_request" => (Family::Charge, "request"),
        "charge_success" => (Family::Charge, "success"),
        "charge_failure" => (Family::Charge, "failure"),
        "charge_back" => (Family::Charge, "back"),
        "refund_request" => (Family::Refund, "request"),
        "refund_success" => (Family::Refund, "success"),
        "refund_failure" => (Family::Refund, "failure"),
        "refund_reverse" => (Family::Refund, "reverse"),
        "cancel_request" => (Family::Cancel, "request"),
        "cancel_success" => (Family::Cancel, "success"),
        "cancel_failure" => (Family::Cancel, "failure"),
        _ => (Family::Other, "other"),
    }
}

#[derive(Debug, Clone, Default)]
pub struct Buckets {
    pub authorized: Decimal,
    pub authorize_pending: Decimal,
    pub charged: Decimal,
    pub charge_pending: Decimal,
    pub refunded: Decimal,
    pub refund_pending: Decimal,
    pub canceled: Decimal,
    pub cancel_pending: Decimal,
}

#[derive(Debug, Clone)]
pub struct CalcEvent {
    pub event_type: String,
    pub psp_reference: Option<String>,
    pub amount: Decimal,
    pub include_in_calculations: bool,
    /// DB `created_at` (+ id tiebreak): Django orders by `created_at` and
    /// every time-sensitive rule keys off it.
    pub created_at: DateTime<Utc>,
    pub id: i32,
}

#[derive(Debug, Clone, Copy)]
struct Role {
    amount: Decimal,
    at: DateTime<Utc>,
}

#[derive(Debug, Default)]
struct Group {
    request: Option<Role>,
    success: Option<Role>,
    failure: Option<Role>,
    adjustment: Option<Role>,
    back: Option<Role>,
    reverse: Option<Role>,
}

pub fn recalculate(events: &[CalcEvent]) -> Buckets {
    use std::collections::HashMap;
    // Chronological like Django's `.order_by("created_at")`; id breaks ties
    // deterministically (same-millisecond PSP pairs).
    let mut ordered: Vec<&CalcEvent> = events
        .iter()
        .filter(|e| e.include_in_calculations)
        .collect();
    ordered.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));

    // Newest-adjustment cutoff over the whole authorize family
    // (Django `_get_authorize_events`): authorize events older than the
    // newest adjustment are skipped; the adjustment itself always survives.
    // Django does NOT require a psp reference on the adjustment — manual
    // adjustments from transactionUpdate cut too.
    let newest_adj = ordered
        .iter()
        .filter(|e| e.event_type == "authorization_adjustment")
        .map(|e| (e.created_at, e.id))
        .max();
    let live = |e: &CalcEvent| -> bool {
        if e.psp_reference.is_none() {
            return true;
        }
        if family_of(&e.event_type).0 != Family::Authorize {
            return true;
        }
        match newest_adj {
            None => true,
            Some(cut) => {
                if e.event_type == "authorization_adjustment" {
                    (e.created_at, e.id) == cut
                } else {
                    (e.created_at, e.id) > cut
                }
            }
        }
    };

    let mut b = Buckets::default();
    // PSP-less events bypass grouping (`_handle_events_without_psp_reference`).
    for e in &ordered {
        if e.psp_reference.is_none() {
            apply_pspless(&mut b, &e.event_type, e.amount);
        }
    }

    // Grouped events. Deterministic order: chronological by the group's
    // first event (matches Django's first-seen-psp dict order), so
    // multi-group assignment stays stable.
    let mut gmap: HashMap<(Family, String), (Group, DateTime<Utc>, i32)> = HashMap::new();
    for e in &ordered {
        if e.psp_reference.is_none() || !live(e) {
            continue;
        }
        let (family, role) = family_of(&e.event_type);
        if family == Family::Other {
            continue;
        }
        let r = Role { amount: e.amount, at: e.created_at };
        let entry = gmap
            .entry((family, e.psp_reference.clone().unwrap()))
            .or_insert_with(|| (Group::default(), e.created_at, e.id));
        // Last chronological wins per (psp, role): Django dict-overwrite.
        match role {
            "request" => entry.0.request = Some(r),
            "success" => entry.0.success = Some(r),
            "failure" => entry.0.failure = Some(r),
            "adjustment" => entry.0.adjustment = Some(r),
            "back" => entry.0.back = Some(r),
            "reverse" => entry.0.reverse = Some(r),
            _ => {}
        }
    }
    let mut gvec: Vec<_> = gmap.into_iter().collect();
    gvec.sort_by(|a, b| (a.1 .1, a.1 .2).cmp(&(b.1 .1, b.1 .2)));

    for ((family, _), (g, _, _)) in &gvec {
        match family {
            Family::Authorize => {
                if let Some(adj) = g.adjustment {
                    b.authorized = adj.amount;
                }
                base(&mut b, g, "authorize_pending", "authorized", None);
            }
            Family::Charge => {
                if let Some(back) = g.back {
                    b.charged -= back.amount;
                }
                base(&mut b, g, "charge_pending", "charged", Some("authorized"));
            }
            Family::Refund => {
                if let Some(rev) = g.reverse {
                    b.charged += rev.amount;
                    b.refunded -= rev.amount;
                }
                base(&mut b, g, "refund_pending", "refunded", Some("charged"));
            }
            Family::Cancel => {
                base(&mut b, g, "cancel_pending", "canceled", Some("authorized"));
            }
            Family::Other => {}
        }
    }
    b
}

/// App-managed amounts without psp_reference: plain adds, no
/// previous-bucket moves; adjustment assigns.
fn apply_pspless(b: &mut Buckets, event_type: &str, amount: Decimal) {
    match event_type {
        "authorization_success" => b.authorized += amount,
        "authorization_adjustment" => b.authorized = amount,
        "charge_success" => b.charged += amount,
        "charge_back" => b.charged -= amount,
        "refund_success" => b.refunded += amount,
        "refund_reverse" => {
            b.charged += amount;
            b.refunded -= amount;
        }
        "cancel_success" => b.canceled += amount,
        _ => {}
    }
}

fn bucket_mut<'a>(b: &'a mut Buckets, name: &str) -> &'a mut Decimal {
    match name {
        "authorized" => &mut b.authorized,
        "authorize_pending" => &mut b.authorize_pending,
        "charged" => &mut b.charged,
        "charge_pending" => &mut b.charge_pending,
        "refunded" => &mut b.refunded,
        "refund_pending" => &mut b.refund_pending,
        "canceled" => &mut b.canceled,
        "cancel_pending" => &mut b.cancel_pending,
        _ => unreachable!("unknown bucket {name}"),
    }
}

fn base(
    b: &mut Buckets,
    g: &Group,
    pending: &str,
    settled: &str,
    previous: Option<&str>,
) {
    // Pending grows only for a lone request (no success/failure at all).
    if let Some(req) = g.request {
        if g.success.is_none() && g.failure.is_none() {
            *bucket_mut(b, pending) += req.amount;
            if let Some(prev) = previous {
                *bucket_mut(b, prev) -= req.amount;
            }
        }
    }
    // Settled grows on success — unless a NEWER failure vetoes it
    // (`_should_increse_amount`: timestamps decide, not mere presence).
    if let Some(succ) = g.success {
        let vetoed = matches!(g.failure, Some(f) if f.at >= succ.at);
        if !vetoed {
            *bucket_mut(b, settled) += succ.amount;
            if let Some(prev) = previous {
                *bucket_mut(b, prev) -= succ.amount;
            }
        }
    }
}

/// `has_money_movement`: any bucket non-zero.
pub fn has_money_movement(b: &Buckets) -> bool {
    [
        b.authorized,
        b.authorize_pending,
        b.charged,
        b.charge_pending,
        b.refunded,
        b.refund_pending,
        b.canceled,
        b.cancel_pending,
    ]
    .iter()
    .any(|v| !v.is_zero())
}

/// Order-level statuses from totals, mirroring
/// `OrderAuthorizeStatus`/`OrderChargeStatus` evaluation:
/// full when covered, partial when > 0, none otherwise.
pub fn coverage_status(covered: Decimal, total: Decimal) -> &'static str {
    if covered <= Decimal::ZERO {
        "none"
    } else if covered >= total {
        "full"
    } else {
        "partial"
    }
}
