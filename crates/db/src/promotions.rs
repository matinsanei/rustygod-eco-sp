//! Promotion engine over Django's discount tables.
//!
//! Mirrors `saleor/discount/utils/{promotion,checkout,voucher}.py`:
//! - catalogue rules apply per variant; **best (max-saving) rule wins**
//!   (`get_best_promotion_discount`);
//! - rule↔variant linkage comes from the explicit M2M
//!   (`discount_promotionrule_variants`) **or** catalogue-predicate match;
//! - line discount rows (`discount_checkoutlinediscount`, `unique_type =
//!   "promotion"`, one per line) carry the whole-line amount
//!   (`_get_rule_discount_amount` = per-unit saving × quantity);
//! - vouchers: `entire_order` and `specific_product` write rows;
//!   `shipping` vouchers need a shipping price (not modeled in v1) and
//!   return a clean NOT_APPLICABLE instead of a wrong number.

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QuerySelect, SelectorTrait, Set,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    catalog::channel_info,
    entities::{
        checkout_checkout, checkout_checkoutline, discount_checkoutdiscount,
        discount_checkoutlinediscount, discount_promotion, discount_promotionrule,
        discount_promotionrule_channels, discount_promotionrule_variants, discount_voucher,
        discount_vouchercode, discount_voucherchannellisting, product_collectionproduct,
        product_product,
    },
    DbError, Result,
};

pub struct ActiveRule {
    pub rule_id: Uuid,
    pub reward_value_type: String,
    pub reward_value: Decimal,
    pub promotion_name: String,
    pub rule_name: String,
    pub predicate: serde_json::Value,
    pub promotion_end: Option<chrono::DateTime<Utc>>,
    pub reward_type: String,
}

/// Active catalogue rules on a channel (date window + channel link),
/// mirroring `get_active_catalogue_promotion_rules`.
pub async fn active_catalogue_rules(
    db: &impl sea_orm::ConnectionTrait,
    channel_id: i32,
) -> Result<Vec<ActiveRule>> {
    let now: sea_orm::prelude::DateTimeWithTimeZone = Utc::now().into();
    let promos = discount_promotion::Entity::find()
        .filter(discount_promotion::Column::Type.eq("catalogue"))
        .filter(discount_promotion::Column::StartDate.lte(now))
        .filter(
            discount_promotion::Column::EndDate
                .is_null()
                .or(discount_promotion::Column::EndDate.gt(now)),
        )
        .all(db)
        .await?;
    let mut out = Vec::new();
    for p in promos {
        let rules = discount_promotionrule::Entity::find()
            .filter(discount_promotionrule::Column::PromotionId.eq(p.id))
            .filter(
                discount_promotionrule::Column::Id.in_subquery(
                    sea_orm::sea_query::Query::select()
                        .column(discount_promotionrule_channels::Column::PromotionruleId)
                        .from(discount_promotionrule_channels::Entity)
                        .and_where(
                            discount_promotionrule_channels::Column::ChannelId.eq(channel_id),
                        )
                        .to_owned(),
                ),
            )
            .all(db)
            .await?;
        for r in rules {
            // Price-affecting rules only (gifts handled by the gift milestone).
            if r.reward_type.as_deref() == Some("gift") {
                continue;
            }
            let (Some(vt), Some(rv)) = (r.reward_value_type.clone(), r.reward_value) else {
                continue;
            };
            out.push(ActiveRule {
                rule_id: r.id,
                reward_value_type: vt,
                reward_value: rv,
                promotion_name: p.name.clone(),
                rule_name: r.name.clone().unwrap_or_default(),
                predicate: r.catalogue_predicate.clone(),
                promotion_end: p.end_date.map(|d| d.into()),
                reward_type: r.reward_type.clone().unwrap_or_default(),
            });
        }
    }
    Ok(out)
}

struct VariantContext {
    product_id: i32,
    category_id: Option<i32>,
    collection_ids: Vec<i32>,
}

async fn variant_context(
    db: &impl sea_orm::ConnectionTrait,
    variant_id: i32,
    product_id: i32,
) -> Result<VariantContext> {
    let category_id: Option<i32> = product_product::Entity::find_by_id(product_id)
        .select_only()
        .column(product_product::Column::CategoryId)
        .into_tuple::<Option<i32>>()
        .one(db)
        .await?
        .flatten();
    let collection_ids: Vec<i32> = product_collectionproduct::Entity::find()
        .select_only()
        .column(product_collectionproduct::Column::CollectionId)
        .filter(product_collectionproduct::Column::ProductId.eq(product_id))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    Ok(VariantContext { product_id, category_id, collection_ids })
}

async fn rule_matches_variant(
    db: &impl sea_orm::ConnectionTrait,
    rule: &ActiveRule,
    variant_id: i32,
    ctx: &VariantContext,
) -> Result<bool> {
    // Explicit M2M wins (Django's variants_to_promotion_rules map).
    let linked = discount_promotionrule_variants::Entity::find()
        .filter(discount_promotionrule_variants::Column::PromotionruleId.eq(rule.rule_id))
        .filter(discount_promotionrule_variants::Column::ProductvariantId.eq(variant_id))
        .one(db)
        .await?
        .is_some();
    if linked {
        return Ok(true);
    }
    // Otherwise the catalogue predicate decides.
    Ok(rustygod_core::discount::predicate_matches(
        &rule.predicate,
        variant_id,
        ctx.product_id,
        ctx.category_id,
        &ctx.collection_ids,
    ))
}

pub struct LineEvaluation {
    pub rule_id: Uuid,
    pub promotion_name: String,
    pub rule_name: String,
    pub reward_value_type: String,
    pub reward_value: Decimal,
    /// Whole-line saving (per-unit saving × quantity), like Django's
    /// `rule_discount_amount`.
    pub amount: Decimal,
    pub promotion_end: Option<chrono::DateTime<Utc>>,
}

/// Best-rule evaluation for one checkout line, mirroring
/// `get_best_promotion_discount` + `_get_rule_discount_amount`.
pub async fn evaluate_line(
    db: &impl sea_orm::ConnectionTrait,
    channel_id: i32,
    variant_id: i32,
    product_id: i32,
    unit_price: Decimal,
    quantity: i32,
    currency: &str,
) -> Result<Option<LineEvaluation>> {
    let rules = active_catalogue_rules(db, channel_id).await?;
    if rules.is_empty() {
        return Ok(None);
    }
    let ctx = variant_context(db, variant_id, product_id).await?;
    let mut applicable = Vec::new();
    for rule in &rules {
        if rule_matches_variant(db, rule, variant_id, &ctx).await? {
            applicable.push((
                rule.rule_id.to_string(),
                rule.reward_value_type.clone(),
                rule.reward_value,
            ));
        }
    }
    if applicable.is_empty() {
        return Ok(None);
    }
    let Some((best_idx, _)) =
        rustygod_core::discount::best_rule(unit_price, &applicable, currency)
    else {
        return Ok(None);
    };
    // Django skips zero-saving "discounts".
    let saving = rustygod_core::discount::saving_for_rule(
        unit_price,
        &applicable[best_idx].1,
        applicable[best_idx].2,
        currency,
    );
    if saving <= Decimal::ZERO {
        return Ok(None);
    }
    let rule = &rules
        .iter()
        .find(|r| r.rule_id.to_string() == applicable[best_idx].0)
        .expect("best rule comes from applicable set");
    Ok(Some(LineEvaluation {
        rule_id: rule.rule_id,
        promotion_name: rule.promotion_name.clone(),
        rule_name: rule.rule_name.clone(),
        reward_value_type: rule.reward_value_type.clone(),
        reward_value: rule.reward_value,
        amount: saving * Decimal::from(quantity),
        promotion_end: rule.promotion_end,
    }))
}

/// Write (or replace) the single catalogue line-discount row
/// (`unique(line_id, unique_type)`), mirroring Django's bulk
/// create/update with `ignore_conflicts`.
pub async fn write_line_promotion_discount(
    db: &impl sea_orm::ConnectionTrait,
    line_id: Uuid,
    currency: &str,
    eval: &LineEvaluation,
) -> Result<()> {
    discount_checkoutlinediscount::Entity::delete_many()
        .filter(discount_checkoutlinediscount::Column::LineId.eq(line_id))
        .filter(discount_checkoutlinediscount::Column::UniqueType.eq("promotion"))
        .exec(db)
        .await?;
    let name = if !eval.promotion_name.is_empty() && !eval.rule_name.is_empty() {
        format!("{}: {}", eval.promotion_name, eval.rule_name)
    } else if !eval.rule_name.is_empty() {
        eval.rule_name.clone()
    } else {
        eval.promotion_name.clone()
    };
    discount_checkoutlinediscount::ActiveModel {
        id: Set(Uuid::new_v4()),
        created_at: Set(Utc::now().into()),
        r#type: Set("promotion".to_string()),
        value_type: Set(eval.reward_value_type.clone()),
        value: Set(eval.reward_value),
        amount_value: Set(eval.amount),
        currency: Set(currency.to_string()),
        name: Set(Some(name)),
        line_id: Set(Some(line_id)),
        promotion_rule_id: Set(Some(eval.rule_id)),
        unique_type: Set(Some("promotion".to_string())),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Remove catalogue line-discounts (line no longer discounted — mirrors
/// Django deleting stale discounts).
pub async fn clear_line_promotion_discounts(
    db: &impl sea_orm::ConnectionTrait,
    line_id: Uuid,
) -> Result<()> {
    discount_checkoutlinediscount::Entity::delete_many()
        .filter(discount_checkoutlinediscount::Column::LineId.eq(line_id))
        .filter(discount_checkoutlinediscount::Column::UniqueType.eq("promotion"))
        .exec(db)
        .await?;
    Ok(())
}

/// Sum of promotion line-discounts for a checkout (whole-line amounts).
pub async fn checkout_promotion_total(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
) -> Result<Decimal> {
    use crate::entities::checkout_checkoutline;
    let line_ids: Vec<Uuid> = checkout_checkoutline::Entity::find()
        .select_only()
        .column(checkout_checkoutline::Column::Id)
        .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
        .into_tuple::<Uuid>()
        .all(db)
        .await?;
    if line_ids.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let rows = discount_checkoutlinediscount::Entity::find()
        .filter(discount_checkoutlinediscount::Column::LineId.is_in(line_ids))
        .filter(discount_checkoutlinediscount::Column::UniqueType.eq("promotion"))
        .all(db)
        .await?;
    Ok(rows.iter().map(|r| r.amount_value).sum())
}

// ---------- Vouchers ----------

pub struct VoucherApplication {
    pub voucher_id: i32,
    pub code: String,
    pub voucher_type: String,
    pub value_type: String,
    pub value: Decimal,
    pub currency: String,
    pub amount: Decimal,
}

/// Apply a voucher to a checkout (entire_order + specific_product).
/// Writes `voucher_code` on the checkout row and the discount rows Django
/// writes; shipping vouchers return NOT_APPLICABLE (no shipping price in v1).
pub async fn apply_voucher(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
    code: &str,
    channel_slug: &str,
) -> Result<VoucherApplication> {
    let (ch_id, _) = channel_info(db, channel_slug).await?;
    let code_row = discount_vouchercode::Entity::find()
        .filter(discount_vouchercode::Column::Code.eq(code))
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(code.into())))?;
    if !code_row.is_active {
        return Err(DbError::SeaOrm(sea_orm::DbErr::Custom("voucher code inactive".into())));
    }
    let v = discount_voucher::Entity::find_by_id(code_row.voucher_id)
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(code.into())))?;
    let now: sea_orm::prelude::DateTimeWithTimeZone = Utc::now().into();
    if v.start_date > now {
        return Err(DbError::SeaOrm(sea_orm::DbErr::Custom("voucher not started".into())));
    }
    if let Some(end) = v.end_date {
        if end < now {
            return Err(DbError::SeaOrm(sea_orm::DbErr::Custom("voucher expired".into())));
        }
    }
    if let Some(limit) = v.usage_limit {
        if code_row.used >= limit {
            return Err(DbError::SeaOrm(sea_orm::DbErr::Custom(
                "voucher usage limit reached".into(),
            )));
        }
    }
    let listing = discount_voucherchannellisting::Entity::find()
        .filter(discount_voucherchannellisting::Column::VoucherId.eq(v.id))
        .filter(discount_voucherchannellisting::Column::ChannelId.eq(ch_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            DbError::SeaOrm(sea_orm::DbErr::Custom("voucher not available on channel".into()))
        })?;

    // Checkout row must exist.
    let co = checkout_checkout::Entity::find_by_id(checkout_token)
        .one(db)
        .await?
        .ok_or_else(|| DbError::CheckoutNotFound(checkout_token.to_string()))?;

    match v.r#type.as_str() {
        "shipping" => Err(DbError::SeaOrm(sea_orm::DbErr::Custom(
            "NOT_APPLICABLE: shipping vouchers need a shipping price (v1 has none)".into(),
        ))),
        "entire_order" => {
            // Django: CheckoutDiscount row (order-level) + voucher_code on checkout.
            let base: Decimal = co.total_gross_amount;
            let discounted = match v.discount_value_type.as_str() {
                "fixed" => rustygod_core::discount::apply_fixed(base, listing.discount_value),
                "percentage" => rustygod_core::discount::apply_percentage(
                    base,
                    listing.discount_value,
                    &listing.currency,
                ),
                other => {
                    return Err(DbError::SeaOrm(sea_orm::DbErr::Custom(format!(
                        "unknown discount value type {other}"
                    ))))
                }
            };
            let amount = (base - discounted).max(Decimal::ZERO);
            let mut co_am: checkout_checkout::ActiveModel = co.into();
            co_am.voucher_code = Set(Some(code.to_string()));
            co_am.update(db).await?;
            // One voucher discount per checkout (replace on re-apply → idempotent).
            discount_checkoutdiscount::Entity::delete_many()
                .filter(discount_checkoutdiscount::Column::CheckoutId.eq(checkout_token))
                .filter(discount_checkoutdiscount::Column::Type.eq("voucher"))
                .exec(db)
                .await?;
            discount_checkoutdiscount::ActiveModel {
                id: Set(Uuid::new_v4()),
                created_at: Set(Utc::now().into()),
                r#type: Set("voucher".to_string()),
                value_type: Set(v.discount_value_type.clone()),
                value: Set(listing.discount_value),
                amount_value: Set(amount),
                currency: Set(listing.currency.clone()),
                voucher_code: Set(Some(code.to_string())),
                checkout_id: Set(Some(checkout_token)),
                voucher_id: Set(Some(v.id)),
                ..Default::default()
            }
            .insert(db)
            .await?;
            Ok(VoucherApplication {
                voucher_id: v.id,
                code: code.to_string(),
                voucher_type: v.r#type,
                value_type: v.discount_value_type,
                value: listing.discount_value,
                currency: listing.currency,
                amount,
            })
        }
        "specific_product" => {
            // Per-line discounts on matching variants (Django: line discounts
            // with voucher FK). Variant scope comes from voucher M2M tables.
            use crate::entities::{
                checkout_checkoutline, discount_voucher_collections, discount_voucher_products,
                discount_voucher_variants,
            };
            let lines = checkout_checkoutline::Entity::find()
                .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
                .all(db)
                .await?;
            let var_ids: Vec<i32> = discount_voucher_variants::Entity::find()
                .select_only()
                .column(discount_voucher_variants::Column::ProductvariantId)
                .filter(discount_voucher_variants::Column::VoucherId.eq(v.id))
                .into_tuple::<i32>()
                .all(db)
                .await?;
            let prod_ids: Vec<i32> = discount_voucher_products::Entity::find()
                .select_only()
                .column(discount_voucher_products::Column::ProductId)
                .filter(discount_voucher_products::Column::VoucherId.eq(v.id))
                .into_tuple::<i32>()
                .all(db)
                .await?;
            let _ = discount_voucher_collections::Entity::find()
                .select_only()
                .column(discount_voucher_collections::Column::CollectionId)
                .filter(discount_voucher_collections::Column::VoucherId.eq(v.id))
                .into_tuple::<i32>()
                .all(db)
                .await?;
            let mut total = Decimal::ZERO;
            let mut matched = 0;
            for line in &lines {
                if !(var_ids.contains(&line.variant_id)) && prod_ids.is_empty() {
                    continue;
                }
                // Product-scope check when voucher lists products.
                if !prod_ids.is_empty() {
                    use crate::entities::product_productvariant;
                    let pid: Option<i32> = product_productvariant::Entity::find_by_id(line.variant_id)
                        .select_only()
                        .column(product_productvariant::Column::ProductId)
                        .into_tuple::<i32>()
                        .one(db)
                        .await?;
                    if pid.map(|p| !prod_ids.contains(&p)).unwrap_or(true)
                        && !var_ids.contains(&line.variant_id)
                    {
                        continue;
                    }
                }
                let unit = line.price_override.unwrap_or(line.undiscounted_unit_price_amount);
                let discounted = match v.discount_value_type.as_str() {
                    "fixed" => rustygod_core::discount::apply_fixed(unit, listing.discount_value),
                    "percentage" => rustygod_core::discount::apply_percentage(
                        unit,
                        listing.discount_value,
                        &listing.currency,
                    ),
                    _ => unit,
                };
                let amount = ((unit - discounted).max(Decimal::ZERO)) * Decimal::from(line.quantity);
                if amount > Decimal::ZERO {
                    matched += 1;
                    total += amount;
                    discount_checkoutlinediscount::Entity::delete_many()
                        .filter(discount_checkoutlinediscount::Column::LineId.eq(line.id))
                        .filter(discount_checkoutlinediscount::Column::UniqueType.eq("voucher"))
                        .exec(db)
                        .await?;
                    discount_checkoutlinediscount::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        created_at: Set(Utc::now().into()),
                        r#type: Set("voucher".to_string()),
                        value_type: Set(v.discount_value_type.clone()),
                        value: Set(listing.discount_value),
                        amount_value: Set(amount),
                        currency: Set(listing.currency.clone()),
                        line_id: Set(Some(line.id)),
                        voucher_id: Set(Some(v.id)),
                        voucher_code: Set(Some(code.to_string())),
                        unique_type: Set(Some("voucher".to_string())),
                        ..Default::default()
                    }
                    .insert(db)
                    .await?;
                }
            }
            if matched == 0 {
                return Err(DbError::SeaOrm(sea_orm::DbErr::Custom(
                    "voucher not applicable to any line".into(),
                )));
            }
            let mut co_am: checkout_checkout::ActiveModel = checkout_checkout::Entity::find_by_id(checkout_token)
                .one(db)
                .await?
                .ok_or_else(|| DbError::CheckoutNotFound(checkout_token.to_string()))?
                .into();
            co_am.voucher_code = Set(Some(code.to_string()));
            co_am.update(db).await?;
            Ok(VoucherApplication {
                voucher_id: v.id,
                code: code.to_string(),
                voucher_type: v.r#type,
                value_type: v.discount_value_type,
                value: listing.discount_value,
                currency: listing.currency,
                amount: total,
            })
        }
        other => Err(DbError::SeaOrm(sea_orm::DbErr::Custom(format!(
            "unknown voucher type {other}"
        )))),
    }
}

/// Checkout-level voucher total (entire_order discounts).
pub async fn checkout_voucher_total(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
) -> Result<Decimal> {
    let rows = discount_checkoutdiscount::Entity::find()
        .filter(discount_checkoutdiscount::Column::CheckoutId.eq(checkout_token))
        .all(db)
        .await?;
    Ok(rows.iter().map(|r| r.amount_value).sum())
}

/// Line-level voucher total (specific_product discounts).
pub async fn lines_voucher_total(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
) -> Result<Decimal> {
    use crate::entities::checkout_checkoutline;
    let line_ids: Vec<Uuid> = checkout_checkoutline::Entity::find()
        .select_only()
        .column(checkout_checkoutline::Column::Id)
        .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
        .into_tuple::<Uuid>()
        .all(db)
        .await?;
    if line_ids.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let rows = discount_checkoutlinediscount::Entity::find()
        .filter(discount_checkoutlinediscount::Column::LineId.is_in(line_ids))
        .filter(discount_checkoutlinediscount::Column::UniqueType.eq("voucher"))
        .all(db)
        .await?;
    Ok(rows.iter().map(|r| r.amount_value).sum())
}

/// Increase voucher + code usage, mirroring
/// `increase_voucher_usage`/`increase_voucher_code_usage_value` with atomic
/// increments (Django uses F() — same here, no lost updates).
pub async fn increase_usage(
    db: &impl sea_orm::ConnectionTrait,
    code: &str,
    customer_email: Option<&str>,
) -> Result<()> {
    use sea_orm::sea_query::Expr;
    let Some(code_row) = discount_vouchercode::Entity::find()
        .filter(discount_vouchercode::Column::Code.eq(code))
        .one(db)
        .await?
    else {
        return Ok(());
    };
    // Atomic increment via raw SQL (F-expression semantics — race-safe like Django).
    increment_code_used(db, code).await?;
    if let Some(email) = customer_email {
        if !email.is_empty() {
            use crate::entities::discount_vouchercustomer;
            let exists = discount_vouchercustomer::Entity::find()
                .filter(discount_vouchercustomer::Column::VoucherCodeId.eq(code_row.id))
                .filter(discount_vouchercustomer::Column::CustomerEmail.eq(email))
                .one(db)
                .await?
                .is_some();
            if !exists {
                discount_vouchercustomer::ActiveModel {
                    customer_email: Set(email.to_string()),
                    voucher_code_id: Set(code_row.id),
                    ..Default::default()
                }
                .insert(db)
                .await?;
            }
        }
    }
    Ok(())
}

async fn increment_code_used(db: &impl sea_orm::ConnectionTrait, code: &str) -> Result<()> {
    use sea_orm::Statement;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE discount_vouchercode SET used = used + 1 WHERE code = $1",
        vec![code.into()],
    ))
    .await?;
    Ok(())
}
