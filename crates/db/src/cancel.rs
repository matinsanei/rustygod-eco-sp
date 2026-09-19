//! Order cancellation: unpaid AND paid (reviewer priority #2, T2).
//!
//! SHARED LOCK CONTRACT (see complete.rs): order row `FOR UPDATE` first,
//! then stock rows ascending by id. Same order as completion — concurrent
//! complete/cancel on one variant cannot deadlock.
//!
//! Eligibility mirrors `Order.can_cancel()`: no ACTIVE fulfillment (only
//! canceled/refunded/replaced/returned/refunded_and_returned may exist)
//! and status not in {canceled, draft, expired}.
//!
//! Two deliberate Saleor parities worth knowing:
//! - cancel-after-PARTIAL-return is refused while any `fulfilled`
//!   fulfillment row exists (Django's gate is row-status based, not
//!   quantity based) — return the rest instead; a fully `returned` order
//!   is terminal and needs no cancel;
//! - paid cancel goes FURTHER than Django (which leaves captured funds
//!   sitting): it auto-refunds via granted refunds, atomically.
//!
//! Money: Saleor's cancel deallocates and flips status but leaves captured
//! funds sitting — the refund is a separate staff flow. Here paid cancel
//! completes the loop atomically: one granted-refund decision per charged
//! transaction (full unrefunded remainder each) + sync manual settlement +
//! allocation release + `canceled`, ALL ONE transaction. Sync-only by
//! design: an async PSP would leave a canceled order with money pending.
//!
//! Effects, all in ONE transaction: granted refunds (paid only),
//! allocations released, status → `canceled`, `canceled` event, outbox
//! deliveries (`order_cancelled` + `order_updated`) for post-commit sending.

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
    sea_query::LockType,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        order_fulfillment, order_order, order_orderline, order_orderevent,
        payment_transactionitem, warehouse_allocation, warehouse_stock,
    },
    granted_refunds, payments, webhooks,
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Cancel(msg.into())
}

#[derive(Debug)]
pub struct CancelOutcome {
    pub order_id: Uuid,
    pub refunded: Decimal,
    pub grant_ids: Vec<i32>,
    pub delivery_ids: Vec<i32>,
}

/// Fulfillment statuses that don't block cancellation — exact port of
/// `Order.can_cancel()`'s allow-list. Anything else (notably `fulfilled`)
/// means goods are out the door: cancel is refused, return them instead.
fn blocks_cancel(status: &str) -> bool {
    !matches!(
        status,
        "canceled" | "refunded" | "replaced" | "returned" | "refunded_and_returned"
    )
}

pub async fn cancel_order(
    db: &DatabaseConnection,
    order_id: Uuid,
) -> Result<CancelOutcome> {
    let txn = db.begin().await?;
    // 1. Lock the order row (shared contract: order first).
    let order = order_order::Entity::find_by_id(order_id)
        .lock(LockType::Update)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("order {order_id} not found")))?;
    // Django's exact rule: only canceled/draft/expired are ineligible by
    // status. Everything else (unfulfilled, fulfilled, partially_*,
    // returned...) goes to the fulfillment gate below, which is the real
    // guard: goods out the door can't be canceled, only returned.
    match order.status.as_str() {
        "canceled" | "draft" | "expired" => {
            return Err(fail(format!("order in status {} cannot be cancelled", order.status)))
        }
        _ => {}
    }
    // can_cancel port: active fulfillments block cancellation.
    let blocking = order_fulfillment::Entity::find()
        .filter(order_fulfillment::Column::OrderId.eq(order_id))
        .all(&txn)
        .await?
        .into_iter()
        .any(|f| blocks_cancel(&f.status));
    if blocking {
        return Err(fail(
            "order has active fulfillments; return the goods instead of cancelling",
        ));
    }

    // 2. Paid path: refund every unrefunded remainder, one grant per
    // charged transaction, inside THIS transaction (atomic with the rest).
    let txns = payment_transactionitem::Entity::find()
        .filter(payment_transactionitem::Column::OrderId.eq(order_id))
        .all(&txn)
        .await?;
    // Net buckets: `charged_value` is already net of refunds.
    let captured: Decimal = txns.iter().map(|t| t.charged_value).sum();
    let mut grant_ids = vec![];
    let mut refunded = Decimal::ZERO;
    if captured > Decimal::ZERO {
        let paid: Vec<granted_refunds::PaidRefund> = granted_refunds::refund_across_charged_in(
            &txn,
            order_id,
            captured,
            "order cancelled",
            &format!("cancel-{order_id}"),
        )
        .await?;
        for p in &paid {
            refunded += p.amount;
            grant_ids.push(p.grant_id);
        }
        payments::refresh_order_statuses(&txn, Some(order_id)).await?;
    }

    // 3. Release allocations, stock rows locked ascending (shared contract).
    let line_ids: Vec<Uuid> = order_orderline::Entity::find()
        .select_only()
        .column(order_orderline::Column::Id)
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .into_tuple()
        .all(&txn)
        .await?;
    if !line_ids.is_empty() {
        let allocs = warehouse_allocation::Entity::find()
            .filter(warehouse_allocation::Column::OrderLineId.is_in(line_ids))
            .all(&txn)
            .await?;
        let mut stock_ids: Vec<i32> = allocs.iter().map(|a| a.stock_id).collect();
        stock_ids.sort_unstable();
        stock_ids.dedup();
        let mut locked = std::collections::HashMap::new();
        for sid in stock_ids {
            if let Some(s) = warehouse_stock::Entity::find_by_id(sid)
                .lock(LockType::Update)
                .one(&txn)
                .await?
            {
                locked.insert(sid, s);
            }
        }
        for a in &allocs {
            if let Some(s) = locked.get(&a.stock_id) {
                let mut sam: warehouse_stock::ActiveModel = s.clone().into();
                sam.quantity_allocated =
                    Set((s.quantity_allocated - a.quantity_allocated).max(0));
                sam.update(&txn).await?;
            }
            warehouse_allocation::Entity::delete_by_id(a.id).exec(&txn).await?;
        }
    }

    // 4. Flip status + event, atomically.
    let channel_id = order.channel_id;
    let mut am: order_order::ActiveModel = order.into();
    am.status = Set("canceled".to_string());
    am.updated_at = Set(Utc::now().into());
    am.update(&txn).await?;
    order_orderevent::ActiveModel {
        date: Set(Utc::now().into()),
        r#type: Set("canceled".to_string()),
        parameters: Set(json!({})),
        order_id: Set(order_id),
        ..Default::default()
    }
    .insert(&txn)
    .await?;

    // 5. Outbox in-transaction (R8, same as complete): Saleor emits both
    // ORDER_CANCELLED and ORDER_UPDATED on cancel.
    let payload = json!({
        "id": order_id.to_string(),
        "refunded": refunded.to_string(),
    })
    .to_string();
    let ch_slug = crate::catalog::channel_slug_for_id(&txn, channel_id).await.ok();
    let mut delivery_ids =
        webhooks::trigger_event_tx(&txn, "order_cancelled", ch_slug.as_deref(), &payload)
            .await
            .unwrap_or_default();
    delivery_ids.extend(
        webhooks::trigger_event_tx(&txn, "order_updated", ch_slug.as_deref(), &payload)
            .await
            .unwrap_or_default(),
    );

    txn.commit().await?;
    Ok(CancelOutcome { order_id, refunded, grant_ids, delivery_ids })
}
