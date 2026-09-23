//! Payment transactions over Django's tables
//! (`payment_transactionitem`, `payment_transactionevent`).
//!
//! Mirrors `saleor/payment/`:
//! - amounts derive from events via the exact recalculation
//!   (`transaction_item_calculations.py` port in `core::payments`);
//! - creation is idempotent on `(app_identifier, idempotency_key)`
//!   (Django's `unique_transaction_idempotency`);
//! - events are idempotent on `(transaction_id, idempotency_key)`, PLUS
//!   Saleor's `deduplicate_event` port: same `(transaction_id,
//!   psp_reference, type)` with the same amount is an already-processed
//!   replay; with a different amount it records a (math-excluded) failure
//!   event and errors — PSP double-delivery can never fork the buckets;
//! - a second `authorization_success` on one transaction is rejected
//!   (`ALREADY_EXISTS`, Django: use `AUTHORIZATION_ADJUSTMENT`);
//! - writes serialize on the transaction row (`FOR UPDATE`), so concurrent
//!   callbacks can't double-settle;
//! - order `authorize_status`/`charge_status` refresh from coverage
//!   (full/partial/none) after every mutation.

use chrono::Utc;
use rust_decimal::Decimal;
use saleor_rustify_core::psp::{Psp, PspAction, PspOutcome};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
    Set, sea_query::LockType,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{order_order, payment_payment, payment_transactionevent, payment_transactionitem},
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

impl std::fmt::Debug for TxnView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TxnView")
            .field("id", &self.id)
            .field("authorized", &self.authorized)
            .field("charged", &self.charged)
            .field("refunded", &self.refunded)
            .field("canceled", &self.canceled)
            .field("pending", &self.pending_total())
            .finish()
    }
}

impl TxnView {
    fn pending_total(&self) -> Decimal {
        self.authorize_pending
            + self.charge_pending
            + self.refund_pending
            + self.cancel_pending
    }
}

/// Idempotent create: same (app_identifier, idempotency_key) returns the
/// existing row instead of duplicating (Django constraint honored).
///
/// R3 guard (stricter than Django): one checkout can't open a second
/// transaction while another one still has money in flight (any pending
/// bucket > 0). The double-Pay-button then fails fast with
/// `ALREADY_IN_PROGRESS` instead of double-charging; retry after failure
/// or settle is unaffected (failed/settled transactions hold no pending).
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
    if let Some(checkout) = new.checkout_id {
        let open = payment_transactionitem::Entity::find()
            .filter(payment_transactionitem::Column::CheckoutId.eq(checkout))
            .all(&txn)
            .await?;
        let inflight = open.iter().any(|t| {
            t.authorize_pending_value
                + t.charge_pending_value
                + t.refund_pending_value
                + t.cancel_pending_value
                > Decimal::ZERO
        });
        if inflight {
            return Err(gateway_err(
                "checkout has a transaction with money in flight; wait for its callback (ALREADY_IN_PROGRESS)",
            ));
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
    pub related_granted_refund_id: Option<i32>,
    /// 3DS/challenge landing URL (Django's `external_url` on
    /// `*_action_required` events). None for plain money events.
    pub external_url: Option<String>,
}

/// Record-only types: Django's `get_already_existing_event` skips dedup
/// matching for these — they are customer-action records, not money.
fn is_record_only(event_type: &str) -> bool {
    event_type == "info" || event_type.ends_with("_action_required")
}

/// Write a math-excluded failure event (Django's
/// `create_failed_transaction_event`): audit trail that never moves
/// buckets, so a rejected write can't fork the amounts.
async fn record_failure_in(
    db: &impl sea_orm::ConnectionTrait,
    transaction_id: i32,
    event_type: &str,
    amount: Decimal,
    currency: &str,
    psp_reference: Option<String>,
    cause: &str,
    idempotency_key: Option<String>,
) -> Result<()> {
    payment_transactionevent::ActiveModel {
        created_at: Set(Utc::now().into()),
        transaction_id: Set(transaction_id),
        r#type: Set(event_type.to_string()),
        amount_value: Set(amount),
        currency: Set(currency.to_string()),
        psp_reference: Set(psp_reference),
        message: Set(Some(cause.to_string())),
        idempotency_key: Set(idempotency_key),
        include_in_calculations: Set(false),
        related_granted_refund_id: Set(None),
        external_url: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Idempotent event report + bucket recalculation in one transaction.
/// (See the `report_event_in` core for the rule set.)
pub async fn report_event(
    db: &DatabaseConnection,
    transaction_id: i32,
    ev: &NewEvent,
) -> Result<TxnView> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    // Rejection paths (mismatch, double auth-success) still write their
    // failure audit trail first (Django saves, then raises) — commit it
    // with the error instead of rolling back.
    if let Err(e) = report_event_in(&txn, transaction_id, ev).await {
        let _ = txn.commit().await;
        return Err(e);
    }
    txn.commit().await?;
    let v = view(db, transaction_id).await?;
    refresh_order_statuses(db, v.order_id).await?;
    Ok(v)
}

/// Transactional core of [`report_event`]: same dedup + guards +
/// recalculation, no transaction management, no order-status refresh.
/// Compose this inside bigger atomic flows (paid cancel, return+refund)
/// so money, stock, and status commit or roll back together.
pub async fn report_event_in(
    txn: &impl sea_orm::ConnectionTrait,
    transaction_id: i32,
    ev: &NewEvent,
) -> Result<()> {
    if ev.amount < Decimal::ZERO {
        return Err(DbError::SeaOrm(sea_orm::DbErr::Custom(
            "event amount must be >= 0".into(),
        )));
    }
    // Serialize all writers of one transaction (Django's select_for_update
    // in the report path): concurrent callbacks can't interleave req/ok
    // pairs or double-settle.
    payment_transactionitem::Entity::find_by_id(transaction_id)
        .lock(LockType::Update)
        .one(txn)
        .await?
        .ok_or_else(|| {
            DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(transaction_id.to_string()))
        })?;
    if let Some(key) = ev.idempotency_key.clone() {
        if payment_transactionevent::Entity::find()
            .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
            .filter(payment_transactionevent::Column::IdempotencyKey.eq(key))
            .one(txn)
            .await?
            .is_some()
        {
            // Replay: no duplicate; the caller owns commit/refresh.
            return Ok(());
        }
    }
    if !is_record_only(&ev.event_type) {
        // PSP-level dedup on (transaction, psp_reference, type).
        if let Some(psp) = ev.psp_reference.clone() {
            if let Some(existing) = payment_transactionevent::Entity::find()
                .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
                .filter(payment_transactionevent::Column::PspReference.eq(psp.clone()))
                .filter(payment_transactionevent::Column::Type.eq(ev.event_type.clone()))
                .one(txn)
                .await?
            {
                if existing.amount_value == ev.amount {
                    return Ok(());
                }
                let msg = "The transaction with provided `pspReference` and `type` already exists with different amount.";
                record_failure_in(
                    txn,
                    transaction_id,
                    &saleor_rustify_core::payments::failure_event_of(&ev.event_type)
                        .unwrap_or("info")
                        .to_string(),
                    ev.amount,
                    &ev.currency,
                    Some(psp),
                    msg,
                    ev.idempotency_key.clone().map(|k| format!("{k}-mismatch")),
                )
                .await?;
                // The failure record stays in the caller's transaction:
                // accepted callers commit it, rejecting flows roll everything
                // back together (atomicity beats the audit trail there).
                return Err(gateway_err(msg));
            }
        }
        // One authorization success per transaction, any psp.
        if ev.event_type == "authorization_success"
            && payment_transactionevent::Entity::find()
                .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
                .filter(
                    payment_transactionevent::Column::Type.eq("authorization_success".to_string()),
                )
                .one(txn)
                .await?
                .is_some()
        {
            let msg = "Event with `AUTHORIZATION_SUCCESS` already reported for the transaction. Use `AUTHORIZATION_ADJUSTMENT` to change the authorization amount.";
            record_failure_in(
                txn,
                transaction_id,
                "authorization_failure",
                ev.amount,
                &ev.currency,
                ev.psp_reference.clone(),
                msg,
                ev.idempotency_key.clone().map(|k| format!("{k}-dup-auth")),
            )
            .await?;
            return Err(gateway_err(msg));
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
        related_granted_refund_id: Set(ev.related_granted_refund_id),
        external_url: Set(ev.external_url.clone()),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    recalc_in(txn, transaction_id).await?;
    Ok(())
}

async fn recalc_in(db: &impl sea_orm::ConnectionTrait, transaction_id: i32) -> Result<()> {
    let events = payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
        .all(db)
        .await?;
    let calc: Vec<saleor_rustify_core::payments::CalcEvent> = events
        .into_iter()
        .map(|e| saleor_rustify_core::payments::CalcEvent {
            event_type: e.r#type,
            psp_reference: e.psp_reference,
            amount: e.amount_value,
            include_in_calculations: e.include_in_calculations,
            created_at: e.created_at.into(),
            id: e.id,
        })
        .collect();
    let b = saleor_rustify_core::payments::recalculate(&calc);
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
    db: &impl sea_orm::ConnectionTrait,
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
        saleor_rustify_core::payments::coverage_status(auth_covered, total).to_string(),
    );
    am.charge_status = Set(
        saleor_rustify_core::payments::coverage_status(charge_covered, total).to_string(),
    );
    am.update(db).await?;
    Ok(())
}

// ---------- PSP-driven gateway ----------
// Every money action runs through a [`Psp`]: validation first (a PSP is
// never consulted for an invalid request — PSP calls may have side
// effects), then the outcome becomes events:
// - Completed → request+success under the PSP's own reference (same group,
//   so buckets move exactly once);
// - Pending → request only (Django's async: lone request = pending bucket).
//   The PSP later calls back and the terminal event settles it;
// - ActionRequired → request + `*_action_required` (redirect in
//   `external_url`, Django's 3DS record). The customer challenge resolves
//   through the same callback path;
// - Failed → a math-excluded failure record, then an error. Retrying with
//   the same key replays state instead of duplicating the failure.

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

/// Post-action `available_actions`, mirroring the manual gateway's sets.
/// None = leave untouched (refund keeps prior actions; pending/challenge
/// states don't advertise new moves).
fn post_actions(action: PspAction) -> Option<&'static [&'static str]> {
    match action {
        PspAction::Authorize => Some(&["charge", "cancel"]),
        PspAction::Charge => Some(&["refund"]),
        PspAction::Refund => None,
        PspAction::Cancel => Some(&[]),
    }
}

/// What a PSP-driven action returned, beyond the transaction state.
#[derive(Debug)]
pub struct PspOutcomeView {
    pub txn: TxnView,
    pub action_required: bool,
    pub redirect_url: Option<String>,
}

/// The engine: validate → consult PSP → persist outcome as events.
pub async fn execute_via(
    db: &DatabaseConnection,
    transaction_id: i32,
    action: PspAction,
    amount: Decimal,
    idempotency_key: &str,
    psp: &dyn Psp,
    return_url: Option<&str>,
    psp_data: Option<&str>,
) -> Result<PspOutcomeView> {
    let verb = match action {
        PspAction::Authorize => "authorize",
        PspAction::Charge => "charge",
        PspAction::Refund => "refund",
        PspAction::Cancel => "cancel",
    };
    if amount <= Decimal::ZERO {
        return Err(gateway_err(format!("{verb} amount must be positive")));
    }
    // Pre-guards on SETTLED buckets (Django's clean_* equivalents). These
    // read outside the write lock (best-effort under races, like Django's
    // validation-then-act); the single-success + dedup guards inside
    // report_event are the hard guarantees.
    let cur = view(db, transaction_id).await?;
    match action {
        PspAction::Authorize => {}
        PspAction::Charge => {
            if amount > cur.authorized {
                return Err(gateway_err(format!(
                    "cannot charge {amount}: only {} authorized",
                    cur.authorized
                )));
            }
        }
        PspAction::Refund => {
            // Net-bucket semantics: every refund_success already subtracted
            // `charged`, so `charged_value` alone IS the unrefunded
            // remainder (NOT charged-minus-refunded — that double-counts
            // past refunds and wrongly blocks sequential partial refunds).
            if amount > cur.charged {
                return Err(gateway_err(format!(
                    "cannot refund {amount}: only {} charged and unrefunded",
                    cur.charged
                )));
            }
        }
        PspAction::Cancel => {
            if amount > cur.authorized {
                return Err(gateway_err(format!(
                    "cannot cancel {amount}: only {} authorized",
                    cur.authorized
                )));
            }
        }
    }
    let req = saleor_rustify_core::psp::PspRequest {
        action,
        amount,
        currency: cur.currency.clone(),
        idempotency_key: idempotency_key.to_string(),
        return_url: return_url.map(|s| s.to_string()),
        data: psp_data.map(|s| s.to_string()),
    };
    let mk = |t: &str, k: String, psp_ref: Option<String>| NewEvent {
        event_type: t.to_string(),
        amount,
        currency: cur.currency.clone(),
        psp_reference: psp_ref,
        message: format!("{} via {}", verb, psp.name()),
        idempotency_key: Some(k),
        include_in_calculations: true,
        related_granted_refund_id: None,
        external_url: None,
    };
    match psp.execute(&req) {
        PspOutcome::Completed { psp_reference } => {
            report_event(db, transaction_id, &mk(action.request_event(), format!("{idempotency_key}-req"), Some(psp_reference.clone()))).await?;
            report_event(db, transaction_id, &mk(action.success_event(), format!("{idempotency_key}-ok"), Some(psp_reference.clone()))).await?;
            if let Some(actions) = post_actions(action) {
                set_actions(db, transaction_id, actions, Some(psp_reference)).await?;
            }
            Ok(PspOutcomeView { txn: view(db, transaction_id).await?, action_required: false, redirect_url: None })
        }
        PspOutcome::Pending { psp_reference } => {
            report_event(db, transaction_id, &mk(action.request_event(), format!("{idempotency_key}-req"), Some(psp_reference))).await?;
            Ok(PspOutcomeView { txn: view(db, transaction_id).await?, action_required: false, redirect_url: None })
        }
        PspOutcome::ActionRequired { psp_reference, redirect_url, message } => {
            let Some(challenge) = action.action_required_event() else {
                return Err(gateway_err(format!("{verb} does not support customer challenges")));
            };
            report_event(db, transaction_id, &mk(action.request_event(), format!("{idempotency_key}-req"), Some(psp_reference.clone()))).await?;
            let mut chal = mk(challenge, format!("{idempotency_key}-3ds"), Some(psp_reference));
            chal.message = message;
            chal.external_url = Some(redirect_url.clone());
            report_event(db, transaction_id, &chal).await?;
            Ok(PspOutcomeView { txn: view(db, transaction_id).await?, action_required: true, redirect_url: Some(redirect_url) })
        }
        PspOutcome::Failed { error } => {
            // Permanent record that moves nothing (failure role in an
            // otherwise empty group), then the error. Same-key retry
            // replays state via the idempotency check.
            let mut fail = mk(action.failure_event(), format!("{idempotency_key}-fail"), None);
            fail.message = error.clone();
            let _ = report_event(db, transaction_id, &fail).await;
            Err(gateway_err(format!("{verb} failed at {}: {error}", psp.name())))
        }
    }
}

#[derive(Debug)]
pub struct CallbackOutcome {
    pub view: TxnView,
    pub replayed: bool,
}

/// Settle a pending request or customer challenge: the PSP's async answer.
/// Validates the answer against the action's legal terminal set (Django's
/// `get_correct_event_types_based_on_request_type` port) and refuses to
/// settle twice — the second terminal event for one psp_reference replays,
/// a conflicting one errors.
pub async fn psp_callback(
    db: &DatabaseConnection,
    transaction_id: i32,
    action: PspAction,
    psp_reference: &str,
    success: bool,
    message: &str,
    idempotency_key: &str,
) -> Result<CallbackOutcome> {
    let terminal = if success { action.success_event() } else { action.failure_event() };
    let other = if success { action.failure_event() } else { action.success_event() };
    let existing: Vec<String> = payment_transactionevent::Entity::find()
        .select_only()
        .column(payment_transactionevent::Column::Type)
        .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
        .filter(payment_transactionevent::Column::PspReference.eq(psp_reference.to_string()))
        .into_tuple::<String>()
        .all(db)
        .await?;
    if !existing.iter().any(|t| t == action.request_event() || t.ends_with("_action_required")) {
        return Err(gateway_err(format!(
            "no pending {} request with psp_reference {psp_reference} (UNKNOWN_REQUEST)",
            action.request_event()
        )));
    }
    if existing.iter().any(|t| t == terminal) {
        return Ok(CallbackOutcome { view: view(db, transaction_id).await?, replayed: true });
    }
    if existing.iter().any(|t| t == other) {
        return Err(gateway_err(format!(
            "transaction already settled {other} for {psp_reference} (TERMINAL)"
        )));
    }
    let cur = view(db, transaction_id).await?;
    let ev = NewEvent {
        event_type: terminal.to_string(),
        amount: Decimal::ZERO, // replaced below with the request's amount
        currency: cur.currency.clone(),
        psp_reference: Some(psp_reference.to_string()),
        message: message.to_string(),
        idempotency_key: Some(format!("{idempotency_key}-fin")),
        include_in_calculations: true,
        related_granted_refund_id: None,
        external_url: None,
    };
    // The terminal event carries the REQUEST's amount (callbacks name the
    // outcome, not the money — Django groups by psp_reference and the
    // success amount settles the bucket).
    let req_amount: Option<Decimal> = payment_transactionevent::Entity::find()
        .select_only()
        .column(payment_transactionevent::Column::AmountValue)
        .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
        .filter(payment_transactionevent::Column::PspReference.eq(psp_reference.to_string()))
        .filter(payment_transactionevent::Column::Type.eq(action.request_event().to_string()))
        .into_tuple::<Decimal>()
        .one(db)
        .await?;
    let mut ev = ev;
    ev.amount = req_amount.unwrap_or(Decimal::ZERO);
    report_event(db, transaction_id, &ev).await?;
    if success {
        if let Some(actions) = post_actions(action) {
            set_actions(db, transaction_id, actions, Some(psp_reference.to_string())).await?;
        }
    }
    Ok(CallbackOutcome { view: view(db, transaction_id).await?, replayed: false })
}

/// Django's escape hatch for the single-success rule: overwrite the
/// authorized bucket without a new success event.
pub async fn adjust_authorization(
    db: &DatabaseConnection,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<TxnView> {
    if amount < Decimal::ZERO {
        return Err(gateway_err("adjustment amount must be >= 0"));
    }
    let cur = view(db, transaction_id).await?;
    report_event(
        db,
        transaction_id,
        &NewEvent {
            event_type: "authorization_adjustment".into(),
            amount,
            currency: cur.currency,
            psp_reference: Some(Uuid::new_v4().to_string()),
            message: "authorization adjustment".into(),
            idempotency_key: Some(idempotency_key.to_string()),
            include_in_calculations: true,
            related_granted_refund_id: None,
            external_url: None,
        },
    )
    .await
}

/// Authorize funds on a transaction (manual gateway: sync request+success).
pub async fn authorize(
    db: &DatabaseConnection,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<TxnView> {
    use saleor_rustify_core::psp::ManualPsp;
    Ok(execute_via(db, transaction_id, PspAction::Authorize, amount, idempotency_key, &ManualPsp, None, None).await?.txn)
}

/// Charge previously authorized funds. Guarded: never above the
/// authorized-but-uncharged remainder.
pub async fn charge(
    db: &DatabaseConnection,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<TxnView> {
    use saleor_rustify_core::psp::ManualPsp;
    Ok(execute_via(db, transaction_id, PspAction::Charge, amount, idempotency_key, &ManualPsp, None, None).await?.txn)
}

/// Refund charged funds. Guarded: never above charged-minus-refunded.
pub async fn refund(
    db: &DatabaseConnection,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<TxnView> {
    use saleor_rustify_core::psp::ManualPsp;
    Ok(execute_via(db, transaction_id, PspAction::Refund, amount, idempotency_key, &ManualPsp, None, None).await?.txn)
}

/// Transactional core of a sync manual refund: same guards as [`refund`],
/// request+success under one psp_reference, no transaction management.
/// Compose inside bigger atomic flows (paid cancel, return+refund).
/// Optionally links both events back to a granted-refund decision.
pub async fn refund_in(
    txn: &impl sea_orm::ConnectionTrait,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
    grant_id: Option<i32>,
) -> Result<()> {
    if amount <= Decimal::ZERO {
        return Err(gateway_err("refund amount must be positive"));
    }
    let cur = view(txn, transaction_id).await?;
    // Net bucket: `charged_value` is already net of refunds (see execute_via).
    if amount > cur.charged {
        return Err(gateway_err(format!(
            "cannot refund {amount}: only {} charged and unrefunded",
            cur.charged
        )));
    }
    // One psp_reference for the pair: request+success must land in the SAME
    // recalculation group, otherwise charged is subtracted twice (once as
    // pending, once as settled).
    let psp = Uuid::new_v4().to_string();
    let message = match grant_id {
        Some(g) => format!("granted refund {g}"),
        None => "manual gateway refund".to_string(),
    };
    let mk = |t: String, k: String| NewEvent {
        event_type: t,
        amount,
        currency: cur.currency.clone(),
        psp_reference: Some(psp.clone()),
        message: message.clone(),
        idempotency_key: Some(k),
        include_in_calculations: true,
        related_granted_refund_id: grant_id,
        external_url: None,
    };
    report_event_in(
        txn,
        transaction_id,
        &mk("refund_request".into(), format!("{idempotency_key}-req")),
    )
    .await?;
    report_event_in(
        txn,
        transaction_id,
        &mk("refund_success".into(), format!("{idempotency_key}-ok")),
    )
    .await?;
    Ok(())
}

/// Refund on behalf of a granted-refund decision: same guards as
/// [`refund`], but the request+success pair is linked back to the grant so
/// status derivation (`granted_refunds::refresh_grant_status`) can find it.
pub async fn refund_for_grant(
    db: &DatabaseConnection,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
    grant_id: i32,
) -> Result<TxnView> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    refund_in(&txn, transaction_id, amount, idempotency_key, Some(grant_id)).await?;
    txn.commit().await?;
    let v = view(db, transaction_id).await?;
    refresh_order_statuses(db, v.order_id).await?;
    Ok(v)
}

/// Cancel authorized-but-uncharged funds.
pub async fn cancel(
    db: &DatabaseConnection,
    transaction_id: i32,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<TxnView> {
    use saleor_rustify_core::psp::ManualPsp;
    Ok(execute_via(db, transaction_id, PspAction::Cancel, amount, idempotency_key, &ManualPsp, None, None).await?.txn)
}

/// Django `transactionUpdate` money path (`create_manual_adjustment_events`
/// parity): increases land as SUCCESS delta events, authorization moves
/// absolutely via AUTHORIZATION_ADJUSTMENT (cutoff semantics in recalc).
/// Decreases are rejected — money moves down only through the refund/void
/// flows, exactly like Django's validation.
/// All amounts must match the item currency (INCORRECT_CURRENCY otherwise).
#[derive(Debug, Default)]
pub struct AmountTargets {
    pub authorized: Option<Decimal>,
    pub charged: Option<Decimal>,
    pub refunded: Option<Decimal>,
    pub canceled: Option<Decimal>,
}

pub async fn apply_amount_targets(
    db: &DatabaseConnection,
    transaction_id: i32,
    targets: &AmountTargets,
    currency: &str,
    idempotency_key: &str,
) -> Result<TxnView> {
    let cur = view(db, transaction_id).await?;
    if cur.currency != currency {
        return Err(gateway_err("incorrect currency"));
    }
    let tag = |s: &str| format!("{idempotency_key}-{s}");
    if let Some(t) = targets.authorized {
        if t != cur.authorized {
            if t < Decimal::ZERO {
                return Err(gateway_err("authorized amount must be >= 0"));
            }
            // First authorization is a SUCCESS; later moves are absolute
            // ADJUSTMENTs (Django switches on prior SUCCESS existence).
            let has_success: bool = payment_transactionevent::Entity::find()
                .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
                .filter(payment_transactionevent::Column::Type.eq("authorization_success".to_string()))
                .one(db)
                .await?
                .is_some();
            let (ty, amt) = if has_success {
                ("authorization_adjustment", t)
            } else {
                ("authorization_success", t)
            };
            report_event(
                db,
                transaction_id,
                &NewEvent {
                    event_type: ty.into(),
                    amount: amt,
                    currency: cur.currency.clone(),
                    psp_reference: None,
                    message: "manual adjustment".into(),
                    idempotency_key: Some(tag("auth")),
                    include_in_calculations: true,
                    related_granted_refund_id: None,
                    external_url: None,
                },
            )
            .await?;
        }
    }
    // Additive families: deltas as SUCCESS events; shrinkage rejected.
    for (family, target, current) in [
        ("charge", targets.charged, view(db, transaction_id).await?.charged),
        ("refund", targets.refunded, view(db, transaction_id).await?.refunded),
        ("cancel", targets.canceled, view(db, transaction_id).await?.canceled),
    ] {
        let Some(t) = target else { continue };
        if t < Decimal::ZERO {
            return Err(gateway_err(format!("{family} amount must be >= 0")));
        }
        if t < current {
            return Err(gateway_err(format!(
                "{family} cannot be reduced via update; use the refund/void flows"
            )));
        }
        if t > current {
            report_event(
                db,
                transaction_id,
                &NewEvent {
                    event_type: format!("{family}_success"),
                    amount: t - current,
                    currency: cur.currency.clone(),
                    psp_reference: None,
                    message: "manual adjustment".into(),
                    idempotency_key: Some(tag(family)),
                    include_in_calculations: true,
                    related_granted_refund_id: None,
                    external_url: None,
                },
            )
            .await?;
        }
    }
    view(db, transaction_id).await
}

/// Scalar patch for `transactionUpdate` (Django `construct_instance` subset
/// that is safe without app context): name/message/pspReference (globally
/// unique, like Django's UNIQUE check), available actions (deduped),
/// metadata merge.
#[derive(Debug, Default)]
pub struct ItemPatch {
    pub name: Option<String>,
    pub message: Option<String>,
    pub psp_reference: Option<String>,
    pub available_actions: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
    pub private_metadata: Option<serde_json::Value>,
}

pub async fn update_transaction_scalars(
    db: &impl sea_orm::ConnectionTrait,
    transaction_id: i32,
    patch: &ItemPatch,
) -> Result<()> {
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
    if let Some(r) = patch.psp_reference.as_ref() {
        let clash = payment_transactionitem::Entity::find()
            .filter(payment_transactionitem::Column::PspReference.eq(r.clone()))
            .filter(payment_transactionitem::Column::Id.ne(transaction_id))
            .one(db)
            .await?
            .is_some();
        if clash {
            return Err(gateway_err("transaction with provided pspReference already exists"));
        }
    }
    let Some(m) = payment_transactionitem::Entity::find_by_id(transaction_id).one(db).await?
    else {
        return Err(DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(transaction_id.to_string())));
    };
    let cur_md = serde_json::to_value(&m.metadata).unwrap_or(serde_json::Value::Null);
    let cur_pmd = serde_json::to_value(&m.private_metadata).unwrap_or(serde_json::Value::Null);
    let mut am: payment_transactionitem::ActiveModel = m.into();
    if let Some(v) = patch.name.as_ref() {
        am.name = Set(Some(v.clone()));
    }
    if let Some(v) = patch.message.as_ref() {
        am.message = Set(Some(v.clone()));
    }
    if patch.psp_reference.is_some() {
        am.psp_reference = Set(patch.psp_reference.clone());
    }
    if let Some(v) = patch.available_actions.as_ref() {
        let mut ded: Vec<String> = v.clone();
        ded.sort();
        ded.dedup();
        am.available_actions = Set(ded);
    }
    if let Some(u) = patch.metadata.as_ref() {
        am.metadata = Set(merge_json(&cur_md, u));
    }
    if let Some(u) = patch.private_metadata.as_ref() {
        am.private_metadata = Set(merge_json(&cur_pmd, u));
    }
    am.update(db).await?;
    Ok(())
}

fn merge_json(cur: &serde_json::Value, upd: &serde_json::Value) -> serde_json::Value {
    let mut base = cur.as_object().cloned().unwrap_or_default();
    if let Some(u) = upd.as_object() {
        for (k, v) in u {
            base.insert(k.clone(), v.clone());
        }
    }
    serde_json::Value::Object(base)
}

/// Staff/app requested action on a transaction (Django
/// `transactionRequestAction`): CHARGE/REFUND/CANCEL through the given PSP.
/// Cancel ignores the amount; charge/refund move real money via execute_via
/// guards (never above remainder).
pub async fn request_action(
    db: &DatabaseConnection,
    transaction_id: i32,
    action: PspAction,
    amount: Decimal,
    idempotency_key: &str,
    psp: &dyn Psp,
    message: Option<String>,
) -> Result<TxnView> {
    // Cancel voids the whole authorization: a zero amount means "all of it".
    let amount = if matches!(action, PspAction::Cancel) && amount <= Decimal::ZERO {
        view(db, transaction_id).await?.authorized
    } else {
        amount
    };
    execute_via(db, transaction_id, action, amount, idempotency_key, psp, None, None).await?;
    if let Some(msg) = message.filter(|m| !m.trim().is_empty()) {
        // Annotate the request event (Django surfaces refundReason this way).
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
        if let Some(ev) = payment_transactionevent::Entity::find()
            .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
            .filter(payment_transactionevent::Column::IdempotencyKey.eq(idempotency_key.to_string()))
            .order_by_desc(payment_transactionevent::Column::Id)
            .one(db)
            .await?
        {
            let mut am: payment_transactionevent::ActiveModel = ev.into();
            am.message = Set(Some(msg));
            use sea_orm::ActiveModelTrait;
            am.update(db).await?;
        }
    }
    Ok(view(db, transaction_id).await?)
}

// ---------------------------------------------------------------------------
// Legacy Payment model actions (paymentCapture/Refund/Void parity)
// ---------------------------------------------------------------------------

/// Slim legacy-payment view for mutation payloads.
pub struct LegacyView {
    pub id: i32,
    pub gateway: String,
    pub is_active: bool,
    pub currency: String,
    pub total: Decimal,
    pub captured: Decimal,
    pub refunded: Decimal,
    pub charge_status: String,
}

fn legacy_view(m: &payment_payment::Model, refunded: Decimal) -> LegacyView {
    LegacyView {
        id: m.id,
        gateway: m.gateway.clone(),
        is_active: m.is_active,
        currency: m.currency.clone(),
        total: m.total,
        captured: m.captured_amount,
        refunded,
        charge_status: m.charge_status.clone(),
    }
}

/// Cumulative refunds live in `extra_data` (Django keeps them gateway-side;
/// we keep an honest local counter so repeated partial refunds can't exceed
/// what was captured).
fn refunded_so_far(m: &payment_payment::Model) -> Decimal {
    serde_json::from_str::<serde_json::Value>(&m.extra_data)
        .ok()
        .and_then(|v| v.get("refunded").cloned())
        .and_then(|v| v.as_str().unwrap_or("0").parse::<Decimal>().ok())
        .unwrap_or(Decimal::ZERO)
}

fn legacy_status(total: Decimal, captured: Decimal, refunded: Decimal, active: bool) -> String {
    if !active && captured == Decimal::ZERO {
        return "cancelled".to_string();
    }
    if refunded >= captured && captured > Decimal::ZERO {
        return "fully-refunded".to_string();
    }
    if refunded > Decimal::ZERO {
        return "partially-refunded".to_string();
    }
    if captured >= total && total > Decimal::ZERO {
        return "fully-charged".to_string();
    }
    if captured > Decimal::ZERO {
        return "partially-charged".to_string();
    }
    "not-charged".to_string()
}

/// Capture up to the uncaptured remainder (Django `paymentCapture`).
pub async fn capture_legacy_payment(
    db: &DatabaseConnection,
    payment_id: i32,
    amount: Decimal,
) -> Result<LegacyView> {
    use sea_orm::{ActiveModelTrait, EntityTrait, TransactionTrait};
    if amount <= Decimal::ZERO {
        return Err(gateway_err("capture amount must be > 0"));
    }
    let txn = db.begin().await?;
    let Some(m) = payment_payment::Entity::find_by_id(payment_id)
        .lock(sea_orm::sea_query::LockType::Update)
        .one(&txn)
        .await?
    else {
        txn.rollback().await?;
        return Err(gateway_err("payment not found"));
    };
    if !m.is_active {
        txn.rollback().await?;
        return Err(gateway_err("payment is not active"));
    }
    if m.captured_amount + amount > m.total {
        txn.rollback().await?;
        return Err(gateway_err("cannot capture more than the uncaptured remainder"));
    }
    let refunded = refunded_so_far(&m);
    let mut am: payment_payment::ActiveModel = m.into();
    am.captured_amount = Set(am.captured_amount.clone().unwrap() + amount);
    let captured = am.captured_amount.clone().unwrap();
    let total = am.total.clone().unwrap();
    am.charge_status = Set(legacy_status(total, captured, refunded, true));
    am.modified_at = Set(chrono::Utc::now().into());
    let m2 = am.update(&txn).await?;
    txn.commit().await?;
    Ok(legacy_view(&m2, refunded))
}

/// Refund up to captured-minus-refunded (Django `paymentRefund`).
pub async fn refund_legacy_payment(
    db: &DatabaseConnection,
    payment_id: i32,
    amount: Decimal,
) -> Result<LegacyView> {
    use sea_orm::{ActiveModelTrait, EntityTrait, TransactionTrait};
    if amount <= Decimal::ZERO {
        return Err(gateway_err("refund amount must be > 0"));
    }
    let txn = db.begin().await?;
    let Some(m) = payment_payment::Entity::find_by_id(payment_id)
        .lock(sea_orm::sea_query::LockType::Update)
        .one(&txn)
        .await?
    else {
        txn.rollback().await?;
        return Err(gateway_err("payment not found"));
    };
    let refunded = refunded_so_far(&m);
    if refunded + amount > m.captured_amount {
        txn.rollback().await?;
        return Err(gateway_err("cannot refund more than captured-minus-refunded"));
    }
    let refunded = refunded + amount;
    let mut extra: serde_json::Value =
        serde_json::from_str(&m.extra_data).unwrap_or(serde_json::Value::Null);
    if !extra.is_object() {
        extra = serde_json::json!({});
    }
    extra["refunded"] = serde_json::Value::String(refunded.to_string());
    let mut am: payment_payment::ActiveModel = m.into();
    am.extra_data = Set(extra.to_string());
    am.charge_status = Set(legacy_status(am.total.clone().unwrap(), am.captured_amount.clone().unwrap(), refunded, am.is_active.clone().unwrap()));
    am.modified_at = Set(chrono::Utc::now().into());
    let m2 = am.update(&txn).await?;
    txn.commit().await?;
    Ok(legacy_view(&m2, refunded))
}

/// Void a pre-auth (Django `paymentVoid`: only before anything captured).
pub async fn void_legacy_payment(db: &DatabaseConnection, payment_id: i32) -> Result<LegacyView> {
    use sea_orm::{ActiveModelTrait, EntityTrait, TransactionTrait};
    let txn = db.begin().await?;
    let Some(m) = payment_payment::Entity::find_by_id(payment_id)
        .lock(sea_orm::sea_query::LockType::Update)
        .one(&txn)
        .await?
    else {
        txn.rollback().await?;
        return Err(gateway_err("payment not found"));
    };
    if m.captured_amount > Decimal::ZERO {
        txn.rollback().await?;
        return Err(gateway_err("cannot void a captured payment; refund instead"));
    }
    if !m.is_active {
        txn.rollback().await?;
        return Err(gateway_err("payment is not active"));
    }
    let refunded = refunded_so_far(&m);
    let mut am: payment_payment::ActiveModel = m.into();
    am.is_active = Set(false);
    am.charge_status = Set("cancelled".to_string());
    am.modified_at = Set(chrono::Utc::now().into());
    let m2 = am.update(&txn).await?;
    txn.commit().await?;
    Ok(legacy_view(&m2, refunded))
}

/// PSP-level idempotency probe: has this (transaction, psp_reference, type)
/// event been recorded? Backs `transactionEventReport.alreadyProcessed`.
pub async fn has_event(
    db: &impl sea_orm::ConnectionTrait,
    transaction_id: i32,
    psp_reference: &str,
    event_type: &str,
) -> Result<bool> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    Ok(payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(transaction_id))
        .filter(payment_transactionevent::Column::PspReference.eq(psp_reference.to_string()))
        .filter(payment_transactionevent::Column::Type.eq(event_type.to_string()))
        .one(db)
        .await?
        .is_some())
}
