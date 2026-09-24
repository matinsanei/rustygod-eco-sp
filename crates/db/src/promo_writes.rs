//! Promotion + voucher writes (Django `discount` mutations parity).
//!
//! Django references (`saleor/graphql/discount/mutations/`):
//! - promotions own UUID pks; rules link channels via
//!   `discount_promotionrule_channels`, gifts via `..._gifts`, explicit
//!   variants via `..._variants`;
//! - voucher `type` is inferred (catalogue assigned → SPECIFIC_PRODUCT,
//!   else ENTIRE_ORDER) — the 3.23 input carries no type field;
//! - voucher codes live in `discount_vouchercode` (many per voucher);
//! - deletes cascade through rule/catalogue/listing/code rows (Django's
//!   collector; no DB cascades here, so explicit).

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect,
    Set, TransactionTrait,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    entities::{
        discount_promotion, discount_promotionrule, discount_promotionrule_channels,
        discount_promotionrule_gifts, discount_promotionrule_variants, discount_voucher,
        discount_voucher_categories, discount_voucher_collections, discount_voucher_products,
        discount_voucher_variants, discount_vouchercode, discount_voucherchannellisting,
    },
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::App(format!("promotion error: {}", msg.into()))
}

#[derive(Default, Clone)]
pub struct NewRule {
    pub name: Option<String>,
    pub description: Value,
    pub catalogue_predicate: Value,
    pub order_predicate: Value,
    pub reward_value_type: Option<String>,
    pub reward_value: Option<Decimal>,
    pub reward_type: Option<String>,
    pub channel_ids: Vec<i32>,
    pub gift_variant_ids: Vec<i32>,
}

async fn insert_rule(
    txn: &impl ConnectionTrait,
    promotion_id: Uuid,
    r: &NewRule,
) -> Result<Uuid> {
    if let Some(v) = r.reward_value {
        if v <= Decimal::ZERO {
            return Err(fail("reward value must be positive"));
        }
    }
    let id = Uuid::new_v4();
    discount_promotionrule::ActiveModel {
        id: Set(id),
        name: Set(r.name.clone()),
        description: Set(Some(r.description.clone())),
        catalogue_predicate: Set(r.catalogue_predicate.clone()),
        order_predicate: Set(r.order_predicate.clone()),
        reward_value_type: Set(r.reward_value_type.clone()),
        reward_value: Set(r.reward_value),
        promotion_id: Set(promotion_id),
        reward_type: Set(r.reward_type.clone()),
        ..Default::default()
    }
    .insert(txn)
    .await
    .map_err(DbError::SeaOrm)?;
    for ch in &r.channel_ids {
        discount_promotionrule_channels::ActiveModel {
            promotionrule_id: Set(id),
            channel_id: Set(*ch),
            ..Default::default()
        }
        .insert(txn)
        .await
        .map_err(DbError::SeaOrm)?;
    }
    for gv in &r.gift_variant_ids {
        discount_promotionrule_gifts::ActiveModel {
            promotionrule_id: Set(id),
            productvariant_id: Set(*gv),
            ..Default::default()
        }
        .insert(txn)
        .await
        .map_err(DbError::SeaOrm)?;
    }
    Ok(id)
}

/// Create a promotion with its rules (Django `promotionCreate`).
pub async fn create_promotion(
    db: &sea_orm::DatabaseConnection,
    name: &str,
    promo_type: &str,
    description: Value,
    start: Option<chrono::DateTime<Utc>>,
    end: Option<chrono::DateTime<Utc>>,
    rules: Vec<NewRule>,
) -> Result<Uuid> {
    if name.trim().is_empty() {
        return Err(fail("name is required"));
    }
    if let (Some(s), Some(e)) = (start, end) {
        if e < s {
            return Err(fail("end date cannot precede start date"));
        }
    }
    let txn = db.begin().await?;
    let id = Uuid::new_v4();
    let t = Utc::now();
    discount_promotion::ActiveModel {
        id: Set(id),
        name: Set(name.trim().to_string()),
        description: Set(Some(description)),
        start_date: Set(start.map(|d| d.into()).unwrap_or_else(|| t.into())),
        end_date: Set(end.map(|d| d.into())),
        created_at: Set(t.into()),
        updated_at: Set(t.into()),
        r#type: Set(promo_type.to_string()),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(DbError::SeaOrm)?;
    for r in &rules {
        insert_rule(&txn, id, r).await?;
    }
    txn.commit().await?;
    Ok(id)
}

/// Update promotion header fields (Django `promotionUpdate`).
pub async fn update_promotion(
    db: &sea_orm::DatabaseConnection,
    id: Uuid,
    name: Option<String>,
    description: Option<Value>,
    start: Option<chrono::DateTime<Utc>>,
    end: Option<Option<chrono::DateTime<Utc>>>,
) -> Result<()> {
    let txn = db.begin().await?;
    let p = discount_promotion::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("promotion {id} not found")))?;
    let new_start: chrono::DateTime<Utc> = start.unwrap_or_else(|| p.start_date.with_timezone(&Utc));
    let new_end: Option<chrono::DateTime<Utc>> = match end {
        Some(e) => e,
        None => p.end_date.map(|d| d.with_timezone(&Utc)),
    };
    if let Some(e) = new_end {
        if e < new_start {
            return Err(fail("end date cannot precede start date"));
        }
    }
    let mut am: discount_promotion::ActiveModel = p.into();
    if let Some(n) = name {
        if n.trim().is_empty() {
            return Err(fail("name cannot be empty"));
        }
        am.name = Set(n.trim().to_string());
    }
    if let Some(d) = description {
        am.description = Set(Some(d));
    }
    if let Some(s) = start {
        am.start_date = Set(s.into());
    }
    if let Some(e) = end {
        am.end_date = Set(e.map(|d| d.into()));
    }
    am.updated_at = Set(Utc::now().into());
    am.update(&txn).await?;
    txn.commit().await?;
    Ok(())
}

async fn delete_rule_in(txn: &impl ConnectionTrait, id: Uuid) -> Result<()> {
    discount_promotionrule_channels::Entity::delete_many()
        .filter(discount_promotionrule_channels::Column::PromotionruleId.eq(id))
        .exec(txn)
        .await?;
    discount_promotionrule_gifts::Entity::delete_many()
        .filter(discount_promotionrule_gifts::Column::PromotionruleId.eq(id))
        .exec(txn)
        .await?;
    discount_promotionrule_variants::Entity::delete_many()
        .filter(discount_promotionrule_variants::Column::PromotionruleId.eq(id))
        .exec(txn)
        .await?;
    if let Some(r) = discount_promotionrule::Entity::find_by_id(id).one(txn).await? {
        let am: discount_promotionrule::ActiveModel = r.into();
        am.delete(txn).await?;
    }
    Ok(())
}

/// Delete a promotion with rules + links (Django `promotionDelete`).
pub async fn delete_promotion(db: &sea_orm::DatabaseConnection, id: Uuid) -> Result<()> {
    let txn = db.begin().await?;
    if discount_promotion::Entity::find_by_id(id).one(&txn).await?.is_none() {
        return Err(fail(format!("promotion {id} not found")));
    }
    let rules: Vec<Uuid> = discount_promotionrule::Entity::find()
        .select_only()
        .column(discount_promotionrule::Column::Id)
        .filter(discount_promotionrule::Column::PromotionId.eq(id))
        .into_tuple()
        .all(&txn)
        .await?;
    for r in rules {
        delete_rule_in(&txn, r).await?;
    }
    if let Some(p) = discount_promotion::Entity::find_by_id(id).one(&txn).await? {
        let am: discount_promotion::ActiveModel = p.into();
        am.delete(&txn).await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Bulk promotion delete (Django `promotionBulkDelete`): survivors commit.
pub async fn bulk_delete_promotions(db: &sea_orm::DatabaseConnection, ids: &[Uuid]) -> Result<i32> {
    let mut n = 0;
    for id in ids {
        if delete_promotion(db, *id).await.is_ok() {
            n += 1;
        }
    }
    Ok(n)
}

/// Create one rule on a promotion (Django `promotionRuleCreate`).
pub async fn create_rule(
    db: &sea_orm::DatabaseConnection,
    promotion_id: Uuid,
    rule: &NewRule,
) -> Result<Uuid> {
    if discount_promotion::Entity::find_by_id(promotion_id).one(db).await?.is_none() {
        return Err(fail(format!("promotion {promotion_id} not found")));
    }
    let txn = db.begin().await?;
    let id = insert_rule(&txn, promotion_id, rule).await?;
    txn.commit().await?;
    Ok(id)
}

#[derive(Default)]
pub struct UpdateRule {
    pub name: Option<String>,
    pub description: Option<Value>,
    pub catalogue_predicate: Option<Value>,
    pub order_predicate: Option<Value>,
    pub reward_value_type: Option<Option<String>>,
    pub reward_value: Option<Option<Decimal>>,
    pub reward_type: Option<Option<String>>,
    pub add_channels: Vec<i32>,
    pub remove_channels: Vec<i32>,
    pub add_gifts: Vec<i32>,
    pub remove_gifts: Vec<i32>,
}

/// Update a rule (Django `promotionRuleUpdate`): header + channel/gift links.
pub async fn update_rule(
    db: &sea_orm::DatabaseConnection,
    id: Uuid,
    upd: &UpdateRule,
) -> Result<()> {
    let txn = db.begin().await?;
    let r = discount_promotionrule::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("promotion rule {id} not found")))?;
    let mut am: discount_promotionrule::ActiveModel = r.into();
    if let Some(n) = upd.name.clone() {
        am.name = Set(Some(n));
    }
    if let Some(d) = upd.description.clone() {
        am.description = Set(Some(d));
    }
    if let Some(p) = upd.catalogue_predicate.clone() {
        am.catalogue_predicate = Set(p);
    }
    if let Some(p) = upd.order_predicate.clone() {
        am.order_predicate = Set(p);
    }
    if let Some(v) = upd.reward_value_type.clone() {
        am.reward_value_type = Set(v);
    }
    if let Some(v) = upd.reward_value {
        if let Some(x) = v {
            if x <= Decimal::ZERO {
                return Err(fail("reward value must be positive"));
            }
        }
        am.reward_value = Set(v);
    }
    if let Some(v) = upd.reward_type.clone() {
        am.reward_type = Set(v);
    }
    am.update(&txn).await?;
    for ch in &upd.add_channels {
        let exists = discount_promotionrule_channels::Entity::find()
            .filter(discount_promotionrule_channels::Column::PromotionruleId.eq(id))
            .filter(discount_promotionrule_channels::Column::ChannelId.eq(*ch))
            .one(&txn)
            .await?
            .is_some();
        if !exists {
            discount_promotionrule_channels::ActiveModel {
                promotionrule_id: Set(id),
                channel_id: Set(*ch),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }
    if !upd.remove_channels.is_empty() {
        discount_promotionrule_channels::Entity::delete_many()
            .filter(discount_promotionrule_channels::Column::PromotionruleId.eq(id))
            .filter(discount_promotionrule_channels::Column::ChannelId.is_in(upd.remove_channels.clone()))
            .exec(&txn)
            .await?;
    }
    for gv in &upd.add_gifts {
        let exists = discount_promotionrule_gifts::Entity::find()
            .filter(discount_promotionrule_gifts::Column::PromotionruleId.eq(id))
            .filter(discount_promotionrule_gifts::Column::ProductvariantId.eq(*gv))
            .one(&txn)
            .await?
            .is_some();
        if !exists {
            discount_promotionrule_gifts::ActiveModel {
                promotionrule_id: Set(id),
                productvariant_id: Set(*gv),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }
    if !upd.remove_gifts.is_empty() {
        discount_promotionrule_gifts::Entity::delete_many()
            .filter(discount_promotionrule_gifts::Column::PromotionruleId.eq(id))
            .filter(discount_promotionrule_gifts::Column::ProductvariantId.is_in(upd.remove_gifts.clone()))
            .exec(&txn)
            .await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Delete one rule (Django `promotionRuleDelete`).
pub async fn delete_rule(db: &sea_orm::DatabaseConnection, id: Uuid) -> Result<()> {
    let txn = db.begin().await?;
    if discount_promotionrule::Entity::find_by_id(id).one(&txn).await?.is_none() {
        return Err(fail(format!("promotion rule {id} not found")));
    }
    delete_rule_in(&txn, id).await?;
    txn.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------- vouchers --

#[derive(Default, Clone)]
pub struct ChannelListingInput {
    pub channel_id: i32,
    pub discount_value: Decimal,
    pub min_spent: Option<Decimal>,
}

#[derive(Default, Clone)]
pub struct NewVoucher {
    pub name: Option<String>,
    pub voucher_type: String,
    pub discount_value_type: String,
    pub products: Vec<i32>,
    pub variants: Vec<i32>,
    pub categories: Vec<i32>,
    pub collections: Vec<i32>,
    pub min_items: Option<i32>,
    pub countries: Vec<String>,
    pub apply_once_per_order: bool,
    pub apply_once_per_customer: bool,
    pub only_for_staff: bool,
    pub single_use: bool,
    pub usage_limit: Option<i32>,
    pub start: chrono::DateTime<Utc>,
    pub end: Option<chrono::DateTime<Utc>>,
    pub codes: Vec<String>,
    pub listings: Vec<ChannelListingInput>,
}

async fn catalogue_links(
    txn: &impl ConnectionTrait,
    voucher_id: i32,
    products: &[i32],
    variants: &[i32],
    categories: &[i32],
    collections: &[i32],
) -> Result<()> {
    for p in products {
        discount_voucher_products::ActiveModel {
            voucher_id: Set(voucher_id),
            product_id: Set(*p),
            ..Default::default()
        }
        .insert(txn)
        .await
        .map_err(DbError::SeaOrm)?;
    }
    for v in variants {
        discount_voucher_variants::ActiveModel {
            voucher_id: Set(voucher_id),
            productvariant_id: Set(*v),
            ..Default::default()
        }
        .insert(txn)
        .await
        .map_err(DbError::SeaOrm)?;
    }
    for c in categories {
        discount_voucher_categories::ActiveModel {
            voucher_id: Set(voucher_id),
            category_id: Set(*c),
            ..Default::default()
        }
        .insert(txn)
        .await
        .map_err(DbError::SeaOrm)?;
    }
    for c in collections {
        discount_voucher_collections::ActiveModel {
            voucher_id: Set(voucher_id),
            collection_id: Set(*c),
            ..Default::default()
        }
        .insert(txn)
        .await
        .map_err(DbError::SeaOrm)?;
    }
    Ok(())
}

async fn add_codes(
    txn: &impl ConnectionTrait,
    voucher_id: i32,
    codes: &[String],
    _currency: &str,
) -> Result<()> {
    for code in codes {
        let code = code.trim();
        if code.is_empty() {
            continue;
        }
        let exists = discount_vouchercode::Entity::find()
            .filter(discount_vouchercode::Column::Code.eq(code))
            .one(txn)
            .await?
            .is_some();
        if exists {
            return Err(fail(format!("voucher code {code} already exists")));
        }
        discount_vouchercode::ActiveModel {
            id: Set(Uuid::new_v4()),
            code: Set(code.to_string()),
            voucher_id: Set(voucher_id),
            is_active: Set(true),
            used: Set(0),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        }
        .insert(txn)
        .await
        .map_err(DbError::SeaOrm)?;
    }
    Ok(())
}

/// Create a voucher with catalogue, codes and listings (Django `voucherCreate`).
pub async fn create_voucher(
    db: &sea_orm::DatabaseConnection,
    v: &NewVoucher,
    currency: &str,
) -> Result<i32> {
    if v.codes.iter().all(|c| c.trim().is_empty()) {
        return Err(fail("at least one voucher code is required"));
    }
    if let Some(e) = v.end {
        if e < v.start {
            return Err(fail("end date cannot precede start date"));
        }
    }
    let txn = db.begin().await?;
    let row = discount_voucher::ActiveModel {
        r#type: Set(v.voucher_type.clone()),
        name: Set(v.name.clone()),
        usage_limit: Set(v.usage_limit),
        start_date: Set(v.start.into()),
        end_date: Set(v.end.map(|d| d.into())),
        discount_value_type: Set(v.discount_value_type.clone()),
        apply_once_per_order: Set(v.apply_once_per_order),
        countries: Set(v.countries.join(",")),
        min_checkout_items_quantity: Set(v.min_items),
        apply_once_per_customer: Set(v.apply_once_per_customer),
        only_for_staff: Set(v.only_for_staff),
        single_use: Set(v.single_use),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(DbError::SeaOrm)?;
    catalogue_links(&txn, row.id, &v.products, &v.variants, &v.categories, &v.collections).await?;
    add_codes(&txn, row.id, &v.codes, currency).await?;
    for l in &v.listings {
        discount_voucherchannellisting::ActiveModel {
            discount_value: Set(l.discount_value),
            currency: Set(currency.to_string()),
            min_spent_amount: Set(l.min_spent),
            channel_id: Set(l.channel_id),
            voucher_id: Set(row.id),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(DbError::SeaOrm)?;
    }
    txn.commit().await?;
    Ok(row.id)
}

#[derive(Default)]
pub struct UpdateVoucher {
    pub name: Option<Option<String>>,
    pub discount_value_type: Option<String>,
    pub usage_limit: Option<Option<i32>>,
    pub start: Option<chrono::DateTime<Utc>>,
    pub end: Option<Option<chrono::DateTime<Utc>>>,
    pub min_items: Option<Option<i32>>,
    pub countries: Option<Vec<String>>,
    pub apply_once_per_order: Option<bool>,
    pub apply_once_per_customer: Option<bool>,
    pub only_for_staff: Option<bool>,
    pub single_use: Option<bool>,
    pub add_codes: Vec<String>,
}

/// Update voucher header + append codes (Django `voucherUpdate`; catalogue
/// edits ride the catalogues mutations, listings ride channel-listing).
pub async fn update_voucher(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    upd: &UpdateVoucher,
    currency: &str,
) -> Result<()> {
    let txn = db.begin().await?;
    let v = discount_voucher::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("voucher {id} not found")))?;
    let mut am: discount_voucher::ActiveModel = v.into();
    if let Some(n) = upd.name.clone() {
        am.name = Set(n);
    }
    if let Some(t) = upd.discount_value_type.clone() {
        am.discount_value_type = Set(t);
    }
    if let Some(u) = upd.usage_limit {
        am.usage_limit = Set(u);
    }
    if let Some(s) = upd.start {
        am.start_date = Set(s.into());
    }
    if let Some(e) = upd.end {
        am.end_date = Set(e.map(|d| d.into()));
    }
    if let Some(m) = upd.min_items {
        am.min_checkout_items_quantity = Set(m);
    }
    if let Some(c) = upd.countries.clone() {
        am.countries = Set(c.join(","));
    }
    if let Some(x) = upd.apply_once_per_order {
        am.apply_once_per_order = Set(x);
    }
    if let Some(x) = upd.apply_once_per_customer {
        am.apply_once_per_customer = Set(x);
    }
    if let Some(x) = upd.only_for_staff {
        am.only_for_staff = Set(x);
    }
    if let Some(x) = upd.single_use {
        am.single_use = Set(x);
    }
    am.update(&txn).await?;
    add_codes(&txn, id, &upd.add_codes, currency).await?;
    txn.commit().await?;
    Ok(())
}

/// Delete a voucher with catalogue, listings and codes (Django `voucherDelete`).
pub async fn delete_voucher(db: &sea_orm::DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await?;
    if discount_voucher::Entity::find_by_id(id).one(&txn).await?.is_none() {
        return Err(fail(format!("voucher {id} not found")));
    }
    discount_voucher_products::Entity::delete_many()
        .filter(discount_voucher_products::Column::VoucherId.eq(id))
        .exec(&txn)
        .await?;
    discount_voucher_variants::Entity::delete_many()
        .filter(discount_voucher_variants::Column::VoucherId.eq(id))
        .exec(&txn)
        .await?;
    discount_voucher_categories::Entity::delete_many()
        .filter(discount_voucher_categories::Column::VoucherId.eq(id))
        .exec(&txn)
        .await?;
    discount_voucher_collections::Entity::delete_many()
        .filter(discount_voucher_collections::Column::VoucherId.eq(id))
        .exec(&txn)
        .await?;
    discount_voucherchannellisting::Entity::delete_many()
        .filter(discount_voucherchannellisting::Column::VoucherId.eq(id))
        .exec(&txn)
        .await?;
    discount_vouchercode::Entity::delete_many()
        .filter(discount_vouchercode::Column::VoucherId.eq(id))
        .exec(&txn)
        .await?;
    if let Some(v) = discount_voucher::Entity::find_by_id(id).one(&txn).await? {
        let am: discount_voucher::ActiveModel = v.into();
        am.delete(&txn).await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Add/remove catalogue rows (Django `voucherCataloguesAdd/Remove`).
#[allow(clippy::too_many_arguments)]
pub async fn voucher_catalogues(
    db: &sea_orm::DatabaseConnection,
    voucher_id: i32,
    add: bool,
    products: &[i32],
    variants: &[i32],
    categories: &[i32],
    collections: &[i32],
) -> Result<()> {
    let txn = db.begin().await?;
    if discount_voucher::Entity::find_by_id(voucher_id).one(&txn).await?.is_none() {
        return Err(fail(format!("voucher {voucher_id} not found")));
    }
    if add {
        catalogue_links(&txn, voucher_id, products, variants, categories, collections).await?;
    } else {
        if !products.is_empty() {
            discount_voucher_products::Entity::delete_many()
                .filter(discount_voucher_products::Column::VoucherId.eq(voucher_id))
                .filter(discount_voucher_products::Column::ProductId.is_in(products.to_vec()))
                .exec(&txn)
                .await?;
        }
        if !variants.is_empty() {
            discount_voucher_variants::Entity::delete_many()
                .filter(discount_voucher_variants::Column::VoucherId.eq(voucher_id))
                .filter(discount_voucher_variants::Column::ProductvariantId.is_in(variants.to_vec()))
                .exec(&txn)
                .await?;
        }
        if !categories.is_empty() {
            discount_voucher_categories::Entity::delete_many()
                .filter(discount_voucher_categories::Column::VoucherId.eq(voucher_id))
                .filter(discount_voucher_categories::Column::CategoryId.is_in(categories.to_vec()))
                .exec(&txn)
                .await?;
        }
        if !collections.is_empty() {
            discount_voucher_collections::Entity::delete_many()
                .filter(discount_voucher_collections::Column::VoucherId.eq(voucher_id))
                .filter(discount_voucher_collections::Column::CollectionId.is_in(collections.to_vec()))
                .exec(&txn)
                .await?;
        }
    }
    txn.commit().await?;
    Ok(())
}

/// Channel listings add/remove (Django `voucherChannelListingUpdate`).
pub async fn voucher_channel_listings(
    db: &sea_orm::DatabaseConnection,
    voucher_id: i32,
    add: &[ChannelListingInput],
    remove_channel_ids: &[i32],
    currency: &str,
) -> Result<()> {
    let txn = db.begin().await?;
    if discount_voucher::Entity::find_by_id(voucher_id).one(&txn).await?.is_none() {
        return Err(fail(format!("voucher {voucher_id} not found")));
    }
    for l in add {
        let existing = discount_voucherchannellisting::Entity::find()
            .filter(discount_voucherchannellisting::Column::VoucherId.eq(voucher_id))
            .filter(discount_voucherchannellisting::Column::ChannelId.eq(l.channel_id))
            .one(&txn)
            .await?;
        match existing {
            Some(row) => {
                let mut am: discount_voucherchannellisting::ActiveModel = row.into();
                am.discount_value = Set(l.discount_value);
                am.min_spent_amount = Set(l.min_spent);
                am.update(&txn).await?;
            }
            None => {
                discount_voucherchannellisting::ActiveModel {
                    discount_value: Set(l.discount_value),
                    currency: Set(currency.to_string()),
                    min_spent_amount: Set(l.min_spent),
                    channel_id: Set(l.channel_id),
                    voucher_id: Set(voucher_id),
                    ..Default::default()
                }
                .insert(&txn)
                .await?;
            }
        }
    }
    if !remove_channel_ids.is_empty() {
        discount_voucherchannellisting::Entity::delete_many()
            .filter(discount_voucherchannellisting::Column::VoucherId.eq(voucher_id))
            .filter(discount_voucherchannellisting::Column::ChannelId.is_in(remove_channel_ids.to_vec()))
            .exec(&txn)
            .await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Bulk code delete by code-row ids (Django `voucherCodeBulkDelete`).
pub async fn delete_voucher_codes(db: &sea_orm::DatabaseConnection, ids: &[Uuid]) -> Result<i32> {
    let r = discount_vouchercode::Entity::delete_many()
        .filter(discount_vouchercode::Column::Id.is_in(ids.to_vec()))
        .exec(db)
        .await?;
    Ok(r.rows_affected as i32)
}

/// Bulk voucher delete (Django `voucherBulkDelete`): survivors commit.
pub async fn bulk_delete_vouchers(db: &sea_orm::DatabaseConnection, ids: &[i32]) -> Result<i32> {
    let mut n = 0;
    for id in ids {
        if delete_voucher(db, *id).await.is_ok() {
            n += 1;
        }
    }
    Ok(n)
}

/// Infer the voucher type from catalogue assignment (3.23 has no type input).
pub fn infer_voucher_type(products: usize, variants: usize, categories: usize, collections: usize) -> String {
    if products + variants + categories + collections > 0 {
        "specific_product".to_string()
    } else {
        "entire_order".to_string()
    }
}

/// Catalogue predicate in engine shape, shared by sales + rules.
pub fn catalogue_predicate_json(products: &[i32], variants: &[i32], categories: &[i32], collections: &[i32]) -> Value {
    use base64::Engine;
    let gid = |kind: &str, pk: i32| {
        base64::engine::general_purpose::STANDARD.encode(format!("{kind}:{pk}"))
    };
    let mut slots = serde_json::Map::new();
    if !products.is_empty() {
        slots.insert("productPredicate".to_string(), json!({"ids": products.iter().map(|p| gid("Product", *p)).collect::<Vec<_>>()}));
    }
    if !variants.is_empty() {
        slots.insert("variantPredicate".to_string(), json!({"ids": variants.iter().map(|v| gid("ProductVariant", *v)).collect::<Vec<_>>()}));
    }
    if !categories.is_empty() {
        slots.insert("categoryPredicate".to_string(), json!({"ids": categories.iter().map(|c| gid("Category", *c)).collect::<Vec<_>>()}));
    }
    if !collections.is_empty() {
        slots.insert("collectionPredicate".to_string(), json!({"ids": collections.iter().map(|c| gid("Collection", *c)).collect::<Vec<_>>()}));
    }
    if slots.is_empty() { json!({}) } else { Value::Object(slots) }
}

/// Legacy-sale bridge: a `sale*` call becomes one promotion + one rule
/// (Saleor ≥3.9 stores legacy sales as promotions; the `Sale` node resolves
/// from the promotion row, tagged `legacy_sale` so the sales list only
/// shows legacy-origin rows like Django's `Sale` manager).
pub async fn create_sale_as_promotion(
    db: &sea_orm::DatabaseConnection,
    name: Option<String>,
    discount_type: &str,
    value: Decimal,
    products: Vec<i32>,
    variants: Vec<i32>,
    categories: Vec<i32>,
    collections: Vec<i32>,
    start: Option<chrono::DateTime<Utc>>,
    channel_ids: Vec<i32>,
) -> Result<Uuid> {
    let pred = catalogue_predicate_json(&products, &variants, &categories, &collections);
    let vt = if discount_type.eq_ignore_ascii_case("percentage") { "percentage" } else { "fixed" };
    let pid = create_promotion(
        db,
        &name.unwrap_or_else(|| "Sale".to_string()),
        "catalogue",
        json!({}),
        start,
        None,
        vec![NewRule {
            name: None,
            description: json!({}),
            catalogue_predicate: pred,
            order_predicate: json!({}),
            reward_value_type: Some(vt.to_string()),
            reward_value: Some(value),
            reward_type: Some("subtotal_discount".to_string()),
            channel_ids,
            gift_variant_ids: vec![],
        }],
    )
    .await?;
    // Legacy-origin tag for the sales list.
    if let Some(p) = discount_promotion::Entity::find_by_id(pid).one(db).await? {
        let mut am: discount_promotion::ActiveModel = p.into();
        am.metadata = Set(json!({"legacy_sale": true}));
        am.update(db).await?;
    }
    Ok(pid)
}

/// Decode a catalogue predicate back to pk lists (for catalogue merges).
fn predicate_ids(pred: &Value) -> (Vec<i32>, Vec<i32>, Vec<i32>, Vec<i32>) {
    use base64::Engine;
    let mut out = (vec![], vec![], vec![], vec![]);
    let slots = [
        ("productPredicate", 0),
        ("variantPredicate", 1),
        ("categoryPredicate", 2),
        ("collectionPredicate", 3),
    ];
    for (key, idx) in slots {
        if let Some(ids) = pred.get(key).and_then(|v| v.get("ids")).and_then(|v| v.as_array()) {
            for gid in ids.iter().filter_map(|v| v.as_str()) {
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(gid) {
                    if let Ok(s) = String::from_utf8(bytes) {
                        if let Some((_, pk)) = s.split_once(':') {
                            if let Ok(n) = pk.parse::<i32>() {
                                match idx {
                                    0 => out.0.push(n),
                                    1 => out.1.push(n),
                                    2 => out.2.push(n),
                                    _ => out.3.push(n),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out.0.sort_unstable();
    out.0.dedup();
    out.1.sort_unstable();
    out.1.dedup();
    out.2.sort_unstable();
    out.2.dedup();
    out.3.sort_unstable();
    out.3.dedup();
    out
}

/// First rule id of a promotion (legacy sales own exactly one).
pub async fn sale_rule_id(db: &impl ConnectionTrait, promotion_id: Uuid) -> Result<Uuid> {
    let id: Option<Uuid> = discount_promotionrule::Entity::find()
        .select_only()
        .column(discount_promotionrule::Column::Id)
        .filter(discount_promotionrule::Column::PromotionId.eq(promotion_id))
        .into_tuple()
        .one(db)
        .await?
        .unwrap_or(None);
    id.ok_or_else(|| fail("sale has no rule"))
}

/// Merge catalogue rows into/out of a sale's rule predicate (Django
/// `saleCataloguesAdd/Remove`).
#[allow(clippy::too_many_arguments)]
pub async fn sale_catalogues(
    db: &sea_orm::DatabaseConnection,
    promotion_id: Uuid,
    add: bool,
    products: &[i32],
    variants: &[i32],
    categories: &[i32],
    collections: &[i32],
) -> Result<()> {
    let txn = db.begin().await?;
    let rid = sale_rule_id(&txn, promotion_id).await?;
    let rule = discount_promotionrule::Entity::find_by_id(rid)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("sale rule not found"))?;
    let (mut p, mut v, mut c, mut co) = predicate_ids(&rule.catalogue_predicate);
    let merge = |cur: &mut Vec<i32>, delta: &[i32]| {
        if add {
            cur.extend(delta.iter().copied());
            cur.sort_unstable();
            cur.dedup();
        } else {
            cur.retain(|x| !delta.contains(x));
        }
    };
    merge(&mut p, products);
    merge(&mut v, variants);
    merge(&mut c, categories);
    merge(&mut co, collections);
    let mut am: discount_promotionrule::ActiveModel = rule.into();
    am.catalogue_predicate = Set(catalogue_predicate_json(&p, &v, &c, &co));
    am.update(&txn).await?;
    txn.commit().await?;
    Ok(())
}

/// Sale channel listings (Django `saleChannelListingUpdate`): links are
/// authoritative; the reward follows iff exactly one distinct value is
/// given (the rule holds a single shared reward — documented boundary).
pub async fn sale_channel_listing(
    db: &sea_orm::DatabaseConnection,
    promotion_id: Uuid,
    add: &[(i32, Decimal)],
    remove: &[i32],
) -> Result<()> {
    let txn = db.begin().await?;
    let rid = sale_rule_id(&txn, promotion_id).await?;
    for (ch, _) in add {
        let exists = discount_promotionrule_channels::Entity::find()
            .filter(discount_promotionrule_channels::Column::PromotionruleId.eq(rid))
            .filter(discount_promotionrule_channels::Column::ChannelId.eq(*ch))
            .one(&txn)
            .await?
            .is_some();
        if !exists {
            discount_promotionrule_channels::ActiveModel {
                promotionrule_id: Set(rid),
                channel_id: Set(*ch),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }
    if !remove.is_empty() {
        discount_promotionrule_channels::Entity::delete_many()
            .filter(discount_promotionrule_channels::Column::PromotionruleId.eq(rid))
            .filter(discount_promotionrule_channels::Column::ChannelId.is_in(remove.to_vec()))
            .exec(&txn)
            .await?;
    }
    let mut distinct: Vec<Decimal> = add.iter().map(|(_, v)| *v).collect();
    distinct.sort();
    distinct.dedup();
    if let [only] = distinct.as_slice() {
        if let Some(rule) = discount_promotionrule::Entity::find_by_id(rid).one(&txn).await? {
            let mut am: discount_promotionrule::ActiveModel = rule.into();
            am.reward_value = Set(Some(*only));
            am.update(&txn).await?;
        }
    }
    txn.commit().await?;
    Ok(())
}
