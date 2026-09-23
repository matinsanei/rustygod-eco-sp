//! Fulfillment contract vs Django's order/warehouse tables.
//! Mirrors `saleor/graphql/order/tests/mutations/test_order_fulfill.py`
//! and `test_order_cancel.py` semantics: remainders enforced, stock moves
//! with lines, statuses follow `determine_order_status`.

use rust_decimal::Decimal;
use saleor_rustify_db::{catalog, checkout_store, database_url, fulfillment, order_store};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

struct OrderCtx {
    order_id: Uuid,
    line_ids: Vec<Uuid>,
    variant_id: i32,
}


/// Cross-process stock lock (fs2 flock): stock rows are shared Django data
/// mutated by several test binaries running concurrently. Dropping the
/// guard closes the fd and releases the lock.
struct StockGuard {
    _f: std::fs::File,
}

fn stock_guard() -> StockGuard {
    use fs2::FileExt;
    let f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open("/tmp/rustify-stock.lock")
        .expect("lock file");
    f.lock_exclusive().expect("stock lock");
    StockGuard { _f: f }
}

/// Mint an order with 2 units of a stocked variant; return ctx.
async fn mint_order(db: &DatabaseConnection) -> OrderCtx {
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    let products = catalog::list_products(db, "default-channel", None, 100).await.unwrap();
    let v = products
        .iter()
        .flat_map(|p| &p.variants)
        .find(|v| v.quantity_available >= 4)
        .expect("need a variant with stock >= 4");
    let vid: i32 = v.id.parse().unwrap();
    let unit_price = v.price.amount;

    let token = checkout_store::create_checkout_row(db, ch_id, &currency, "").await.unwrap();
    checkout_store::add_lines_tx(
        db,
        token,
        ch_id,
        &currency,
        &[checkout_store::NewLine { variant_id: vid, quantity: 2, unit_price, price_override: None }],
    )
    .await
    .unwrap();
    let (co, lines) = checkout_store::load_checkout(db, token).await.unwrap().unwrap();
    let domain = checkout_store::to_domain(&co, &lines, "default-channel");
    let pricing = catalog::checkout_pricing(
        db,
        "default-channel",
        &domain.lines.iter().filter_map(|l| l.variant_id.parse().ok()).collect::<Vec<i32>>(),
    )
    .await
    .unwrap();
    let order = order_store::mint_from_checkout(db, &domain, ch_id, "default-channel", &pricing)
        .await
        .unwrap();
    checkout_store::delete_checkout_row(db, token).await.unwrap();

    let oid: Uuid = order.id.parse().unwrap();
    let (_, olines) = order_store::get_order_rows(db, oid).await.unwrap().unwrap();
    assert_eq!(olines.len(), 1);
    OrderCtx { order_id: oid, line_ids: olines.into_iter().map(|l| l.id).collect(), variant_id: vid }
}

async fn cleanup_order(db: &DatabaseConnection, ctx: &OrderCtx) {
    let oid = ctx.order_id;
    use saleor_rustify_db::entities::{order_fulfillment, order_fulfillmentline, order_order, order_orderline};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let fulfills: Vec<i32> = {
        use sea_orm::QuerySelect;
        order_fulfillment::Entity::find()
            .select_only()
            .column(order_fulfillment::Column::Id)
            .filter(order_fulfillment::Column::OrderId.eq(oid))
            .into_tuple::<i32>()
            .all(db)
            .await
            .unwrap()
    };
    for fid in fulfills {
        order_fulfillmentline::Entity::delete_many()
            .filter(order_fulfillmentline::Column::FulfillmentId.eq(fid))
            .exec(db)
            .await
            .unwrap();
        order_fulfillment::Entity::delete_by_id(fid).exec(db).await.unwrap();
    }
    order_orderline::Entity::delete_many()
        .filter(order_orderline::Column::OrderId.eq(oid))
        .exec(db)
        .await
        .unwrap();
    order_order::Entity::delete_by_id(oid).exec(db).await.unwrap();
}

async fn order_status(db: &DatabaseConnection, oid: Uuid) -> String {
    use saleor_rustify_db::entities::order_order;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    order_order::Entity::find_by_id(oid)
        .select_only()
        .column(order_order::Column::Status)
        .filter(order_order::Column::Status.is_not_null())
        .into_tuple::<String>()
        .one(db)
        .await
        .unwrap()
        .unwrap()
}

async fn free_stock(db: &DatabaseConnection, vid: i32) -> i32 {
    saleor_rustify_db::commerce::stocks_for_variant(db, vid)
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.quantity - s.quantity_allocated)
        .sum()
}

#[tokio::test]
async fn fulfill_partial_then_full_moves_status_and_stock() {
    let _stock = stock_guard();
    let db = db().await;
    let ctx = mint_order(&db).await;
    let before = free_stock(&db, ctx.variant_id).await;

    // Over-fulfill rejected (only 2 units).
    assert!(
        fulfillment::create_fulfillment(
            &db,
            ctx.order_id,
            &[fulfillment::FulfillItem { order_line_id: ctx.line_ids[0], quantity: 3, stock_id: None }],
            ""
        )
        .await
        .is_err()
    );

    let f1 = fulfillment::create_fulfillment(
        &db,
        ctx.order_id,
        &[fulfillment::FulfillItem { order_line_id: ctx.line_ids[0], quantity: 1, stock_id: None }],
        "TRK-1",
    )
    .await
    .unwrap();
    assert_eq!(f1.fulfillment_order, 1);
    assert_eq!(f1.status, "fulfilled");
    assert_eq!(order_status(&db, ctx.order_id).await, "partially fulfilled");
    assert_eq!(free_stock(&db, ctx.variant_id).await, before - 1);

    let f2 = fulfillment::create_fulfillment(
        &db,
        ctx.order_id,
        &[fulfillment::FulfillItem { order_line_id: ctx.line_ids[0], quantity: 1, stock_id: None }],
        ""
    )
    .await
    .unwrap();
    assert_eq!(f2.fulfillment_order, 2);
    assert_eq!(order_status(&db, ctx.order_id).await, "fulfilled");
    assert_eq!(free_stock(&db, ctx.variant_id).await, before - 2);

    // Nothing left: rejected.
    assert!(
        fulfillment::create_fulfillment(
            &db,
            ctx.order_id,
            &[fulfillment::FulfillItem { order_line_id: ctx.line_ids[0], quantity: 1, stock_id: None }],
            ""
        )
        .await
        .is_err()
    );

    // Restore stock via cancel (test hygiene on shared rows).
    for f in fulfillment::list_fulfillments(&db, ctx.order_id).await.unwrap() {
        fulfillment::cancel_fulfillment(&db, f.id).await.unwrap();
    }
    assert_eq!(free_stock(&db, ctx.variant_id).await, before);

    cleanup_order(&db, &ctx).await;
}

#[tokio::test]
async fn cancel_restores_stock_and_status() {
    let _stock = stock_guard();
    let db = db().await;
    let ctx = mint_order(&db).await;
    let before = free_stock(&db, ctx.variant_id).await;

    let f = fulfillment::create_fulfillment(
        &db,
        ctx.order_id,
        &[fulfillment::FulfillItem { order_line_id: ctx.line_ids[0], quantity: 2, stock_id: None }],
        "",
    )
    .await
    .unwrap();
    assert_eq!(order_status(&db, ctx.order_id).await, "fulfilled");

    let c = fulfillment::cancel_fulfillment(&db, f.id).await.unwrap();
    assert_eq!(c.status, "canceled");
    assert_eq!(order_status(&db, ctx.order_id).await, "unfulfilled");
    assert_eq!(free_stock(&db, ctx.variant_id).await, before);

    cleanup_order(&db, &ctx).await;
}

#[tokio::test]
async fn refund_moves_to_returned() {
    let _stock = stock_guard();
    let db = db().await;
    let ctx = mint_order(&db).await;
    let before = free_stock(&db, ctx.variant_id).await;

    fulfillment::create_fulfillment(
        &db,
        ctx.order_id,
        &[fulfillment::FulfillItem { order_line_id: ctx.line_ids[0], quantity: 2, stock_id: None }],
        "",
    )
    .await
    .unwrap();
    let r = fulfillment::refund_fulfillment(
        &db,
        ctx.order_id,
        &[fulfillment::FulfillItem { order_line_id: ctx.line_ids[0], quantity: 2, stock_id: None }],
        "defective",
    )
    .await
    .unwrap();
    assert_eq!(r.status, "refunded");
    assert_eq!(order_status(&db, ctx.order_id).await, "returned");
    assert_eq!(free_stock(&db, ctx.variant_id).await, before);

    // Refunding more than fulfilled-and-unreturned is rejected.
    assert!(
        fulfillment::refund_fulfillment(
            &db,
            ctx.order_id,
            &[fulfillment::FulfillItem { order_line_id: ctx.line_ids[0], quantity: 1, stock_id: None }],
            "x",
        )
        .await
        .is_err()
    );

    cleanup_order(&db, &ctx).await;
}

#[tokio::test]
async fn status_branches_match_django() {
    use saleor_rustify_db::fulfillment::determine_status;
    assert_eq!(determine_status(10, 0, 0), "unfulfilled");
    assert_eq!(determine_status(10, 5, 0), "partially fulfilled");
    assert_eq!(determine_status(10, 10, 0), "fulfilled");
    assert_eq!(determine_status(10, 10, 3), "partially_returned");
    assert_eq!(determine_status(10, 10, 10), "returned");
    let _ = Decimal::ZERO;
}
