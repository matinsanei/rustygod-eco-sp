//! Order cancellation for unpaid orders (reviewer priority #2).
//!
//! SHARED LOCK CONTRACT (see complete.rs): order row `FOR UPDATE` first,
//! then stock rows ascending by id. Same order as completion — concurrent
//! complete/cancel on one variant cannot deadlock.
//!
//! Scope is deliberately unpaid-only: paid orders need the refund flow
//! (granted refunds + PSP), which lands with payment orchestration —
//! cancelling those here would silently eat money. Paid input gets a clean
//! `REQUIRES_REFUND` error, never a partial cancel.
//!
//! Effects, all in ONE transaction: allocations released (quantity_allocated
//! rolled back), status → `canceled`, `canceled` event, outbox deliveries
//! (`order_cancelled`) for post-commit sending.

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
        order_order, order_orderline, order_orderevent, payment_transactionitem,
        warehouse_allocation, warehouse_stock,
    },
    webhooks,
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Cancel(msg.into())
}

#[derive(Debug)]
pub struct CancelOutcome {
    pub order_id: Uuid,
    pub delivery_ids: Vec<i32>,
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
    match order.status.as_str() {
        "unfulfilled" | "unconfirmed" => {}
        s => return Err(fail(format!("order in status {s} cannot be cancelled"))),
    }

    // 2. Paid guard: any charged value routes to the refund flow.
    let charged: Decimal = payment_transactionitem::Entity::find()
        .select_only()
        .column(payment_transactionitem::Column::ChargedValue)
        .filter(payment_transactionitem::Column::OrderId.eq(order_id))
        .into_tuple::<Decimal>()
        .all(&txn)
        .await?
        .into_iter()
        .sum();
    if charged > Decimal::ZERO {
        return Err(fail(
            "order has captured payments; use the refund flow (REQUIRES_REFUND)",
        ));
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

    // 5. Outbox in-transaction (R8, same as complete).
    let payload = json!({"id": order_id.to_string()}).to_string();
    let ch_slug = crate::catalog::channel_slug_for_id(&txn, channel_id).await.ok();
    let delivery_ids =
        webhooks::trigger_event_tx(&txn, "order_cancelled", ch_slug.as_deref(), &payload)
            .await
            .unwrap_or_default();

    txn.commit().await?;
    Ok(CancelOutcome { order_id, delivery_ids })
}
