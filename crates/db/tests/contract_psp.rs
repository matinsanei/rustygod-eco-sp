//! PSP contract: Saleor's payment hardness, ported.
//! Single AUTHORIZATION_SUCCESS per transaction (use ADJUSTMENT), PSP-level
//! dedup with amount-mismatch failure records, async pending + callbacks,
//! 3DS challenge loop, and the R3 per-checkout in-flight guard.
//! Covers audit R3, E9.

use rust_decimal::Decimal;
use rustygod_core::psp::{ChallengePsp, PspAction, PspOutcome, ScriptedPsp};
use rustygod_db::{catalog, checkout_store, database_url, payments};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

async fn bare_txn(db: &DatabaseConnection, tag: &str) -> payments::TxnView {
    payments::create_transaction(
        db,
        &payments::NewTransaction {
            checkout_id: None,
            order_id: None,
            currency: "USD".into(),
            name: "manual".into(),
            app_identifier: Some("rustygod-manual".into()),
            idempotency_key: Some(format!("psp-{tag}-{}", Uuid::new_v4())),
            available_actions: vec!["authorize".into()],
        },
    )
    .await
    .unwrap()
}

async fn event_count(db: &DatabaseConnection, txn_id: i32) -> usize {
    use rustygod_db::entities::payment_transactionevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(txn_id))
        .all(db)
        .await
        .unwrap()
        .len()
}

async fn failures(db: &DatabaseConnection, txn_id: i32) -> Vec<String> {
    use rustygod_db::entities::payment_transactionevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(txn_id))
        .filter(payment_transactionevent::Column::Type.ends_with("_failure"))
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.r#type)
        .collect()
}

/// include_in_calculations flag of the first event of a type (None if absent).
async fn include_flag(db: &DatabaseConnection, txn_id: i32, t: &str) -> Option<bool> {
    use rustygod_db::entities::payment_transactionevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(txn_id))
        .filter(payment_transactionevent::Column::Type.eq(t))
        .one(db)
        .await
        .unwrap()
        .map(|e| e.include_in_calculations)
}

async fn cleanup(db: &DatabaseConnection, txn_id: i32) {
    use rustygod_db::entities::{payment_transactionevent, payment_transactionitem};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    payment_transactionevent::Entity::delete_many()
        .filter(payment_transactionevent::Column::TransactionId.eq(txn_id))
        .exec(db)
        .await
        .unwrap();
    payment_transactionitem::Entity::delete_by_id(txn_id)
        .exec(db)
        .await
        .unwrap();
}

fn ev(t: &str, amount: Decimal, psp: Option<&str>, key: &str) -> payments::NewEvent {
    payments::NewEvent {
        event_type: t.into(),
        amount,
        currency: "USD".into(),
        psp_reference: psp.map(|s| s.into()),
        message: "test".into(),
        idempotency_key: Some(key.into()),
        include_in_calculations: true,
        related_granted_refund_id: None,
        external_url: None,
    }
}

#[tokio::test]
async fn second_auth_success_rejected_use_adjustment() {
    let db = db().await;
    let t = bare_txn(&db, "dup-auth").await;
    payments::authorize(&db, t.id, dec("100.00"), "dup-auth-1").await.unwrap();
    // Same money, new key, new psp (double-Pay at the PSP): Django says no.
    let err = payments::authorize(&db, t.id, dec("100.00"), "dup-auth-2")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("AUTHORIZATION_SUCCESS"), "{err}");
    assert!(err.to_string().contains("ADJUSTMENT"), "{err}");
    let v = payments::view(&db, t.id).await.unwrap();
    assert_eq!(v.authorized, dec("100.00"), "rejected write moves nothing");
    assert_eq!(failures(&db, t.id).await, vec!["authorization_failure"]);
    cleanup(&db, t.id).await;
}

#[tokio::test]
async fn psp_replay_same_triple_returns_state() {
    let db = db().await;
    let t = bare_txn(&db, "replay-triple").await;
    payments::report_event(&db, t.id, &ev("authorization_request", dec("50.00"), Some("psp-1"), "rt-1"))
        .await
        .unwrap();
    let n = event_count(&db, t.id).await;
    // Same (txn, psp, type, amount), fresh key: already-processed, no row.
    payments::report_event(&db, t.id, &ev("authorization_request", dec("50.00"), Some("psp-1"), "rt-2"))
        .await
        .unwrap();
    assert_eq!(event_count(&db, t.id).await, n, "replay must not append");
    cleanup(&db, t.id).await;
}

#[tokio::test]
async fn psp_amount_mismatch_records_failure() {
    let db = db().await;
    let t = bare_txn(&db, "mismatch").await;
    payments::report_event(&db, t.id, &ev("charge_request", dec("30.00"), Some("psp-9"), "mm-1"))
        .await
        .unwrap();
    let err = payments::report_event(&db, t.id, &ev("charge_request", dec("31.00"), Some("psp-9"), "mm-2"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("different amount"), "{err}");
    assert_eq!(failures(&db, t.id).await, vec!["charge_failure"]);
    // Rejected writes stay out of the math (Django include=false).
    assert_eq!(include_flag(&db, t.id, "charge_failure").await, Some(false));
    let v = payments::view(&db, t.id).await.unwrap();
    assert_eq!(v.charge_pending, dec("30.00"), "original pending stands");
    cleanup(&db, t.id).await;
}

#[tokio::test]
async fn async_authorize_then_callback_settles() {
    let db = db().await;
    let t = bare_txn(&db, "async").await;
    let sim = ScriptedPsp::pending("sim");
    let out = payments::execute_via(
        &db, t.id, PspAction::Authorize, dec("80.00"), "async-1", &sim, None, None,
    )
    .await
    .unwrap();
    assert!(!out.action_required);
    assert_eq!(out.txn.authorize_pending, dec("80.00"));
    assert_eq!(out.txn.authorized, dec("0"));
    // PSP answers late: find its reference from the request event.
    use rustygod_db::entities::payment_transactionevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let req = payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(t.id))
        .filter(payment_transactionevent::Column::Type.eq("authorization_request"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let psp = req.psp_reference.clone().unwrap();
    let cb = payments::psp_callback(&db, t.id, PspAction::Authorize, &psp, true, "psp ok", "async-1")
        .await
        .unwrap();
    assert!(!cb.replayed);
    assert_eq!(cb.view.authorized, dec("80.00"));
    assert_eq!(cb.view.authorize_pending, dec("0"));
    assert!(cb.view.available_actions.contains(&"charge".to_string()));
    cleanup(&db, t.id).await;
}

#[tokio::test]
async fn callback_replay_and_terminal_conflict() {
    let db = db().await;
    let t = bare_txn(&db, "cb-term").await;
    let sim = ScriptedPsp::pending("sim");
    payments::execute_via(&db, t.id, PspAction::Authorize, dec("10.00"), "cb-1", &sim, None, None)
        .await
        .unwrap();
    use rustygod_db::entities::payment_transactionevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let req = payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(t.id))
        .filter(payment_transactionevent::Column::Type.eq("authorization_request"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let psp = req.psp_reference.clone().unwrap();
    let cb = payments::psp_callback(&db, t.id, PspAction::Authorize, &psp, false, "declined", "cb-1")
        .await
        .unwrap();
    assert!(!cb.replayed);
    assert_eq!(cb.view.authorized, dec("0"));
    // Same answer again: replay, no new row.
    let n = event_count(&db, t.id).await;
    let cb2 = payments::psp_callback(&db, t.id, PspAction::Authorize, &psp, false, "declined", "cb-1")
        .await
        .unwrap();
    assert!(cb2.replayed);
    assert_eq!(event_count(&db, t.id).await, n);
    // Opposite answer after terminal: refused.
    let err = payments::psp_callback(&db, t.id, PspAction::Authorize, &psp, true, "late ok", "cb-1")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("TERMINAL"), "{err}");
    cleanup(&db, t.id).await;
}

#[tokio::test]
async fn challenge_flow_3ds_settles_once() {
    let db = db().await;
    let t = bare_txn(&db, "3ds").await;
    let psp = ChallengePsp::new("3ds.test");
    let out = payments::execute_via(
        &db, t.id, PspAction::Authorize, dec("120.00"), "3ds-1", &psp, Some("https://shop/return"), None,
    )
    .await
    .unwrap();
    assert!(out.action_required);
    let url = out.redirect_url.unwrap();
    assert!(url.starts_with("https://3ds.test/3ds/challenge?"), "{url}");
    // The challenge record carries the landing URL (Django external_url).
    use rustygod_db::entities::payment_transactionevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let chal = payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(t.id))
        .filter(payment_transactionevent::Column::Type.eq("authorization_action_required"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(chal.external_url.as_deref(), Some(url.as_str()));
    assert_eq!(out.txn.authorized, dec("0"), "challenge moves no money");
    let psp_ref = chal.psp_reference.clone().unwrap();
    // Customer completes the challenge at the PSP.
    let cb = payments::psp_callback(&db, t.id, PspAction::Authorize, &psp_ref, true, "3ds ok", "3ds-1")
        .await
        .unwrap();
    assert_eq!(cb.view.authorized, dec("120.00"));
    // A second completed challenge can't double-settle (single success).
    let out2 = payments::execute_via(
        &db, t.id, PspAction::Authorize, dec("120.00"), "3ds-2", &psp, None, None,
    )
    .await
    .unwrap();
    assert!(out2.action_required);
    let chal2 = payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(t.id))
        .filter(payment_transactionevent::Column::Type.eq("authorization_action_required"))
        .filter(payment_transactionevent::Column::IdempotencyKey.eq("3ds-2-3ds"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let err = payments::psp_callback(&db, t.id, PspAction::Authorize, chal2.psp_reference.as_deref().unwrap(), true, "3ds ok again", "3ds-2")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("AUTHORIZATION_SUCCESS"), "{err}");
    cleanup(&db, t.id).await;
}

#[tokio::test]
async fn adjustment_overwrites_authorized() {
    let db = db().await;
    let t = bare_txn(&db, "adj").await;
    payments::authorize(&db, t.id, dec("100.00"), "adj-1").await.unwrap();
    let v = payments::adjust_authorization(&db, t.id, dec("60.00"), "adj-2").await.unwrap();
    assert_eq!(v.authorized, dec("60.00"), "adjustment overwrites, not adds");
    cleanup(&db, t.id).await;
}

#[tokio::test]
async fn failed_psp_records_and_errors() {
    let db = db().await;
    let t = bare_txn(&db, "fail").await;
    let sim = ScriptedPsp::pending("sim");
    sim.push(PspOutcome::Failed { error: "insufficient funds".into() });
    // Failed on authorize (no pre-guard beyond positive amount).
    let err = payments::execute_via(&db, t.id, PspAction::Authorize, dec("10.00"), "fail-1", &sim, None, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("insufficient funds"), "{err}");
    assert_eq!(failures(&db, t.id).await, vec!["authorization_failure"]);
    // A real PSP answer stays visible to dedup (include=true) but is
    // bucket-neutral by its failure role.
    assert_eq!(include_flag(&db, t.id, "authorization_failure").await, Some(true));
    let v = payments::view(&db, t.id).await.unwrap();
    assert_eq!(v.authorized, dec("0"));
    cleanup(&db, t.id).await;
}

#[tokio::test]
async fn refund_challenge_unsupported() {
    let db = db().await;
    let t = bare_txn(&db, "no3ds-refund").await;
    payments::authorize(&db, t.id, dec("50.00"), "nr-auth").await.unwrap();
    payments::charge(&db, t.id, dec("50.00"), "nr-chg").await.unwrap();
    let psp = ChallengePsp::new("3ds.test");
    let err = payments::execute_via(&db, t.id, PspAction::Refund, dec("5.00"), "nr-1", &psp, None, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("does not support customer challenges"), "{err}");
    cleanup(&db, t.id).await;
}

#[tokio::test]
async fn r3_second_transaction_blocked_while_inflight() {
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token =
        checkout_store::create_checkout_row(&db, ch_id, &currency, "r3@example.com")
            .await
            .unwrap();
    let mk = |tag: String| payments::NewTransaction {
        checkout_id: Some(token),
        order_id: None,
        currency: currency.clone(),
        name: "manual".into(),
        app_identifier: Some("rustygod-manual".into()),
        idempotency_key: Some(tag),
        available_actions: vec!["authorize".into()],
    };
    let t1 = payments::create_transaction(&db, &mk(format!("r3-{}", Uuid::new_v4())))
        .await
        .unwrap();
    // Money in flight on t1 (async authorize, no callback yet).
    let sim = ScriptedPsp::pending("sim");
    payments::execute_via(&db, t1.id, PspAction::Authorize, dec("25.00"), "r3-auth", &sim, None, None)
        .await
        .unwrap();
    // Double-Pay button: second transaction for the same checkout fails fast.
    let err = payments::create_transaction(&db, &mk(format!("r3-{}", Uuid::new_v4())))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("ALREADY_IN_PROGRESS"), "{err}");
    // After the callback settles, a new transaction is fine (retry path).
    use rustygod_db::entities::payment_transactionevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let req = payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(t1.id))
        .filter(payment_transactionevent::Column::Type.eq("authorization_request"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    payments::psp_callback(&db, t1.id, PspAction::Authorize, req.psp_reference.as_deref().unwrap(), true, "ok", "r3-auth")
        .await
        .unwrap();
    let t3 = payments::create_transaction(&db, &mk(format!("r3-{}", Uuid::new_v4())))
        .await
        .unwrap();
    assert_ne!(t1.id, t3.id);
    cleanup(&db, t1.id).await;
    cleanup(&db, t3.id).await;
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn sequential_partial_refunds_each_succeed() {
    // Net-bucket regression: after refund 20 of 100, a second refund of 30
    // must succeed (remainder 80, then 50) — the old charged-minus-refunded
    // guard wrongly blocked it.
    let db = db().await;
    let t = bare_txn(&db, "seq-ref").await;
    payments::authorize(&db, t.id, dec("100.00"), "seq-a").await.unwrap();
    payments::charge(&db, t.id, dec("100.00"), "seq-c").await.unwrap();
    payments::refund(&db, t.id, dec("20.00"), "seq-r1").await.unwrap();
    let v = payments::refund(&db, t.id, dec("30.00"), "seq-r2").await.unwrap();
    assert_eq!(v.refunded, dec("50.00"));
    assert_eq!(v.charged, dec("50.00"));
    // And the remainder is still refundable.
    let v = payments::refund(&db, t.id, dec("50.00"), "seq-r3").await.unwrap();
    assert_eq!(v.charged, dec("0"));
    assert_eq!(v.refunded, dec("100.00"));
    cleanup(&db, t.id).await;
}
