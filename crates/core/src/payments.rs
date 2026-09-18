//! Transaction amount recalculation, exact port of
//! `saleor/payment/transaction_item_calculations.py`.
//!
//! Rules:
//! - Events group by action family (authorize/charge/refund/cancel) ×
//!   psp_reference; only events with `include_in_calculations` count.
//! - `*_request` with no success/failure yet → pending bucket += amount
//!   (and the previous bucket -= amount for charge/refund/cancel).
//! - `*_success` with no failure → settled bucket += amount (same previous
//!   move). Success + failure together → nothing moves.
//! - `authorization_adjustment` overwrites `authorized`.
//! - `charge_back` subtracts `charged`; `refund_reverse` adds `charged`
//!   and subtracts `refunded`.

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
}

#[derive(Debug, Default)]
struct Group {
    request: Option<Decimal>,
    success: Option<Decimal>,
    failure: bool,
    adjustment: Option<Decimal>,
    back: Option<Decimal>,
    reverse: Option<Decimal>,
}

pub fn recalculate(events: &[CalcEvent]) -> Buckets {
    use std::collections::HashMap;
    let mut groups: HashMap<(Family, String), Group> = HashMap::new();
    for e in events.iter().filter(|e| e.include_in_calculations) {
        let (family, role) = family_of(&e.event_type);
        if family == Family::Other {
            continue;
        }
        let key = (family, e.psp_reference.clone().unwrap_or_default());
        let g = groups.entry(key).or_default();
        match role {
            "request" => g.request = Some(e.amount),
            "success" => g.success = Some(e.amount),
            "failure" => g.failure = true,
            "adjustment" => g.adjustment = Some(e.amount),
            "back" => g.back = Some(e.amount),
            "reverse" => g.reverse = Some(e.amount),
            _ => {}
        }
    }

    let mut b = Buckets::default();
    // Django iterates per-psp groups; summation order is irrelevant (pure sums).
    let mut auth_groups: Vec<&Group> = vec![];
    let mut charge_groups: Vec<&Group> = vec![];
    let mut refund_groups: Vec<&Group> = vec![];
    let mut cancel_groups: Vec<&Group> = vec![];
    for ((family, _), g) in &groups {
        match family {
            Family::Authorize => auth_groups.push(g),
            Family::Charge => charge_groups.push(g),
            Family::Refund => refund_groups.push(g),
            Family::Cancel => cancel_groups.push(g),
            Family::Other => {}
        }
    }

    for g in auth_groups {
        if let Some(adj) = g.adjustment {
            b.authorized = adj;
        }
        base(&mut b, g, "authorize_pending", "authorized", None);
    }
    for g in charge_groups {
        if let Some(back) = g.back {
            b.charged -= back;
        }
        base(&mut b, g, "charge_pending", "charged", Some("authorized"));
    }
    for g in refund_groups {
        if let Some(rev) = g.reverse {
            b.charged += rev;
            b.refunded -= rev;
        }
        base(&mut b, g, "refund_pending", "refunded", Some("charged"));
    }
    for g in cancel_groups {
        base(&mut b, g, "cancel_pending", "canceled", Some("authorized"));
    }
    b
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
    // Pending grows only when a lone request exists (no success/failure).
    if let Some(req) = g.request {
        if g.success.is_none() && !g.failure {
            *bucket_mut(b, pending) += req;
            if let Some(prev) = previous {
                *bucket_mut(b, prev) -= req;
            }
        }
    }
    // Settled grows on success-without-failure.
    if let Some(succ) = g.success {
        if !g.failure {
            *bucket_mut(b, settled) += succ;
            if let Some(prev) = previous {
                *bucket_mut(b, prev) -= succ;
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
