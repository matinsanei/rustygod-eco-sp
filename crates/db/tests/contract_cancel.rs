//! Cancel-order contract: unpaid cancel releases allocations and flips
//! status; paid orders refuse (refund flow); double cancel is a clean
//! error; concurrent complete/cancel on one variant never deadlocks
//! (shared lock order — reviewer week-1 deadlock test).

use rustygod_db::{cancel, catalog, checkout_store, complete, database_url};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn stock_guard() -> StockGuard {
    StockGuard::new()
}

struct StockGuard {
    _f: std::fs::File,
}
impl StockGuard {
    fn new() -> Self {
        use fs2::FileExt;
        let f = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("/tmp/rustygod-stock.lock")
            .expect("lock file");
        f.lock_exclusive().expect("stock lock");
        Self { _f: f }
    }
}

async fn stocked_checkout(db: &DatabaseConnection, qty: i32) -> (Uuid, i32) {
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(db, ch_id, &currency, "cancel@example.com")
        .await
        .unwrap();
    let products = catalog::list_products(db, "default-channel", None, 100)
        .await
        .unwrap();
    let vid: i32 = products
        .iter()
        .flat_map(|p| &p.variants)
        .find(|v| v.quantity_available >= qty)
        .expect("need a stocked variant")
        .id
        .parse()
        .unwrap();
    let pricing = catalog::checkout_pricing(db, "default-channel", &[vid])
        .await
        .unwrap();
    let unit = pricing[&vid].0.amount;
    checkout_store::add_line_row(db, token, vid, qty, unit, &currency, None)
        .await
        .unwrap();
    (token, vid)
}

async fn free_for(db: &DatabaseConnection, vid: i32) -> i32 {
    rustygod_db::commerce::stocks_for_variant(db, vid)
        .await
        .unwrap()
        .iter()
        .map(|s| s.quantity - s.quantity_allocated)
        .sum()
}

#[tokio::test]
async fn cancel_releases_allocations_and_flips_status() {
    let _guard = stock_guard();
    let db = db().await;
    let (token, vid) = stocked_checkout(&db, 2).await;
    let free_before = free_for(&db, vid).await;

    let done = complete::complete_checkout(&db, token).await.unwrap();
    assert_eq!(free_for(&db, vid).await, free_before - 2);

    let out = cancel::cancel_order(&db, done.order_id).await.unwrap();
    assert_eq!(out.order_id, done.order_id);

    // Status flipped, allocations gone, stock back.
    let (header, _) = rustygod_db::order_store::get_order_rows(&db, done.order_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(header.status, "canceled");
    assert_eq!(free_for(&db, vid).await, free_before);

    // Canceled event written.
    use rustygod_db::entities::order_orderevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let ev = order_orderevent::Entity::find()
        .filter(order_orderevent::Column::OrderId.eq(done.order_id))
        .filter(order_orderevent::Column::Type.eq("canceled"))
        .one(&db)
        .await
        .unwrap();
    assert!(ev.is_some());

    // Double cancel is a clean error, not a second mutation.
    let err = cancel::cancel_order(&db, done.order_id).await.unwrap_err();
    assert!(err.to_string().contains("cannot be cancelled"), "{err}");

    // Unknown order.
    let err = cancel::cancel_order(&db, Uuid::new_v4()).await.unwrap_err();
    assert!(err.to_string().contains("not found"), "{err}");
}

#[tokio::test]
async fn cancel_refuses_paid_orders() {
    let _guard = stock_guard();
    let db = db().await;
    let (token, _vid) = stocked_checkout(&db, 1).await;
    let done = complete::complete_checkout(&db, token).await.unwrap();

    // Capture funds against the order (authorize then charge, like a PSP).
    let one = rust_decimal::Decimal::new(1, 0);
    let txn = rustygod_db::payments::create_transaction(
        &db,
        &rustygod_db::payments::NewTransaction {
            checkout_id: None,
            order_id: Some(done.order_id),
            currency: "USD".into(),
            name: "ci".into(),
            app_identifier: None,
            idempotency_key: Some(format!("ci-{}", Uuid::new_v4())),
            available_actions: vec!["charge".into(), "refund".into()],
        },
    )
    .await
    .unwrap();
    rustygod_db::payments::authorize(&db, txn.id, one, &format!("k-{}", Uuid::new_v4()))
        .await
        .unwrap();
    rustygod_db::payments::charge(&db, txn.id, one, &format!("k-{}", Uuid::new_v4()))
        .await
        .unwrap();

    let err = cancel::cancel_order(&db, done.order_id).await.unwrap_err();
    assert!(err.to_string().contains("REQUIRES_REFUND"), "{err}");

    // Cleanup: refund the cent, delete txn rows + order graph (test-only).
    rustygod_db::payments::refund(&db, txn.id, rust_decimal::Decimal::new(1, 0), &format!("k-{}", Uuid::new_v4()))
        .await
        .unwrap();
    use rustygod_db::entities::{
        order_order, order_orderevent, order_orderline, payment_transactionevent,
        payment_transactionitem, warehouse_allocation,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let (_, lines) = rustygod_db::order_store::get_order_rows(&db, done.order_id)
        .await
        .unwrap()
        .unwrap();
    for l in &lines {
        warehouse_allocation::Entity::delete_many()
            .filter(warehouse_allocation::Column::OrderLineId.eq(l.id))
            .exec(&db)
            .await
            .unwrap();
        order_orderline::Entity::delete_by_id(l.id).exec(&db).await.unwrap();
    }
    let txns = payment_transactionitem::Entity::find()
        .filter(payment_transactionitem::Column::OrderId.eq(done.order_id))
        .all(&db)
        .await
        .unwrap();
    for t in &txns {
        payment_transactionevent::Entity::delete_many()
            .filter(payment_transactionevent::Column::TransactionId.eq(t.id))
            .exec(&db)
            .await
            .unwrap();
        payment_transactionitem::Entity::delete_by_id(t.id).exec(&db).await.unwrap();
    }
    order_orderevent::Entity::delete_many()
        .filter(order_orderevent::Column::OrderId.eq(done.order_id))
        .exec(&db)
        .await
        .unwrap();
    order_order::Entity::delete_by_id(done.order_id).exec(&db).await.unwrap();
}

#[tokio::test]
async fn concurrent_complete_and_cancel_never_deadlock() {
    let _guard = stock_guard();
    let db = db().await;
    // Two checkouts on the SAME stocked variant + one completed order for cancel.
    let (t1, vid) = stocked_checkout(&db, 1).await;
    let (t2, _) = stocked_checkout(&db, 1).await;
    let done = complete::complete_checkout(&db, t2).await.unwrap();

    let db1 = db.clone();
    let db2 = db.clone();
    let h1 = tokio::spawn(async move { complete::complete_checkout(&db1, t1).await });
    let h2 = tokio::spawn(async move { cancel::cancel_order(&db2, done.order_id).await });
    let (r1, r2) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        (h1.await.unwrap(), h2.await.unwrap())
    })
    .await
    .expect("complete+cancel must not deadlock");
    assert!(r1.is_ok(), "complete failed: {r1:?}");
    assert!(r2.is_ok(), "cancel failed: {r2:?}");
    let oid1 = r1.unwrap().order_id;

    // Cleanup both orders (allocations, events, lines, order).
    use rustygod_db::entities::{order_order, order_orderevent, order_orderline, warehouse_allocation};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    for oid in [oid1, done.order_id] {
        if let Some((_, lines)) = rustygod_db::order_store::get_order_rows(&db, oid).await.unwrap() {
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
    }
    let _ = vid;
}
