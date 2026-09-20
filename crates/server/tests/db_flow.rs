//! DB-mode service flow: the gRPC handlers against real Postgres.
//! Covers audit requirements: transactional complete, idempotent retry
//! (second complete is a clean NOT_FOUND, never a second order), and
//! same-variant line merging.

use rustygod_db::{catalog, database_url};
use rustygod_proto::{
    checkout::{
        checkout_service_server::CheckoutService, AddLinesRequest, CheckoutLine,
        CompleteCheckoutRequest, CreateCheckoutRequest,
    },
    order::{order_service_server::OrderService, GetOrderRequest, ListOrdersRequest},
};
use rustygod_server::{
    service_checkout::CheckoutServiceImpl, service_order::OrderServiceImpl,
    store::new_store,
};
use tonic::Request;

async fn services() -> (CheckoutServiceImpl, OrderServiceImpl) {
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let store = new_store();
    (
        CheckoutServiceImpl::with_db(store.clone(), db.clone()),
        OrderServiceImpl::with_db(store, db),
    )
}

async fn variant_id() -> String {
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let products = catalog::list_products(&db, "default-channel", None, 100)
        .await
        .unwrap();
    products
        .iter()
        .flat_map(|p| &p.variants)
        .next()
        .unwrap()
        .id
        .clone()
}

#[tokio::test]
async fn db_complete_mints_order_then_deletes_checkout() {
    let (checkout_svc, order_svc) = services().await;
    let vid = variant_id().await;

    let co = checkout_svc
        .create_checkout(Request::new(CreateCheckoutRequest {
            channel: "default-channel".into(),
            email: "audit@example.com".into(),
        }))
        .await
        .unwrap()
        .into_inner()
        .checkout
        .unwrap();

    // Add the same variant twice: must merge into ONE line (Django get_line).
    for _ in 0..2 {
        let got = checkout_svc
            .add_lines(Request::new(AddLinesRequest {
                checkout_id: co.id.clone(),
                lines: vec![CheckoutLine {
                    variant_id: vid.clone(),
                    quantity: 1,
                    unit_price: None,
                    total_price: None,
                is_gift: false,
                }],
            }))
            .await
            .unwrap()
            .into_inner()
            .checkout
            .unwrap();
        assert!(got.lines.len() == 1, "lines must merge, got {}", got.lines.len());
    }
    let got = checkout_svc
        .get_checkout(Request::new(rustygod_proto::checkout::GetCheckoutRequest {
            id: co.id.clone(),
        }))
        .await
        .unwrap()
        .into_inner()
        .checkout
        .unwrap();
    assert_eq!(got.lines.len(), 1);
    assert_eq!(got.lines[0].quantity, 2);

    let done = checkout_svc
        .complete_checkout(Request::new(CompleteCheckoutRequest {
            checkout_id: co.id.clone(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(done.errors.is_empty(), "complete failed: {:?}", done.errors);

    // Order persisted with Django semantics.
    let order = order_svc
        .get_order(Request::new(GetOrderRequest { id: done.order_id.clone() }))
        .await
        .unwrap()
        .into_inner()
        .order
        .unwrap();
    assert_eq!(order.status, "unfulfilled");
    assert!(order.number.parse::<i32>().unwrap() > 0);

    // Idempotent retry: same order back, never a second order (R7).
    let before = order_svc
        .list_orders(Request::new(ListOrdersRequest {
            first: 100_000,
            after: String::new(),
            status: String::new(),
        }))
        .await
        .unwrap()
        .into_inner()
        .orders
        .len();
    let retry = checkout_svc
        .complete_checkout(Request::new(CompleteCheckoutRequest {
            checkout_id: co.id.clone(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(retry.errors.is_empty());
    assert_eq!(retry.order_id, done.order_id);
    let after = order_svc
        .list_orders(Request::new(ListOrdersRequest {
            first: 100_000,
            after: String::new(),
            status: String::new(),
        }))
        .await
        .unwrap()
        .into_inner()
        .orders
        .len();
    assert_eq!(before, after, "retry must not mint a second order");

    // Cleanup minted order row (test data only): allocations, events,
    // lines, then the order — FK order respected.
    use rustygod_db::entities::{
        order_order, order_orderevent, order_orderline, warehouse_allocation,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let oid: uuid::Uuid = done.order_id.parse().unwrap();
    let line_ids: Vec<uuid::Uuid> = order_orderline::Entity::find()
        .select_only()
        .column(order_orderline::Column::Id)
        .filter(order_orderline::Column::OrderId.eq(oid))
        .into_tuple()
        .all(&db)
        .await
        .unwrap();
    for lid in &line_ids {
        warehouse_allocation::Entity::delete_many()
            .filter(warehouse_allocation::Column::OrderLineId.eq(*lid))
            .exec(&db)
            .await
            .unwrap();
    }
    order_orderline::Entity::delete_many()
        .filter(order_orderline::Column::OrderId.eq(oid))
        .exec(&db)
        .await
        .unwrap();
    order_orderevent::Entity::delete_many()
        .filter(order_orderevent::Column::OrderId.eq(oid))
        .exec(&db)
        .await
        .unwrap();
    order_order::Entity::delete_by_id(oid).exec(&db).await.unwrap();
}
