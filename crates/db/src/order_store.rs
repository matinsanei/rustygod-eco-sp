//! Order writes on the Django tables (`order_order`, `order_orderline`).
//!
//! Django parity notes (`saleor/order/models.py`, `saleor/order/__init__.py`):
//! - Order PK is **`id` (UUID)**; the human number comes from the Postgres
//!   sequence **`order_order_number_seq`** — Rust calls the same sequence, so
//!   Django and Rust never collide.
//! - Status `"unfulfilled"` for auto-confirm channels
//!   (`automatically_confirm_all_new_orders = true`); authorize/charge `"none"`.
//! - `origin = "checkout"`, `checkout_token` is the checkout UUID string.
//! - Pre-tax lines: net == gross, `tax_rate` NULL (unknown until tax plugins),
//!   discounts zero, `quantity_fulfilled = 0`.
//! - `is_shipping_required` resolves via variant → product → product type,
//!   `is_gift_card` via `product_type.kind == 'gift_card'`.

use chrono::Utc;
use rust_decimal::Decimal;
use rustygod_core::{
    checkout::Checkout,
    money::Money,
    order::{Order, OrderLine, OrderStatus},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, Statement,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{order_order, order_orderline, product_product, product_producttype, product_productvariant},
    DbError, Result,
};

/// Same sequence Django's `get_order_number()` uses.
pub async fn next_number(db: &impl sea_orm::ConnectionTrait) -> Result<i32> {
    let row = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT nextval('order_order_number_seq') AS n".to_string(),
        ))
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound("sequence".into())))?;
    row.try_get::<i32>("", "n")
        .or_else(|_| row.try_get::<i64>("", "n").map(|v| v as i32))
        .map_err(DbError::SeaOrm)
}

pub struct NewOrder {
    pub channel_id: i32,
    pub channel_slug: String,
    pub email: String,
    pub currency: String,
    pub language_code: String,
    pub checkout_token: Uuid,
    pub user_id: Option<i32>,
    pub status: OrderStatus,
}

/// Persist the order header. Returns (order_id, number).
pub async fn create_order_row(
    db: &impl sea_orm::ConnectionTrait,
    new: &NewOrder,
    total: Decimal,
    lines_count: i32,
) -> Result<(Uuid, i32)> {
    let id = Uuid::new_v4();
    let number = next_number(db).await?;
    let t = Utc::now();
    let row = order_order::ActiveModel {
        id: Set(id),
        number: Set(number),
        created_at: Set(t.into()),
        updated_at: Set(t.into()),
        status: Set(new.status.as_str().to_string()),
        authorize_status: Set("none".to_string()),
        charge_status: Set("none".to_string()),
        origin: Set("checkout".to_string()),
        checkout_token: Set(new.checkout_token.to_string()),
        channel_id: Set(new.channel_id),
        currency: Set(new.currency.clone()),
        user_email: Set(new.email.clone()),
        user_id: Set(new.user_id),
        language_code: Set(new.language_code.clone()),
        display_gross_prices: Set(true),
        customer_note: Set(String::new()),
        weight: Set(0.0),
        total_net_amount: Set(total),
        total_gross_amount: Set(total),
        undiscounted_total_net_amount: Set(total),
        undiscounted_total_gross_amount: Set(total),
        subtotal_net_amount: Set(total),
        subtotal_gross_amount: Set(total),
        total_charged_amount: Set(Decimal::ZERO),
        total_authorized_amount: Set(Decimal::ZERO),
        shipping_price_net_amount: Set(Decimal::ZERO),
        shipping_price_gross_amount: Set(Decimal::ZERO),
        base_shipping_price_amount: Set(Decimal::ZERO),
        undiscounted_base_shipping_price_amount: Set(Decimal::ZERO),
        lines_count: Set(lines_count),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        shipping_tax_class_metadata: Set(json!({})),
        shipping_tax_class_private_metadata: Set(json!({})),
        search_document: Set(String::new()),
        use_old_id: Set(false),
        should_refresh_prices: Set(true),
        tax_exemption: Set(false),
        tracking_client_id: Set(String::new()),
        ..Default::default()
    };
    row.insert(db).await?;
    Ok((id, number))
}

/// Variant details needed for faithful order lines.
pub struct VariantDetail {
    pub variant_id: i32,
    pub sku: Option<String>,
    pub product_name: String,
    pub variant_name: String,
    pub product_type_id: i32,
    pub is_shipping_required: bool,
    pub is_gift_card: bool,
}

pub async fn variant_details(
    db: &impl sea_orm::ConnectionTrait,
    variant_ids: &[i32],
) -> Result<std::collections::HashMap<i32, VariantDetail>> {
    use std::collections::HashMap;
    let mut out = HashMap::new();
    if variant_ids.is_empty() {
        return Ok(out);
    }
    let variants = product_productvariant::Entity::find()
        .filter(product_productvariant::Column::Id.is_in(variant_ids.to_vec()))
        .all(db)
        .await?;
    let product_ids: Vec<i32> = variants.iter().map(|v| v.product_id).collect();
    let products = product_product::Entity::find()
        .select_only()
        .column(product_product::Column::Id)
        .column(product_product::Column::Name)
        .column(product_product::Column::ProductTypeId)
        .filter(product_product::Column::Id.is_in(product_ids))
        .into_tuple::<(i32, String, i32)>()
        .all(db)
        .await?;
    let pmap: HashMap<i32, (String, i32)> =
        products.into_iter().map(|(id, n, t)| (id, (n, t))).collect();
    let type_ids: Vec<i32> = pmap.values().map(|(_, t)| *t).collect();
    let types = product_producttype::Entity::find()
        .select_only()
        .column(product_producttype::Column::Id)
        .column(product_producttype::Column::IsShippingRequired)
        .column(product_producttype::Column::Kind)
        .filter(product_producttype::Column::Id.is_in(type_ids))
        .into_tuple::<(i32, bool, String)>()
        .all(db)
        .await?;
    let tmap: HashMap<i32, (bool, String)> =
        types.into_iter().map(|(id, s, k)| (id, (s, k))).collect();

    for v in variants {
        if let Some((pname, tid)) = pmap.get(&v.product_id) {
            let (ship, kind) = tmap.get(tid).cloned().unwrap_or((true, "normal".into()));
            out.insert(
                v.id,
                VariantDetail {
                    variant_id: v.id,
                    sku: v.sku.clone(),
                    product_name: pname.clone(),
                    variant_name: v.name.clone(),
                    product_type_id: *tid,
                    is_shipping_required: ship,
                    is_gift_card: kind == "gift_card",
                },
            );
        }
    }
    Ok(out)
}

/// Persist one order line (pre-tax: net == gross).
#[allow(clippy::too_many_arguments)]
pub async fn create_line_row(
    db: &impl sea_orm::ConnectionTrait,
    order_id: Uuid,
    detail: &VariantDetail,
    quantity: i32,
    unit_price: Decimal,
    currency: &str,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let total = unit_price * Decimal::from(quantity);
    let row = order_orderline::ActiveModel {
        id: Set(id),
        order_id: Set(order_id),
        variant_id: Set(Some(detail.variant_id)),
        product_name: Set(detail.product_name.clone()),
        variant_name: Set(detail.variant_name.clone()),
        translated_product_name: Set(String::new()),
        translated_variant_name: Set(String::new()),
        product_sku: Set(detail.sku.clone()),
        product_type_id: Set(Some(detail.product_type_id)),
        is_shipping_required: Set(detail.is_shipping_required),
        is_gift_card: Set(detail.is_gift_card),
        quantity: Set(quantity),
        quantity_fulfilled: Set(0),
        currency: Set(currency.to_string()),
        unit_price_net_amount: Set(unit_price),
        unit_price_gross_amount: Set(unit_price),
        total_price_net_amount: Set(total),
        total_price_gross_amount: Set(total),
        undiscounted_unit_price_net_amount: Set(unit_price),
        undiscounted_unit_price_gross_amount: Set(unit_price),
        undiscounted_total_price_net_amount: Set(total),
        undiscounted_total_price_gross_amount: Set(total),
        base_unit_price_amount: Set(unit_price),
        undiscounted_base_unit_price_amount: Set(unit_price),
        unit_discount_amount: Set(Decimal::ZERO),
        unit_discount_value: Set(Decimal::ZERO),
        is_gift: Set(false),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        tax_class_metadata: Set(json!({})),
        tax_class_private_metadata: Set(json!({})),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    };
    row.insert(db).await?;
    Ok(id)
}

/// Slim order header (never SELECT * — `search_vector` tsvector mistype).
#[derive(Debug)]
pub struct OrderHeader {
    pub id: Uuid,
    pub number: i32,
    pub user_email: String,
    pub status: String,
    pub currency: String,
    pub total_gross_amount: Decimal,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_order_rows(
    db: &impl sea_orm::ConnectionTrait,
    id: Uuid,
) -> Result<Option<(OrderHeader, Vec<order_orderline::Model>)>> {
    let row: Option<(Uuid, i32, String, String, String, Decimal, chrono::DateTime<chrono::Utc>)> =
        order_order::Entity::find_by_id(id)
            .select_only()
            .column(order_order::Column::Id)
            .column(order_order::Column::Number)
            .column(order_order::Column::UserEmail)
            .column(order_order::Column::Status)
            .column(order_order::Column::Currency)
            .column(order_order::Column::TotalGrossAmount)
            .column(order_order::Column::CreatedAt)
            .into_tuple()
            .one(db)
            .await?;
    let Some((oid, number, user_email, status, currency, total, created_at)) = row else {
        return Ok(None);
    };
    let lines = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(id))
        .order_by_asc(order_orderline::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(Some((
        OrderHeader {
            id: oid,
            number,
            user_email,
            status,
            currency,
            total_gross_amount: total,
            created_at,
        },
        lines,
    )))
}

/// Rebuild the domain Order from Django rows.
pub fn to_domain(
    o: &OrderHeader,
    lines: &[order_orderline::Model],
    channel_slug: &str,
) -> Order {
    Order {
        id: o.id.to_string(),
        number: o.number.to_string(),
        channel: channel_slug.to_string(),
        email: o.user_email.clone(),
        status: OrderStatus::from_str(&o.status),
        lines: lines
            .iter()
            .map(|l| OrderLine {
                variant_id: l.variant_id.map(|v| v.to_string()).unwrap_or_default(),
                product_name: format!("{} ({})", l.product_name, l.variant_name),
                quantity: l.quantity,
                unit_price: Money::new(l.unit_price_net_amount, l.currency.clone()),
                total_price: Money::new(l.total_price_net_amount, l.currency.clone()),
            })
            .collect(),
        total: Money::new(o.total_gross_amount, o.currency.clone()),
        currency: o.currency.clone(),
        created_at: o.created_at.into(),
    }
}

/// Mint a persisted order from a domain checkout. Returns the domain Order
/// (id == Django row id, number == sequence value).
///
/// Status mirrors Django's order creation: channels with
/// `automatically_confirm_all_new_orders` (default true) mint
/// `unfulfilled`, others mint `unconfirmed`.
pub async fn mint_from_checkout(
    db: &impl sea_orm::ConnectionTrait,
    checkout: &Checkout,
    channel_id: i32,
    channel_slug: &str,
    pricing: &std::collections::HashMap<i32, (Money, String)>,
) -> Result<Order> {
    use crate::entities::channel_channel;

    let email = checkout.email.clone();
    let currency = checkout.currency.clone();
    let total = checkout.total();
    let auto_confirm: Option<bool> = channel_channel::Entity::find_by_id(channel_id)
        .select_only()
        .column(channel_channel::Column::AutomaticallyConfirmAllNewOrders)
        .into_tuple::<Option<bool>>()
        .one(db)
        .await?
        .flatten();
    let status = match auto_confirm {
        Some(false) => OrderStatus::Unconfirmed,
        _ => OrderStatus::Unfulfilled,
    };
    let new = NewOrder {
        channel_id,
        channel_slug: channel_slug.to_string(),
        email,
        currency: currency.clone(),
        language_code: "en".to_string(),
        checkout_token: checkout.id.parse().unwrap_or(Uuid::nil()),
        user_id: None,
        status,
    };
    let (id, number) =
        create_order_row(db, &new, total.amount, checkout.lines.len() as i32).await?;

    let vids: Vec<i32> = checkout
        .lines
        .iter()
        .filter_map(|l| l.variant_id.parse::<i32>().ok())
        .collect();
    let details = variant_details(db, &vids).await?;
    let mut domain_lines = Vec::new();
    for line in &checkout.lines {
        let vid: i32 = line.variant_id.parse().map_err(|_| {
            DbError::SeaOrm(sea_orm::DbErr::Custom(format!("bad variant {}", line.variant_id)))
        })?;
        let (unit, name) = pricing.get(&vid).ok_or_else(|| {
            DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(format!("variant {vid}")))
        })?;
        let detail = details.get(&vid).ok_or_else(|| {
            DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(format!("variant {vid}")))
        })?;
        create_line_row(db, id, detail, line.quantity, unit.amount, &currency).await?;
        domain_lines.push(OrderLine {
            variant_id: line.variant_id.clone(),
            product_name: name.clone(),
            quantity: line.quantity,
            unit_price: unit.clone(),
            total_price: Money::new(
                unit.amount * Decimal::from(line.quantity),
                currency.clone(),
            ),
        });
    }
    Ok(Order {
        id: id.to_string(),
        number: number.to_string(),
        channel: channel_slug.to_string(),
        email: new.email,
        status,
        lines: domain_lines,
        total,
        currency,
        created_at: Utc::now(),
    })
}
