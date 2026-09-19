//! PaymentService flow over gRPC handlers: PSP selection, async callbacks,
//! 3DS challenges, and adjustments — all against real Postgres.

use rustygod_db::database_url;
use rustygod_proto::payment::{
    payment_service_server::PaymentService, AdjustAuthorizationRequest, CreateTransactionRequest,
    GatewayActionRequest, GetTransactionRequest, PspCallbackRequest,
};
use rustygod_server::service_payment::PaymentServiceImpl;
use tonic::Request;

fn test_key() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../db/tests/testdata/test_rsa.pem"
    ))
    .expect("test RSA key must exist")
}

async fn staff_req<T>(msg: T) -> Request<T> {
    std::env::set_var("RSA_PRIVATE_KEY", test_key());
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let (user, _) = rustygod_db::auth::find_for_login(&db, "admin@example.com")
        .await
        .unwrap()
        .unwrap();
    let pair = rustygod_core::auth::mint_tokens_with_key(
        &test_key(),
        "test",
        &user.email,
        user.id,
        user.is_staff,
        &user.jwt_token_key,
    )
    .unwrap();
    let mut req = Request::new(msg);
    req.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", pair.access).parse().unwrap(),
    );
    req
}

async fn svc() -> PaymentServiceImpl {
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    PaymentServiceImpl::new(Some(db))
}

async fn new_txn(svc: &PaymentServiceImpl, tag: &str) -> i32 {
    let resp = svc
        .create_transaction(
            staff_req(CreateTransactionRequest {
                checkout_id: String::new(),
                order_id: String::new(),
                currency: "USD".into(),
                name: "manual".into(),
                app_identifier: "rustygod-manual".into(),
                idempotency_key: format!("payflow-{tag}-{}", uuid::Uuid::new_v4()),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    resp.transaction.unwrap().id.parse().unwrap()
}

async fn request_psp(db: &sea_orm::DatabaseConnection, txn_id: i32, t: &str) -> String {
    use rustygod_db::entities::payment_transactionevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(txn_id))
        .filter(payment_transactionevent::Column::Type.eq(t))
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .psp_reference
        .unwrap()
}

async fn cleanup(db: &sea_orm::DatabaseConnection, txn_id: i32) {
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

fn action_req(txn_id: i32, amount: &str, key: &str, gateway: &str) -> GatewayActionRequest {
    GatewayActionRequest {
        transaction_id: txn_id.to_string(),
        amount: amount.into(),
        idempotency_key: key.into(),
        gateway: gateway.into(),
        return_url: String::new(),
    }
}

#[tokio::test]
async fn manual_authorize_then_charge_over_rpc() {
    let svc = svc().await;
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let id = new_txn(&svc, "manual").await;
    let r = svc
        .authorize(staff_req(action_req(id, "100.000", "m-a", "manual")).await)
        .await
        .unwrap()
        .into_inner();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(!r.action_required);
    assert_eq!(r.transaction.unwrap().authorized_amount, "100.000");
    let r = svc
        .charge(staff_req(action_req(id, "40.000", "m-c", "")).await)
        .await
        .unwrap()
        .into_inner();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.transaction.unwrap().charged_amount, "40.000");
    // Unknown gateway is rejected, not panicked.
    let r = svc
        .authorize(staff_req(action_req(id, "1.00", "m-x", "stripe")).await)
        .await
        .unwrap()
        .into_inner();
    assert!(!r.errors.is_empty());
    cleanup(&db, id).await;
}

#[tokio::test]
async fn async_sim_pending_then_callback_over_rpc() {
    let svc = svc().await;
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let id = new_txn(&svc, "async").await;
    let r = svc
        .authorize(staff_req(action_req(id, "75.000", "a-1", "async-sim")).await)
        .await
        .unwrap()
        .into_inner();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let t = r.transaction.unwrap();
    assert_eq!(t.authorized_amount, "0");
    let psp = request_psp(&db, id, "authorization_request").await;
    let cb = svc
        .psp_callback(
            staff_req(PspCallbackRequest {
                transaction_id: id.to_string(),
                action: "authorize".into(),
                psp_reference: psp,
                success: true,
                message: "psp settled".into(),
                idempotency_key: "a-1".into(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(cb.errors.is_empty(), "{:?}", cb.errors);
    assert!(!cb.replayed);
    assert_eq!(cb.transaction.unwrap().authorized_amount, "75.000");
    cleanup(&db, id).await;
}

#[tokio::test]
async fn challenge_then_adjust_over_rpc() {
    let svc = svc().await;
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let id = new_txn(&svc, "3ds").await;
    let r = svc
        .authorize(staff_req(action_req(id, "120.000", "c-1", "challenge")).await)
        .await
        .unwrap()
        .into_inner();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(r.action_required);
    assert!(r.redirect_url.starts_with("https://"), "{}", r.redirect_url);
    let psp = request_psp(&db, id, "authorization_request").await;
    let cb = svc
        .psp_callback(
            staff_req(PspCallbackRequest {
                transaction_id: id.to_string(),
                action: "authorize".into(),
                psp_reference: psp,
                success: true,
                message: "3ds passed".into(),
                idempotency_key: "c-1".into(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(cb.errors.is_empty(), "{:?}", cb.errors);
    assert_eq!(cb.transaction.unwrap().authorized_amount, "120.000");
    // Adjustment overwrites (Django escape hatch).
    let adj = svc
        .adjust_authorization(
            staff_req(AdjustAuthorizationRequest {
                transaction_id: id.to_string(),
                amount: "90.000".into(),
                idempotency_key: "c-adj".into(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(adj.errors.is_empty(), "{:?}", adj.errors);
    assert_eq!(adj.transaction.unwrap().authorized_amount, "90.000");
    // Read path works too.
    let g = svc
        .get_transaction(
            staff_req(GetTransactionRequest { id: id.to_string() }).await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(g.errors.is_empty());
    cleanup(&db, id).await;
}
