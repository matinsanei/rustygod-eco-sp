//! Staff order operations (Django `saleor/graphql/order/mutations/*` parity).
//!
//! Django references (`saleor-core/saleor/graphql/order/mutations/`):
//! - Editable guard: `ORDER_EDITABLE_STATUS = (DRAFT, UNCONFIRMED)` —
//!   `orderLinesCreate/Update/Delete`, `orderDiscount*`, `orderLineDiscount*`
//!   reject anything else with `NOT_EDITABLE` ("Only draft and unconfirmed
//!   orders can be edited.").
//! - `orderConfirm`: unconfirmed + has lines → unfulfilled (`update_order_status`).
//! - `orderUpdate` rejects DRAFT; `orderUpdateShipping` clears on None.
//! - `orderMarkAsPaid` books a manual transaction for the remainder.
//! - `orderCapture/Refund/Void` are legacy-payment in Django; here they run
//!   the modern equivalent — CHARGE/REFUND/CANCEL request-actions across the
//!   order's transactions via the manual PSP (same ledger `transactionUpdate`
//!   uses), so money math never diverges between the two surfaces.
//! - Bucket semantics (match `payments::execute_via` guards): `authorized`
//!   is the REMAINING authorized (charge/cancel already decrement it),
//!   `charged` is NET of refunds. So capture/void take from `authorized`,
//!   refunds take from `charged` — never `authorized-charged` or
//!   `charged-refunded` (both double-count).
//! - Event type strings mirror `saleor/order/__init__.py::OrderEvents`
//!   (lowercase: `confirmed`, `order_marked_as_paid`, ...).

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    catalog::{channel_info, channel_slug_for_id, checkout_pricing},
    entities::{
        account_address, discount_orderdiscount, discount_orderlinediscount,
        order_order, order_orderevent, order_orderline, payment_transactionitem,
        shipping_shippingmethod, shipping_shippingmethodchannellisting,
    },
    order_store::{create_line_row, variant_details},
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Order(msg.into())
}

/// Django `ORDER_EDITABLE_STATUS`.
fn require_editable(status: &str) -> Result<()> {
    if status != "draft" && status != "unconfirmed" {
        return Err(fail("Only draft and unconfirmed orders can be edited."));
    }
    Ok(())
}

async fn write_event(
    txn: &impl ConnectionTrait,
    order_id: Uuid,
    event_type: &str,
    parameters: serde_json::Value,
    user_id: Option<i32>,
) -> Result<()> {
    order_orderevent::ActiveModel {
        date: Set(Utc::now().into()),
        r#type: Set(event_type.to_string()),
        user_id: Set(user_id),
        parameters: Set(parameters),
        order_id: Set(order_id),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Ok(())
}

async fn lock_order(txn: &impl ConnectionTrait, order_id: Uuid) -> Result<order_order::Model> {
    order_order::Entity::find_by_id(order_id)
        .one(txn)
        .await?
        .ok_or_else(|| fail(format!("order {order_id} not found")))
}

/// Recompute header totals from lines (pre-tax: net == gross).
async fn recalc_totals(txn: &impl ConnectionTrait, order_id: Uuid) -> Result<()> {
    let total: Decimal = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .all(txn)
        .await?
        .iter()
        .map(|l| l.total_price_gross_amount)
        .sum();
    let count = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .count(txn)
        .await? as i32;
    let order = lock_order(txn, order_id).await?;
    let mut am: order_order::ActiveModel = order.into();
    am.total_net_amount = Set(total);
    am.total_gross_amount = Set(total);
    am.undiscounted_total_net_amount = Set(total);
    am.undiscounted_total_gross_amount = Set(total);
    am.subtotal_net_amount = Set(total);
    am.subtotal_gross_amount = Set(total);
    am.lines_count = Set(count);
    am.updated_at = Set(Utc::now().into());
    am.update(txn).await?;
    Ok(())
}

/// Confirm an unconfirmed order (must hold lines) → unfulfilled.
pub async fn confirm_order(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    actor_id: Option<i32>,
) -> Result<()> {
    let txn = db.begin().await?;
    let order = lock_order(&txn, order_id).await?;
    if order.status != "unconfirmed" {
        return Err(fail(
            "Provided order id belongs to an order with status different than unconfirmed.",
        ));
    }
    let n = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .count(&txn)
        .await?;
    if n == 0 {
        return Err(fail(
            "Provided order id belongs to an order without products.",
        ));
    }
    let mut am: order_order::ActiveModel = order.into();
    am.status = Set("unfulfilled".to_string());
    am.updated_at = Set(Utc::now().into());
    am.update(&txn).await?;
    write_event(&txn, order_id, "confirmed", json!({}), actor_id).await?;
    let slug = channel_slug_for_id(&txn, am_channel(&txn, order_id).await?).await.unwrap_or_default();
    let payload = json!({"order_id": order_id.to_string()}).to_string();
    crate::webhooks::trigger_event_tx(&txn, "ORDER_CONFIRMED", Some(&slug), &payload).await?;
    txn.commit().await?;
    Ok(())
}

async fn am_channel(txn: &impl ConnectionTrait, order_id: Uuid) -> Result<i32> {
    Ok(lock_order(txn, order_id).await?.channel_id)
}

/// Refresh the order's money columns from its transaction sums (Django's
/// `update_order_charge_status` also writes these, not just the statuses).
async fn refresh_order_money(db: &sea_orm::DatabaseConnection, order_id: Uuid) -> Result<()> {
    let txns = payment_transactionitem::Entity::find()
        .filter(payment_transactionitem::Column::OrderId.eq(order_id))
        .all(db)
        .await?;
    let authorized: Decimal = txns.iter().map(|t| t.authorized_value).sum();
    let charged: Decimal = txns.iter().map(|t| t.charged_value).sum();
    if let Some(order) = order_order::Entity::find_by_id(order_id).one(db).await? {
        let mut am: order_order::ActiveModel = order.into();
        am.total_authorized_amount = Set(authorized);
        am.total_charged_amount = Set(charged);
        am.updated_at = Set(Utc::now().into());
        am.update(db).await?;
    }
    crate::payments::refresh_order_statuses(db, Some(order_id)).await?;
    Ok(())
}

/// Capture `amount` across the order's transactions (charge authorized remainders).
/// Returns the captured total.
pub async fn capture_order(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    amount: Decimal,
    actor_id: Option<i32>,
) -> Result<Decimal> {
    if amount <= Decimal::ZERO {
        return Err(fail("capture amount must be positive"));
    }
    let order = order_order::Entity::find_by_id(order_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("order {order_id} not found")))?;
    let ids: Vec<i32> = payment_transactionitem::Entity::find()
        .select_only()
        .column(payment_transactionitem::Column::Id)
        .filter(payment_transactionitem::Column::OrderId.eq(order_id))
        .into_tuple()
        .all(db)
        .await?;
    if ids.is_empty() {
        return Err(fail("order has no transactions to capture"));
    }
    let mut left = amount;
    let mut captured = Decimal::ZERO;
    for tid in ids {
        if left <= Decimal::ZERO {
            break;
        }
        let v = crate::payments::view(db, tid).await?;
        let avail = v.authorized;
        if avail <= Decimal::ZERO {
            continue;
        }
        let take = avail.min(left);
        let key = format!("order-capture-{order_id}-{tid}");
        crate::payments::charge(db, tid, take, &key).await?;
        captured += take;
        left -= take;
    }
    if captured <= Decimal::ZERO {
        return Err(fail("nothing available to capture"));
    }
    let txn = db.begin().await?;
    write_event(
        &txn,
        order_id,
        "payment_captured",
        json!({"amount": captured.to_string()}),
        actor_id,
    )
    .await?;
    let fresh: Decimal = payment_transactionitem::Entity::find()
        .select_only()
        .column(payment_transactionitem::Column::ChargedValue)
        .filter(payment_transactionitem::Column::OrderId.eq(order_id))
        .into_tuple::<Decimal>()
        .all(&txn)
        .await?
        .into_iter()
        .sum();
    if fresh >= order.total_gross_amount {
        write_event(&txn, order_id, "order_fully_paid", json!({}), actor_id).await?;
    }
    txn.commit().await?;
    refresh_order_money(db, order_id).await?;
    Ok(captured)
}

/// Refund `amount` across the order's transactions (refund charged remainders).
pub async fn refund_order(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    amount: Decimal,
    actor_id: Option<i32>,
) -> Result<Decimal> {
    if amount <= Decimal::ZERO {
        return Err(fail("refund amount must be positive"));
    }
    if order_order::Entity::find_by_id(order_id).one(db).await?.is_none() {
        return Err(fail(format!("order {order_id} not found")));
    }
    let ids: Vec<i32> = payment_transactionitem::Entity::find()
        .select_only()
        .column(payment_transactionitem::Column::Id)
        .filter(payment_transactionitem::Column::OrderId.eq(order_id))
        .into_tuple()
        .all(db)
        .await?;
    let mut left = amount;
    let mut refunded = Decimal::ZERO;
    for tid in ids {
        if left <= Decimal::ZERO {
            break;
        }
        let v = crate::payments::view(db, tid).await?;
        let avail = v.charged;
        if avail <= Decimal::ZERO {
            continue;
        }
        let take = avail.min(left);
        let key = format!("order-refund-{order_id}-{tid}");
        crate::payments::refund(db, tid, take, &key).await?;
        refunded += take;
        left -= take;
    }
    if refunded <= Decimal::ZERO {
        return Err(fail("nothing available to refund"));
    }
    let txn = db.begin().await?;
    write_event(
        &txn,
        order_id,
        "payment_refunded",
        json!({"amount": refunded.to_string()}),
        actor_id,
    )
    .await?;
    txn.commit().await?;
    refresh_order_money(db, order_id).await?;
    Ok(refunded)
}

/// Void = cancel every authorized remainder on the order's transactions.
pub async fn void_order(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    actor_id: Option<i32>,
) -> Result<()> {
    if order_order::Entity::find_by_id(order_id).one(db).await?.is_none() {
        return Err(fail(format!("order {order_id} not found")));
    }
    let ids: Vec<i32> = payment_transactionitem::Entity::find()
        .select_only()
        .column(payment_transactionitem::Column::Id)
        .filter(payment_transactionitem::Column::OrderId.eq(order_id))
        .into_tuple()
        .all(db)
        .await?;
    let mut n = 0;
    for tid in ids {
        let v = crate::payments::view(db, tid).await?;
        let avail = v.authorized;
        if avail > Decimal::ZERO {
            let key = format!("order-void-{order_id}-{tid}");
            crate::payments::cancel(db, tid, avail, &key).await?;
            n += 1;
        }
    }
    if n == 0 {
        return Err(fail("nothing available to void"));
    }
    let txn = db.begin().await?;
    write_event(&txn, order_id, "payment_voided", json!({}), actor_id).await?;
    txn.commit().await?;
    refresh_order_money(db, order_id).await?;
    Ok(())
}

/// Mark as paid: book one manual transaction for the outstanding remainder
/// (Django `mark_order_as_paid_with_transaction`).
pub async fn mark_order_as_paid(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    transaction_reference: Option<String>,
    actor_id: Option<i32>,
) -> Result<()> {
    let order = order_order::Entity::find_by_id(order_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("order {order_id} not found")))?;
    if order.status == "draft" {
        return Err(fail("cannot mark a draft order as paid"));
    }
    let charged: Decimal = payment_transactionitem::Entity::find()
        .select_only()
        .column(payment_transactionitem::Column::ChargedValue)
        .filter(payment_transactionitem::Column::OrderId.eq(order_id))
        .into_tuple::<Decimal>()
        .all(db)
        .await?
        .into_iter()
        .sum();
    let outstanding = order.total_gross_amount - charged;
    if outstanding <= Decimal::ZERO {
        return Err(fail("order is already paid"));
    }
    let key = format!("mark-paid-{order_id}");
    let created = crate::payments::create_transaction(
        db,
        &crate::payments::NewTransaction {
            checkout_id: None,
            order_id: Some(order_id),
            currency: order.currency.clone(),
            name: String::new(),
            app_identifier: None,
            idempotency_key: Some(key.clone()),
            available_actions: vec!["refund".into()],
        },
    )
    .await?;
    crate::payments::authorize(db, created.id, outstanding, &format!("{key}-auth")).await?;
    crate::payments::charge(db, created.id, outstanding, &format!("{key}-charge")).await?;
    if let Some(r) = transaction_reference.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        if let Some(row) = payment_transactionitem::Entity::find_by_id(created.id).one(db).await? {
            let mut am: payment_transactionitem::ActiveModel = row.into();
            am.psp_reference = Set(Some(r));
            am.update(db).await?;
        }
    }
    let txn = db.begin().await?;
    write_event(
        &txn,
        order_id,
        "order_marked_as_paid",
        json!({"amount": outstanding.to_string()}),
        actor_id,
    )
    .await?;
    txn.commit().await?;
    refresh_order_money(db, order_id).await?;
    Ok(())
}

pub struct OrderAddressInput {
    pub first_name: String,
    pub last_name: String,
    pub street1: String,
    pub street2: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub country_area: String,
    pub phone: String,
    pub company_name: String,
}

async fn insert_address(
    txn: &impl ConnectionTrait,
    a: &OrderAddressInput,
) -> Result<i32> {
    let row = account_address::ActiveModel {
        first_name: Set(a.first_name.clone()),
        last_name: Set(a.last_name.clone()),
        company_name: Set(a.company_name.clone()),
        street_address_1: Set(a.street1.clone()),
        street_address_2: Set(a.street2.clone()),
        city: Set(a.city.clone()),
        postal_code: Set(a.postal_code.clone()),
        country: Set(a.country.clone()),
        country_area: Set(a.country_area.clone()),
        phone: Set(a.phone.clone()),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Ok(row.id)
}

/// Staff order update (Django `orderUpdate`): addresses, email, language,
/// external reference. Rejects DRAFT (drafts go through `draftOrderUpdate`).
#[allow(clippy::too_many_arguments)]
pub async fn update_order(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    user_email: Option<String>,
    external_reference: Option<String>,
    language_code: Option<String>,
    billing: Option<OrderAddressInput>,
    shipping: Option<OrderAddressInput>,
    actor_id: Option<i32>,
) -> Result<()> {
    let txn = db.begin().await?;
    let order = lock_order(&txn, order_id).await?;
    if order.status == "draft" {
        return Err(fail("draft orders are edited via draftOrderUpdate"));
    }
    let mut am: order_order::ActiveModel = order.into();
    if let Some(e) = user_email.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        am.user_email = Set(e);
    }
    if let Some(r) = external_reference {
        am.external_reference = Set(Some(r));
    }
    if let Some(l) = language_code {
        am.language_code = Set(l);
    }
    if let Some(b) = billing.as_ref() {
        am.billing_address_id = Set(Some(insert_address(&txn, b).await?));
    }
    if let Some(s) = shipping.as_ref() {
        am.shipping_address_id = Set(Some(insert_address(&txn, s).await?));
    }
    am.updated_at = Set(Utc::now().into());
    am.update(&txn).await?;
    write_event(&txn, order_id, "updated_address", json!({}), actor_id).await?;
    txn.commit().await?;
    Ok(())
}

/// Change (or clear with None) the shipping method, repricing from the
/// channel listing (Django `ShippingMethodUpdateMixin`).
pub async fn update_order_shipping(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    shipping_method_id: Option<i32>,
    actor_id: Option<i32>,
) -> Result<()> {
    let txn = db.begin().await?;
    let order = lock_order(&txn, order_id).await?;
    let mut am: order_order::ActiveModel = order.into();
    match shipping_method_id {
        None => {
            am.shipping_method_id = Set(None);
            am.shipping_method_name = Set(None);
            am.base_shipping_price_amount = Set(Decimal::ZERO);
            am.shipping_price_net_amount = Set(Decimal::ZERO);
            am.shipping_price_gross_amount = Set(Decimal::ZERO);
            am.undiscounted_base_shipping_price_amount = Set(Decimal::ZERO);
        }
        Some(mid) => {
            let m = shipping_shippingmethod::Entity::find_by_id(mid)
                .one(&txn)
                .await?
                .ok_or_else(|| fail(format!("shipping method {mid} not found")))?;
            let listing = shipping_shippingmethodchannellisting::Entity::find()
                .filter(shipping_shippingmethodchannellisting::Column::ShippingMethodId.eq(mid))
                .filter(
                    shipping_shippingmethodchannellisting::Column::ChannelId
                        .eq(am_channel(&txn, order_id).await?),
                )
                .one(&txn)
                .await?
                .ok_or_else(|| fail("shipping method is not listed in this channel"))?;
            am.shipping_method_id = Set(Some(mid));
            am.shipping_method_name = Set(Some(m.name.clone()));
            am.base_shipping_price_amount = Set(listing.price_amount);
            am.shipping_price_net_amount = Set(listing.price_amount);
            am.shipping_price_gross_amount = Set(listing.price_amount);
            am.undiscounted_base_shipping_price_amount = Set(listing.price_amount);
        }
    }
    am.updated_at = Set(Utc::now().into());
    am.update(&txn).await?;
    write_event(&txn, order_id, "updated_address", json!({}), actor_id).await?;
    txn.commit().await?;
    Ok(())
}

/// Edit a staff note's message (Django `orderNoteUpdate`, event gid).
pub async fn update_note(
    db: &sea_orm::DatabaseConnection,
    event_id: i32,
    message: &str,
) -> Result<Uuid> {
    let message = message.trim();
    if message.is_empty() {
        return Err(fail("message is required"));
    }
    let ev = order_orderevent::Entity::find_by_id(event_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("note event {event_id} not found")))?;
    if ev.r#type != "note_added" {
        return Err(fail("only staff notes can be edited"));
    }
    let oid = ev.order_id;
    let mut am: order_orderevent::ActiveModel = ev.into();
    am.parameters = Set(json!({"message": message}));
    am.update(db).await?;
    Ok(oid)
}

pub struct NewOrderLine {
    pub variant_id: i32,
    pub quantity: i32,
}

/// Add lines to a draft/unconfirmed order (Django `orderLinesCreate`).
pub async fn create_lines(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    lines: Vec<NewOrderLine>,
    actor_id: Option<i32>,
) -> Result<()> {
    if lines.is_empty() {
        return Err(fail("no lines to add"));
    }
    let probe = order_order::Entity::find_by_id(order_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("order {order_id} not found")))?;
    require_editable(&probe.status)?;
    let channel_slug = channel_slug_for_id(db, probe.channel_id).await?;
    let (ch_id, _) = channel_info(db, &channel_slug).await?;
    let vids: Vec<i32> = lines.iter().map(|l| l.variant_id).collect();
    let pricing = checkout_pricing(db, &channel_slug, &vids).await?;
    let details = variant_details(db, &vids).await?;
    let txn = db.begin().await?;
    let order = lock_order(&txn, order_id).await?;
    require_editable(&order.status)?;
    if order.channel_id != ch_id {
        return Err(fail("order belongs to a different channel"));
    }
    for l in &lines {
        if l.quantity < 1 {
            return Err(fail("line quantity must be positive"));
        }
        let detail = details
            .get(&l.variant_id)
            .ok_or_else(|| fail(format!("variant {} not found", l.variant_id)))?;
        let (listed, _) = pricing
            .get(&l.variant_id)
            .ok_or_else(|| fail(format!("variant {} is not listed in this channel", l.variant_id)))?;
        create_line_row(&txn, order_id, detail, l.quantity, listed.amount, &order.currency).await?;
    }
    recalc_totals(&txn, order_id).await?;
    write_event(&txn, order_id, "added_products", json!({"lines_added": lines.len()}), actor_id).await?;
    txn.commit().await?;
    Ok(())
}

/// Change a line's quantity (draft/unconfirmed only).
pub async fn update_line_quantity(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    line_id: Uuid,
    quantity: i32,
    actor_id: Option<i32>,
) -> Result<()> {
    if quantity < 1 {
        return Err(fail("line quantity must be positive"));
    }
    let txn = db.begin().await?;
    let order = lock_order(&txn, order_id).await?;
    require_editable(&order.status)?;
    let line = order_orderline::Entity::find_by_id(line_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("order line not found"))?;
    if line.order_id != order_id {
        return Err(fail("order line does not belong to this order"));
    }
    let unit = line.unit_price_gross_amount;
    let mut lam: order_orderline::ActiveModel = line.into();
    lam.quantity = Set(quantity);
    lam.total_price_net_amount = Set(unit * Decimal::from(quantity));
    lam.total_price_gross_amount = Set(unit * Decimal::from(quantity));
    lam.undiscounted_total_price_net_amount = Set(unit * Decimal::from(quantity));
    lam.undiscounted_total_price_gross_amount = Set(unit * Decimal::from(quantity));
    lam.update(&txn).await?;
    recalc_totals(&txn, order_id).await?;
    write_event(&txn, order_id, "added_products", json!({"line_updated": line_id.to_string()}), actor_id).await?;
    txn.commit().await?;
    Ok(())
}

/// Delete a line (draft/unconfirmed only).
pub async fn delete_line(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    line_id: Uuid,
    actor_id: Option<i32>,
) -> Result<()> {
    let txn = db.begin().await?;
    let order = lock_order(&txn, order_id).await?;
    require_editable(&order.status)?;
    let line = order_orderline::Entity::find_by_id(line_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("order line not found"))?;
    if line.order_id != order_id {
        return Err(fail("order line does not belong to this order"));
    }
    let lam: order_orderline::ActiveModel = line.into();
    lam.delete(&txn).await?;
    recalc_totals(&txn, order_id).await?;
    write_event(&txn, order_id, "removed_products", json!({"line_deleted": line_id.to_string()}), actor_id).await?;
    txn.commit().await?;
    Ok(())
}

fn parse_discount_value(value_type: &str, value: Decimal) -> Result<Decimal> {
    if value <= Decimal::ZERO {
        return Err(fail("discount value must be positive"));
    }
    if value_type.eq_ignore_ascii_case("percentage") && value > Decimal::from(100) {
        return Err(fail("percentage discount cannot exceed 100"));
    }
    Ok(value)
}

/// Manual order-level discount (draft/unconfirmed; Django `orderDiscountAdd`).
pub async fn discount_add(
    db: &sea_orm::DatabaseConnection,
    order_id: Uuid,
    reason: Option<String>,
    value_type: &str,
    value: Decimal,
    actor_id: Option<i32>,
) -> Result<Uuid> {
    let value = parse_discount_value(value_type, value)?;
    let txn = db.begin().await?;
    let order = lock_order(&txn, order_id).await?;
    require_editable(&order.status)?;
    let amount = if value_type.eq_ignore_ascii_case("percentage") {
        order.total_gross_amount * value / Decimal::from(100)
    } else {
        value.min(order.total_gross_amount)
    };
    let id = Uuid::new_v4();
    discount_orderdiscount::ActiveModel {
        id: Set(id),
        r#type: Set("manual".to_string()),
        value_type: Set(value_type.to_lowercase()),
        value: Set(value),
        amount_value: Set(amount),
        currency: Set(order.currency.clone()),
        reason: Set(reason),
        order_id: Set(Some(order_id)),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    write_event(&txn, order_id, "order_discount_added", json!({"discount_id": id.to_string()}), actor_id).await?;
    txn.commit().await?;
    Ok(id)
}

/// Update a manual order discount's value/reason.
pub async fn discount_update(
    db: &sea_orm::DatabaseConnection,
    discount_id: Uuid,
    reason: Option<String>,
    value_type: Option<String>,
    value: Option<Decimal>,
    actor_id: Option<i32>,
) -> Result<Uuid> {
    let txn = db.begin().await?;
    let d = discount_orderdiscount::Entity::find()
        .filter(discount_orderdiscount::Column::Id.eq(discount_id))
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("discount {discount_id} not found")))?;
    let oid = d.order_id.ok_or_else(|| fail("discount is not attached to an order"))?;
    let order = lock_order(&txn, oid).await?;
    require_editable(&order.status)?;
    if d.r#type != "manual" {
        return Err(fail("only manual discounts can be edited"));
    }
    let vt = value_type.as_deref().unwrap_or(&d.value_type).to_string();
    let vv = value.unwrap_or(d.value);
    parse_discount_value(&vt, vv)?;
    let amount = if vt.eq_ignore_ascii_case("percentage") {
        order.total_gross_amount * vv / Decimal::from(100)
    } else {
        vv.min(order.total_gross_amount)
    };
    let mut am: discount_orderdiscount::ActiveModel = d.into();
    am.value_type = Set(vt.to_lowercase());
    am.value = Set(vv);
    am.amount_value = Set(amount);
    if reason.is_some() {
        am.reason = Set(reason);
    }
    am.update(&txn).await?;
    write_event(&txn, oid, "order_discount_updated", json!({"discount_id": discount_id.to_string()}), actor_id).await?;
    txn.commit().await?;
    Ok(oid)
}

/// Delete a manual order discount.
pub async fn discount_delete(
    db: &sea_orm::DatabaseConnection,
    discount_id: Uuid,
    actor_id: Option<i32>,
) -> Result<Uuid> {
    let txn = db.begin().await?;
    let d = discount_orderdiscount::Entity::find()
        .filter(discount_orderdiscount::Column::Id.eq(discount_id))
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("discount {discount_id} not found")))?;
    let oid = d.order_id.ok_or_else(|| fail("discount is not attached to an order"))?;
    let order = lock_order(&txn, oid).await?;
    require_editable(&order.status)?;
    if d.r#type != "manual" {
        return Err(fail("only manual discounts can be deleted"));
    }
    let am: discount_orderdiscount::ActiveModel = d.into();
    am.delete(&txn).await?;
    write_event(&txn, oid, "order_discount_deleted", json!({"discount_id": discount_id.to_string()}), actor_id).await?;
    txn.commit().await?;
    Ok(oid)
}

/// Create-or-update the manual line discount (Django `orderLineDiscountUpdate`).
pub async fn line_discount_update(
    db: &sea_orm::DatabaseConnection,
    order_line_id: Uuid,
    value_type: &str,
    value: Decimal,
    reason: Option<String>,
    actor_id: Option<i32>,
) -> Result<Uuid> {
    let value = parse_discount_value(value_type, value)?;
    let txn = db.begin().await?;
    let line = order_orderline::Entity::find_by_id(order_line_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("order line not found"))?;
    let order = lock_order(&txn, line.order_id).await?;
    require_editable(&order.status)?;
    let amount = if value_type.eq_ignore_ascii_case("percentage") {
        line.total_price_gross_amount * value / Decimal::from(100)
    } else {
        value.min(line.total_price_gross_amount)
    };
    let existing = discount_orderlinediscount::Entity::find()
        .filter(discount_orderlinediscount::Column::LineId.eq(order_line_id))
        .filter(discount_orderlinediscount::Column::Type.eq("manual"))
        .one(&txn)
        .await?;
    match existing {
        Some(d) => {
            let mut am: discount_orderlinediscount::ActiveModel = d.into();
            am.value_type = Set(value_type.to_lowercase());
            am.value = Set(value);
            am.amount_value = Set(amount);
            if reason.is_some() {
                am.reason = Set(reason);
            }
            am.update(&txn).await?;
        }
        None => {
            discount_orderlinediscount::ActiveModel {
                id: Set(Uuid::new_v4()),
                r#type: Set("manual".to_string()),
                value_type: Set(value_type.to_lowercase()),
                value: Set(value),
                amount_value: Set(amount),
                currency: Set(order.currency.clone()),
                reason: Set(reason),
                line_id: Set(Some(order_line_id)),
                created_at: Set(Utc::now().into()),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }
    write_event(&txn, line.order_id, "order_line_discount_updated", json!({"line_id": order_line_id.to_string()}), actor_id).await?;
    txn.commit().await?;
    Ok(line.order_id)
}

/// Remove the manual line discount (Django `orderLineDiscountRemove`).
pub async fn line_discount_remove(
    db: &sea_orm::DatabaseConnection,
    order_line_id: Uuid,
    actor_id: Option<i32>,
) -> Result<Uuid> {
    let txn = db.begin().await?;
    let line = order_orderline::Entity::find_by_id(order_line_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("order line not found"))?;
    let order = lock_order(&txn, line.order_id).await?;
    require_editable(&order.status)?;
    let existing = discount_orderlinediscount::Entity::find()
        .filter(discount_orderlinediscount::Column::LineId.eq(order_line_id))
        .filter(discount_orderlinediscount::Column::Type.eq("manual"))
        .one(&txn)
        .await?
        .ok_or_else(|| fail("order line has no manual discount"))?;
    let am: discount_orderlinediscount::ActiveModel = existing.into();
    am.delete(&txn).await?;
    write_event(&txn, line.order_id, "order_line_discount_removed", json!({"line_id": order_line_id.to_string()}), actor_id).await?;
    txn.commit().await?;
    Ok(line.order_id)
}

/// Variant price lookup for order-line creation (channel listing).
pub async fn variant_unit_price(
    db: &sea_orm::DatabaseConnection,
    channel_slug: &str,
    variant_id: i32,
) -> Result<Decimal> {
    let pricing = checkout_pricing(db, channel_slug, &[variant_id]).await?;
    pricing
        .get(&variant_id)
        .map(|(m, _)| m.amount)
        .ok_or_else(|| fail(format!("variant {variant_id} is not listed in this channel")))
}
