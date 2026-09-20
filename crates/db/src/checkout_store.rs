//! Checkout writes on the Django tables (`checkout_checkout`,
//! `checkout_checkoutline`).
//!
//! Django parity notes (`saleor/checkout/models.py`, settings):
//! - PK of checkout is **`token` (UUID)** — also the public identifier Django
//!   exposes in URLs/APIs, so it doubles as our gRPC id (zero friction).
//! - Line PK is **`id` (UUID)**; `quantity` has `CHECK (quantity >= 0)` plus
//!   the app-level `MinValueValidator(1)` we enforce before insert.
//! - Money columns are `numeric(20,3)`; pre-tax lines store net == gross with
//!   `tax_rate = 0`, exactly like Django's base calculations.
//! - `country` default `"US"`, `language_code` `"en"`,
//!   statuses `"none"`, `metadata`/`private_metadata` `'{}'`.
//! - Same variant added twice **merges** into one line (Django `get_line`
//!   behavior) instead of duplicating rows.
//! - Denormalized `total_*`/`subtotal_*`/`base_*` columns are refreshed on
//!   every write, so Django readers see correct totals without recompute.
//! - All multi-statement flows run inside one transaction; every function
//!   takes `impl ConnectionTrait` so callers pass either a pool connection
//!   or an open transaction.

use chrono::Utc;
use rust_decimal::Decimal;
use rustygod_core::{checkout::Checkout, money::Money};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{checkout_checkout, checkout_checkoutline},
    DbError, Result,
};

fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}

fn empty_meta() -> serde_json::Value {
    json!({})
}

/// One line to add: resolved unit price, validated quantity.
pub struct NewLine {
    pub variant_id: i32,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub price_override: Option<Decimal>,
}

/// Insert a `checkout_checkout` row. Returns the Django token (PK).
pub async fn create_checkout_row(
    db: &impl sea_orm::ConnectionTrait,
    channel_id: i32,
    channel_currency: &str,
    email: &str,
) -> Result<Uuid> {
    let token = Uuid::new_v4();
    let t = now();
    let row = checkout_checkout::ActiveModel {
        token: Set(token),
        created_at: Set(t.into()),
        last_change: Set(t.into()),
        email: Set(if email.is_empty() { None } else { Some(email.to_string()) }),
        channel_id: Set(channel_id),
        currency: Set(channel_currency.to_string()),
        country: Set("US".to_string()),
        language_code: Set("en".to_string()),
        note: Set(String::new()),
        metadata: Set(Some(empty_meta())),
        private_metadata: Set(Some(empty_meta())),
        authorize_status: Set("none".to_string()),
        charge_status: Set("none".to_string()),
        price_expiration: Set(t.into()),
        discount_expiration: Set(t.into()),
        discount_amount: Set(Decimal::ZERO),
        total_net_amount: Set(Decimal::ZERO),
        total_gross_amount: Set(Decimal::ZERO),
        base_total_amount: Set(Decimal::ZERO),
        subtotal_net_amount: Set(Decimal::ZERO),
        subtotal_gross_amount: Set(Decimal::ZERO),
        base_subtotal_amount: Set(Decimal::ZERO),
        shipping_price_net_amount: Set(Decimal::ZERO),
        shipping_price_gross_amount: Set(Decimal::ZERO),
        shipping_tax_rate: Set(Decimal::ZERO),
        undiscounted_base_shipping_price_amount: Set(Decimal::ZERO),
        tax_exemption: Set(false),
        automatically_refundable: Set(false),
        is_voucher_usage_increased: Set(false),
        save_billing_address: Set(true),
        save_shipping_address: Set(true),
        search_index_dirty: Set(true),
        ..Default::default()
    };
    row.insert(db).await?;
    Ok(token)
}

/// Insert one `checkout_checkoutline` row. Totals are pre-tax (net == gross),
/// mirroring Django's base line calculation before tax plugins run.
pub async fn add_line_row(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
    variant_id: i32,
    quantity: i32,
    unit_price: Decimal,
    currency: &str,
    price_override: Option<Decimal>,
) -> Result<Uuid> {
    validate_quantity(quantity)?;
    let line_total = unit_price * Decimal::from(quantity);
    let id = Uuid::new_v4();
    let row = checkout_checkoutline::ActiveModel {
        id: Set(id),
        checkout_id: Set(checkout_token),
        variant_id: Set(variant_id),
        quantity: Set(quantity),
        currency: Set(currency.to_string()),
        price_override: Set(price_override),
        undiscounted_unit_price_amount: Set(unit_price),
        total_price_net_amount: Set(line_total),
        total_price_gross_amount: Set(line_total),
        tax_rate: Set(Decimal::ZERO),
        is_gift: Set(false),
        metadata: Set(empty_meta()),
        private_metadata: Set(empty_meta()),
        created_at: Set(now().into()),
        ..Default::default()
    };
    row.insert(db).await?;
    Ok(id)
}

fn validate_quantity(quantity: i32) -> Result<()> {
    if quantity < 1 {
        return Err(DbError::SeaOrm(sea_orm::DbErr::Custom(
            "quantity must be >= 1 (MinValueValidator)".to_string(),
        )));
    }
    Ok(())
}

/// Add lines **transactionally**: same-variant lines merge (quantity bump +
/// totals recompute, Django `get_line` semantics), new variants insert,
/// catalogue promotions evaluate per line (best rule wins, discount rows
/// written), and denormalized checkout totals refresh — all atomically.
pub async fn add_lines_tx(
    db: &DatabaseConnection,
    checkout_token: Uuid,
    channel_id: i32,
    currency: &str,
    items: &[NewLine],
) -> Result<()> {
    use sea_orm::TransactionTrait;
    for item in items {
        validate_quantity(item.quantity)?;
    }
    let txn = db.begin().await?;
    for item in items {
        // Merge target: same non-gift variant, no override (overridden lines
        // stay separate, mirroring Django where differing `data`/override
        // means a new line; gift lines are promotion-managed and never
        // absorb manual adds — Django keeps `is_gift` lines separate too).
        let existing = checkout_checkoutline::Entity::find()
            .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
            .filter(checkout_checkoutline::Column::VariantId.eq(item.variant_id))
            .filter(checkout_checkoutline::Column::IsGift.eq(false))
            .filter(checkout_checkoutline::Column::PriceOverride.is_null())
            .order_by_asc(checkout_checkoutline::Column::CreatedAt)
            .one(&txn)
            .await?;
        let line_id = match (existing, item.price_override) {
            (Some(line), None) => {
                let qty = line.quantity + item.quantity;
                let total = line.undiscounted_unit_price_amount * Decimal::from(qty);
                let lid = line.id;
                let mut am: checkout_checkoutline::ActiveModel = line.into();
                am.quantity = Set(qty);
                am.total_price_net_amount = Set(total);
                am.total_price_gross_amount = Set(total);
                am.update(&txn).await?;
                lid
            }
            _ => {
                add_line_row(
                    &txn,
                    checkout_token,
                    item.variant_id,
                    item.quantity,
                    item.unit_price,
                    currency,
                    item.price_override,
                )
                .await?
            }
        };
        // Catalogue promotion evaluation per line (best rule wins).
        evaluate_line_promotion(&txn, channel_id, line_id, currency).await?;
    }
    refresh_totals(&txn, checkout_token).await?;
    txn.commit().await?;
    Ok(())
}

/// Evaluate catalogue promotions for one line and sync its discount row.
async fn evaluate_line_promotion(
    db: &impl sea_orm::ConnectionTrait,
    channel_id: i32,
    line_id: Uuid,
    currency: &str,
) -> Result<()> {
    use crate::entities::product_productvariant;
    let line = checkout_checkoutline::Entity::find_by_id(line_id)
        .one(db)
        .await?
        .ok_or_else(|| DbError::CheckoutNotFound(line_id.to_string()))?;
    let product_id: Option<i32> = product_productvariant::Entity::find_by_id(line.variant_id)
        .select_only()
        .column(product_productvariant::Column::ProductId)
        .into_tuple::<i32>()
        .one(db)
        .await?;
    let Some(pid) = product_id else {
        crate::promotions::clear_line_promotion_discounts(db, line_id).await?;
        return Ok(());
    };
    let unit = line.price_override.unwrap_or(line.undiscounted_unit_price_amount);
    match crate::promotions::evaluate_line(
        db,
        channel_id,
        line.variant_id,
        pid,
        unit,
        line.quantity,
        currency,
    )
    .await?
    {
        Some(eval) => {
            crate::promotions::write_line_promotion_discount(db, line_id, currency, &eval).await?
        }
        None => crate::promotions::clear_line_promotion_discounts(db, line_id).await?,
    }
    Ok(())
}

/// Recompute denormalized totals from lines (pre-tax: net == gross),
/// subtracting catalogue promotion, order promotion (T3), and voucher
/// discounts — the totals Django recalculates after line mutations.
/// Floored at zero like Django's `max(total - discounts, zero)`.
/// Gift lines total 0 by construction, so they need no term.
pub async fn refresh_totals(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
) -> Result<()> {
    // Order promotions re-evaluate on every totals refresh (single choke
    // point: every line mutation lands here, like Django's checkout
    // recalculation on refresh). Errors propagate — a broken promotion
    // must fail the mutation loudly, never charge a stale total.
    crate::order_promotions::refresh_order_promotion_in(db, checkout_token).await?;
    let lines = checkout_checkoutline::Entity::find()
        .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
        .all(db)
        .await?;
    let lines_sum: Decimal = lines
        .iter()
        .map(|l| l.total_price_gross_amount)
        .sum();
    let promo = crate::promotions::checkout_promotion_total(db, checkout_token).await?;
    let order_promo = crate::order_promotions::order_promotion_total(db, checkout_token).await?;
    let line_vouchers = crate::promotions::lines_voucher_total(db, checkout_token).await?;
    let order_vouchers = crate::promotions::checkout_voucher_total(db, checkout_token).await?;
    let total = (lines_sum - promo - order_promo - line_vouchers - order_vouchers).max(Decimal::ZERO);
    let mut co: checkout_checkout::ActiveModel = checkout_checkout::Entity::find_by_id(checkout_token)
        .one(db)
        .await?
        .ok_or_else(|| DbError::CheckoutNotFound(checkout_token.to_string()))?
        .into();
    co.total_net_amount = Set(total);
    co.total_gross_amount = Set(total);
    co.base_total_amount = Set(total);
    co.subtotal_net_amount = Set(total);
    co.subtotal_gross_amount = Set(total);
    co.base_subtotal_amount = Set(total);
    co.last_change = Set(now().into());
    co.update(db).await?;
    Ok(())
}

/// Load a checkout + its lines in Django ordering (`created_at`, `id`).
/// Missing token -> `Ok(None)` (mirrors `Checkout.DoesNotExist` handling).
pub async fn load_checkout(
    db: &impl sea_orm::ConnectionTrait,
    token: Uuid,
) -> Result<Option<(checkout_checkout::Model, Vec<checkout_checkoutline::Model>)>> {
    let Some(co) = checkout_checkout::Entity::find_by_id(token)
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let lines = checkout_checkoutline::Entity::find()
        .filter(checkout_checkoutline::Column::CheckoutId.eq(token))
        .order_by_asc(checkout_checkoutline::Column::CreatedAt)
        .order_by_asc(checkout_checkoutline::Column::Id)
        .all(db)
        .await?;
    Ok(Some((co, lines)))
}

/// Delete the checkout, its lines, and its discount rows. Django's FKs have
/// no DB-level `ON DELETE CASCADE` (the ORM collector deletes dependents
/// first), so the order is: line discounts → checkout discounts → lines →
/// checkout — same end state as `checkout.delete()`.
pub async fn delete_checkout_row(
    db: &impl sea_orm::ConnectionTrait,
    token: Uuid,
) -> Result<()> {
    use crate::entities::{discount_checkoutdiscount, discount_checkoutlinediscount};
    let line_ids: Vec<Uuid> = checkout_checkoutline::Entity::find()
        .select_only()
        .column(checkout_checkoutline::Column::Id)
        .filter(checkout_checkoutline::Column::CheckoutId.eq(token))
        .into_tuple::<Uuid>()
        .all(db)
        .await?;
    if !line_ids.is_empty() {
        discount_checkoutlinediscount::Entity::delete_many()
            .filter(discount_checkoutlinediscount::Column::LineId.is_in(line_ids.clone()))
            .exec(db)
            .await?;
        // Reservations reference lines (FK, no cascade): release them with
        // the checkout, or they rot and pin stock math forever.
        crate::entities::warehouse_reservation::Entity::delete_many()
            .filter(crate::entities::warehouse_reservation::Column::CheckoutLineId.is_in(line_ids))
            .exec(db)
            .await?;
    }
    discount_checkoutdiscount::Entity::delete_many()
        .filter(discount_checkoutdiscount::Column::CheckoutId.eq(token))
        .exec(db)
        .await?;
    checkout_checkoutline::Entity::delete_many()
        .filter(checkout_checkoutline::Column::CheckoutId.eq(token))
        .exec(db)
        .await?;
    checkout_checkout::Entity::delete_by_id(token)
        .exec(db)
        .await?;
    Ok(())
}

/// Rebuild the pure domain `Checkout` from Django rows. The line unit price
/// follows Saleor precedence: `price_override` wins over the stored
/// (undiscounted) unit price — same rule as
/// `calculate_base_line_unit_price`.
pub fn to_domain(
    co: &checkout_checkout::Model,
    lines: &[checkout_checkoutline::Model],
    channel_slug: &str,
) -> Checkout {
    let mut out = Checkout {
        id: co.token.to_string(),
        channel: channel_slug.to_string(),
        email: co.email.clone().unwrap_or_default(),
        currency: co.currency.clone(),
        lines: Vec::with_capacity(lines.len()),
    };
    for l in lines {
        let unit = l.price_override.unwrap_or(l.undiscounted_unit_price_amount);
        out.lines.push(rustygod_core::checkout::CheckoutLine {
            variant_id: l.variant_id.to_string(),
            quantity: l.quantity,
            unit_price: Money::new(unit, l.currency.clone()),
            is_gift: l.is_gift,
        });
    }
    out
}
