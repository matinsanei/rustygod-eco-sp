//! Channel + listing writes on Django's tables.
//!
//! Mirrors `saleor/graphql/channel/mutations.py` and the listing inputs
//! (`ProductChannelListingUpdate`, variant `ChannelListingUpdate`):
//! - create fills Django's model defaults (60-day expiry, 6-hour fund TTL,
//!   payment-flow strategies) so a Rust-made channel behaves identically;
//! - allocation strategy is a closed set; currency is a 3-letter code;
//! - delete refuses channels with orders (never orphan financial history);
//! - listing upserts carry the channel currency, like Django's listings.

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;

use crate::{
    catalog::channel_info,
    entities::{
        channel_channel, checkout_checkout, order_order, product_product,
        product_productchannellisting, product_productvariant,
        product_productvariantchannellisting,
    },
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Channel(msg.into())
}

/// Closed set — mirrors `AllocationStrategy`.
pub fn valid_allocation_strategy(s: &str) -> bool {
    matches!(s, "prioritize-sorting-order" | "prioritize-high-stock")
}

pub struct NewChannel {
    pub name: String,
    pub slug: String,
    pub currency_code: String,
    pub default_country: String,
    pub allocation_strategy: String,
}

/// Slim view (never SELECT * — INTERVAL columns mistype, like tsvector).
#[derive(Debug)]
pub struct ChannelView {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub currency_code: String,
    pub default_country: String,
    pub allocation_strategy: String,
    pub is_active: bool,
}

async fn channel_view(
    db: &impl ConnectionTrait,
    id: i32,
) -> Result<ChannelView> {
    let row: Option<(i32, String, String, String, String, String, bool)> =
        channel_channel::Entity::find_by_id(id)
            .select_only()
            .column(channel_channel::Column::Id)
            .column(channel_channel::Column::Name)
            .column(channel_channel::Column::Slug)
            .column(channel_channel::Column::CurrencyCode)
            .column(channel_channel::Column::DefaultCountry)
            .column(channel_channel::Column::AllocationStrategy)
            .column(channel_channel::Column::IsActive)
            .into_tuple()
            .one(db)
            .await?;
    row.map(|(id, name, slug, currency_code, default_country, allocation_strategy, is_active)| {
        ChannelView { id, name, slug, currency_code, default_country, allocation_strategy, is_active }
    })
    .ok_or_else(|| fail("channel vanished after write"))
}

pub async fn create_channel(
    db: &DatabaseConnection,
    new: NewChannel,
) -> Result<ChannelView> {
    if new.slug.is_empty() || new.name.is_empty() {
        return Err(fail("channel needs a name and a slug"));
    }
    if new.currency_code.len() != 3 {
        return Err(fail("currency_code must be a 3-letter ISO code"));
    }
    if new.default_country.len() != 2 {
        return Err(fail("default_country must be a 2-letter ISO code"));
    }
    let strategy = if new.allocation_strategy.is_empty() {
        "prioritize-sorting-order".to_string()
    } else {
        new.allocation_strategy
    };
    if !valid_allocation_strategy(&strategy) {
        return Err(fail("unknown allocation strategy"));
    }
    let txn = db.begin().await?;
    let taken: bool = channel_channel::Entity::find()
        .select_only()
        .column(channel_channel::Column::Id)
        .filter(channel_channel::Column::Slug.eq(&new.slug))
        .into_tuple::<i32>()
        .one(&txn)
        .await?
        .is_some();
    if taken {
        return Err(fail(format!("channel slug '{}' is taken", new.slug)));
    }
    let slug = new.slug.clone();
    let row = channel_channel::ActiveModel {
        name: Set(new.name),
        slug: Set(slug.clone()),
        is_active: Set(true),
        currency_code: Set(new.currency_code.to_uppercase()),
        default_country: Set(new.default_country.to_uppercase()),
        allocation_strategy: Set(strategy),
        automatically_confirm_all_new_orders: Set(Some(true)),
        automatically_fulfill_non_shippable_gift_card: Set(Some(true)),
        order_mark_as_paid_strategy: Set("payment_flow".to_string()),
        default_transaction_flow_strategy: Set("charge".to_string()),
        expire_orders_after: Set(None),
        // INTERVAL columns use the DB defaults ('60 days', '6 hours' — the
        // Django model defaults). Never send text: Postgres rejects it.
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        allow_unpaid_orders: Set(false),
        use_legacy_error_flow_for_checkout: Set(false),
        include_draft_order_in_voucher_usage: Set(false),
        automatically_complete_fully_paid_checkouts: Set(false),
        draft_order_line_price_freeze_period: Set(None),
        use_legacy_line_discount_propagation_for_order: Set(false),
        checkout_release_funds_cut_off_date: Set(None),
        // DB default ('6 hours') — see above.
        release_funds_for_expired_checkouts: Set(true),
        automatic_completion_delay: Set(None),
        automatic_completion_cut_off_date: Set(None),
        allow_legacy_gift_card_use: Set(false),
        ..Default::default()
    };
    // exec, not insert: no RETURNING * — INTERVAL columns can't decode
    // (same class of problem as tsvector). Re-read the slim view instead.
    channel_channel::Entity::insert(row).exec(&txn).await?;
    let id: i32 = channel_channel::Entity::find()
        .select_only()
        .column(channel_channel::Column::Id)
        .filter(channel_channel::Column::Slug.eq(&slug))
        .into_tuple()
        .one(&txn)
        .await?
        .ok_or_else(|| fail("channel vanished after insert"))?;
    txn.commit().await?;
    channel_view(db, id).await
}

pub struct ChannelPatch {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub default_country: Option<String>,
    pub allocation_strategy: Option<String>,
    pub auto_confirm: Option<bool>,
}

pub async fn update_channel(
    db: &DatabaseConnection,
    slug: &str,
    patch: ChannelPatch,
) -> Result<ChannelView> {
    let txn = db.begin().await?;
    let row = channel_channel::Entity::find()
        .filter(channel_channel::Column::Slug.eq(slug))
        .select_only()
        .column(channel_channel::Column::Id)
        .into_tuple::<i32>()
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("channel '{slug}' not found")))?;
    if let Some(s) = &patch.allocation_strategy {
        if !valid_allocation_strategy(s) {
            return Err(fail("unknown allocation strategy"));
        }
    }
    if let Some(c) = &patch.default_country {
        if c.len() != 2 {
            return Err(fail("default_country must be a 2-letter ISO code"));
        }
    }
    let mut sets: Vec<String> = vec![];
    let mut vals: Vec<sea_orm::Value> = vec![];
    let mut push = |col: &str, v: sea_orm::Value| {
        vals.push(v);
        sets.push(format!("{col} = ${}", vals.len()));
    };
    if let Some(n) = patch.name {
        if n.is_empty() {
            return Err(fail("channel name cannot be empty"));
        }
        push("name", n.into());
    }
    if let Some(a) = patch.is_active {
        push("is_active", a.into());
    }
    if let Some(c) = patch.default_country {
        push("default_country", c.to_uppercase().into());
    }
    if let Some(s) = patch.allocation_strategy {
        push("allocation_strategy", s.into());
    }
    if let Some(c) = patch.auto_confirm {
        push("automatically_confirm_all_new_orders", c.into());
    }
    if sets.is_empty() {
        return Err(fail("nothing to update"));
    }
    vals.push(row.into());
    let sql = format!(
        "UPDATE channel_channel SET {} WHERE id = ${}",
        sets.join(", "),
        vals.len()
    );
    txn.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vals,
    ))
    .await?;
    txn.commit().await?;
    channel_view(db, row).await
}

/// Delete a channel. Refuses when orders or checkouts reference it —
/// financial history is never orphaned (Django blocks this too).
pub async fn delete_channel(db: &DatabaseConnection, slug: &str) -> Result<()> {
    let txn = db.begin().await?;
    let row: i32 = channel_channel::Entity::find()
        .select_only()
        .column(channel_channel::Column::Id)
        .filter(channel_channel::Column::Slug.eq(slug))
        .into_tuple()
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("channel '{slug}' not found")))?;
    if order_order::Entity::find()
        .select_only()
        .column(order_order::Column::Id)
        .filter(order_order::Column::ChannelId.eq(row))
        .into_tuple::<uuid::Uuid>()
        .one(&txn)
        .await?
        .is_some()
    {
        return Err(fail("cannot delete a channel with orders"));
    }
    if checkout_checkout::Entity::find()
        .select_only()
        .column(checkout_checkout::Column::Token)
        .filter(checkout_checkout::Column::ChannelId.eq(row))
        .into_tuple::<uuid::Uuid>()
        .one(&txn)
        .await?
        .is_some()
    {
        return Err(fail("cannot delete a channel with checkouts"));
    }
    product_productchannellisting::Entity::delete_many()
        .filter(product_productchannellisting::Column::ChannelId.eq(row))
        .exec(&txn)
        .await?;
    product_productvariantchannellisting::Entity::delete_many()
        .filter(product_productvariantchannellisting::Column::ChannelId.eq(row))
        .exec(&txn)
        .await?;
    channel_channel::Entity::delete_by_id(row).exec(&txn).await?;
    txn.commit().await?;
    Ok(())
}

/// Publish/unpublish a product on a channel (upserts the listing row).
pub async fn set_product_listing(
    db: &DatabaseConnection,
    channel_slug: &str,
    product_id: i32,
    is_published: bool,
    visible_in_listings: bool,
) -> Result<()> {
    let (ch_id, currency) = channel_info(db, channel_slug).await?;
    let exists = product_product::Entity::find_by_id(product_id)
        .select_only()
        .column(product_product::Column::Id)
        .into_tuple::<i32>()
        .one(db)
        .await?
        .is_some();
    if !exists {
        return Err(fail(format!("product {product_id} not found")));
    }
    let existing = product_productchannellisting::Entity::find()
        .filter(product_productchannellisting::Column::ChannelId.eq(ch_id))
        .filter(product_productchannellisting::Column::ProductId.eq(product_id))
        .one(db)
        .await?;
    match existing {
        Some(row) => {
            let mut am: product_productchannellisting::ActiveModel = row.into();
            am.is_published = Set(is_published);
            am.visible_in_listings = Set(visible_in_listings);
            if is_published {
                am.published_at = Set(Some(chrono::Utc::now().into()));
            }
            am.update(db).await?;
        }
        None => {
            product_productchannellisting::ActiveModel {
                published_at: Set(is_published.then(|| chrono::Utc::now().into())),
                is_published: Set(is_published),
                currency: Set(currency),
                channel_id: Set(ch_id),
                product_id: Set(product_id),
                visible_in_listings: Set(visible_in_listings),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

/// Set a variant's channel price (upserts the listing row, channel currency).
pub async fn set_variant_price(
    db: &DatabaseConnection,
    channel_slug: &str,
    variant_id: i32,
    price: Option<Decimal>,
    cost_price: Option<Decimal>,
) -> Result<()> {
    if let Some(p) = price {
        if p < Decimal::ZERO {
            return Err(fail("price cannot be negative"));
        }
    }
    let (ch_id, currency) = channel_info(db, channel_slug).await?;
    let exists = product_productvariant::Entity::find_by_id(variant_id)
        .select_only()
        .column(product_productvariant::Column::Id)
        .into_tuple::<i32>()
        .one(db)
        .await?
        .is_some();
    if !exists {
        return Err(fail(format!("variant {variant_id} not found")));
    }
    let existing = product_productvariantchannellisting::Entity::find()
        .filter(product_productvariantchannellisting::Column::ChannelId.eq(ch_id))
        .filter(product_productvariantchannellisting::Column::VariantId.eq(variant_id))
        .one(db)
        .await?;
    match existing {
        Some(row) => {
            let mut am: product_productvariantchannellisting::ActiveModel = row.into();
            if price.is_some() {
                am.price_amount = Set(price);
            }
            if cost_price.is_some() {
                am.cost_price_amount = Set(cost_price);
            }
            am.currency = Set(currency);
            am.update(db).await?;
        }
        None => {
            product_productvariantchannellisting::ActiveModel {
                currency: Set(currency),
                channel_id: Set(ch_id),
                variant_id: Set(variant_id),
                price_amount: Set(price),
                cost_price_amount: Set(cost_price),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

/// Test cleanup: delete a channel created by tests (listings first).
/// Refuses channels that gained orders/checkouts since creation.
pub async fn delete_channel_deep(db: &DatabaseConnection, slug: &str) -> Result<()> {
    delete_channel(db, slug).await
}
