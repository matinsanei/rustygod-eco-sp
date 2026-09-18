//! Atomic completion contract: idempotent replay, single-transaction
//! mint+voucher+giftcard+allocate+event+delete, and full rollback on
//! insufficient stock. Covers audit R1–R7, E1, E13, RC3, RC8.

use rustygod_db::{catalog, checkout_store, complete, database_url};
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

/// Build a checkout with 1 unit of a stocked variant; return (token, vid).
async fn stocked_checkout(db: &DatabaseConnection, qty: i32) -> (Uuid, i32) {
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(db, ch_id, &currency, "complete@example.com")
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
                .expect("lock file");
            f.lock_exclusive().expect("stock lock");
            Self { _f: f }
        }
    }
}

#[tokio::test]
async fn complete_mints_allocates_events_and_replays() {
    let _guard = stock_guard();
    let db = db().await;
    let (token, _vid) = stocked_checkout(&db, 1).await;

    let out = complete::complete_checkout(&db, token).await.unwrap();
    assert!(!out.replayed);
    assert!(out.status == "unfulfilled" || out.status == "unconfirmed");

    // Checkout is gone (same end state as Django).
    assert!(checkout_store::load_checkout(&db, token).await.unwrap().is_none());

    // Order stamped with the token + PLACED event.
    let (header, lines) = rustygod_db::order_store::get_order_rows(&db, out.order_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(header.id, out.order_id);
    assert!(!lines.is_empty());
    use rustygod_db::entities::order_orderevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let placed = order_orderevent::Entity::find()
        .filter(order_orderevent::Column::OrderId.eq(out.order_id))
        .filter(order_orderevent::Column::Type.eq("placed"))
        .one(&db)
        .await
        .unwrap();
    assert!(placed.is_some(), "PLACED event must exist");

    // Allocations cover tracked lines (RC3).
    use rustygod_db::entities::warehouse_allocation;
    for l in &lines {
        if l.variant_id.is_none() {
            continue;
        }
        let alloced: i32 = warehouse_allocation::Entity::find()
            .filter(warehouse_allocation::Column::OrderLineId.eq(l.id))
            .all(&db)
            .await
            .unwrap()
            .iter()
            .map(|a| a.quantity_allocated)
            .sum();
        // Tracked variants allocate fully; untracked skip (0).
        assert!(
            alloced == l.quantity || alloced == 0,
            "line {} qty {} allocated {alloced}",
            l.id,
            l.quantity
        );
    }

    // Idempotent replay: same order, no second mint.
    let again = complete::complete_checkout(&db, token).await.unwrap();
    assert!(again.replayed);
    assert_eq!(again.order_id, out.order_id);

    // Reconciliation: a fresh atomic completion is fully consistent.
    let checks = rustygod_db::reconcile::reconcile_order(&db, out.order_id)
        .await
        .unwrap();
    assert_eq!(checks.len(), 9);
    for c in &checks {
        assert!(c.ok, "{}: {}", c.name, c.detail);
    }
}

#[tokio::test]
async fn complete_rolls_back_on_insufficient_stock() {
    let _guard = stock_guard();
    let db = db().await;
    let (token, _vid) = stocked_checkout(&db, 1).await;
    // Blow the quantity past any warehouse: allocation must fail and the
    // checkout must survive untouched (all-or-nothing).
    let (_, lines) = checkout_store::load_checkout(&db, token).await.unwrap().unwrap();
    // Bump line qty to absurd via direct update of the checkout line.
    use rustygod_db::entities::checkout_checkoutline;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};
    let line = checkout_checkoutline::Entity::find_by_id(lines[0].id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let mut am: checkout_checkoutline::ActiveModel = line.into();
    am.quantity = Set(1_000_000);
    am.update(&db).await.unwrap();

    let err = complete::complete_checkout(&db, token).await.unwrap_err();
    assert!(err.to_string().contains("insufficient stock"), "{err}");

    // Nothing persisted: checkout intact, no order stamped.
    let (co, lines) = checkout_store::load_checkout(&db, token).await.unwrap().unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(co.token, token);
    use rustygod_db::entities::order_order;
    use sea_orm::{ColumnTrait, QueryFilter};
    let orphan = order_order::Entity::find()
        .filter(order_order::Column::CheckoutToken.eq(token.to_string()))
        .one(&db)
        .await
        .unwrap();
    assert!(orphan.is_none(), "rolled-back completion must not mint");

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn complete_unknown_token_is_not_found() {
    let db = db().await;
    let err = complete::complete_checkout(&db, Uuid::new_v4()).await.unwrap_err();
    assert!(err.to_string().contains("not found"), "{err}");
}
