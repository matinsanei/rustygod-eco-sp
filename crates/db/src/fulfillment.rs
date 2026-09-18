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
    QuerySelect, SelectorTrait, Set,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        order_fulfillment, order_fulfillmentline, order_order, order_orderline, warehouse_stock,
    },
    DbError, Result,
};

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
        let done = fulfilled_qty(&txn, item.order_line_id).await?;
        if done + item.quantity > line.quantity {
            return Err(fail(format!(
                "cannot fulfill {}: only {} of {} remaining",
                item.quantity,
                line.quantity - done,
                line.quantity
            )));
        }
        // Resolve stock: explicit or auto-pick.
        let stock_id = match item.stock_id {
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
        };
        order_fulfillmentline::ActiveModel {
            order_line_id: Set(item.order_line_id),
            fulfillment_id: Set(f.id),
            quantity: Set(item.quantity),
            stock_id: Set(Some(stock_id)),
            reason: Set(String::new()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        // Bump fulfilled + decrease stock (Django decrease_stock semantics).
        let mut lam: order_orderline::ActiveModel = line.into();
        lam.quantity_fulfilled = Set(lam.quantity_fulfilled.clone().unwrap() + item.quantity);
        lam.update(&txn).await?;
        decrease_stock_qty(&txn, stock_id, item.quantity).await?;
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
    if items.is_empty() {
        return Err(fail("refund needs at least one line"));
    }
    let txn = db.begin().await?;
    order_order::Entity::find_by_id(order_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("order not found"))?;
    let seq = next_fulfillment_order(&txn, order_id).await?;
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
    .insert(&txn)
    .await?;
    for item in items {
        if item.quantity < 1 {
            return Err(fail("refund quantity must be positive"));
        }
        let line = order_orderline::Entity::find_by_id(item.order_line_id)
            .one(&txn)
            .await?
            .ok_or_else(|| fail("order line not found"))?;
        if line.order_id != order_id {
            return Err(fail("order line does not belong to this order"));
        }
        // Refundable = fulfilled (non-canceled) minus already returned.
        let done = fulfilled_qty(&txn, item.order_line_id).await?;
        let returned = returned_qty(&txn, item.order_line_id).await?;
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
            None => stock_of_fulfilled_line(&txn, item.order_line_id).await?,
        };
        order_fulfillmentline::ActiveModel {
            order_line_id: Set(item.order_line_id),
            fulfillment_id: Set(f.id),
            quantity: Set(item.quantity),
            stock_id: Set(stock_id),
            reason: Set(reason.to_string()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        if let Some(sid) = stock_id {
            increase_stock_qty(&txn, sid, item.quantity).await?;
        }
    }
    refresh_order_status(&txn, order_id).await?;
    txn.commit().await?;
    view(db, f.id).await
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
