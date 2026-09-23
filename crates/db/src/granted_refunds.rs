//! Granted refunds: the DECISION record behind a money refund.
//! (`order_ordergrantedrefund` + `order_ordergrantedrefundline`.)
//!
//! Mirrors `saleor/graphql/order/mutations/order_grant_refund_create.py`
//! and `saleor/order/utils.py::calculate_order_granted_refund_status`:
//! - create validates (amount xor lines/shipping, lines belong to the
//!   order, quantities fit, amount covered by the transaction's charged);
//! - the decision moves NO money by itself; `execute` runs the transaction
//!   refund and links the money event back via
//!   `payment_transactionevent.related_granted_refund_id`;
//! - `status` derives ONLY from the last refund-family event on the linked
//!   transaction (success/pending/failure/none) — never set by hand.
//!
//! This is the missing RC2 piece: decision vs money are separate rows,
//! which unblocks cancel-after-payment (E14) and the return flow (E7).

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};

use crate::{
    entities::{
        order_order, order_ordergrantedrefund, order_ordergrantedrefundline, order_orderline,
        payment_transactionitem,
    },
    payments, webhooks, DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::SeaOrm(sea_orm::DbErr::Custom(msg.into()))
}

pub struct GrantLineInput {
    pub order_line_id: uuid::Uuid,
    pub quantity: i32,
}

pub struct NewGrant {
    pub order_id: uuid::Uuid,
    pub transaction_item_id: Option<i32>,
    pub amount: Option<Decimal>,
    pub lines: Vec<GrantLineInput>,
    pub reason: String,
    pub shipping_costs_included: bool,
    pub user_id: Option<i32>,
    pub app_id: Option<i32>,
}

#[derive(Debug)]
pub struct GrantLineView {
    pub order_line_id: uuid::Uuid,
    pub quantity: i32,
}

#[derive(Debug)]
pub struct GrantView {
    pub id: i32,
    pub order_id: uuid::Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub status: String,
    pub reason: String,
    pub transaction_item_id: Option<i32>,
    pub lines: Vec<GrantLineView>,
}

/// Already-decided (not yet necessarily executed) quantity for one order
/// line, summed over all existing grants. Guards double-deciding the same
/// units; the money-level guard lives in `payments::refund`.
async fn granted_qty_for_line(
    db: &impl sea_orm::ConnectionTrait,
    order_id: uuid::Uuid,
    order_line_id: uuid::Uuid,
) -> Result<i32> {
    let grant_ids: Vec<i32> = order_ordergrantedrefund::Entity::find()
        .select_only()
        .column(order_ordergrantedrefund::Column::Id)
        .filter(order_ordergrantedrefund::Column::OrderId.eq(order_id))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    if grant_ids.is_empty() {
        return Ok(0);
    }
    let qtys: Vec<i32> = order_ordergrantedrefundline::Entity::find()
        .select_only()
        .column(order_ordergrantedrefundline::Column::Quantity)
        .filter(order_ordergrantedrefundline::Column::GrantedRefundId.is_in(grant_ids))
        .filter(order_ordergrantedrefundline::Column::OrderLineId.eq(order_line_id))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    Ok(qtys.into_iter().sum())
}

/// Create the decision row + lines. Moves no money (status starts `none`).
pub async fn create_granted_refund(db: &DatabaseConnection, new: &NewGrant) -> Result<GrantView> {
    let txn = db.begin().await?;
    let id = create_granted_refund_in(&txn, new).await?;
    txn.commit().await?;
    view(db, id).await
}

/// Transactional core of [`create_granted_refund`]: same Saleor validation,
/// no transaction management. Returns the new grant id; read the view with
/// [`view`] on the caller's transaction.
pub async fn create_granted_refund_in(
    txn: &impl sea_orm::ConnectionTrait,
    new: &NewGrant,
) -> Result<i32> {
    let order = order_order::Entity::find_by_id(new.order_id)
        .one(txn)
        .await?
        .ok_or_else(|| fail(format!("order {} not found", new.order_id)))?;

    // Rule 1 (Saleor): at least one of amount / lines / shipping.
    if new.amount.is_none() && new.lines.is_empty() && !new.shipping_costs_included {
        return Err(fail(
            "provide at least one of amount, lines, or shipping_costs_included",
        ));
    }
    if new.reason.is_empty() {
        return Err(fail("reason is required"));
    }

    // Rule 3 (Saleor clean_grant_refund_lines): lines belong to this order
    // and quantities fit the not-yet-granted remainder.
    let mut auto_amount = Decimal::ZERO;
    for l in &new.lines {
        if l.quantity <= 0 {
            return Err(fail("line quantity must be positive"));
        }
        let ol = order_orderline::Entity::find_by_id(l.order_line_id)
            .one(txn)
            .await?
            .ok_or_else(|| fail(format!("order line {} not found", l.order_line_id)))?;
        if ol.order_id != new.order_id {
            return Err(fail(format!(
                "order line {} does not belong to order {}",
                l.order_line_id, new.order_id
            )));
        }
        let already = granted_qty_for_line(txn, new.order_id, l.order_line_id).await?;
        if l.quantity > ol.quantity - already {
            return Err(fail(format!(
                "line {} only has {} grantable units left",
                l.order_line_id,
                ol.quantity - already
            )));
        }
        auto_amount += ol.unit_price_gross_amount * Decimal::from(l.quantity);
    }
    if new.shipping_costs_included {
        auto_amount += order.shipping_price_gross_amount;
    }

    let amount = match new.amount {
        Some(a) => {
            if a <= Decimal::ZERO {
                return Err(fail("amount must be positive"));
            }
            a
        }
        None => {
            if auto_amount <= Decimal::ZERO {
                return Err(fail("calculated amount is zero; provide amount explicitly"));
            }
            auto_amount
        }
    };

    // Rule 2 (Saleor): explicit amount must be covered by charged.
    // (Cross-decision over-grant is stopped at execute by the money guard.)
    if let Some(txn_id) = new.transaction_item_id {
        let t = payment_transactionitem::Entity::find_by_id(txn_id)
            .one(txn)
            .await?
            .ok_or_else(|| fail(format!("transaction {txn_id} not found")))?;
        if t.charged_value < amount {
            return Err(fail(format!(
                "cannot grant {amount}: only {} charged on transaction {txn_id}",
                t.charged_value
            )));
        }
        if t.currency != order.currency {
            return Err(fail("transaction currency must match the order"));
        }
    }

    let t = Utc::now();
    let row = order_ordergrantedrefund::ActiveModel {
        created_at: Set(t.into()),
        updated_at: Set(t.into()),
        amount_value: Set(amount),
        currency: Set(order.currency.clone()),
        reason: Set(new.reason.clone()),
        app_id: Set(new.app_id),
        order_id: Set(new.order_id),
        user_id: Set(new.user_id),
        shipping_costs_included: Set(new.shipping_costs_included),
        transaction_item_id: Set(new.transaction_item_id),
        status: Set("none".to_string()),
        reason_reference_id: Set(None),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    for l in &new.lines {
        order_ordergrantedrefundline::ActiveModel {
            quantity: Set(l.quantity),
            granted_refund_id: Set(row.id),
            order_line_id: Set(l.order_line_id),
            reason: Set(None),
            reason_reference_id: Set(None),
            ..Default::default()
        }
        .insert(txn)
        .await?;
    }
    Ok(row.id)
}

/// Recompute status from the last refund-family event on the linked
/// transaction — exact port of Django's
/// `calculate_order_granted_refund_status`. No event yet → keep `none`.
pub async fn refresh_grant_status(
    db: &impl sea_orm::ConnectionTrait,
    grant_id: i32,
) -> Result<String> {
    use crate::entities::payment_transactionevent;
    let grant = order_ordergrantedrefund::Entity::find_by_id(grant_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("granted refund {grant_id} not found")))?;
    let Some(txn_id) = grant.transaction_item_id else {
        return Ok(grant.status);
    };
    let last = payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(txn_id))
        .filter(
            payment_transactionevent::Column::Type.is_in(vec![
                "refund_request".to_string(),
                "refund_success".to_string(),
                "refund_reverse".to_string(),
                "refund_failure".to_string(),
            ]),
        )
        .order_by_asc(payment_transactionevent::Column::CreatedAt)
        .all(db)
        .await?
        .pop();
    let Some(ev) = last else {
        return Ok(grant.status);
    };
    let next = match ev.r#type.as_str() {
        "refund_success" => "success",
        "refund_request" => "pending",
        "refund_failure" => "failure",
        _ => "none",
    };
    if next != grant.status {
        let mut am: order_ordergrantedrefund::ActiveModel = grant.into();
        am.status = Set(next.to_string());
        am.updated_at = Set(Utc::now().into());
        am.update(db).await?;
    }
    Ok(next.to_string())
}

#[derive(Debug)]
pub struct ExecuteOutcome {
    pub view: GrantView,
    pub replayed: bool,
    pub delivery_ids: Vec<i32>,
}

/// Execute the decision: run the money refund (guarded, idempotent on the
/// key) with the event linked back to this grant, then flip status.
/// Replay (already `success`) returns the view without touching money.
pub async fn execute_granted_refund(
    db: &DatabaseConnection,
    grant_id: i32,
    idempotency_key: &str,
) -> Result<ExecuteOutcome> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let replayed = execute_granted_refund_in(&txn, grant_id, idempotency_key).await?;
    txn.commit().await?;
    let current = view(db, grant_id).await?;
    payments::refresh_order_statuses(db, Some(current.order_id)).await?;
    if replayed {
        return Ok(ExecuteOutcome { view: current, replayed: true, delivery_ids: vec![] });
    }

    // Outbox, same contract as complete/cancel (post-commit send by caller).
    let payload = serde_json::json!({
        "id": current.order_id.to_string(),
        "granted_refund_id": grant_id,
        "status": "success",
    })
    .to_string();
    let delivery_ids = webhooks::trigger_event(db, "order_updated", None, &payload).await?;
    Ok(ExecuteOutcome {
        view: view(db, grant_id).await?,
        replayed: false,
        delivery_ids,
    })
}

/// Transactional core of [`execute_granted_refund`]: guarded money move +
/// status flip, no transaction management, no outbox (the caller emits
/// events on its own transaction). Returns `replayed`.
pub async fn execute_granted_refund_in(
    txn: &impl sea_orm::ConnectionTrait,
    grant_id: i32,
    idempotency_key: &str,
) -> Result<bool> {
    let current = view(txn, grant_id).await?;
    if current.status == "success" {
        return Ok(true);
    }
    let Some(txn_id) = current.transaction_item_id else {
        return Err(fail("granted refund has no transaction linked"));
    };
    payments::refund_in(txn, txn_id, current.amount, idempotency_key, Some(grant_id)).await?;
    refresh_grant_status(txn, grant_id).await?;
    Ok(false)
}

pub async fn view(db: &impl sea_orm::ConnectionTrait, grant_id: i32) -> Result<GrantView> {
    let row = order_ordergrantedrefund::Entity::find_by_id(grant_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("granted refund {grant_id} not found")))?;
    let lines = order_ordergrantedrefundline::Entity::find()
        .filter(order_ordergrantedrefundline::Column::GrantedRefundId.eq(grant_id))
        .all(db)
        .await?;
    Ok(GrantView {
        id: row.id,
        order_id: row.order_id,
        amount: row.amount_value,
        currency: row.currency,
        status: row.status,
        reason: row.reason,
        transaction_item_id: row.transaction_item_id,
        lines: lines
            .into_iter()
            .map(|l| GrantLineView {
                order_line_id: l.order_line_id,
                quantity: l.quantity,
            })
            .collect(),
    })
}

pub struct PaidRefund {
    pub grant_id: i32,
    pub transaction_id: i32,
    pub amount: Decimal,
}

/// Refund `amount` across the order's charged transactions, oldest first.
/// One grant per touched transaction (each a complete decision: amount +
/// link + execution + success status), all inside the caller's transaction.
///
/// Django links a grant to a single transaction and never auto-distributes;
/// distribution here is our paid-cancel/return improvement (staff picks
/// nothing, money still traces per-transaction). Sync manual settlement
/// only: an async PSP would leave the business operation half-done.
pub async fn refund_across_charged_in(
    txn: &impl sea_orm::ConnectionTrait,
    order_id: uuid::Uuid,
    amount: Decimal,
    reason: &str,
    key_prefix: &str,
) -> Result<Vec<PaidRefund>> {
    if amount <= Decimal::ZERO {
        return Err(fail("refund amount must be positive"));
    }
    let txns = payment_transactionitem::Entity::find()
        .filter(payment_transactionitem::Column::OrderId.eq(order_id))
        .order_by_asc(payment_transactionitem::Column::Id)
        .all(txn)
        .await?;
    // Net buckets: `charged_value` is already net of refunds, so it alone
    // is the per-transaction unrefunded remainder.
    let avail: Decimal = txns.iter().map(|t| t.charged_value).sum();
    if avail < amount {
        return Err(fail(format!(
            "cannot refund {amount}: only {avail} charged and unrefunded on order {order_id}"
        )));
    }
    let mut remaining = amount;
    let mut out = Vec::new();
    for t in &txns {
        if remaining <= Decimal::ZERO {
            break;
        }
        let take = t.charged_value.min(remaining);
        if take <= Decimal::ZERO {
            continue;
        }
        let grant_id = create_granted_refund_in(
            txn,
            &NewGrant {
                order_id,
                transaction_item_id: Some(t.id),
                amount: Some(take),
                lines: vec![],
                reason: reason.to_string(),
                shipping_costs_included: false,
                user_id: None,
                app_id: None,
            },
        )
        .await?;
        payments::refund_in(txn, t.id, take, &format!("{key_prefix}-t{}", t.id), Some(grant_id))
            .await?;
        refresh_grant_status(txn, grant_id).await?;
        remaining -= take;
        out.push(PaidRefund { grant_id, transaction_id: t.id, amount: take });
    }
    debug_assert!(remaining <= Decimal::ZERO);
    Ok(out)
}

pub struct UpdateGrant {
    pub amount: Option<Decimal>,
    pub reason: Option<String>,
    pub transaction_item_id: Option<Option<i32>>,
    pub grant_refund_for_shipping: bool,
    pub add_lines: Vec<GrantLineInput>,
    pub remove_line_ids: Vec<i32>,
}

/// Update a granted refund (Django `orderGrantRefundUpdate`):
/// - at least one field required;
/// - status pending/success → only reason editable;
/// - remove_lines are `OrderGrantedRefundLine` pks of THIS grant;
/// - added lines re-run the grantable-remainder guard.
pub async fn update_granted_refund(
    db: &DatabaseConnection,
    grant_id: i32,
    upd: &UpdateGrant,
) -> Result<GrantView> {
    use sea_orm::TransactionTrait;
    let touched = upd.amount.is_some()
        || upd.reason.is_some()
        || upd.transaction_item_id.is_some()
        || upd.grant_refund_for_shipping
        || !upd.add_lines.is_empty()
        || !upd.remove_line_ids.is_empty();
    if !touched {
        return Err(fail("at least one field needs to be provided to process update"));
    }
    let txn = db.begin().await?;
    let g = order_ordergrantedrefund::Entity::find_by_id(grant_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("granted refund {grant_id} not found")))?;
    let locked = g.status == "pending" || g.status == "success";
    if locked && (upd.amount.is_some() || upd.transaction_item_id.is_some() || upd.grant_refund_for_shipping || !upd.add_lines.is_empty() || !upd.remove_line_ids.is_empty()) {
        return Err(fail("only reason can be updated when status is pending or success"));
    }
    let order = order_order::Entity::find_by_id(g.order_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("order not found"))?;
    let mut am: order_ordergrantedrefund::ActiveModel = g.into();
    if let Some(r) = upd.reason.clone() {
        if r.is_empty() {
            return Err(fail("reason cannot be empty"));
        }
        am.reason = Set(r);
    }
    if let Some(a) = upd.amount {
        if a <= Decimal::ZERO {
            return Err(fail("amount must be positive"));
        }
        am.amount_value = Set(a);
    }
    if let Some(t) = upd.transaction_item_id {
        if let Some(tid) = t {
            let row = payment_transactionitem::Entity::find_by_id(tid)
                .one(&txn)
                .await?
                .ok_or_else(|| fail(format!("transaction {tid} not found")))?;
            if row.currency != order.currency {
                return Err(fail("transaction currency must match the order"));
            }
        }
        am.transaction_item_id = Set(t);
    }
    if upd.grant_refund_for_shipping {
        am.shipping_costs_included = Set(true);
    }
    am.updated_at = Set(Utc::now().into());
    am.update(&txn).await?;
    for lid in &upd.remove_line_ids {
        let l = order_ordergrantedrefundline::Entity::find_by_id(*lid)
            .one(&txn)
            .await?
            .ok_or_else(|| fail(format!("granted refund line {lid} not found")))?;
        if l.granted_refund_id != grant_id {
            return Err(fail(format!("line {lid} does not belong to this granted refund")));
        }
        let dam: order_ordergrantedrefundline::ActiveModel = l.into();
        dam.delete(&txn).await?;
    }
    for l in &upd.add_lines {
        if l.quantity <= 0 {
            return Err(fail("line quantity must be positive"));
        }
        let ol = order_orderline::Entity::find_by_id(l.order_line_id)
            .one(&txn)
            .await?
            .ok_or_else(|| fail(format!("order line {} not found", l.order_line_id)))?;
        if ol.order_id != order.id {
            return Err(fail("order line does not belong to this order"));
        }
        let already = granted_qty_for_line(&txn, order.id, l.order_line_id).await?;
        if l.quantity > ol.quantity - already {
            return Err(fail(format!(
                "line {} only has {} grantable units left",
                l.order_line_id,
                ol.quantity - already
            )));
        }
        order_ordergrantedrefundline::ActiveModel {
            quantity: Set(l.quantity),
            granted_refund_id: Set(grant_id),
            order_line_id: Set(l.order_line_id),
            reason: Set(None),
            reason_reference_id: Set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    txn.commit().await?;
    view(db, grant_id).await
}
