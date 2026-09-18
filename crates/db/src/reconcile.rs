//! Order reconciliation (audit RC1–RC10): verify money, stock, and audit
//! invariants for one order and report each check. Read-only — it never
//! mutates, so it is safe to run on a schedule or on demand.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect, SelectorTrait};
use uuid::Uuid;

use crate::{
    entities::{
        discount_vouchercode, giftcard_giftcard, order_order, order_order_gift_cards,
        order_orderevent, order_orderline, payment_transactionitem, warehouse_allocation,
    },
    DbError, Result,
};

#[derive(Debug)]
pub struct ReconCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

fn check(name: &str, ok: bool, detail: impl Into<String>) -> ReconCheck {
    ReconCheck { name: name.into(), ok, detail: detail.into() }
}

/// Reconcile one order. Every check is independent — all run even when
/// earlier ones fail, so one call gives the full picture.
pub async fn reconcile_order(
    db: &impl sea_orm::ConnectionTrait,
    order_id: Uuid,
) -> Result<Vec<ReconCheck>> {
    let mut out = Vec::new();

    // Header (slim — no tsvector/interval).
    let header: Option<(String, Decimal, String, Option<String>, String)> =
        order_order::Entity::find_by_id(order_id)
            .select_only()
            .column(order_order::Column::Status)
            .column(order_order::Column::TotalGrossAmount)
            .column(order_order::Column::Currency)
            .column(order_order::Column::VoucherCode)
            .column(order_order::Column::CheckoutToken)
            .into_tuple()
            .one(db)
            .await?;
    let Some((status, total, currency, voucher_code, checkout_token)) = header else {
        return Err(DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(format!(
            "order {order_id}"
        ))));
    };

    let lines = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .all(db)
        .await?;

    // RC1: charged vs total.
    let txns = payment_transactionitem::Entity::find()
        .filter(payment_transactionitem::Column::OrderId.eq(order_id))
        .all(db)
        .await?;
    let charged: Decimal = txns.iter().map(|t| t.charged_value).sum();
    let refunded: Decimal = txns.iter().map(|t| t.refunded_value).sum();
    out.push(check(
        "charged_vs_total",
        charged - refunded <= total,
        format!("charged={charged} refunded={refunded} total={total} {currency}"),
    ));

    // RC4: fulfilled vs lines.
    let bad_fulfilled: Vec<String> = lines
        .iter()
        .filter(|l| l.quantity_fulfilled > l.quantity)
        .map(|l| l.id.to_string())
        .collect();
    out.push(check(
        "fulfilled_vs_lines",
        bad_fulfilled.is_empty(),
        if bad_fulfilled.is_empty() {
            format!("{} lines ok", lines.len())
        } else {
            format!("over-fulfilled: {}", bad_fulfilled.join(","))
        },
    ));

    // RC3: allocated vs lines.
    let mut bad_alloc = vec![];
    for l in &lines {
        let alloced: i32 = warehouse_allocation::Entity::find()
            .filter(warehouse_allocation::Column::OrderLineId.eq(l.id))
            .all(db)
            .await?
            .iter()
            .map(|a| a.quantity_allocated)
            .sum();
        if alloced > l.quantity {
            bad_alloc.push(format!("{}:{alloced}>{}", l.id, l.quantity));
        }
    }
    out.push(check(
        "allocated_vs_lines",
        bad_alloc.is_empty(),
        if bad_alloc.is_empty() { "allocations within quantities".into() } else { bad_alloc.join(",") },
    ));

    // RC5: voucher usage recorded.
    if let Some(code) = voucher_code.filter(|c| !c.is_empty()) {
        let used: Option<i32> = discount_vouchercode::Entity::find()
            .select_only()
            .column(discount_vouchercode::Column::Used)
            .filter(discount_vouchercode::Column::Code.eq(&code))
            .into_tuple()
            .one(db)
            .await?;
        out.push(check(
            "voucher_usage",
            used.map(|u| u > 0).unwrap_or(false),
            format!("code={code} used={}", used.map(|u| u.to_string()).unwrap_or("?".into())),
        ));
    } else {
        out.push(check("voucher_usage", true, "no voucher on order"));
    }

    // RC6: linked gift cards never negative.
    let card_ids: Vec<i32> = order_order_gift_cards::Entity::find()
        .select_only()
        .column(order_order_gift_cards::Column::GiftcardId)
        .filter(order_order_gift_cards::Column::OrderId.eq(order_id))
        .into_tuple()
        .all(db)
        .await?;
    let mut negative = vec![];
    for cid in &card_ids {
        let bal: Option<Decimal> = giftcard_giftcard::Entity::find_by_id(*cid)
            .select_only()
            .column(giftcard_giftcard::Column::CurrentBalanceAmount)
            .into_tuple()
            .one(db)
            .await?;
        if bal.map(|b| b < Decimal::ZERO).unwrap_or(false) {
            negative.push(cid.to_string());
        }
    }
    out.push(check(
        "giftcard_balances",
        negative.is_empty(),
        if negative.is_empty() {
            format!("{} linked cards ok", card_ids.len())
        } else {
            format!("negative: {}", negative.join(","))
        },
    ));

    // RC7: payments link to this order and agree on currency.
    let bad_cur = txns.iter().filter(|t| t.currency != currency).count();
    out.push(check(
        "payments_linked",
        bad_cur == 0,
        format!("{} transactions, {bad_cur} currency mismatches", txns.len()),
    ));

    // RC8: event trail starts with PLACED.
    let has_placed = order_orderevent::Entity::find()
        .filter(order_orderevent::Column::OrderId.eq(order_id))
        .filter(order_orderevent::Column::Type.eq("placed"))
        .one(db)
        .await?
        .is_some();
    out.push(check(
        "event_trail",
        has_placed,
        if has_placed { format!("placed + status={status}") } else { "no placed event".into() },
    ));

    // RC9: single completion per checkout token (idempotency record).
    if checkout_token.is_empty() {
        out.push(check("single_completion", true, "no checkout token (draft origin)"));
    } else {
        let count: u64 = order_order::Entity::find()
            .select_only()
            .column(order_order::Column::Id)
            .filter(order_order::Column::CheckoutToken.eq(&checkout_token))
            .into_tuple::<Uuid>()
            .all(db)
            .await?
            .len() as u64;
        out.push(check(
            "single_completion",
            count == 1,
            format!("{count} orders for token {checkout_token}"),
        ));
    }

    // RC2/RC10: refunded never exceeds charged (granted-refund decision
    // parity until the granted-refund milestone lands).
    out.push(check(
        "refunded_within_charged",
        refunded <= charged,
        format!("refunded={refunded} charged={charged}"),
    ));

    Ok(out)
}
