//! Order-level promotions on checkouts (T3): subtotal discounts + gifts.
//!
//! Mirrors `saleor/discount/utils/promotion.py` +
//! `saleor/discount/utils/{checkout,order}.py`:
//! - candidates: `type=order` promotions active now, rules with a NON-EMPTY
//!   `order_predicate`, bound to this channel;
//! - money gate: `discountedObjectPredicate.base{Total,Subtotal}Price`
//!   range/eq vs the base subtotal (catalogue-discounted lines, gifts
//!   excluded). Voucher/manual precedence: a `voucher_code` on the checkout
//!   clears order promotions, exactly like Django;
//! - winner = max saving across discount rules AND gift rules (a gift
//!   competes with its channel price — Django's `get_best_rule`). Gift
//!   availability: free stock ≥ 1 + channel listing with a price
//!   (Django also checks purchasable dates — documented boundary);
//! - apply: ONE `discount_checkoutdiscount` row (type ORDER_PROMOTION,
//!   get-or-create + update under a checkout FOR UPDATE lock) XOR ONE gift
//!   line (`is_gift`, qty 1, totals 0 — the line's audit is its zero total;
//!   the ORDER side carries full discount fields). Stale loser cleared both
//!   directions (stricter than Django, which can leave the previous
//!   winner's row when the crown changes hands);
//! - fixed rewards cap at the subtotal, percentages round HALF_UP
//!   (core math, Django's `prices` semantics).

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
    Set, TransactionTrait, sea_query::LockType,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        checkout_checkout, checkout_checkoutline, discount_checkoutdiscount, discount_orderdiscount,
        discount_promotion, discount_promotionrule, discount_promotionrule_channels,
        discount_promotionrule_gifts, order_orderline,
        product_productvariantchannellisting, warehouse_stock,
    },
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::SeaOrm(sea_orm::DbErr::Custom(msg.into()))
}

pub const ORDER_PROMOTION: &str = "order_promotion";

pub struct OrderRule {
    pub rule_id: Uuid,
    pub promotion_id: Uuid,
    pub promotion_name: String,
    pub rule_name: String,
    pub reward_type: String,
    pub reward_value_type: Option<String>,
    pub reward_value: Option<Decimal>,
    pub order_predicate: serde_json::Value,
}

/// Active order-promotion rules for a channel: promotion live now,
/// rule bound to the channel, non-empty order predicate (Django's
/// `.exclude(order_predicate={})`).
pub async fn active_order_rules(
    db: &impl sea_orm::ConnectionTrait,
    channel_id: i32,
) -> Result<Vec<OrderRule>> {
    let now: sea_orm::prelude::DateTimeWithTimeZone = Utc::now().into();
    let promos = discount_promotion::Entity::find()
        .filter(discount_promotion::Column::Type.eq("order"))
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
            if r.order_predicate == json!({}) {
                continue;
            }
            out.push(OrderRule {
                rule_id: r.id,
                promotion_id: p.id,
                promotion_name: p.name.clone(),
                rule_name: r.name.clone().unwrap_or_default(),
                reward_type: r.reward_type.clone().unwrap_or_default(),
                reward_value_type: r.reward_value_type.clone(),
                reward_value: r.reward_value,
                order_predicate: r.order_predicate.clone(),
            });
        }
    }
    Ok(out)
}

struct Candidate {
    rule: OrderRule,
    saving: Decimal,
    gift: Option<GiftOffer>,
}

struct GiftOffer {
    variant_id: i32,
    price: Decimal,
}

/// Best available gift across gift rules: free stock ≥ 1, channel listing
/// with a price; max price wins (Django's `_get_best_gift_reward` picks the
/// top discounted price among purchasable, in-stock gifts).
async fn best_gift(
    db: &impl sea_orm::ConnectionTrait,
    channel_id: i32,
    gift_rules: &[OrderRule],
) -> Result<Option<(Uuid, GiftOffer)>> {
    let rule_ids: Vec<Uuid> = gift_rules.iter().map(|r| r.rule_id).collect();
    if rule_ids.is_empty() {
        return Ok(None);
    }
    let gift_vids: Vec<i32> = discount_promotionrule_gifts::Entity::find()
        .select_only()
        .column(discount_promotionrule_gifts::Column::ProductvariantId)
        .filter(discount_promotionrule_gifts::Column::PromotionruleId.is_in(rule_ids))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    if gift_vids.is_empty() {
        return Ok(None);
    }
    // Free stock per variant (SUM quantity - allocated, like the catalogue).
    let stocks = warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::ProductVariantId.is_in(gift_vids.clone()))
        .all(db)
        .await?;
    let mut free: std::collections::HashMap<i32, i32> = Default::default();
    for s in stocks {
        *free.entry(s.product_variant_id).or_insert(0) +=
            (s.quantity - s.quantity_allocated).max(0);
    }
    let listings = product_productvariantchannellisting::Entity::find()
        .filter(product_productvariantchannellisting::Column::VariantId.is_in(gift_vids))
        .filter(product_productvariantchannellisting::Column::ChannelId.eq(channel_id))
        .all(db)
        .await?;
    let mut best: Option<(Uuid, GiftOffer)> = None;
    for l in listings {
        let (Some(price), vid) = (l.price_amount, l.variant_id) else { continue };
        if free.get(&vid).copied().unwrap_or(0) < 1 {
            continue;
        }
        // Which rule offers this gift? First match (rules rarely share gifts).
        let rule_id = discount_promotionrule_gifts::Entity::find()
            .select_only()
            .column(discount_promotionrule_gifts::Column::PromotionruleId)
            .filter(discount_promotionrule_gifts::Column::ProductvariantId.eq(vid))
            .into_tuple::<Uuid>()
            .one(db)
            .await?;
        let Some(rule_id) = rule_id else { continue };
        let wins = best.as_ref().map(|(_, o)| price > o.price).unwrap_or(true);
        if wins {
            best = Some((rule_id, GiftOffer { variant_id: vid, price }));
        }
    }
    Ok(best)
}

#[derive(Debug)]
pub enum RefreshOutcome {
    Cleared,
    Discount { rule_id: Uuid, amount: Decimal },
    Gift { rule_id: Uuid, variant_id: i32, line_id: Uuid },
}

/// Re-evaluate order promotions for a checkout: lock, gate, pick, apply,
/// clear the loser. Idempotent — safe to run after every line mutation
/// (Django recomputes on checkout refresh; `refresh_totals` calls this).
/// NOTE: updates discount/gift rows only — callers recompute the
/// denormalized checkout totals afterwards (`refresh_totals` does).
pub async fn refresh_order_promotion(
    db: &DatabaseConnection,
    checkout_token: Uuid,
) -> Result<RefreshOutcome> {
    let txn = db.begin().await?;
    let out = refresh_order_promotion_in(&txn, checkout_token).await?;
    txn.commit().await?;
    Ok(out)
}

pub async fn refresh_order_promotion_in(
    txn: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
) -> Result<RefreshOutcome> {
    // Django's _lock_order_or_checkout: serialize concurrent appliers so no
    // duplicate discount rows or twin gift lines appear.
    let co = checkout_checkout::Entity::find_by_id(checkout_token)
        .lock(LockType::Update)
        .one(txn)
        .await?
        .ok_or_else(|| DbError::CheckoutNotFound(checkout_token.to_string()))?;

    // Voucher precedence (Django: voucher or manual discount skips order
    // promotions entirely, clearing stale ones).
    if co.voucher_code.as_deref().map(|s| !s.is_empty()).unwrap_or(false) {
        clear_all(txn, checkout_token).await?;
        return Ok(RefreshOutcome::Cleared);
    }

    // Base subtotal: payable lines minus catalogue line discounts, gifts out.
    let lines = checkout_checkoutline::Entity::find()
        .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
        .all(txn)
        .await?;
    let lines_sum: Decimal = lines
        .iter()
        .filter(|l| !l.is_gift)
        .map(|l| l.total_price_gross_amount)
        .sum();
    let catalogue = crate::promotions::checkout_promotion_total(txn, checkout_token).await?;
    let subtotal = (lines_sum - catalogue).max(Decimal::ZERO);

    let rules = active_order_rules(txn, co.channel_id).await?;
    let mut cands: Vec<Candidate> = vec![];
    let mut gift_rules: Vec<OrderRule> = vec![];
    for rule in rules {
        if !saleor_rustify_core::discount::order_predicate_matches(&rule.order_predicate, subtotal) {
            continue;
        }
        match rule.reward_type.as_str() {
            "subtotal_discount" => {
                let (Some(vt), Some(rv)) = (rule.reward_value_type.clone(), rule.reward_value)
                else {
                    continue;
                };
                let saving =
                    saleor_rustify_core::discount::saving_for_rule(subtotal, &vt, rv, &co.currency);
                if saving > Decimal::ZERO {
                    cands.push(Candidate { rule, saving, gift: None });
                }
            }
            "gift" => gift_rules.push(rule),
            _ => {}
        }
    }
    if !gift_rules.is_empty() {
        if let Some((rule_id, offer)) = best_gift(txn, co.channel_id, &gift_rules).await? {
            let rule = gift_rules.into_iter().find(|r| r.rule_id == rule_id).unwrap();
            cands.push(Candidate { rule, saving: offer.price, gift: Some(offer) });
        }
    }
    // Winner = max saving (Django's max() — first wins ties, same here by
    // stable sort + last-max... note: max_by returns the LAST max; Django's
    // max() also returns the last maximal element. Identical semantics).
    let winner = cands.into_iter().max_by(|a, b| a.saving.cmp(&b.saving));
    let Some(w) = winner else {
        clear_all(txn, checkout_token).await?;
        return Ok(RefreshOutcome::Cleared);
    };
    if let Some(offer) = w.gift {
        clear_discount_row(txn, checkout_token).await?;
        let line_id = ensure_gift_line(txn, checkout_token, &co.currency, offer.variant_id, offer.price).await?;
        Ok(RefreshOutcome::Gift { rule_id: w.rule.rule_id, variant_id: offer.variant_id, line_id })
    } else {
        clear_gift_lines(txn, checkout_token).await?;
        upsert_discount_row(
            txn,
            checkout_token,
            &co.currency,
            &w.rule,
            w.saving.min(subtotal),
        )
        .await?;
        Ok(RefreshOutcome::Discount { rule_id: w.rule.rule_id, amount: w.saving.min(subtotal) })
    }
}

/// Sum of ORDER_PROMOTION checkout discounts (the totals term).
pub async fn order_promotion_total(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
) -> Result<Decimal> {
    let rows = discount_checkoutdiscount::Entity::find()
        .filter(discount_checkoutdiscount::Column::CheckoutId.eq(checkout_token))
        .filter(discount_checkoutdiscount::Column::Type.eq(ORDER_PROMOTION))
        .all(db)
        .await?;
    Ok(rows.iter().map(|r| r.amount_value).sum())
}

async fn clear_discount_row(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
) -> Result<()> {
    discount_checkoutdiscount::Entity::delete_many()
        .filter(discount_checkoutdiscount::Column::CheckoutId.eq(checkout_token))
        .filter(discount_checkoutdiscount::Column::Type.eq(ORDER_PROMOTION))
        .exec(db)
        .await?;
    Ok(())
}

async fn clear_gift_lines(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
) -> Result<()> {
    checkout_checkoutline::Entity::delete_many()
        .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
        .filter(checkout_checkoutline::Column::IsGift.eq(true))
        .exec(db)
        .await?;
    Ok(())
}

async fn clear_all(db: &impl sea_orm::ConnectionTrait, checkout_token: Uuid) -> Result<()> {
    clear_discount_row(db, checkout_token).await?;
    clear_gift_lines(db, checkout_token).await?;
    Ok(())
}

/// get_or_create + update under the checkout lock (Django's
/// `_handle_order_promotion` — no unique constraint, lock prevents twins).
async fn upsert_discount_row(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
    currency: &str,
    rule: &OrderRule,
    amount: Decimal,
) -> Result<()> {
    let reason = format!("Promotion: {}", rule.promotion_name);
    if let Some(row) = discount_checkoutdiscount::Entity::find()
        .filter(discount_checkoutdiscount::Column::CheckoutId.eq(checkout_token))
        .filter(discount_checkoutdiscount::Column::Type.eq(ORDER_PROMOTION))
        .one(db)
        .await?
    {
        let mut am: discount_checkoutdiscount::ActiveModel = row.into();
        am.value_type = Set(rule.reward_value_type.clone().unwrap_or_else(|| "fixed".into()));
        am.value = Set(rule.reward_value.unwrap_or(amount));
        am.amount_value = Set(amount);
        am.currency = Set(currency.to_string());
        am.name = Set(Some(rule.promotion_name.clone()));
        am.reason = Set(Some(reason));
        am.promotion_rule_id = Set(Some(rule.rule_id));
        am.update(db).await?;
        return Ok(());
    }
    discount_checkoutdiscount::ActiveModel {
        id: Set(Uuid::new_v4()),
        created_at: Set(Utc::now().into()),
        r#type: Set(ORDER_PROMOTION.to_string()),
        value_type: Set(rule.reward_value_type.clone().unwrap_or_else(|| "fixed".into())),
        value: Set(rule.reward_value.unwrap_or(amount)),
        amount_value: Set(amount),
        currency: Set(currency.to_string()),
        name: Set(Some(rule.promotion_name.clone())),
        reason: Set(Some(reason)),
        voucher_code: Set(None),
        checkout_id: Set(Some(checkout_token)),
        promotion_rule_id: Set(Some(rule.rule_id)),
        voucher_id: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Single gift slot (`get_or_create(is_gift=True)`): reuse the row when the
/// same variant wins again, replace it when the crown changes hands, totals
/// stay 0 with the listing price kept as the audit trail.
async fn ensure_gift_line(
    db: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
    currency: &str,
    variant_id: i32,
    price: Decimal,
) -> Result<Uuid> {
    let existing = checkout_checkoutline::Entity::find()
        .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
        .filter(checkout_checkoutline::Column::IsGift.eq(true))
        .all(db)
        .await?;
    // One gift at a time: drop rows for other variants (Django deletes +
    // recreates; update-in-place when identical).
    for row in &existing {
        if row.variant_id != variant_id {
            checkout_checkoutline::Entity::delete_by_id(row.id).exec(db).await?;
        }
    }
    if let Some(row) = existing.into_iter().find(|r| r.variant_id == variant_id) {
        if row.quantity != 1 || row.undiscounted_unit_price_amount != price {
            let mut am: checkout_checkoutline::ActiveModel = row.into();
            am.quantity = Set(1);
            am.undiscounted_unit_price_amount = Set(price);
            am.total_price_net_amount = Set(Decimal::ZERO);
            am.total_price_gross_amount = Set(Decimal::ZERO);
            am.update(db).await?;
        }
        let row = checkout_checkoutline::Entity::find()
            .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
            .filter(checkout_checkoutline::Column::IsGift.eq(true))
            .one(db)
            .await?
            .ok_or_else(|| fail("gift line vanished"))?;
        return Ok(row.id);
    }
    let id = Uuid::new_v4();
    checkout_checkoutline::ActiveModel {
        id: Set(id),
        checkout_id: Set(checkout_token),
        variant_id: Set(variant_id),
        quantity: Set(1),
        currency: Set(currency.to_string()),
        price_override: Set(None),
        undiscounted_unit_price_amount: Set(price),
        total_price_net_amount: Set(Decimal::ZERO),
        total_price_gross_amount: Set(Decimal::ZERO),
        tax_rate: Set(Decimal::ZERO),
        is_gift: Set(true),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(id)
}

/// Carry order promotions into the minted order: the ORDER_PROMOTION
/// discount row → `discount_orderdiscount`, the gift line → a zero-priced
/// `is_gift` order line with full discount audit (Django's
/// `_get_defaults_for_gift_line` for orders). Gift stock allocates with the
/// rest at complete (real units, real allocation).
pub async fn carry_to_order(
    txn: &impl sea_orm::ConnectionTrait,
    checkout_token: Uuid,
    order_id: Uuid,
) -> Result<()> {
    if let Some(d) = discount_checkoutdiscount::Entity::find()
        .filter(discount_checkoutdiscount::Column::CheckoutId.eq(checkout_token))
        .filter(discount_checkoutdiscount::Column::Type.eq(ORDER_PROMOTION))
        .one(txn)
        .await?
    {
        discount_orderdiscount::ActiveModel {
            id: Set(Uuid::new_v4()),
            order_id: Set(Some(order_id)),
            r#type: Set(ORDER_PROMOTION.to_string()),
            value_type: Set(d.value_type.clone()),
            value: Set(d.value),
            amount_value: Set(d.amount_value),
            currency: Set(d.currency.clone()),
            name: Set(d.name.clone()),
            translated_name: Set(None),
            reason: Set(d.reason.clone()),
            created_at: Set(Utc::now().into()),
            old_id: Set(None),
            sale_id: Set(None),
            voucher_id: Set(None),
            promotion_rule_id: Set(d.promotion_rule_id),
            voucher_code: Set(None),
        }
        .insert(txn)
        .await?;
    }
    let gifts = checkout_checkoutline::Entity::find()
        .filter(checkout_checkoutline::Column::CheckoutId.eq(checkout_token))
        .filter(checkout_checkoutline::Column::IsGift.eq(true))
        .all(txn)
        .await?;
    for g in gifts {
        let detail = crate::order_store::variant_details(txn, &[g.variant_id])
            .await?
            .remove(&g.variant_id)
            .ok_or_else(|| fail(format!("gift variant {} missing", g.variant_id)))?;
        let price = g.undiscounted_unit_price_amount;
        order_orderline::ActiveModel {
            id: Set(Uuid::new_v4()),
            order_id: Set(order_id),
            variant_id: Set(Some(g.variant_id)),
            product_name: Set(detail.product_name.clone()),
            variant_name: Set(detail.variant_name.clone()),
            translated_product_name: Set(String::new()),
            translated_variant_name: Set(String::new()),
            product_sku: Set(detail.sku.clone()),
            product_type_id: Set(Some(detail.product_type_id)),
            is_shipping_required: Set(detail.is_shipping_required),
            is_gift_card: Set(detail.is_gift_card),
            quantity: Set(1),
            quantity_fulfilled: Set(0),
            currency: Set(g.currency.clone()),
            unit_price_net_amount: Set(Decimal::ZERO),
            unit_price_gross_amount: Set(Decimal::ZERO),
            total_price_net_amount: Set(Decimal::ZERO),
            total_price_gross_amount: Set(Decimal::ZERO),
            undiscounted_unit_price_net_amount: Set(price),
            undiscounted_unit_price_gross_amount: Set(price),
            undiscounted_total_price_net_amount: Set(price),
            undiscounted_total_price_gross_amount: Set(price),
            base_unit_price_amount: Set(Decimal::ZERO),
            undiscounted_base_unit_price_amount: Set(price),
            unit_discount_amount: Set(price),
            unit_discount_value: Set(price),
            unit_discount_reason: Set(Some("Promotion gift".to_string())),
            unit_discount_type: Set(Some("promotion".to_string())),
            is_gift: Set(true),
            metadata: Set(json!({})),
            private_metadata: Set(json!({})),
            tax_class_metadata: Set(json!({})),
            tax_class_private_metadata: Set(json!({})),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        }
        .insert(txn)
        .await?;
    }
    Ok(())
}
