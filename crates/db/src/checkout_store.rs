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
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
    sea_query::LockType,
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
    // Saleor `SET_NULL` on TransactionItem.checkout / Payment.checkout: the
    // money trail survives its checkout, detached (Django collector NULLs
    // instead of cascading).
    use crate::entities::{payment_payment, payment_transactionitem};
    payment_transactionitem::Entity::update_many()
        .col_expr(payment_transactionitem::Column::CheckoutId, sea_orm::sea_query::Expr::value(None::<Uuid>))
        .filter(payment_transactionitem::Column::CheckoutId.eq(token))
        .exec(db)
        .await?;
    payment_payment::Entity::update_many()
        .col_expr(payment_payment::Column::CheckoutId, sea_orm::sea_query::Expr::value(None::<Uuid>))
        .filter(payment_payment::Column::CheckoutId.eq(token))
        .exec(db)
        .await?;
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

/// Delete expired checkouts — Saleor `delete_expired_checkouts` parity (E10).
/// Inactivity is `last_change`-based with Saleor's three buckets:
/// anonymous (no email AND no user) after 30d, user checkouts after 90d,
/// empty (no lines) checkouts after 6h — all configurable via
/// `RUSTYGOD_{ANONYMOUS,USER}_CHECKOUT_DAYS` / `RUSTYGOD_EMPTY_CHECKOUT_HOURS`
/// (Saleor reads the same from Django settings env). Checkouts holding
/// TransactionItem money (authorized/pending/charged/...) are never touched.
/// Deletes reuse `delete_checkout_row` (discounts + reservations + lines +
/// header, FK-safe order); each row is locked `FOR UPDATE` first like
/// Django's `delete_checkouts`. Batched 2000 x up to 5 passes per run.
pub async fn sweep_expired_checkouts(
    db: &DatabaseConnection,
) -> Result<u64> {
    fn env_u64(name: &str, dflt: u64) -> u64 {
        std::env::var(name).ok().and_then(|s| s.parse().ok()).unwrap_or(dflt)
    }
    let anon_cut = Utc::now() - chrono::Duration::days(env_u64("RUSTYGOD_ANONYMOUS_CHECKOUT_DAYS", 30) as i64);
    let user_cut = Utc::now() - chrono::Duration::days(env_u64("RUSTYGOD_USER_CHECKOUT_DAYS", 90) as i64);
    let empty_cut = Utc::now() - chrono::Duration::hours(env_u64("RUSTYGOD_EMPTY_CHECKOUT_HOURS", 6) as i64);
    let mut total: u64 = 0;
    for _ in 0..5 {
        let sel = sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT c.token FROM checkout_checkout c WHERE ( \
               (c.last_change < $1 AND c.email IS NULL AND c.user_id IS NULL) \
               OR (c.last_change < $2 AND (c.email IS NOT NULL OR c.user_id IS NOT NULL)) \
               OR (c.last_change < $3 AND NOT EXISTS \
                 (SELECT 1 FROM checkout_checkoutline l WHERE l.checkout_id = c.token)) \
             ) AND NOT EXISTS ( \
               SELECT 1 FROM payment_transactionitem t WHERE t.checkout_id = c.token \
               AND (t.authorized_value > 0 OR t.authorize_pending_value > 0 \
                 OR t.charged_value > 0 OR t.charge_pending_value > 0 \
                 OR t.refund_pending_value > 0 OR t.cancel_pending_value > 0) \
             ) ORDER BY c.last_change LIMIT 2000",
            [anon_cut.into(), user_cut.into(), empty_cut.into()],
        );
        let rows = db.query_all(sel).await?;
        if rows.is_empty() {
            break;
        }
        let mut batch: u64 = 0;
        for r in rows {
            let token: Uuid = r.try_get("", "token")?;
            let txn = db.begin().await?;
            // FOR UPDATE lock parity with Django's delete_checkouts.
            let locked = checkout_checkout::Entity::find_by_id(token)
                .lock(LockType::Update)
                .one(&txn)
                .await?;
            if locked.is_none() {
                txn.rollback().await?;
                continue;
            }
            delete_checkout_row(&txn, token).await?;
            txn.commit().await?;
            batch += 1;
        }
        total += batch;
        if batch < 2000 {
            break;
        }
    }
    Ok(total)
}

/// Outcome of one automatic-completion beat run.
#[derive(Debug, Default)]
pub struct AutoCompleteReport {
    pub attempted: u64,
    pub completed: u64,
    pub failed: u64,
}

/// Automatically complete fully-paid checkouts — Saleor
/// `trigger_automatic_checkout_completion_task` parity (E10 final).
///
/// Django-faithful selection: channel opted in
/// (`automatically_complete_fully_paid_checkouts`), checkout fully
/// authorized, idle past the channel's `automatic_completion_delay`
/// (minutes, default 0), created after the channel's cut-off date,
/// modified within the safety window (30d default — very old checkouts are
/// never auto-completed), has email or user, has a billing address, has a
/// positive total and at least one line. Ordering is attempt-time
/// nulls-first (never-tried wins) then `last_change`, like Django's
/// `order_by(F(attempt).asc(nulls_first=True), "last_change")`.
///
/// Each candidate stamps `last_automatic_completion_attempt` in its own
/// transaction BEFORE trying (Django does the same), then runs the shared
/// `complete::complete_checkout` — idempotent via the order's checkout
/// token, so a replayed beat never double-mints. One bad checkout never
/// aborts the batch; failures are counted and retried after the retry
/// window (24h default). Tunables: `RUSTYGOD_AUTO_COMPLETE_BATCH` (20),
/// `RUSTYGOD_AUTO_COMPLETE_MAX_AGE_DAYS` (30),
/// `RUSTYGOD_AUTO_COMPLETE_RETRY_HOURS` (24).
pub async fn auto_complete_expired_checkouts(
    db: &DatabaseConnection,
) -> Result<AutoCompleteReport> {
    fn env_u64(name: &str, dflt: u64) -> u64 {
        std::env::var(name).ok().and_then(|s| s.parse().ok()).unwrap_or(dflt)
    }
    let batch = env_u64("RUSTYGOD_AUTO_COMPLETE_BATCH", 20).clamp(1, 500) as i64;
    let oldest_cut =
        Utc::now() - chrono::Duration::days(env_u64("RUSTYGOD_AUTO_COMPLETE_MAX_AGE_DAYS", 30) as i64);
    let retry_cut =
        Utc::now() - chrono::Duration::hours(env_u64("RUSTYGOD_AUTO_COMPLETE_RETRY_HOURS", 24) as i64);
    let sel = sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT c.token FROM checkout_checkout c \
         JOIN channel_channel ch ON ch.id = c.channel_id \
         WHERE ch.automatically_complete_fully_paid_checkouts \
           AND c.authorize_status = 'full' \
           AND c.last_change < now() - make_interval(mins => COALESCE(ch.automatic_completion_delay, 0)) \
           AND c.last_change >= $1 \
           AND (c.email IS NOT NULL OR c.user_id IS NOT NULL) \
           AND c.billing_address_id IS NOT NULL \
           AND c.total_gross_amount > 0 \
           AND EXISTS (SELECT 1 FROM checkout_checkoutline l WHERE l.checkout_id = c.token) \
           AND (ch.automatic_completion_cut_off_date IS NULL OR c.created_at >= ch.automatic_completion_cut_off_date) \
           AND (c.last_automatic_completion_attempt IS NULL OR c.last_automatic_completion_attempt < $2) \
         ORDER BY c.last_automatic_completion_attempt NULLS FIRST, c.last_change \
         LIMIT $3",
        [oldest_cut.into(), retry_cut.into(), batch.into()],
    );
    let mut report = AutoCompleteReport::default();
    for r in db.query_all(sel).await? {
        let token: Uuid = r.try_get("", "token")?;
        // Stamp the attempt first, in its own transaction — a crash between
        // stamp and completion still leaves the audit trail, and the retry
        // window (not a lock) paces the next try.
        let txn = db.begin().await?;
        let locked = checkout_checkout::Entity::find_by_id(token)
            .lock(LockType::Update)
            .one(&txn)
            .await?;
        let Some(m) = locked else {
            txn.rollback().await?;
            continue;
        };
        let mut am: checkout_checkout::ActiveModel = m.into();
        am.last_automatic_completion_attempt = Set(Some(Utc::now().into()));
        am.update(&txn).await?;
        txn.commit().await?;
        report.attempted += 1;
        match crate::complete::complete_checkout(db, token).await {
            Ok(_) => report.completed += 1,
            // One bad checkout never aborts the batch; the stamped attempt
            // paces the retry (RETRY_HOURS) and `failed` is surfaced by the
            // beat log line.
            Err(_) => report.failed += 1,
        }
    }
    Ok(report)
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

/// New-checkout-API row writers (Saleor `checkout*` mutations parity).
/// Every writer refreshes totals via the caller (GQL layer calls
/// `refresh_totals` once per mutation, like Django's recalculation).

/// Update a line's quantity (0 deletes the line, like Django).
pub async fn update_line_quantity(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
    line_id: Uuid,
    quantity: i32,
) -> Result<()> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
    let Some(l) = checkout_checkoutline::Entity::find_by_id(line_id)
        .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
        .one(db)
        .await?
    else {
        return Err(DbError::CheckoutNotFound(format!("line {line_id}")));
    };
    if quantity <= 0 {
        return delete_line(db, checkout_token, line_id).await;
    }
    let mut am: checkout_checkoutline::ActiveModel = l.into();
    am.quantity = Set(quantity);
    am.update(db).await?;
    Ok(())
}

/// Delete one line (no-op when missing — Django is idempotent here).
pub async fn delete_line(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
    line_id: Uuid,
) -> Result<()> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    checkout_checkoutline::Entity::delete_many()
        .filter(checkout_checkoutline::Column::Id.eq(line_id))
        .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
        .exec(db)
        .await?;
    Ok(())
}

macro_rules! checkout_setter {
    ($name:ident, $col:ident, $ty:ty) => {
        pub async fn $name(
            db: &impl sea_orm::ConnectionTrait,
            checkout_token: Uuid,
            value: $ty,
        ) -> Result<()> {
            use sea_orm::EntityTrait;
            let Some(co) = checkout_checkout::Entity::find_by_id(checkout_token).one(db).await?
            else {
                return Err(DbError::CheckoutNotFound(checkout_token.to_string()));
            };
            let mut am: checkout_checkout::ActiveModel = co.into();
            am.$col = Set(value);
            am.update(db).await?;
            Ok(())
        }
    };
}

checkout_setter!(set_email_opt, email, Option<String>);
checkout_setter!(set_note_text, note, String);
checkout_setter!(set_language_code, language_code, String);
checkout_setter!(set_customer_opt, user_id, Option<i32>);
checkout_setter!(set_shipping_method_opt, shipping_method_id, Option<i32>);
checkout_setter!(set_collection_point_opt, collection_point_id, Option<Uuid>);
checkout_setter!(set_metadata_opt, metadata, Option<serde_json::Value>);

/// Standalone address row for a checkout side (Django checkout addresses
/// are owned rows, not address-book links).
pub async fn set_checkout_address(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
    billing: bool,
    first_name: &str,
    last_name: &str,
    street_1: &str,
    city: &str,
    postal_code: &str,
    country: &str,
) -> Result<i32> {
    use sea_orm::EntityTrait;
    let aid = crate::entities::account_address::ActiveModel {
        first_name: Set(first_name.to_string()),
        last_name: Set(last_name.to_string()),
        company_name: Set(String::new()),
        street_address_1: Set(street_1.to_string()),
        street_address_2: Set(String::new()),
        city: Set(city.to_string()),
        postal_code: Set(postal_code.to_string()),
        country: Set(country.to_string()),
        country_area: Set(String::new()),
        city_area: Set(String::new()),
        phone: Set(String::new()),
        metadata: Set(serde_json::json!({})),
        private_metadata: Set(serde_json::json!({})),
        validation_skipped: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await?
    .id;
    let Some(co) = checkout_checkout::Entity::find_by_id(checkout_token).one(db).await?
    else {
        return Err(DbError::CheckoutNotFound(checkout_token.to_string()));
    };
    let mut am: checkout_checkout::ActiveModel = co.into();
    if billing {
        am.billing_address_id = Set(Some(aid));
    } else {
        am.shipping_address_id = Set(Some(aid));
    }
    am.update(db).await?;
    Ok(aid)
}

/// Clear the voucher code (Django `removePromoCode` for vouchers; usage is
/// only increased at completion, so nothing else unwinds).
pub async fn remove_voucher_code(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
    code: Option<&str>,
) -> Result<bool> {
    use sea_orm::EntityTrait;
    let Some(co) = checkout_checkout::Entity::find_by_id(checkout_token).one(db).await?
    else {
        return Err(DbError::CheckoutNotFound(checkout_token.to_string()));
    };
    let cur = co.voucher_code.clone().unwrap_or_default();
    if code.is_some_and(|c| !cur.eq_ignore_ascii_case(c)) {
        return Ok(false);
    }
    if co.voucher_code.is_none() {
        return Ok(false);
    }
    let mut am: checkout_checkout::ActiveModel = co.into();
    am.voucher_code = Set(None);
    am.update(db).await?;
    Ok(true)
}
