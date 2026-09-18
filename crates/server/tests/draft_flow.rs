//! DB-mode draft-order flow over gRPC handlers: create → add →
//! complete → guarded edits, against real Postgres.

use rustygod_db::{catalog, database_url};
use rustygod_proto::draft::{
    draft_order_service_server::DraftOrderService, CreateDraftOrderRequest, DraftLineInput,
    DraftOrderIdRequest, DraftOrderLinesRequest,
};
use rustygod_server::service_draft::DraftOrderServiceImpl;
use tonic::Request;

async fn svc() -> DraftOrderServiceImpl {
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    DraftOrderServiceImpl::new(Some(db))
}

async fn stocked_variant() -> String {
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let products = catalog::list_products(&db, "default-channel", None, 100)
        .await
        .unwrap();
    products
        .iter()
        .flat_map(|p| &p.variants)
        .find(|v| v.quantity_available >= 4)
        .expect("need a stocked variant")
        .id
        .clone()
}


fn test_key() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../db/tests/testdata/test_rsa.pem"
    ))
    .expect("test RSA key must exist")
}

/// Staff request: populatedb admin JWT in metadata.
async fn staff_req<T>(msg: T) -> Request<T> {
    std::env::set_var("RSA_PRIVATE_KEY", test_key());
    let db = rustygod_db::connect(&rustygod_db::database_url()).await.unwrap();
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

#[tokio::test]
async fn grpc_draft_lifecycle() {
    use fs2::FileExt;
    let _lock = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open("/tmp/rustygod-stock.lock")
        .unwrap();
    _lock.lock_exclusive().unwrap();

    let svc = svc().await;
    let vid = stocked_variant().await;

    let order = svc
        .create_draft_order(staff_req(CreateDraftOrderRequest {
            channel: "default-channel".into(),
            email: "staff@example.com".into(),
            lines: vec![DraftLineInput {
                variant_id: vid.clone(),
                quantity: 2,
                custom_price: String::new(),
                force_new_line: false,
            }],
        }).await)
        .await
        .unwrap()
        .into_inner()
        .order
        .unwrap();
    assert_eq!(order.status, "draft");
    assert_eq!(order.lines_count, 1);

    // Add same variant → merged.
    let order = svc
        .add_draft_lines(staff_req(DraftOrderLinesRequest {
            order_id: order.id.clone(),
            channel: "default-channel".into(),
            lines: vec![DraftLineInput {
                variant_id: vid.clone(),
                quantity: 1,
                custom_price: String::new(),
                force_new_line: false,
            }],
        }).await)
        .await
        .unwrap()
        .into_inner()
        .order
        .unwrap();
    assert_eq!(order.lines_count, 1);

    // Complete → unfulfilled/unconfirmed, never draft.
    let done = svc
        .complete_draft_order(staff_req(DraftOrderIdRequest { order_id: order.id.clone() }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(done.errors.is_empty());
    assert!(done.status == "unfulfilled" || done.status == "unconfirmed");

    // Completed draft rejects edits with NOT_APPLICABLE.
    let denied = svc
        .add_draft_lines(staff_req(DraftOrderLinesRequest {
            order_id: order.id.clone(),
            channel: "default-channel".into(),
            lines: vec![DraftLineInput {
                variant_id: vid,
                quantity: 1,
                custom_price: String::new(),
                force_new_line: false,
            }],
        }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(!denied.errors.is_empty());
    assert_eq!(denied.errors[0].code, "NOT_APPLICABLE");

    // Completed draft cannot be deleted either.
    let denied = svc
        .delete_draft_order(staff_req(DraftOrderIdRequest { order_id: order.id.clone() }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(!denied.errors.is_empty());

    // Cleanup the completed order directly (test-only).
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let oid: uuid::Uuid = order.id.parse().unwrap();
    use rustygod_db::entities::{
        order_order, order_orderevent, order_orderline, warehouse_allocation,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let lines = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(oid))
        .all(&db)
        .await
        .unwrap();
    for l in &lines {
        warehouse_allocation::Entity::delete_many()
            .filter(warehouse_allocation::Column::OrderLineId.eq(l.id))
            .exec(&db)
            .await
            .unwrap();
        order_orderline::Entity::delete_by_id(l.id).exec(&db).await.unwrap();
    }
    order_orderevent::Entity::delete_many()
        .filter(order_orderevent::Column::OrderId.eq(oid))
        .exec(&db)
        .await
        .unwrap();
    order_order::Entity::delete_by_id(oid).exec(&db).await.unwrap();
}
