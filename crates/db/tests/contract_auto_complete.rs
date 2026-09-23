//! Auto-complete contract (E10 final): fully-authorized expired checkouts on
//! opted-in channels complete into orders; everyone else is untouched.

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

/// Flip the auto-complete flag on default-channel; returns a restore closure.
async fn set_flag(db: &DatabaseConnection, on: bool) {
    use rustygod_db::entities::channel_channel;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    // update_many (never a full-model select: channel rows carry INTERVAL
    // columns SeaORM cannot decode).
    let r = channel_channel::Entity::update_many()
        .col_expr(
            channel_channel::Column::AutomaticallyCompleteFullyPaidCheckouts,
            sea_orm::sea_query::Expr::value(on),
        )
        .filter(channel_channel::Column::Slug.eq("default-channel"))
        .exec(db)
        .await
        .unwrap();
    assert_eq!(r.rows_affected, 1, "default-channel must exist");
}

async fn any_address_id(db: &DatabaseConnection) -> i32 {
    use rustygod_db::entities::account_address;
    use sea_orm::EntityTrait;
    account_address::Entity::find()
        .one(db)
        .await
        .unwrap()
        .expect("seed must contain an address")
        .id
}

/// Build an expired, fully-authorized, paid checkout eligible for
/// auto-complete. Returns the token.
async fn eligible_checkout(db: &DatabaseConnection, email: &str) -> Uuid {
    use chrono::Utc;
    use rustygod_db::entities::{checkout_checkout, payment_transactionitem};
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(db, ch_id, &currency, email)
        .await
        .unwrap();
    let products = catalog::list_products(db, "default-channel", None, 100)
        .await
        .unwrap();
    let vid: i32 = products
        .iter()
        .flat_map(|p| &p.variants)
        .find(|v| v.quantity_available >= 1)
        .expect("need a stocked variant")
        .id
        .parse()
        .unwrap();
    let pricing = catalog::checkout_pricing(db, "default-channel", &[vid])
        .await
        .unwrap();
    checkout_store::add_line_row(db, token, vid, 1, pricing[&vid].0.amount, &currency, None)
        .await
        .unwrap();
    checkout_store::refresh_totals(db, token).await.unwrap();
    // Billing + FULL authorize + a backing authorized transaction.
    let total = pricing[&vid].0.amount;
    let addr = any_address_id(db).await;
    let m = checkout_checkout::Entity::find_by_id(token).one(db).await.unwrap().unwrap();
    let mut am: checkout_checkout::ActiveModel = m.into();
    am.billing_address_id = Set(Some(addr));
    am.authorize_status = Set("full".to_string());
    am.charge_status = Set("full".to_string());
    am.last_change = Set((Utc::now() - chrono::Duration::hours(48)).into());
    am.update(db).await.unwrap();
    let t = Utc::now();
    payment_transactionitem::ActiveModel {
        token: Set(Uuid::new_v4()),
        created_at: Set(t.into()),
        modified_at: Set(t.into()),
        private_metadata: Set(serde_json::json!({})),
        metadata: Set(serde_json::json!({})),
        available_actions: Set(vec![]),
        currency: Set(currency),
        charged_value: Set(total),
        authorized_value: Set(rust_decimal::Decimal::ZERO),
        refunded_value: Set(rust_decimal::Decimal::ZERO),
        checkout_id: Set(Some(token)),
        order_id: Set(None),
        app_id: Set(None),
        app_identifier: Set(None),
        authorize_pending_value: Set(rust_decimal::Decimal::ZERO),
        cancel_pending_value: Set(rust_decimal::Decimal::ZERO),
        canceled_value: Set(rust_decimal::Decimal::ZERO),
        charge_pending_value: Set(rust_decimal::Decimal::ZERO),
        external_url: Set(None),
        message: Set(None),
        name: Set(Some("auto-complete cover".into())),
        psp_reference: Set(Some(format!("auto-{token}"))),
        refund_pending_value: Set(rust_decimal::Decimal::ZERO),
        user_id: Set(None),
        use_old_id: Set(false),
        last_refund_success: Set(false),
        idempotency_key: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    token
}

async fn checkout_gone(db: &DatabaseConnection, token: Uuid) -> bool {
    use rustygod_db::entities::checkout_checkout;
    use sea_orm::EntityTrait;
    checkout_checkout::Entity::find_by_id(token).one(db).await.unwrap().is_none()
}

#[tokio::test]
async fn auto_complete_mints_order_and_links_money() {
    let _guard = stock_guard();
    let db = db().await;
    set_flag(&db, true).await;
    let token = eligible_checkout(&db, "auto@example.com").await;

    let r = checkout_store::auto_complete_expired_checkouts(&db).await.unwrap();
    assert_eq!(r.attempted, 1, "exactly this checkout is eligible");
    assert_eq!(r.completed, 1);
    assert_eq!(r.failed, 0);
    assert!(checkout_gone(&db, token).await, "checkout must be consumed");

    // Order stamped with the token, FULL statuses carried over.
    use rustygod_db::entities::order_order;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let order = order_order::Entity::find()
        .filter(order_order::Column::CheckoutToken.eq(token.to_string()))
        .one(&db)
        .await
        .unwrap()
        .expect("order must exist");
    assert_eq!(order.authorize_status, "full");
    assert_eq!(order.charge_status, "full");

    // Money trail follows the order (Django payments.update parity).
    use rustygod_db::entities::payment_transactionitem;
    let item = payment_transactionitem::Entity::find()
        .filter(payment_transactionitem::Column::PspReference.eq(format!("auto-{token}")))
        .one(&db)
        .await
        .unwrap()
        .expect("transaction must survive");
    assert_eq!(item.order_id, Some(order.id));
    assert_eq!(item.checkout_id, None);

    // Replay is a no-op success (idempotent): nothing eligible anymore.
    let r2 = checkout_store::auto_complete_expired_checkouts(&db).await.unwrap();
    assert_eq!(r2.attempted, 0);
    set_flag(&db, false).await;
}

#[tokio::test]
async fn auto_complete_skips_opted_out_channel() {
    let _guard = stock_guard();
    let db = db().await;
    set_flag(&db, false).await;
    let token = eligible_checkout(&db, "optout@example.com").await;

    let r = checkout_store::auto_complete_expired_checkouts(&db).await.unwrap();
    assert_eq!(r.attempted, 0, "flag off: nothing eligible");
    assert!(!checkout_gone(&db, token).await, "checkout must survive");

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
    use rustygod_db::entities::payment_transactionitem;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    payment_transactionitem::Entity::delete_many()
        .filter(payment_transactionitem::Column::CheckoutId.eq(token))
        .exec(&db)
        .await
        .unwrap();
}

#[tokio::test]
async fn auto_complete_skips_partially_authorized() {
    let _guard = stock_guard();
    let db = db().await;
    set_flag(&db, true).await;
    let token = eligible_checkout(&db, "partial@example.com").await;
    // Downgrade to partial: selection requires FULL.
    use rustygod_db::entities::checkout_checkout;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};
    let m = checkout_checkout::Entity::find_by_id(token).one(&db).await.unwrap().unwrap();
    let mut am: checkout_checkout::ActiveModel = m.into();
    am.authorize_status = Set("partial".to_string());
    am.update(&db).await.unwrap();

    let r = checkout_store::auto_complete_expired_checkouts(&db).await.unwrap();
    assert_eq!(r.attempted, 0, "partial authorize: not eligible");
    assert!(!checkout_gone(&db, token).await, "checkout must survive");

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
    use rustygod_db::entities::payment_transactionitem;
    use sea_orm::{ColumnTrait, QueryFilter};
    payment_transactionitem::Entity::delete_many()
        .filter(payment_transactionitem::Column::CheckoutId.eq(token))
        .exec(&db)
        .await
        .unwrap();
    set_flag(&db, false).await;
}

/// Manual checkout_complete still links payments to the order (7b parity).
#[tokio::test]
async fn manual_complete_links_payments_to_order() {
    let _guard = stock_guard();
    let db = db().await;
    let token = eligible_checkout(&db, "manual@example.com").await;

    let out = complete::complete_checkout(&db, token).await.unwrap();
    assert!(!out.replayed);

    use rustygod_db::entities::payment_transactionitem;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let item = payment_transactionitem::Entity::find()
        .filter(payment_transactionitem::Column::PspReference.eq(format!("auto-{token}")))
        .one(&db)
        .await
        .unwrap()
        .expect("transaction must survive");
    assert_eq!(item.order_id, Some(out.order_id));
    assert_eq!(item.checkout_id, None);
}
