//! Return-and-refund contract (E7/T2): a `refunded` fulfillment plus
//! line-based granted refund executed on one transaction, atomically.
//! Over-return is rejected with no partial side effects; restock=false
//! books damaged/lost returns without touching the shelf.

use rust_decimal::Decimal;
use rustygod_db::{catalog, checkout_store, complete, database_url, fulfillment, order_store, payments};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn stock_guard() -> fs2::FileLockGuard {
    fs2::FileLockGuard::new()
}

mod fs2 {
    pub struct FileLockGuard {
        _f: std::fs::File,
    }
    impl FileLockGuard {
        pub fn new() -> Self {
            use fs2::FileExt;
            let f = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open("/tmp/rustygod-stock.lock")
                .expect("stock lock");
            f.lock_exclusive().expect("stock lock");
            Self { _f: f }
        }
    }
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

struct PaidFulfilled {
    order_id: Uuid,
    line_id: Uuid,
    unit_gross: Decimal,
    txn_id: i32,
    vid: i32,
    free_before: i32,
}

async fn free_for(db: &DatabaseConnection, vid: i32) -> i32 {
    rustygod_db::commerce::stocks_for_variant(db, vid)
        .await
        .unwrap()
        .iter()
        .map(|s| s.quantity - s.quantity_allocated)
        .sum()
}

/// Complete 2 units, fulfill both, charge the full total.
async fn paid_fulfilled(db: &DatabaseConnection, tag: &str) -> PaidFulfilled {
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(db, ch_id, &currency, "ret@example.com")
        .await
        .unwrap();
    let products = catalog::list_products(db, "default-channel", None, 100).await.unwrap();
    // Tracked variant only: untracked (digital) variants never touch the
    // shelf, which would make the stock assertions meaningless.
    use rustygod_db::entities::product_productvariant;
    use sea_orm::EntityTrait;
    let mut vid: Option<i32> = None;
    for p in products.iter().flat_map(|p| &p.variants) {
        let id: i32 = p.id.parse().unwrap();
        if p.quantity_available < 2 {
            continue;
        }
        let v = product_productvariant::Entity::find_by_id(id).one(db).await.unwrap().unwrap();
        if v.track_inventory {
            vid = Some(id);
            break;
        }
    }
    let vid = vid.expect("need a tracked, stocked variant");
    let free_before = free_for(db, vid).await;
    let pricing = catalog::checkout_pricing(db, "default-channel", &[vid]).await.unwrap();
    let unit = pricing[&vid].0.amount;
    checkout_store::add_line_row(db, token, vid, 2, unit, &currency, None).await.unwrap();
    let done = complete::complete_checkout(db, token).await.unwrap();
    let (header, lines) = order_store::get_order_rows(db, done.order_id).await.unwrap().unwrap();
    fulfillment::create_fulfillment(
        db,
        done.order_id,
        &[fulfillment::FulfillItem { order_line_id: lines[0].id, quantity: 2, stock_id: None }],
        "",
    )
    .await
    .unwrap();
    let txn = payments::create_transaction(
        db,
        &payments::NewTransaction {
            checkout_id: None,
            order_id: Some(done.order_id),
            currency: header.currency.clone(),
            name: "manual".into(),
            app_identifier: Some("rustygod-manual".into()),
            idempotency_key: Some(format!("ret-{tag}-{}", Uuid::new_v4())),
            available_actions: vec!["authorize".into()],
        },
    )
    .await
    .unwrap();
    payments::authorize(db, txn.id, header.total_gross_amount, &format!("ret-a-{tag}")).await.unwrap();
    payments::charge(db, txn.id, header.total_gross_amount, &format!("ret-c-{tag}")).await.unwrap();
    PaidFulfilled {
        order_id: done.order_id,
        line_id: lines[0].id,
        unit_gross: lines[0].unit_price_gross_amount,
        txn_id: txn.id,
        vid,
        free_before,
    }
}

async fn cleanup(db: &DatabaseConnection, o: &PaidFulfilled, grant_ids: &[i32], fulfillment_ids: &[i32]) {
    use rustygod_db::entities::*;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    for fid in fulfillment_ids {
        order_fulfillmentline::Entity::delete_many()
            .filter(order_fulfillmentline::Column::FulfillmentId.eq(*fid))
            .exec(db)
            .await
            .unwrap();
        order_fulfillment::Entity::delete_by_id(*fid).exec(db).await.unwrap();
    }
    payment_transactionevent::Entity::delete_many()
        .filter(payment_transactionevent::Column::TransactionId.eq(o.txn_id))
        .exec(db)
        .await
        .unwrap();
    for g in grant_ids {
        order_ordergrantedrefundline::Entity::delete_many()
            .filter(order_ordergrantedrefundline::Column::GrantedRefundId.eq(*g))
            .exec(db)
            .await
            .unwrap();
        order_ordergrantedrefund::Entity::delete_by_id(*g).exec(db).await.unwrap();
    }
    payment_transactionitem::Entity::delete_by_id(o.txn_id).exec(db).await.unwrap();
}

#[tokio::test]
async fn return_one_unit_restocks_and_refunds() {
    let _guard = stock_guard();
    let db = db().await;
    let o = paid_fulfilled(&db, "one").await;
    let out = fulfillment::return_and_refund(
        &db,
        o.order_id,
        &[fulfillment::FulfillItem { order_line_id: o.line_id, quantity: 1, stock_id: None }],
        "size too small",
        true,
        None,
    )
    .await
    .unwrap();
    // Money: exactly one unit gross back, decision success.
    assert_eq!(out.amount, o.unit_gross);
    let g = rustygod_db::granted_refunds::view(&db, out.granted_refund_id).await.unwrap();
    assert_eq!(g.status, "success");
    assert_eq!(g.lines.len(), 1);
    assert_eq!(g.lines[0].quantity, 1);
    let v = payments::view(&db, o.txn_id).await.unwrap();
    assert_eq!(v.refunded, o.unit_gross);
    // Shelf: complete pinned 2 allocations, fulfill took 2 units off,
    // return put 1 back → net -3.
    assert_eq!(free_for(&db, o.vid).await, o.free_before - 3);
    // Order status reflects the partial return (Django branches).
    let (header, _) = order_store::get_order_rows(&db, o.order_id).await.unwrap().unwrap();
    assert!(header.status.contains("return"), "status={}", header.status);
    // Reconcile green with a live grant on the books.
    let checks = rustygod_db::reconcile::reconcile_order(&db, o.order_id).await.unwrap();
    assert!(checks.iter().all(|c| c.ok), "{checks:?}");
    cleanup(&db, &o, &[out.granted_refund_id], &[out.fulfillment_id]).await;
}

#[tokio::test]
async fn over_return_rejected_atomically() {
    let _guard = stock_guard();
    let db = db().await;
    let o = paid_fulfilled(&db, "over").await;
    // Only 2 fulfilled: asking 3 must fail with NOTHING persisted.
    let err = fulfillment::return_and_refund(
        &db,
        o.order_id,
        &[fulfillment::FulfillItem { order_line_id: o.line_id, quantity: 3, stock_id: None }],
        "greedy",
        true,
        None,
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("only 2 fulfilled and unreturned"), "{err}");
    use rustygod_db::entities::{order_fulfillment, order_ordergrantedrefund};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    assert_eq!(
        order_fulfillment::Entity::find()
            .filter(order_fulfillment::Column::OrderId.eq(o.order_id))
            .all(&db)
            .await
            .unwrap()
            .len(),
        1,
        "no refunded fulfillment row may leak"
    );
    assert!(
        order_ordergrantedrefund::Entity::find()
            .filter(order_ordergrantedrefund::Column::OrderId.eq(o.order_id))
            .all(&db)
            .await
            .unwrap()
            .is_empty(),
        "no grant row may leak"
    );
    let v = payments::view(&db, o.txn_id).await.unwrap();
    assert_eq!(v.refunded, dec("0"));
    cleanup(&db, &o, &[], &[]).await;
}

#[tokio::test]
async fn damaged_return_keeps_shelf_but_moves_money() {
    let _guard = stock_guard();
    let db = db().await;
    let o = paid_fulfilled(&db, "damaged").await;
    let out = fulfillment::return_and_refund(
        &db,
        o.order_id,
        &[fulfillment::FulfillItem { order_line_id: o.line_id, quantity: 1, stock_id: None }],
        "arrived broken",
        false,
        None,
    )
    .await
    .unwrap();
    assert_eq!(out.amount, o.unit_gross);
    let v = payments::view(&db, o.txn_id).await.unwrap();
    assert_eq!(v.refunded, o.unit_gross);
    // Shelf untouched by the return: 2 allocations still pinned and
    // 2 units still out → net -4 from before checkout.
    assert_eq!(free_for(&db, o.vid).await, o.free_before - 4);
    cleanup(&db, &o, &[out.granted_refund_id], &[out.fulfillment_id]).await;
}
