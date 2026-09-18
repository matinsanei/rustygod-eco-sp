//! Payment transactions over Django's tables
//! (`payment_transactionitem`, `payment_transactionevent`).
//!
//! Mirrors `saleor/payment/`:
//! - amounts derive from events via the exact recalculation
//!   (`transaction_item_calculations.py` port in `core::payments`);
//! - creation is idempotent on `(app_identifier, idempotency_key)`
//!   (Django's `unique_transaction_idempotency`);
//! - events are idempotent on `(transaction_id, idempotency_key)`;
//! - order `authorize_status`/`charge_status` refresh from coverage
//!   (full/partial/none) after every mutation.

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{order_order, payment_transactionevent, payment_transactionitem},
    DbError, Result,
};

pub struct NewTransaction {
    pub checkout_id: Option<Uuid>,
    pub order_id: Option<Uuid>,
    pub currency: String,
    pub name: String,
    pub app_identifier: Option<String>,
    pub idempotency_key: Option<String>,
    pub available_actions: Vec<String>,
}

pub struct TxnView {
    pub id: i32,
    pub token: Uuid,
    pub currency: String,
    pub order_id: Option<Uuid>,
    pub authorized: Decimal,
    pub charged: Decimal,
    pub refunded: Decimal,
    pub canceled: Decimal,
    pub authorize_pending: Decimal,
    pub charge_pending: Decimal,
    pub refund_pending: Decimal,
    pub cancel_pending: Decimal,
    pub available_actions: Vec<String>,
    pub psp_reference: Option<String>,
}

/// Idempotent create: same (app_identifier, idempotency_key) returns the
/// existing row instead of duplicating (Django constraint honored).
pub async fn create_transaction(
    db: &DatabaseConnection,
    new: &NewTransaction,
) -> Result<TxnView> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    if let (Some(app), Some(key)) = (new.app_identifier.clone(), new.idempotency_key.clone()) {
        if let Some(existing) = payment_transactionitem::Entity::find()
            .filter(payment_transactionitem::Column::AppIdentifier.eq(app))
            .filter(payment_transactionitem::Column::IdempotencyKey.eq(key))
            .one(&txn)
            .await?
        {
            let v = view(&txn, existing.id).await?;
            txn.commit().await?;
            return Ok(v);
        }
    }
    let t = Utc::now();
    let row = payment_transactionitem::ActiveModel {
        token: Set(Uuid::new_v4()),
        created_at: Set(t.into()),
        modified_at: Set(t.into()),
        currency: Set(new.currency.clone()),
        name: Set(Some(new.name.clone())),
        message: Set(Some(String::new())),
        checkout_id: Set(new.checkout_id),
        order_id: Set(new.order_id),
        app_identifier: Set(new.app_identifier.clone()),
        idempotency_key: Set(new.idempotency_key.clone()),
        available_actions: Set(new.available_actions.clone()),
        charged_value: Set(Decimal::ZERO),
        authorized_value: Set(Decimal::ZERO),
        refunded_value: Set(Decimal::ZERO),
        canceled_value: Set(Decimal::ZERO),
        refund_pending_value: Set(Decimal::ZERO),
        charge_pending_value: Set(Decimal::ZERO),
        authorize_pending_value: Set(Decimal::ZERO),
        cancel_pending_value: Set(Decimal::ZERO),
        last_refund_success: Set(true),
        use_old_id: Set(false),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let id = row.id;
    txn.commit().await?;
    view(db, id).await
}

pub struct NewEvent {
    pub event_type: String,
    pub amount: Decimal,
    pub currency: String,
    pub psp_reference: Option<String>,
    pub message: String,
    pub idempotency_key: Option<String>,
    pub include_in_calculations: bool,
}

/// Idempotent event report + bucket recalculation in one transaction.
pub async fn report_event(
    db: &DatabaseConnection,
    transaction_id: i32,
    ev: &NewEvent,
) -> Result<TxnView> {
    use sea_orm::TransactionTrait;
    if ev.amount < Decimal::ZERO {
        return Err(DbError::SeaOrm(sea_orm::DbErr::Custom(
            "event amount must be >= 0".into(),
        )));
    }
    let txn = db.begin().await?;
    if let Some(key) = ev.idempotency_key.clone() {
        if payment_transactionevent::Entity::find()
            .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
            .filter(payment_transactionevent::Column::IdempotencyKey.eq(key))
            .one(&txn)
            .await?
            .is_some()
        {
            // Replay: no duplicate, just return current state.
            txn.commit().await?;
            return view(db, transaction_id).await;
        }
    }
    payment_transactionevent::ActiveModel {
        created_at: Set(Utc::now().into()),
        transaction_id: Set(transaction_id),
        r#type: Set(ev.event_type.clone()),
        amount_value: Set(ev.amount),
        currency: Set(ev.currency.clone()),
        psp_reference: Set(ev.psp_reference.clone()),
        message: Set(Some(ev.message.clone())),
        idempotency_key: Set(ev.idempotency_key.clone()),
        include_in_calculations: Set(ev.include_in_calculations),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    recalc_in(&txn, transaction_id).await?;
    txn.commit().await?;
    let v = view(db, transaction_id).await?;
    refresh_order_statuses(db, v.order_id).await?;
    Ok(v)
}

async fn recalc_in(db: &impl sea_orm::ConnectionTrait, transaction_id: i32) -> Result<()> {
    let events = payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
        .all(db)
        .await?;
    let calc: Vec<rustygod_core::payments::CalcEvent> = events
        .into_iter()
        .map(|e| rustygod_core::payments::CalcEvent {
            event_type: e.r#type,
            psp_reference: e.psp_reference,
            amount: e.amount_value,
            include_in_calculations: e.include_in_calculations,
        })
        .collect();
    let b = rustygod_core::payments::recalculate(&calc);
    let row = payment_transactionitem::Entity::find_by_id(transaction_id)
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(transaction_id.to_string())))?;
    let mut am: payment_transactionitem::ActiveModel = row.into();
    am.authorized_value = Set(b.authorized);
    am.authorize_pending_value = Set(b.authorize_pending);
    am.charged_value = Set(b.charged);
    am.charge_pending_value = Set(b.charge_pending);
    am.refunded_value = Set(b.refunded);
    am.refund_pending_value = Set(b.refund_pending);
    am.canceled_value = Set(b.canceled);
    am.cancel_pending_value = Set(b.cancel_pending);
    am.modified_at = Set(Utc::now().into());
    am.update(db).await?;
    Ok(())
}

pub async fn view(
    db: &impl sea_orm::ConnectionTrait,
    transaction_id: i32,
) -> Result<TxnView> {
    let row = payment_transactionitem::Entity::find_by_id(transaction_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(transaction_id.to_string()))
        })?;
    Ok(TxnView {
        id: row.id,
        token: row.token,
        currency: row.currency,
        order_id: row.order_id,
        authorized: row.authorized_value,
        charged: row.charged_value,
        refunded: row.refunded_value,
        canceled: row.canceled_value,
        authorize_pending: row.authorize_pending_value,
        charge_pending: row.charge_pending_value,
        refund_pending: row.refund_pending_value,
        cancel_pending: row.cancel_pending_value,
        available_actions: row.available_actions,
        psp_reference: row.psp_reference,
    })
}

/// Refresh an order's authorize/charge statuses from its transactions'
/// coverage, mirroring Django's evaluation (full/partial/none over
/// order total; granted refunds out of v1 scope).
pub async fn refresh_order_statuses(
    db: &DatabaseConnection,
    order_id: Option<Uuid>,
) -> Result<()> {
    let Some(oid) = order_id else { return Ok(()) };
    let txns = payment_transactionitem::Entity::find()
        .filter(payment_transactionitem::Column::OrderId.eq(oid))
        .all(db)
        .await?;
    let auth_covered: Decimal = txns.iter().map(|t| t.authorized_value + t.charged_value).sum();
    let charge_covered: Decimal = txns.iter().map(|t| t.charged_value).sum();
    let Some(order) = order_order::Entity::find_by_id(oid).one(db).await? else {
        return Ok(());
    };
    let total = order.total_gross_amount;
    let mut am: order_order::ActiveModel = order.into();
    am.authorize_status = Set(
        rustygod_core::payments::coverage_status(auth_covered, total).to_string(),
    );
    am.charge_status = Set(
        rustygod_core::payments::coverage_status(charge_covered, total).to_string(),
    );
    am.update(db).await?;
    Ok(())
}

// ---------- Manual gateway ----------
// Synchronous request+success pairs (like Saleor's manual/dummy gateway):
// funds move immediately, psp_reference generated, available_actions
// updated to the post-action set.

fn gateway_err(msg: impl Into<String>) -> DbError {
    DbError::SeaOrm(sea_orm::DbErr::Custom(msg.into()))
}

async fn set_actions(
    db: &impl sea_orm::ConnectionTrait,
    transaction_id: i32,
    actions: &[&str],
    psp_reference: Option<String>,
) -> Result<()> {
    let row = payment_transactionitem::Entity::find_by_id(transaction_id)
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(transaction_id.to_string())))?;
    let mut am: payment_transactionitem::ActiveModel = row.into();
    am.available_actions = Set(actions.iter().map(|s| s.to_string()).collect());
    if let Some(psp) = psp_reference {
        am.psp_reference = Set(Some(psp));
    }
    am.update(db).await?;
    Ok(())
}

fn pair_events(
    action: &str,
    amount: Decimal,
    currency: &str,
    psp: &str,
    key: &str,
) -> (NewEvent, NewEvent) {
    let mk = |t: String, k: String| NewEvent {
        event_type: t,
        amount,
        currency: currency.to_string(),
        psp_reference: Some(psp.to_string()),
        message: format!("manual gateway {action}"),
        idempotency_key: Some(k),
        include_in_calculations: true,
    };
    (
        mk(format!("{action}_request"), format!("{key}-req")),
        mk(format!("{action}_success"), format!("{key}-ok")),
    )
}

/// Authorize funds on a transaction.
pub async fn authorize(
    db: &DatabaseConnection,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<TxnView> {
    if amount <= Decimal::ZERO {
        return Err(gateway_err("authorize amount must be positive"));
    }
    let cur = view(db, transaction_id).await?;
    let (req, ok) = pair_events("authorization", amount, &cur.currency, &Uuid::new_v4().to_string(), idempotency_key);
    report_event(db, transaction_id, &req).await?;
    let v = report_event(db, transaction_id, &ok).await?;
    set_actions(db, transaction_id, &["charge", "cancel"], v.psp_reference.clone()).await?;
    view(db, transaction_id).await
}

/// Charge previously authorized funds. Guarded: never above the
/// authorized-but-uncharged remainder.
pub async fn charge(
    db: &DatabaseConnection,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<TxnView> {
    if amount <= Decimal::ZERO {
        return Err(gateway_err("charge amount must be positive"));
    }
    let cur = view(db, transaction_id).await?;
    if amount > cur.authorized {
        return Err(gateway_err(format!(
            "cannot charge {amount}: only {} authorized",
            cur.authorized
        )));
    }
    let (req, ok) = pair_events("charge", amount, &cur.currency, &Uuid::new_v4().to_string(), idempotency_key);
    report_event(db, transaction_id, &req).await?;
    let v = report_event(db, transaction_id, &ok).await?;
    set_actions(db, transaction_id, &["refund"], v.psp_reference.clone()).await?;
    view(db, transaction_id).await
}

/// Refund charged funds. Guarded: never above charged-minus-refunded.
pub async fn refund(
    db: &DatabaseConnection,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<TxnView> {
    if amount <= Decimal::ZERO {
        return Err(gateway_err("refund amount must be positive"));
    }
    let cur = view(db, transaction_id).await?;
    if amount > cur.charged - cur.refunded {
        return Err(gateway_err(format!(
            "cannot refund {amount}: only {} charged and unrefunded",
            cur.charged - cur.refunded
        )));
    }
    let (req, ok) = pair_events("refund", amount, &cur.currency, &Uuid::new_v4().to_string(), idempotency_key);
    report_event(db, transaction_id, &req).await?;
    report_event(db, transaction_id, &ok).await?;
    view(db, transaction_id).await
}

/// Cancel authorized-but-uncharged funds.
pub async fn cancel(
    db: &DatabaseConnection,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<TxnView> {
    if amount <= Decimal::ZERO {
        return Err(gateway_err("cancel amount must be positive"));
    }
    let cur = view(db, transaction_id).await?;
    if amount > cur.authorized {
        return Err(gateway_err(format!(
            "cannot cancel {amount}: only {} authorized",
            cur.authorized
        )));
    }
    let (req, ok) = pair_events("cancel", amount, &cur.currency, &Uuid::new_v4().to_string(), idempotency_key);
    report_event(db, transaction_id, &req).await?;
    report_event(db, transaction_id, &ok).await?;
    set_actions(db, transaction_id, &[], None).await?;
    view(db, transaction_id).await
}
