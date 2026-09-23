//! Gift cards over Django's `giftcard_*` tables.
//!
//! Mirrors `saleor/giftcard/{models,utils,events}.py`:
//! - codes are `XXXX-XXXX-XXXX` uppercase hex (`generate_random_code`);
//! - `active(date)` = `is_active` and (`expiry_date` null or `>= today`);
//! - checkout attach requires currency match + no usage restriction, and
//!   fails with a *generic* error so assignees are never leaked
//!   (`InvalidPromoCode`);
//! - balances never go negative (writers clamp; the DB check constraint
//!   `giftcard_current_balance_non_negative` is the backstop);
//! - every mutation writes a `giftcard_giftcardevent` row.

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use saleor_rustify_core::giftcard as domain;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, sea_query::LockType,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        checkout_checkout_gift_cards, giftcard_giftcard, giftcard_giftcardevent,
    },
    DbError, Result,
};

/// Event type strings — mirrors `GiftCardEvents` in `saleor/giftcard/__init__.py`.
pub mod events {
    pub const ISSUED: &str = "issued";
    pub const ACTIVATED: &str = "activated";
    pub const DEACTIVATED: &str = "deactivated";
    pub const BALANCE_ADJUSTED: &str = "balance_adjusted";
    pub const USED_IN_ORDER: &str = "used_in_order";
    pub const REFUNDED_IN_ORDER: &str = "refunded_in_order";
    pub const ASSIGNED_TO_USER: &str = "assigned_to_user";
}

pub struct IssueInput {
    pub initial_balance: Decimal,
    pub currency: String,
    pub created_by_email: Option<String>,
    pub expiry_date: Option<NaiveDate>,
    pub is_active: bool,
    /// Caller-supplied code (Django `code` on create; validated unique).
    /// None mints via the generator loop.
    pub custom_code: Option<String>,
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

async fn write_event(
    txn: &impl ConnectionTrait,
    gift_card_id: i32,
    event_type: &str,
    parameters: serde_json::Value,
    user_id: Option<i32>,
    order_id: Option<Uuid>,
) -> Result<()> {
    giftcard_giftcardevent::ActiveModel {
        date: Set(Utc::now().into()),
        r#type: Set(event_type.to_string()),
        parameters: Set(parameters),
        app_id: Set(None),
        gift_card_id: Set(gift_card_id),
        user_id: Set(user_id),
        order_id: Set(order_id),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Ok(())
}

fn balance_params(card: &giftcard_giftcard::Model) -> serde_json::Value {
    json!({"balance": {
        "currency": card.currency,
        "initial_balance": card.initial_balance_amount.to_string(),
        "current_balance": card.current_balance_amount.to_string(),
    }})
}

/// Issue a new card with a fresh unique code. Retries on code collision
/// (mirrors `generate_promo_code`'s availability loop).
pub async fn issue(
    db: &DatabaseConnection,
    input: IssueInput,
    user_id: Option<i32>,
) -> Result<giftcard_giftcard::Model> {
    use sea_orm::TransactionTrait;
    if input.initial_balance < Decimal::ZERO {
        return Err(DbError::GiftCardNotApplicable("balance cannot be negative".into()));
    }
    let txn = db.begin().await?;
    if let Some(custom) = input.custom_code.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        if custom.len() > 16 {
            return Err(DbError::GiftCardNotApplicable("code must be 16 characters or fewer".into()));
        }
        let exists = giftcard_giftcard::Entity::find()
            .filter(giftcard_giftcard::Column::Code.eq(&custom))
            .one(&txn)
            .await?;
        if exists.is_some() {
            return Err(DbError::GiftCardConflict("promo code already exists".into()));
        }
        let card = giftcard_giftcard::ActiveModel {
            code: Set(custom),
            created_at: Set(Utc::now().into()),
            last_used_on: Set(None),
            is_active: Set(input.is_active),
            initial_balance_amount: Set(input.initial_balance),
            current_balance_amount: Set(input.initial_balance),
            currency: Set(input.currency.clone()),
            created_by_email: Set(input.created_by_email.clone()),
            expiry_date: Set(input.expiry_date),
            metadata: Set(json!({})),
            private_metadata: Set(json!({})),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        let mut params = balance_params(&card);
        if let Some(exp) = input.expiry_date {
            params["expiry_date"] = json!(exp.to_string());
        }
        write_event(&txn, card.id, events::ISSUED, params, user_id, None).await?;
        txn.commit().await?;
        return Ok(card);
    }
    for _ in 0..10 {
        let code = domain::generate_code();
        let exists = giftcard_giftcard::Entity::find()
            .filter(giftcard_giftcard::Column::Code.eq(&code))
            .one(&txn)
            .await?;
        if exists.is_some() {
            continue;
        }
        let card = giftcard_giftcard::ActiveModel {
            code: Set(code),
            created_at: Set(Utc::now().into()),
            last_used_on: Set(None),
            is_active: Set(input.is_active),
            initial_balance_amount: Set(input.initial_balance),
            current_balance_amount: Set(input.initial_balance),
            currency: Set(input.currency.clone()),
            created_by_email: Set(input.created_by_email.clone()),
            expiry_date: Set(input.expiry_date),
            metadata: Set(json!({})),
            private_metadata: Set(json!({})),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        let mut params = balance_params(&card);
        if let Some(exp) = input.expiry_date {
            params["expiry_date"] = json!(exp.to_string());
        }
        write_event(&txn, card.id, events::ISSUED, params, user_id, None).await?;
        txn.commit().await?;
        return Ok(card);
    }
    Err(DbError::GiftCardConflict("could not mint a unique code".into()))
}

pub async fn get_by_code(
    db: &impl ConnectionTrait,
    code: &str,
) -> Result<Option<giftcard_giftcard::Model>> {
    Ok(giftcard_giftcard::Entity::find()
        .filter(giftcard_giftcard::Column::Code.eq(code))
        .one(db)
        .await?)
}

/// Shared usability gate for checkout attach: currency match, active filter,
/// no usage restriction. Generic error — never leaks the assignee.
fn require_usable_for_checkout(
    card: &giftcard_giftcard::Model,
    currency: &str,
    user_id: Option<i32>,
) -> Result<()> {
    if card.currency != currency
        || !domain::is_active_card(card.is_active, card.expiry_date, today())
        || domain::restriction_reason(
            card.assigned_to_id,
            card.assigned_to_email.as_deref(),
            user_id,
        )
        .is_some()
    {
        return Err(DbError::GiftCardNotApplicable("promo code is invalid".into()));
    }
    Ok(())
}

/// Attach a card to a checkout (`add_gift_card_code_to_checkout`).
/// Idempotent — re-attaching is a no-op, like Django's m2m `.add()`.
pub async fn attach_to_checkout(
    db: &DatabaseConnection,
    checkout_token: Uuid,
    code: &str,
    currency: &str,
    user_id: Option<i32>,
) -> Result<giftcard_giftcard::Model> {
    let card = get_by_code(db, code)
        .await?
        .ok_or_else(|| DbError::GiftCardNotApplicable("promo code is invalid".into()))?;
    require_usable_for_checkout(&card, currency, user_id)?;
    let linked = checkout_checkout_gift_cards::Entity::find()
        .filter(checkout_checkout_gift_cards::Column::CheckoutId.eq(checkout_token))
        .filter(checkout_checkout_gift_cards::Column::GiftcardId.eq(card.id))
        .one(db)
        .await?;
    if linked.is_none() {
        checkout_checkout_gift_cards::ActiveModel {
            checkout_id: Set(checkout_token),
            giftcard_id: Set(card.id),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(card)
}

/// Detach a card (`remove_gift_card_code_from_checkout_or_error`).
pub async fn detach_from_checkout(
    db: &impl ConnectionTrait,
    checkout_token: Uuid,
    code: &str,
) -> Result<()> {
    let card = get_by_code(db, code)
        .await?
        .ok_or_else(|| DbError::GiftCardNotFound(code.to_string()))?;
    let deleted = checkout_checkout_gift_cards::Entity::delete_many()
        .filter(checkout_checkout_gift_cards::Column::CheckoutId.eq(checkout_token))
        .filter(checkout_checkout_gift_cards::Column::GiftcardId.eq(card.id))
        .exec(db)
        .await?;
    if deleted.rows_affected == 0 {
        return Err(DbError::GiftCardNotApplicable(
            "cannot remove a gift card not attached to this checkout".into(),
        ));
    }
    Ok(())
}

/// Cards attached to a checkout, ordered by code (stable redemption order).
pub async fn checkout_cards(
    db: &impl ConnectionTrait,
    checkout_token: Uuid,
) -> Result<Vec<giftcard_giftcard::Model>> {
    
    Ok(giftcard_giftcard::Entity::find()
        .inner_join(checkout_checkout_gift_cards::Entity)
        .filter(checkout_checkout_gift_cards::Column::CheckoutId.eq(checkout_token))
        .order_by_asc(giftcard_giftcard::Column::Code)
        .all(db)
        .await?)
}

/// Sum of *active* attached balances — mirrors
/// `Checkout.get_total_gift_cards_balance`.
pub async fn checkout_balance(
    db: &impl ConnectionTrait,
    checkout_token: Uuid,
    currency: &str,
) -> Result<Decimal> {
    let cards = checkout_cards(db, checkout_token).await?;
    let sum: Decimal = cards
        .iter()
        .filter(|c| {
            c.currency == currency
                && domain::is_active_card(c.is_active, c.expiry_date, today())
        })
        .map(|c| c.current_balance_amount.max(Decimal::ZERO))
        .sum();
    Ok(sum)
}

/// Lock a card row for update inside the caller's transaction.
async fn lock_card(
    txn: &impl ConnectionTrait,
    code: &str,
) -> Result<giftcard_giftcard::Model> {
    giftcard_giftcard::Entity::find()
        .filter(giftcard_giftcard::Column::Code.eq(code))
        .lock(LockType::Update)
        .one(txn)
        .await?
        .ok_or_else(|| DbError::GiftCardNotFound(code.to_string()))
}

/// Spend a card against an order. Returns the amount actually taken
/// (`min(balance, requested)`, floored at zero) and records USED_IN_ORDER.
/// `order_id` is optional so redemptions can be recorded without a live
/// order row (the Django event carries the order when one exists).
pub async fn redeem_for_order(
    db: &DatabaseConnection,
    code: &str,
    order_id: Option<Uuid>,
    requested: Decimal,
    user_id: Option<i32>,
) -> Result<Decimal> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let take = redeem_for_order_tx(&txn, code, order_id, requested, user_id).await?;
    txn.commit().await?;
    Ok(take)
}

/// Transactional core of [`redeem_for_order`]: runs inside the caller's
/// transaction (used by the atomic checkout-complete pipeline, where the
/// card debit must commit or roll back with the order itself).
pub async fn redeem_for_order_tx(
    txn: &impl ConnectionTrait,
    code: &str,
    order_id: Option<Uuid>,
    requested: Decimal,
    user_id: Option<i32>,
) -> Result<Decimal> {
    let card = lock_card(txn, code).await?;
    if !domain::is_active_card(card.is_active, card.expiry_date, today()) {
        return Err(DbError::GiftCardNotApplicable(
            "gift card is not active".into(),
        ));
    }
    let take = domain::redeem_amount(card.current_balance_amount, requested);
    let mut am: giftcard_giftcard::ActiveModel = card.clone().into();
    am.current_balance_amount = Set(card.current_balance_amount - take);
    am.last_used_on = Set(Some(Utc::now().into()));
    am.used_by_id = Set(user_id);
    let updated = am.update(txn).await?;
    let mut params = balance_params(&updated);
    if let Some(oid) = order_id {
        params["order_id"] = json!(oid.to_string());
    }
    params["amount_taken"] = json!(take.to_string());
    write_event(txn, card.id, events::USED_IN_ORDER, params, user_id, order_id).await?;
    Ok(take)
}

/// Return funds to a card (order refund path). Records REFUNDED_IN_ORDER.
pub async fn refund_to_card(
    db: &DatabaseConnection,
    code: &str,
    amount: Decimal,
    order_id: Option<Uuid>,
) -> Result<giftcard_giftcard::Model> {
    use sea_orm::TransactionTrait;
    if amount < Decimal::ZERO {
        return Err(DbError::GiftCardNotApplicable("refund cannot be negative".into()));
    }
    let txn = db.begin().await?;
    let card = lock_card(&txn, code).await?;
    let mut am: giftcard_giftcard::ActiveModel = card.clone().into();
    am.current_balance_amount = Set(card.current_balance_amount + amount);
    let updated = am.update(&txn).await?;
    let mut params = balance_params(&updated);
    params["amount_refunded"] = json!(amount.to_string());
    write_event(&txn, card.id, events::REFUNDED_IN_ORDER, params, None, order_id).await?;
    txn.commit().await?;
    Ok(updated)
}

/// Staff balance adjustment. Records BALANCE_ADJUSTED with old values.
pub async fn adjust_balance(
    db: &DatabaseConnection,
    code: &str,
    new_balance: Decimal,
    user_id: Option<i32>,
) -> Result<giftcard_giftcard::Model> {
    use sea_orm::TransactionTrait;
    if new_balance < Decimal::ZERO {
        return Err(DbError::GiftCardNotApplicable("balance cannot be negative".into()));
    }
    let txn = db.begin().await?;
    let card = lock_card(&txn, code).await?;
    let mut am: giftcard_giftcard::ActiveModel = card.clone().into();
    am.current_balance_amount = Set(new_balance);
    let updated = am.update(&txn).await?;
    let params = json!({"balance": {
        "currency": updated.currency,
        "current_balance": updated.current_balance_amount.to_string(),
        "initial_balance": updated.initial_balance_amount.to_string(),
        "old_current_balance": card.current_balance_amount.to_string(),
        "old_initial_balance": card.initial_balance_amount.to_string(),
    }});
    write_event(&txn, card.id, events::BALANCE_ADJUSTED, params, user_id, None).await?;
    txn.commit().await?;
    Ok(updated)
}

/// Activate/deactivate. Records ACTIVATED/DEACTIVATED.
pub async fn set_active(
    db: &DatabaseConnection,
    code: &str,
    active: bool,
    user_id: Option<i32>,
) -> Result<giftcard_giftcard::Model> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let card = lock_card(&txn, code).await?;
    let mut am: giftcard_giftcard::ActiveModel = card.clone().into();
    am.is_active = Set(active);
    let updated = am.update(&txn).await?;
    let event = if active { events::ACTIVATED } else { events::DEACTIVATED };
    write_event(&txn, card.id, event, balance_params(&updated), user_id, None).await?;
    txn.commit().await?;
    Ok(updated)
}

/// Restrict a card to a customer (`assign_gift_card_to_user`).
/// Refuses cards already spent in an order — same guard as Django.
pub async fn assign(
    db: &DatabaseConnection,
    code: &str,
    user_id: i32,
    email: &str,
    actor_id: Option<i32>,
) -> Result<giftcard_giftcard::Model> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let card = lock_card(&txn, code).await?;
    if card.last_used_on.is_some() {
        return Err(DbError::GiftCardNotApplicable(
            "cannot assign a gift card that was already used in an order".into(),
        ));
    }
    let mut am: giftcard_giftcard::ActiveModel = card.clone().into();
    am.assigned_to_id = Set(Some(user_id));
    am.assigned_to_email = Set(Some(email.to_string()));
    let updated = am.update(&txn).await?;
    write_event(&txn, card.id, events::ASSIGNED_TO_USER, balance_params(&updated), actor_id, None).await?;
    txn.commit().await?;
    Ok(updated)
}

/// Tag helpers (Django `GiftCard.tags` m2m).
pub async fn add_tags(db: &impl ConnectionTrait, card_id: i32, tags: &[String]) -> Result<()> {
    use crate::entities::{giftcard_giftcard_tags, giftcard_giftcardtag};
    for name in tags {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let tag = match giftcard_giftcardtag::Entity::find()
            .filter(giftcard_giftcardtag::Column::Name.eq(name))
            .one(db)
            .await?
        {
            Some(t) => t,
            None => giftcard_giftcardtag::ActiveModel {
                name: Set(name.to_string()),
                ..Default::default()
            }
            .insert(db)
            .await?,
        };
        let exists = giftcard_giftcard_tags::Entity::find()
            .filter(giftcard_giftcard_tags::Column::GiftcardId.eq(card_id))
            .filter(giftcard_giftcard_tags::Column::GiftcardtagId.eq(tag.id))
            .one(db)
            .await?
            .is_some();
        if !exists {
            giftcard_giftcard_tags::ActiveModel {
                giftcard_id: Set(card_id),
                giftcardtag_id: Set(tag.id),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

pub async fn remove_tags(db: &impl ConnectionTrait, card_id: i32, tags: &[String]) -> Result<()> {
    use crate::entities::{giftcard_giftcard_tags, giftcard_giftcardtag};
    for name in tags {
        if let Some(tag) = giftcard_giftcardtag::Entity::find()
            .filter(giftcard_giftcardtag::Column::Name.eq(name.trim()))
            .one(db)
            .await?
        {
            giftcard_giftcard_tags::Entity::delete_many()
                .filter(giftcard_giftcard_tags::Column::GiftcardId.eq(card_id))
                .filter(giftcard_giftcard_tags::Column::GiftcardtagId.eq(tag.id))
                .exec(db)
                .await?;
        }
    }
    Ok(())
}

#[derive(Default)]
pub struct UpdateCard {
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
    pub expiry_date: Option<Option<chrono::NaiveDate>>,
    pub balance_amount: Option<Decimal>,
    pub is_active: Option<bool>,
}

/// Staff gift-card update (Django `giftCardUpdate`): tags, expiry, balance
/// (resets both initial and current, like Django), active flag.
pub async fn update_card(
    db: &DatabaseConnection,
    card_id: i32,
    upd: &UpdateCard,
    actor_id: Option<i32>,
) -> Result<giftcard_giftcard::Model> {
    use sea_orm::TransactionTrait;
    if let Some(b) = upd.balance_amount {
        if b < Decimal::ZERO {
            return Err(DbError::GiftCardNotApplicable("balance cannot be negative".into()));
        }
    }
    let txn = db.begin().await?;
    let card = giftcard_giftcard::Entity::find_by_id(card_id)
        .one(&txn)
        .await?
        .ok_or_else(|| DbError::GiftCardNotFound(card_id.to_string()))?;
    add_tags(&txn, card_id, &upd.add_tags).await?;
    remove_tags(&txn, card_id, &upd.remove_tags).await?;
    let mut am: giftcard_giftcard::ActiveModel = card.into();
    if let Some(e) = upd.expiry_date {
        am.expiry_date = Set(e);
    }
    if let Some(b) = upd.balance_amount {
        am.initial_balance_amount = Set(b);
        am.current_balance_amount = Set(b);
    }
    if let Some(a) = upd.is_active {
        am.is_active = Set(a);
    }
    let updated = am.update(&txn).await?;
    write_event(&txn, card_id, events::BALANCE_ADJUSTED, balance_params(&updated), actor_id, None).await?;
    txn.commit().await?;
    Ok(updated)
}

/// Delete a card (Django `giftCardDelete`): unused balances only — spent
/// cards are history and refuse.
pub async fn delete_card(db: &DatabaseConnection, card_id: i32) -> Result<()> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let card = giftcard_giftcard::Entity::find_by_id(card_id)
        .one(&txn)
        .await?
        .ok_or_else(|| DbError::GiftCardNotFound(card_id.to_string()))?;
    if card.last_used_on.is_some() || card.current_balance_amount < card.initial_balance_amount {
        return Err(DbError::GiftCardNotApplicable(
            "only unused gift cards can be deleted".into(),
        ));
    }
    use crate::entities::{giftcard_giftcard_tags, giftcard_giftcardevent};
    giftcard_giftcard_tags::Entity::delete_many()
        .filter(giftcard_giftcard_tags::Column::GiftcardId.eq(card_id))
        .exec(&txn)
        .await?;
    // Django cascades events at the ORM level (no DB cascade here).
    giftcard_giftcardevent::Entity::delete_many()
        .filter(giftcard_giftcardevent::Column::GiftCardId.eq(card_id))
        .exec(&txn)
        .await?;
    let am: giftcard_giftcard::ActiveModel = card.into();
    am.delete(&txn).await?;
    txn.commit().await?;
    Ok(())
}

/// Staff note on a card (Django `giftCardAddNote` → event row).
/// Returns the card plus the new event id for the payload.
pub async fn add_note(
    db: &DatabaseConnection,
    card_id: i32,
    message: &str,
    actor_id: Option<i32>,
) -> Result<(giftcard_giftcard::Model, i32)> {
    let message = message.trim();
    if message.is_empty() {
        return Err(DbError::GiftCardNotApplicable("message is required".into()));
    }
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let card = giftcard_giftcard::Entity::find_by_id(card_id)
        .one(&txn)
        .await?
        .ok_or_else(|| DbError::GiftCardNotFound(card_id.to_string()))?;
    let ev = giftcard_giftcardevent::ActiveModel {
        date: Set(Utc::now().into()),
        r#type: Set("note_added".to_string()),
        parameters: Set(json!({"message": message})),
        app_id: Set(None),
        gift_card_id: Set(card_id),
        user_id: Set(actor_id),
        order_id: Set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    txn.commit().await?;
    Ok((card, ev.id))
}

/// Resend the card code email (Django `giftCardResend`): records the resend
/// event + optional new delivery email. The actual send rides the SMTP
/// milestone (same documented gap as all mails); the dashboard flow
/// (event + no error) works today.
pub async fn resend(
    db: &DatabaseConnection,
    card_id: i32,
    email: Option<String>,
    actor_id: Option<i32>,
) -> Result<giftcard_giftcard::Model> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let card = giftcard_giftcard::Entity::find_by_id(card_id)
        .one(&txn)
        .await?
        .ok_or_else(|| DbError::GiftCardNotFound(card_id.to_string()))?;
    if !card.is_active {
        return Err(DbError::GiftCardNotApplicable("inactive cards cannot be resent".into()));
    }
    if let Some(e) = email.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        let mut am: giftcard_giftcard::ActiveModel = card.clone().into();
        am.assigned_to_email = Set(Some(e.clone()));
        am.update(&txn).await?;
        write_event(&txn, card_id, events::ASSIGNED_TO_USER, json!({"email": e}), actor_id, None).await?;
    } else {
        write_event(&txn, card_id, "resent", json!({}), actor_id, None).await?;
    }
    txn.commit().await?;
    Ok(giftcard_giftcard::Entity::find_by_id(card_id)
        .one(db)
        .await?
        .ok_or_else(|| DbError::GiftCardNotFound(card_id.to_string()))?)
}

/// Expiry settings on the site row (Django `giftCardSettingsUpdate`).
pub async fn update_settings(
    db: &DatabaseConnection,
    expiry_type: Option<String>,
    expiry_days: Option<i32>,
) -> Result<()> {
    use crate::entities::site_sitesettings;
    let row = site_sitesettings::Entity::find_by_id(1)
        .one(db)
        .await?
        .ok_or_else(|| DbError::GiftCardNotFound("site settings".into()))?;
    let mut am: site_sitesettings::ActiveModel = row.into();
    if let Some(t) = expiry_type {
        if t != "never_expire" && t != "expiry_period" {
            return Err(DbError::GiftCardNotApplicable("unknown expiry type".into()));
        }
        am.gift_card_expiry_type = Set(t);
    }
    if let Some(d) = expiry_days {
        if d < 1 {
            return Err(DbError::GiftCardNotApplicable("expiry period must be positive".into()));
        }
        am.gift_card_expiry_period = Set(Some(d));
        am.gift_card_expiry_period_type = Set(Some("days".to_string()));
    }
    am.update(db).await?;
    Ok(())
}

/// Bulk issue `count` cards with one balance (Django `giftCardBulkCreate`).
pub async fn bulk_issue(
    db: &DatabaseConnection,
    count: i32,
    balance: Decimal,
    currency: &str,
    tags: &[String],
    expiry: Option<chrono::NaiveDate>,
    is_active: bool,
    actor_id: Option<i32>,
) -> Result<Vec<giftcard_giftcard::Model>> {
    if count < 1 || count > 100 {
        return Err(DbError::GiftCardNotApplicable("count must be 1-100".into()));
    }
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let card = issue(
            db,
            IssueInput {
                initial_balance: balance,
                currency: currency.to_string(),
                created_by_email: None,
                expiry_date: expiry,
                is_active,
                custom_code: None,
            },
            actor_id,
        )
        .await?;
        if !tags.is_empty() {
            add_tags(db, card.id, tags).await?;
        }
        out.push(card);
    }
    Ok(out)
}

/// Clear a card's assignment (Django `giftCardUnassignUser`).
pub async fn unassign(
    db: &DatabaseConnection,
    code: &str,
    actor_id: Option<i32>,
) -> Result<giftcard_giftcard::Model> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let card = lock_card(&txn, code).await?;
    if card.assigned_to_id.is_none() && card.assigned_to_email.is_none() {
        return Err(DbError::GiftCardNotApplicable("gift card is not assigned".into()));
    }
    let mut am: giftcard_giftcard::ActiveModel = card.into();
    am.assigned_to_id = Set(None);
    am.assigned_to_email = Set(None);
    let updated = am.update(&txn).await?;
    write_event(&txn, updated.id, "unassigned", json!({}), actor_id, None).await?;
    txn.commit().await?;
    Ok(updated)
}
