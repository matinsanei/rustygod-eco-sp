//! Fulfillment lifecycle over Django's tables
//! (`order_fulfillment`, `order_fulfillmentline`).
//!
//! Mirrors `saleor/order/actions.py` + `saleor/order/utils.py` +
//! `saleor/warehouse/management.py::decrease_stock`:
//! - `fulfillment_order` auto-increments per order (1, 2, 3…);
//! - lines may not exceed the unfulfilled remainder;
//! - fulfilling decreases warehouse stock, canceling/refunding restores it;
//! - order status refreshes from fulfillment quantities
//!   (`determine_order_status` — exact Django branches).
//! - everything runs inside one transaction per operation.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        order_fulfillment, order_fulfillmentline, order_order, order_orderline,
        product_productvariant, warehouse_stock,
    },
    DbError, Result,
};

/// Track-inventory flag for the variants on these order lines (one query).
/// Missing variants default to tracked (safe side: stock still moves).
async fn tracked_map(
    db: &impl sea_orm::ConnectionTrait,
    variant_ids: &[i32],
) -> Result<std::collections::HashMap<i32, bool>> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    if variant_ids.is_empty() {
        return Ok(Default::default());
    }
    Ok(product_productvariant::Entity::find()
        .select_only()
        .column(product_productvariant::Column::Id)
        .column(product_productvariant::Column::TrackInventory)
        .filter(product_productvariant::Column::Id.is_in(variant_ids.to_vec()))
        .into_tuple::<(i32, bool)>()
        .all(db)
        .await?
        .into_iter()
        .collect())
}

fn fail(msg: impl Into<String>) -> DbError {
    DbError::SeaOrm(sea_orm::DbErr::Custom(msg.into()))
}

pub struct FulfillItem {
    pub order_line_id: Uuid,
    pub quantity: i32,
    pub stock_id: Option<i32>,
}

pub struct FulfillmentView {
    pub id: i32,
    pub order_id: Uuid,
    pub fulfillment_order: i32,
    pub status: String,
    pub tracking_number: String,
    pub lines: Vec<FulfillmentLineView>,
}

pub struct FulfillmentLineView {
    pub order_line_id: Uuid,
    pub quantity: i32,
    pub stock_id: Option<i32>,
}

/// Next `fulfillment_order` for the order (max + 1, Django `save()` logic).
async fn next_fulfillment_order(
    db: &impl sea_orm::ConnectionTrait,
    order_id: Uuid,
) -> Result<i32> {
    let max: Option<i32> = order_fulfillment::Entity::find()
        .select_only()
        .column(order_fulfillment::Column::FulfillmentOrder)
        .filter(order_fulfillment::Column::OrderId.eq(order_id))
        .order_by_desc(order_fulfillment::Column::FulfillmentOrder)
        .into_tuple()
        .one(db)
        .await?;
    Ok(max.map(|m| m + 1).unwrap_or(1))
}

/// Quantity fulfilled: lines in `fulfilled` fulfillments only (refunds and
/// cancels never count — Django tracks this on the order line counter,
/// incremented solely by fulfilling).
async fn fulfilled_qty(
    db: &impl sea_orm::ConnectionTrait,
    order_line_id: Uuid,
) -> Result<i32> {
    let rows = order_fulfillmentline::Entity::find()
        .filter(order_fulfillmentline::Column::OrderLineId.eq(order_line_id))
        .all(db)
        .await?;
    let mut total = 0;
    for fl in rows {
        let f = order_fulfillment::Entity::find_by_id(fl.fulfillment_id)
            .one(db)
            .await?
            .ok_or_else(|| fail("fulfillment row missing"))?;
        if f.status == "fulfilled" {
            total += fl.quantity;
        }
    }
    Ok(total)
}

/// Pick the stock with most free quantity for a variant (v1 allocation:
/// Django pre-allocates at order creation; we resolve at fulfill time and
/// note the allocation-table milestone in the docs).
async fn pick_stock(
    db: &impl sea_orm::ConnectionTrait,
    variant_id: i32,
    quantity: i32,
) -> Result<i32> {
    let stocks = warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::ProductVariantId.eq(variant_id))
        .all(db)
        .await?;
    let mut best: Option<&warehouse_stock::Model> = None;
    for s in &stocks {
        let free = s.quantity - s.quantity_allocated;
        if free >= quantity && best.map(|b| free > b.quantity - b.quantity_allocated).unwrap_or(true) {
            best = Some(s);
        }
    }
    best
        .map(|s| s.id)
        .ok_or_else(|| fail(format!("insufficient stock for variant {variant_id}")))
}

/// Create a `fulfilled` fulfillment: validates remainders, writes rows,
/// bumps `quantity_fulfilled`, decreases stock — one transaction.
pub async fn create_fulfillment(
    db: &DatabaseConnection,
    order_id: Uuid,
    items: &[FulfillItem],
    tracking_number: &str,
) -> Result<FulfillmentView> {
    use sea_orm::TransactionTrait;
    if items.is_empty() {
        return Err(fail("fulfillment needs at least one line"));
    }
    let txn = db.begin().await?;
    // Order must exist and be fulfillable.
    let order = order_order::Entity::find_by_id(order_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("order not found"))?;
    if order.status == "canceled" || order.status == "draft" {
        return Err(fail(format!("order in status {} cannot be fulfilled", order.status)));
    }
    let seq = next_fulfillment_order(&txn, order_id).await?;
    let f = order_fulfillment::ActiveModel {
        fulfillment_order: Set(seq),
        order_id: Set(order_id),
        status: Set("fulfilled".to_string()),
        tracking_number: Set(tracking_number.to_string()),
        created_at: Set(Utc::now().into()),
        reason: Set(String::new()),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        ..Default::default()
    }
    .insert(&txn)
    .await?;

    // Pre-load lines + track flags (Q8: untracked variants move no stock).
    let mut line_by_id = std::collections::HashMap::new();
    for item in items {
        if item.quantity < 1 {
            return Err(fail("fulfillment quantity must be positive"));
        }
        let line = order_orderline::Entity::find_by_id(item.order_line_id)
            .one(&txn)
            .await?
            .ok_or_else(|| fail("order line not found"))?;
        if line.order_id != order_id {
            return Err(fail("order line does not belong to this order"));
        }
        line_by_id.insert(item.order_line_id, line);
    }
    let vids: Vec<i32> = line_by_id.values().filter_map(|l| l.variant_id).collect();
    let tracked = tracked_map(&txn, &vids).await?;

    for item in items {
        let line = line_by_id.remove(&item.order_line_id).expect("pre-loaded");
        let is_tracked = line
            .variant_id
            .map(|v| tracked.get(&v).copied().unwrap_or(true))
            .unwrap_or(false);
        let done = fulfilled_qty(&txn, item.order_line_id).await?;
        if done + item.quantity > line.quantity {
            return Err(fail(format!(
                "cannot fulfill {}: only {} of {} remaining",
                item.quantity,
                line.quantity - done,
                line.quantity
            )));
        }
        // Resolve stock: explicit or auto-pick. Untracked variants (digital
        // goods) skip stock entirely — Q8, the classic Saleor bug.
        let stock_id = if !is_tracked {
            None
        } else {
            Some(match item.stock_id {
                Some(sid) => {
                    let s = warehouse_stock::Entity::find_by_id(sid)
                        .one(&txn)
                        .await?
                        .ok_or_else(|| fail("stock not found"))?;
                    if s.quantity - s.quantity_allocated < item.quantity {
                        return Err(fail("insufficient stock in warehouse"));
                    }
                    sid
                }
                None => {
                    let vid = line.variant_id.ok_or_else(|| fail("line has no variant"))?;
                    pick_stock(&txn, vid, item.quantity).await?
                }
            })
        };
        order_fulfillmentline::ActiveModel {
            order_line_id: Set(item.order_line_id),
            fulfillment_id: Set(f.id),
            quantity: Set(item.quantity),
            stock_id: Set(stock_id),
            reason: Set(String::new()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        // Bump fulfilled; decrease stock only for tracked variants
        // (Django decrease_stock semantics).
        let mut lam: order_orderline::ActiveModel = line.into();
        lam.quantity_fulfilled = Set(lam.quantity_fulfilled.clone().unwrap() + item.quantity);
        lam.update(&txn).await?;
        if let Some(sid) = stock_id {
            decrease_stock_qty(&txn, sid, item.quantity).await?;
        }
    }
    refresh_order_status(&txn, order_id).await?;
    txn.commit().await?;
    view(db, f.id).await
}

async fn decrease_stock_qty(
    db: &impl sea_orm::ConnectionTrait,
    stock_id: i32,
    quantity: i32,
) -> Result<()> {
    let s = warehouse_stock::Entity::find_by_id(stock_id)
        .one(db)
        .await?
        .ok_or_else(|| fail("stock not found"))?;
    if s.quantity - s.quantity_allocated < quantity {
        return Err(fail("insufficient stock"));
    }
    let mut am: warehouse_stock::ActiveModel = s.into();
    am.quantity = Set(am.quantity.clone().unwrap() - quantity);
    am.update(db).await?;
    Ok(())
}

async fn increase_stock_qty(
    db: &impl sea_orm::ConnectionTrait,
    stock_id: i32,
    quantity: i32,
) -> Result<()> {
    let s = warehouse_stock::Entity::find_by_id(stock_id)
        .one(db)
        .await?
        .ok_or_else(|| fail("stock not found"))?;
    let mut am: warehouse_stock::ActiveModel = s.into();
    am.quantity = Set(am.quantity.clone().unwrap() + quantity);
    am.update(db).await?;
    Ok(())
}

/// Cancel a fulfillment: status → canceled, stock restored, line counters
/// rolled back, order status refreshed. Mirrors Django's cancel_fulfillment.
pub async fn cancel_fulfillment(
    db: &DatabaseConnection,
    fulfillment_id: i32,
) -> Result<FulfillmentView> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let f = order_fulfillment::Entity::find_by_id(fulfillment_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("fulfillment not found"))?;
    if f.status == "canceled" {
        txn.commit().await?;
        return view(db, fulfillment_id).await;
    }
    let order_id = f.order_id;
    let lines = order_fulfillmentline::Entity::find()
        .filter(order_fulfillmentline::Column::FulfillmentId.eq(fulfillment_id))
        .all(&txn)
        .await?;
    for fl in &lines {
        let line = order_orderline::Entity::find_by_id(fl.order_line_id)
            .one(&txn)
            .await?
            .ok_or_else(|| fail("order line not found"))?;
        let mut lam: order_orderline::ActiveModel = line.into();
        lam.quantity_fulfilled = Set((lam.quantity_fulfilled.clone().unwrap() - fl.quantity).max(0));
        lam.update(&txn).await?;
        if let Some(sid) = fl.stock_id {
            increase_stock_qty(&txn, sid, fl.quantity).await?;
        }
    }
    let mut fam: order_fulfillment::ActiveModel = f.into();
    fam.status = Set("canceled".to_string());
    fam.update(&txn).await?;
    refresh_order_status(&txn, order_id).await?;
    txn.commit().await?;
    view(db, fulfillment_id).await
}

/// Refund/return lines: creates a `refunded` fulfillment, restores stock,
/// refreshes status (→ returned / partially_returned via Django branches).
pub async fn refund_fulfillment(
    db: &DatabaseConnection,
    order_id: Uuid,
    items: &[FulfillItem],
    reason: &str,
) -> Result<FulfillmentView> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let id = refund_fulfillment_in(&txn, order_id, items, reason, true).await?;
    txn.commit().await?;
    view(db, id).await
}

/// Transactional core of [`refund_fulfillment`]: same validation, no
/// transaction management. `restock=false` books the return without
/// putting units back on the shelf (damaged/lost goods still leave
/// `quantity_fulfilled` accounting to the caller — Django's return flow
/// separates the stock move from the money the same way).
pub async fn refund_fulfillment_in(
    txn: &impl sea_orm::ConnectionTrait,
    order_id: Uuid,
    items: &[FulfillItem],
    reason: &str,
    restock: bool,
) -> Result<i32> {
    if items.is_empty() {
        return Err(fail("refund needs at least one line"));
    }
    order_order::Entity::find_by_id(order_id)
        .one(txn)
        .await?
        .ok_or_else(|| fail("order not found"))?;
    let seq = next_fulfillment_order(txn, order_id).await?;
    let f = order_fulfillment::ActiveModel {
        fulfillment_order: Set(seq),
        order_id: Set(order_id),
        status: Set("refunded".to_string()),
        tracking_number: Set(String::new()),
        created_at: Set(Utc::now().into()),
        reason: Set(reason.to_string()),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    for item in items {
        if item.quantity < 1 {
            return Err(fail("refund quantity must be positive"));
        }
        let line = order_orderline::Entity::find_by_id(item.order_line_id)
            .one(txn)
            .await?
            .ok_or_else(|| fail("order line not found"))?;
        if line.order_id != order_id {
            return Err(fail("order line does not belong to this order"));
        }
        // Refundable = fulfilled (non-canceled) minus already returned.
        let done = fulfilled_qty(txn, item.order_line_id).await?;
        let returned = returned_qty(txn, item.order_line_id).await?;
        if done - returned < item.quantity {
            return Err(fail(format!(
                "cannot refund {}: only {} fulfilled and unreturned",
                item.quantity,
                done - returned
            )));
        }
        // Default to the fulfilled lines' stock (Django restores the stock
        // the units were taken from).
        let stock_id = match item.stock_id {
            Some(sid) => Some(sid),
            None => stock_of_fulfilled_line(txn, item.order_line_id).await?,
        };
        order_fulfillmentline::ActiveModel {
            order_line_id: Set(item.order_line_id),
            fulfillment_id: Set(f.id),
            quantity: Set(item.quantity),
            stock_id: Set(stock_id),
            reason: Set(reason.to_string()),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        // Damaged/lost returns (restock=false) book the return without
        // putting units back on the shelf.
        if restock {
            if let Some(sid) = stock_id {
                increase_stock_qty(txn, sid, item.quantity).await?;
            }
        }
    }
    refresh_order_status(txn, order_id).await?;
    Ok(f.id)
}

/// Stock the units were fulfilled from (latest non-canceled fulfillment
/// line carrying a stock id).
async fn stock_of_fulfilled_line(
    db: &impl sea_orm::ConnectionTrait,
    order_line_id: Uuid,
) -> Result<Option<i32>> {
    let rows = order_fulfillmentline::Entity::find()
        .filter(order_fulfillmentline::Column::OrderLineId.eq(order_line_id))
        .order_by_desc(order_fulfillmentline::Column::Id)
        .all(db)
        .await?;
    for fl in rows {
        if fl.stock_id.is_none() {
            continue;
        }
        let f = order_fulfillment::Entity::find_by_id(fl.fulfillment_id)
            .one(db)
            .await?
            .ok_or_else(|| fail("fulfillment row missing"))?;
        if f.status != "canceled" {
            return Ok(fl.stock_id);
        }
    }
    Ok(None)
}

async fn returned_qty(    db: &impl sea_orm::ConnectionTrait,
    order_line_id: Uuid,
) -> Result<i32> {
    let rows = order_fulfillmentline::Entity::find()
        .filter(order_fulfillmentline::Column::OrderLineId.eq(order_line_id))
        .all(db)
        .await?;
    let mut total = 0;
    for fl in rows {
        let f = order_fulfillment::Entity::find_by_id(fl.fulfillment_id)
            .one(db)
            .await?
            .ok_or_else(|| fail("fulfillment row missing"))?;
        if f.status == "refunded" || f.status == "returned" || f.status == "refunded_and_returned" {
            total += fl.quantity;
        }
    }
    Ok(total)
}

/// Exact port of `determine_order_status` (v1 scope: no replacements /
/// awaiting-approval variants — approval flow is its own milestone).
pub fn determine_status(total: i32, fulfilled: i32, returned: i32) -> &'static str {
    if fulfilled <= 0 {
        "unfulfilled"
    } else if 0 < returned && returned < total {
        "partially_returned"
    } else if returned == total && total > 0 {
        "returned"
    } else if fulfilled < total {
        "partially fulfilled"
    } else {
        "fulfilled"
    }
}

pub async fn refresh_order_status(
    db: &impl sea_orm::ConnectionTrait,
    order_id: Uuid,
) -> Result<String> {
    let lines = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .all(db)
        .await?;
    let total: i32 = lines.iter().map(|l| l.quantity).sum();
    let mut fulfilled = 0;
    let mut returned = 0;
    for l in &lines {
        fulfilled += fulfilled_qty(db, l.id).await?;
        returned += returned_qty(db, l.id).await?;
    }
    let status = determine_status(total, fulfilled, returned).to_string();
    if let Some(o) = order_order::Entity::find_by_id(order_id).one(db).await? {
        let mut am: order_order::ActiveModel = o.into();
        am.status = Set(status.clone());
        am.update(db).await?;
    }
    Ok(status)
}

pub async fn view(db: &impl sea_orm::ConnectionTrait, fulfillment_id: i32) -> Result<FulfillmentView> {
    let f = order_fulfillment::Entity::find_by_id(fulfillment_id)
        .one(db)
        .await?
        .ok_or_else(|| fail("fulfillment not found"))?;
    let lines = order_fulfillmentline::Entity::find()
        .filter(order_fulfillmentline::Column::FulfillmentId.eq(fulfillment_id))
        .all(db)
        .await?;
    Ok(FulfillmentView {
        id: f.id,
        order_id: f.order_id,
        fulfillment_order: f.fulfillment_order,
        status: f.status,
        tracking_number: f.tracking_number,
        lines: lines
            .into_iter()
            .map(|l| FulfillmentLineView {
                order_line_id: l.order_line_id,
                quantity: l.quantity,
                stock_id: l.stock_id,
            })
            .collect(),
    })
}

pub async fn list_fulfillments(
    db: &impl sea_orm::ConnectionTrait,
    order_id: Uuid,
) -> Result<Vec<FulfillmentView>> {
    let rows = order_fulfillment::Entity::find()
        .filter(order_fulfillment::Column::OrderId.eq(order_id))
        .order_by_asc(order_fulfillment::Column::FulfillmentOrder)
        .all(db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(view(db, r.id).await?);
    }
    Ok(out)
}

#[derive(Debug)]
pub struct ReturnRefundView {
    pub fulfillment_id: i32,
    pub granted_refund_id: i32,
    pub amount: rust_decimal::Decimal,
}

/// Return lines AND refund their money, atomically: a `refunded`
/// fulfillment (optional restock) + a line-based granted refund (amount
/// derived from unit gross × qty) executed on one transaction.
///
/// Django links a grant to a single transaction and leaves multi-payment
/// orders to staff judgment — same here: pass `transaction_item_id`
/// explicitly when the order has several charged transactions, otherwise
/// the sole charged one is used (error if there isn't exactly one).
pub async fn return_and_refund(
    db: &DatabaseConnection,
    order_id: Uuid,
    items: &[FulfillItem],
    reason: &str,
    restock: bool,
    transaction_item_id: Option<i32>,
) -> Result<ReturnRefundView> {
    return_and_refund_full(db, order_id, items, reason, restock, transaction_item_id, false, None).await
}

/// Full variant: `include_shipping` adds the order's shipping price into the
/// granted amount (Django `includeShippingCosts`), `amount_override` books
/// an explicit figure instead (Django `amountToRefund`).
#[allow(clippy::too_many_arguments)]
pub async fn return_and_refund_full(
    db: &DatabaseConnection,
    order_id: Uuid,
    items: &[FulfillItem],
    reason: &str,
    restock: bool,
    transaction_item_id: Option<i32>,
    include_shipping: bool,
    amount_override: Option<rust_decimal::Decimal>,
) -> Result<ReturnRefundView> {
    use sea_orm::{sea_query::LockType, TransactionTrait};
    let txn = db.begin().await?;
    // Shared lock contract: the order row first.
    let order = order_order::Entity::find_by_id(order_id)
        .lock(LockType::Update)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("order {order_id} not found")))?;
    match order.status.as_str() {
        "canceled" | "draft" | "expired" => {
            return Err(fail(format!("order in status {} cannot be returned", order.status)))
        }
        _ => {}
    }
    if reason.is_empty() {
        return Err(fail("reason is required"));
    }
    let fulfillment_id = refund_fulfillment_in(&txn, order_id, items, reason, restock).await?;

    // Resolve the single money source for the line-based grant.
    // Net buckets: `charged_value` is already net of refunds.
    let charged: Vec<(i32, rust_decimal::Decimal, String)> =
        crate::entities::payment_transactionitem::Entity::find()
            .filter(crate::entities::payment_transactionitem::Column::OrderId.eq(order_id))
            .all(&txn)
            .await?
            .into_iter()
            .map(|t| (t.id, t.charged_value, t.currency))
            .filter(|(_, avail, _)| *avail > rust_decimal::Decimal::ZERO)
            .collect();
    let txn_id = match transaction_item_id {
        Some(id) => {
            if !charged.iter().any(|(i, _, _)| *i == id) {
                return Err(fail(format!(
                    "transaction {id} has no unrefunded charge on order {order_id}"
                )));
            }
            id
        }
        None => match charged.as_slice() {
            [(id, _, _)] => *id,
            [] => return Err(fail("order has no charged transaction to refund from")),
            _ => {
                return Err(fail(
                    "order has several charged transactions; pass transaction_item_id explicitly",
                ))
            }
        },
    };
    let grant_id = crate::granted_refunds::create_granted_refund_in(
        &txn,
        &crate::granted_refunds::NewGrant {
            order_id,
            transaction_item_id: Some(txn_id),
            amount: amount_override,
            lines: items
                .iter()
                .map(|i| crate::granted_refunds::GrantLineInput {
                    order_line_id: i.order_line_id,
                    quantity: i.quantity,
                })
                .collect(),
            reason: reason.to_string(),
            shipping_costs_included: include_shipping,
            user_id: None,
            app_id: None,
        },
    )
    .await?;
    let amount = crate::granted_refunds::view(&txn, grant_id).await?.amount;
    crate::payments::refund_in(&txn, txn_id, amount, &format!("return-{fulfillment_id}"), Some(grant_id))
        .await?;
    crate::granted_refunds::refresh_grant_status(&txn, grant_id).await?;
    crate::payments::refresh_order_statuses(&txn, Some(order_id)).await?;
    let payload = serde_json::json!({
        "id": order_id.to_string(),
        "fulfillment_id": fulfillment_id,
        "granted_refund_id": grant_id,
    })
    .to_string();
    let ch_slug = crate::catalog::channel_slug_for_id(&txn, order.channel_id).await.ok();
    let mut delivery_ids = crate::webhooks::trigger_event_tx(&txn, "order_updated", ch_slug.as_deref(), &payload).await?;
    delivery_ids.extend(
        crate::webhooks::trigger_event_tx(&txn, "fulfillment_returned", ch_slug.as_deref(), &payload).await?,
    );
    txn.commit().await?;
    crate::order_ops::refresh_order_money(db, order_id).await?;
    Ok(ReturnRefundView { fulfillment_id, granted_refund_id: grant_id, amount })
}

/// Resolve the dashboard-chosen warehouse to the variant's stock row in it
/// (Saleor's `OrderFulfillStockInput.warehouse`). None when the variant has
/// no stock there — callers fall back to auto-pick, never fail.
pub async fn stock_for_variant_warehouse(
    db: &impl sea_orm::ConnectionTrait,
    variant_id: i32,
    warehouse_id: Uuid,
) -> Result<Option<i32>> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    Ok(warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::ProductVariantId.eq(variant_id))
        .filter(warehouse_stock::Column::WarehouseId.eq(warehouse_id))
        .one(db)
        .await?
        .map(|s| s.id))
}

/// Approve a waiting-for-approval fulfillment (Django `approve_fulfillment`):
/// guard status, decrease tracked stock now (approval is when Django moves
/// inventory for waiting rows), status → fulfilled, order status refreshed.
///
/// Our `create_fulfillment` books `fulfilled` directly, so waiting rows only
/// arrive from Django-shared flows — no double-decrease inside our paths.
pub async fn approve_fulfillment(
    db: &DatabaseConnection,
    fulfillment_id: i32,
    allow_exceeded: bool,
) -> Result<FulfillmentView> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let f = order_fulfillment::Entity::find_by_id(fulfillment_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("fulfillment not found"))?;
    if f.status != "waiting_for_approval" {
        return Err(fail("only fulfillments waiting for approval can be approved"));
    }
    let order_id = f.order_id;
    let lines = order_fulfillmentline::Entity::find()
        .filter(order_fulfillmentline::Column::FulfillmentId.eq(fulfillment_id))
        .all(&txn)
        .await?;
    let vids: Vec<i32> = {
        let mut v = vec![];
        for fl in &lines {
            if let Some(l) = order_orderline::Entity::find_by_id(fl.order_line_id).one(&txn).await? {
                if let Some(vid) = l.variant_id {
                    v.push(vid);
                }
            }
        }
        v
    };
    let tracked = tracked_map(&txn, &vids).await?;
    for fl in &lines {
        let Some(l) = order_orderline::Entity::find_by_id(fl.order_line_id).one(&txn).await? else {
            continue;
        };
        let Some(vid) = l.variant_id else { continue };
        if !tracked.get(&vid).copied().unwrap_or(true) {
            continue;
        }
        let sid = match fl.stock_id {
            Some(s) => s,
            None => pick_stock(&txn, vid, fl.quantity).await?,
        };
        if allow_exceeded {
            let s = warehouse_stock::Entity::find_by_id(sid)
                .one(&txn)
                .await?
                .ok_or_else(|| fail("stock not found"))?;
            let mut am: warehouse_stock::ActiveModel = s.into();
            am.quantity = Set(am.quantity.clone().unwrap() - fl.quantity);
            am.update(&txn).await?;
        } else {
            decrease_stock_qty(&txn, sid, fl.quantity).await?;
        }
    }
    let mut fam: order_fulfillment::ActiveModel = f.into();
    fam.status = Set("fulfilled".to_string());
    fam.update(&txn).await?;
    refresh_order_status(&txn, order_id).await?;
    txn.commit().await?;
    view(db, fulfillment_id).await
}

/// Update the tracking number (Django `orderFulfillmentUpdateTracking`;
/// the customer email is an SMTP-side effect, out of scope — recorded
/// in the fulfillment row only).
pub async fn update_tracking(
    db: &DatabaseConnection,
    fulfillment_id: i32,
    tracking_number: &str,
) -> Result<FulfillmentView> {
    let f = order_fulfillment::Entity::find_by_id(fulfillment_id)
        .one(db)
        .await?
        .ok_or_else(|| fail("fulfillment not found"))?;
    if f.status == "canceled" {
        return Err(fail("canceled fulfillments cannot be updated"));
    }
    let mut am: order_fulfillment::ActiveModel = f.into();
    am.tracking_number = Set(tracking_number.to_string());
    am.update(db).await?;
    view(db, fulfillment_id).await
}

/// Cancel with an optional restock-warehouse override (Django
/// `FulfillmentCancelInput.warehouseId`): credits that warehouse's stock
/// row for each tracked variant, else the line's original stock. Missing
/// stock rows in the target warehouse are an honest error (Django would
/// create negative/inconsistent piles; we refuse instead).
pub async fn cancel_fulfillment_to(
    db: &DatabaseConnection,
    fulfillment_id: i32,
    warehouse_id: Option<Uuid>,
) -> Result<FulfillmentView> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let f = order_fulfillment::Entity::find_by_id(fulfillment_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("fulfillment not found"))?;
    if f.status == "canceled" {
        txn.commit().await?;
        return view(db, fulfillment_id).await;
    }
    if f.status == "refunded" || f.status == "returned" {
        return Err(fail(format!("{} fulfillments cannot be canceled", f.status)));
    }
    let order_id = f.order_id;
    let lines = order_fulfillmentline::Entity::find()
        .filter(order_fulfillmentline::Column::FulfillmentId.eq(fulfillment_id))
        .all(&txn)
        .await?;
    for fl in &lines {
        let line = order_orderline::Entity::find_by_id(fl.order_line_id)
            .one(&txn)
            .await?
            .ok_or_else(|| fail("order line not found"))?;
        let mut lam: order_orderline::ActiveModel = line.into();
        lam.quantity_fulfilled = Set((lam.quantity_fulfilled.clone().unwrap() - fl.quantity).max(0));
        lam.update(&txn).await?;
        let vid: Option<i32> = order_orderline::Entity::find_by_id(fl.order_line_id)
            .one(&txn)
            .await?
            .and_then(|l| l.variant_id);
        let sid = match (warehouse_id, vid) {
            (Some(wid), Some(vid)) => stock_for_variant_warehouse(&txn, vid, wid)
                .await?
                .ok_or_else(|| fail(format!("variant {vid} is not stocked in the chosen warehouse")))?,
            _ => match fl.stock_id {
                Some(s) => s,
                None => continue,
            },
        };
        increase_stock_qty(&txn, sid, fl.quantity).await?;
    }
    let mut fam: order_fulfillment::ActiveModel = f.into();
    fam.status = Set("canceled".to_string());
    fam.update(&txn).await?;
    refresh_order_status(&txn, order_id).await?;
    txn.commit().await?;
    view(db, fulfillment_id).await
}
