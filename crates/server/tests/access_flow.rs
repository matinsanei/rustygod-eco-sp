//! Access-control contract: staff-only RPCs demand a permission codename
//! via staff JWT or app token; everyone else gets gRPC UNAUTHENTICATED /
//! PERMISSION_DENIED. Mirrors Saleor's permission decorators on the
//! `MANAGE_*` mutations.

use rustygod_core::auth as core_auth;
use rustygod_db::{apps, auth as db_auth, database_url};
use rustygod_proto::draft::{
    draft_order_service_server::DraftOrderService, CreateDraftOrderRequest, DraftLineInput,
};
use rustygod_server::{
    access::{self, MANAGE_GIFT_CARD, MANAGE_ORDERS},
    service_draft::DraftOrderServiceImpl,
};
use sea_orm::DatabaseConnection;
use tonic::{
    metadata::{MetadataMap, MetadataValue},
    Code, Request,
};
use std::str::FromStr;

fn test_key() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../db/tests/testdata/test_rsa.pem"
    ))
    .expect("test RSA key must exist")
}

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn admin_access() -> (String, i32) {
    let db = db().await;
    let (user, _) = db_auth::find_for_login(&db, "admin@example.com")
        .await
        .unwrap()
        .expect("populatedb admin must exist");
    let pair = core_auth::mint_tokens_with_key(
        &test_key(), "test", &user.email, user.id, user.is_staff, &user.jwt_token_key,
    )
    .unwrap();
    (pair.access, user.id)
}

fn authed_req<T>(msg: T, token: &str) -> Request<T> {
    let mut req = Request::new(msg);
    req.metadata_mut().insert(
        "authorization",
        MetadataValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    req
}

fn authed_meta(token: &str) -> MetadataMap {
    let mut m = MetadataMap::new();
    m.insert(
        "authorization",
        MetadataValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    m
}

#[tokio::test]
async fn missing_or_garbage_credentials_rejected() {
    let db = db().await;
    let meta = tonic::metadata::MetadataMap::new();
    let err = access::authorize_with_key(&db, &meta, MANAGE_ORDERS, &test_key())
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Unauthenticated);

    let mut bad_meta = tonic::metadata::MetadataMap::new();
    bad_meta.insert("authorization", MetadataValue::from_static("Bearer nope"));
    let err = access::authorize_with_key(&db, &bad_meta, MANAGE_ORDERS, &test_key())
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Unauthenticated);
}

#[tokio::test]
async fn refresh_tokens_cannot_authorize() {
    let db = db().await;
    let (user, _) = db_auth::find_for_login(&db, "admin@example.com").await.unwrap().unwrap();
    let pair = core_auth::mint_tokens_with_key(
        &test_key(), "test", &user.email, user.id, user.is_staff, &user.jwt_token_key,
    )
    .unwrap();
    let meta = authed_meta(&pair.refresh);
    let err = access::authorize_with_key(&db, &meta, MANAGE_ORDERS, &test_key())
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Unauthenticated);
}

#[tokio::test]
async fn staff_jwt_authorizes_and_tampering_fails() {
    let db = db().await;
    let (token, _) = admin_access().await;
    let meta = authed_meta(&token);
    let who = access::authorize_with_key(&db, &meta, MANAGE_ORDERS, &test_key())
        .await
        .unwrap();
    assert!(matches!(who, access::Requestor::Staff { .. }), "{who:?}");

    let mut bad = token.clone();
    bad.push('x');
    let meta = authed_meta(&bad);
    let err = access::authorize_with_key(&db, &meta, MANAGE_ORDERS, &test_key())
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Unauthenticated);
}

#[tokio::test]
async fn app_token_follows_its_grants() {
    let db = db().await;
    let app_id = apps::create_app(&db, "ci-access-app", &["manage_orders"]).await.unwrap();
    let (_, raw) = apps::create_app_token(&db, app_id, "t").await.unwrap();

    let meta = authed_meta(&raw);
    let who = access::authorize_with_key(&db, &meta, MANAGE_ORDERS, &test_key())
        .await
        .unwrap();
    assert!(matches!(who, access::Requestor::App { .. }), "{who:?}");

    // Granted manage_orders but NOT manage_gift_card.
    let err = access::authorize_with_key(&db, &meta, MANAGE_GIFT_CARD, &test_key())
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::PermissionDenied);

    apps::delete_app(&db, app_id).await.unwrap();
}

#[tokio::test]
async fn draft_rpc_enforces_manage_orders() {
    std::env::set_var("RSA_PRIVATE_KEY", test_key());
    let db = db().await;
    let svc = DraftOrderServiceImpl::new(Some(db));

    // Anonymous → gRPC error, no order created.
    let err = svc
        .create_draft_order(Request::new(CreateDraftOrderRequest {
            channel: "default-channel".into(),
            email: "anon@example.com".into(),
            lines: vec![],
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Unauthenticated);

    // Staff JWT → passes auth, fails on business validation (empty lines),
    // proving the gate let it through.
    let (token, _) = admin_access().await;
    let err = svc
        .create_draft_order(authed_req(
            CreateDraftOrderRequest {
                channel: "default-channel".into(),
                email: "staff@example.com".into(),
                lines: vec![],
            },
            &token,
        ))
        .await
        .unwrap()
        .into_inner();
    assert!(!err.errors.is_empty(), "auth passed; empty-lines must error");
    assert!(err.errors[0].message.contains("at least one line"));

    // App token without the grant → denied.
    let db2 = rustygod_db::connect(&database_url()).await.unwrap();
    let app_id = apps::create_app(&db2, "ci-draft-app", &[]).await.unwrap();
    let (_, raw) = apps::create_app_token(&db2, app_id, "t").await.unwrap();
    let err = svc
        .create_draft_order(authed_req(
            CreateDraftOrderRequest {
                channel: "default-channel".into(),
                email: "app@example.com".into(),
                lines: vec![DraftLineInput {
                    variant_id: "1".into(),
                    quantity: 1,
                    custom_price: String::new(),
                    force_new_line: false,
                }],
            },
            &raw,
        ))
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::PermissionDenied);
    apps::delete_app(&db2, app_id).await.unwrap();
}
