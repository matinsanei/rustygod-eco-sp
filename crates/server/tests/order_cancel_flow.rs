//! Order cancel/return end to end over gRPC: buy → charge → return one
//! unit → cancel the rest. Money, stock, and status stay consistent and
//! reconcile green at every step.

use rustygod_db::{catalog, checkout_store, complete, database_url, fulfillment, order_store, payments};
use rustygod_proto::order::{
    order_service_server::OrderService, CancelOrderRequest, ReturnLineInput, ReturnOrderLinesRequest,
};
use rustygod_server::service_order::OrderServiceImpl;
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

#[tokio::test]
async fn return_then_cancel_refused_then_full_return_over_rpc() {
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let store = rustygod_server::store::new_store();
    let svc = OrderServiceImpl::with_db(store, db.clone());

    // Buy 2 tracked units.
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "e2e@example.com")
        .await
        .unwrap();
    let products = catalog::list_products(&db, "default-channel", None, 100).await.unwrap();
    use rustygod_db::entities::product_productvariant;
    use sea_orm::EntityTrait;
    let mut vid = 0;
    for p in products.iter().flat_map(|p| &p.variants) {
        let id: i32 = p.id.parse().unwrap();
        if p.quantity_available < 2 {
            continue;
        }
        let v = product_productvariant::Entity::find_by_id(id).one(&db).await.unwrap().unwrap();
        if v.track_inventory {
            vid = id;
            break;
        }
    }
    assert_ne!(vid, 0);
    let pricing = catalog::checkout_pricing(&db, "default-channel", &[vid]).await.unwrap();
    checkout_store::add_line_row(&db, token, vid, 2, pricing[&vid].0.amount, &currency, None)
        .await
        .unwrap();
    let done = complete::complete_checkout(&db, token).await.unwrap();
    let (header, lines) = order_store::get_order_rows(&db, done.order_id).await.unwrap().unwrap();

    // Charge in full + ship both units (db-level, like a PSP + warehouse).
    let txn = payments::create_transaction(
        &db,
        &payments::NewTransaction {
            checkout_id: None,
            order_id: Some(done.order_id),
            currency: header.currency.clone(),
            name: "manual".into(),
            app_identifier: Some("rustygod-manual".into()),
            idempotency_key: Some(format!("e2e-{}", uuid::Uuid::new_v4())),
            available_actions: vec!["authorize".into()],
        },
    )
    .await
    .unwrap();
    payments::authorize(&db, txn.id, header.total_gross_amount, "e2e-a").await.unwrap();
    payments::charge(&db, txn.id, header.total_gross_amount, "e2e-c").await.unwrap();
    fulfillment::create_fulfillment(
        &db,
        done.order_id,
        &[fulfillment::FulfillItem { order_line_id: lines[0].id, quantity: 2, stock_id: None }],
        "",
    )
    .await
    .unwrap();

    // Return 1 unit over RPC: money + restock + status.
    let ret = svc
        .return_order_lines(
            staff_req(ReturnOrderLinesRequest {
                order_id: done.order_id.to_string(),
                lines: vec![ReturnLineInput {
                    order_line_id: lines[0].id.to_string(),
                    quantity: 1,
                    stock_id: 0,
                }],
                reason: "too big".into(),
                restock: true,
                transaction_item_id: 0,
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert!(ret.fulfillment_id > 0 && ret.granted_refund_id > 0);
    let unit_back: rust_decimal::Decimal = ret.amount.parse().unwrap();

    // One unit still out the door: cancel is REFUSED (Saleor's can_cancel —
    // the active `fulfilled` fulfillment blocks it, same as Django).
    let refused = svc
        .cancel_order(staff_req(CancelOrderRequest { id: done.order_id.to_string() }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(!refused.errors.is_empty());
    assert!(refused.errors[0].message.contains("active fulfillments"), "{:?}", refused.errors);

    // Return the second unit: order becomes fully returned, money whole.
    let ret2 = svc
        .return_order_lines(
            staff_req(ReturnOrderLinesRequest {
                order_id: done.order_id.to_string(),
                lines: vec![ReturnLineInput {
                    order_line_id: lines[0].id.to_string(),
                    quantity: 1,
                    stock_id: 0,
                }],
                reason: "too big as well".into(),
                restock: true,
                transaction_item_id: 0,
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(ret2.errors.is_empty(), "{:?}", ret2.errors);

    // Final state: returned, fully refunded, reconcile green.
    let (header2, _) = order_store::get_order_rows(&db, done.order_id).await.unwrap().unwrap();
    assert_eq!(header2.status, "returned");
    let v = payments::view(&db, txn.id).await.unwrap();
    assert_eq!(v.refunded, header.total_gross_amount);
    assert_eq!(unit_back + ret2.amount.parse::<rust_decimal::Decimal>().unwrap(), header.total_gross_amount);
    let checks = rustygod_db::reconcile::reconcile_order(&db, done.order_id).await.unwrap();
    assert!(checks.iter().all(|c| c.ok), "{checks:?}");
}
