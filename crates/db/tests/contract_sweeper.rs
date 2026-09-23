//! Sweeper contract: expired reservations rot away, live ones survive,
//! delivery claiming is single-flight.

use saleor_rustify_db::{catalog, checkout_store, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn expired_reservations_swept_live_ones_kept() {
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "sw@example.com")
        .await
        .unwrap();
    let products = catalog::list_products(&db, "default-channel", None, 100)
        .await
        .unwrap();
    let v: &saleor_rustify_core::product::ProductVariant = products
        .iter()
        .flat_map(|p| &p.variants)
        .find(|v| v.quantity_available >= 2)
        .unwrap();
    let vid: i32 = v.id.parse().unwrap();
    let pricing = catalog::checkout_pricing(&db, "default-channel", &[vid])
        .await
        .unwrap();
    let lid1 = checkout_store::add_line_row(
        &db, token, vid, 1, pricing[&vid].0.amount, &currency, None,
    )
    .await
    .unwrap();
    let lid2 = checkout_store::add_line_row(
        &db, token, vid, 1, pricing[&vid].0.amount, &currency, None,
    )
    .await
    .unwrap();
    let stocks = saleor_rustify_db::commerce::stocks_for_variant(&db, vid).await.unwrap();
    let wh: uuid::Uuid = stocks[0].warehouse_id.parse().unwrap();

    // One already-expired reservation, one live.
    saleor_rustify_db::commerce::reserve_stock(&db, vid, wh, 1, lid1, -60).await.unwrap();
    saleor_rustify_db::commerce::reserve_stock(&db, vid, wh, 1, lid2, 3600).await.unwrap();

    let swept = saleor_rustify_db::commerce::sweep_expired_reservations(&db).await.unwrap();
    assert!(swept >= 1, "must drop the expired row");

    use saleor_rustify_db::entities::warehouse_reservation;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    assert!(warehouse_reservation::Entity::find()
        .filter(warehouse_reservation::Column::CheckoutLineId.eq(lid1))
        .one(&db)
        .await
        .unwrap()
        .is_none());
    assert!(warehouse_reservation::Entity::find()
        .filter(warehouse_reservation::Column::CheckoutLineId.eq(lid2))
        .one(&db)
        .await
        .unwrap()
        .is_some());

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn delivery_claim_is_single_flight() {
    let db = db().await;
    // Unknown id: nobody wins.
    assert!(!saleor_rustify_db::webhooks::claim_delivery(&db, i32::MAX).await.unwrap());
}

#[tokio::test]
async fn expired_checkouts_swept_per_saleor_buckets() {
    use chrono::Utc;
    use saleor_rustify_db::entities::{checkout_checkout, payment_transactionitem};
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    async fn backdate(db: &sea_orm::DatabaseConnection, token: uuid::Uuid, hours: i64) {
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};
        let m = checkout_checkout::Entity::find_by_id(token).one(db).await.unwrap().unwrap();
        let mut am: checkout_checkout::ActiveModel = m.into();
        am.last_change = Set((Utc::now() - chrono::Duration::hours(hours)).into());
        am.update(db).await.unwrap();
    }
    async fn gone(db: &sea_orm::DatabaseConnection, token: uuid::Uuid) -> bool {
        use sea_orm::EntityTrait;
        checkout_checkout::Entity::find_by_id(token).one(db).await.unwrap().is_none()
    }
    // A: anonymous + empty + 7h idle -> EMPTY bucket (6h) sweeps it.
    let a = checkout_store::create_checkout_row(&db, ch_id, &currency, "").await.unwrap();
    backdate(&db, a, 7).await;
    // B: fresh anonymous empty -> survives.
    let b = checkout_store::create_checkout_row(&db, ch_id, &currency, "").await.unwrap();
    // C: user checkout with a line, 31d idle -> USER bucket is 90d -> survives.
    let c = checkout_store::create_checkout_row(&db, ch_id, &currency, "old@example.com").await.unwrap();
    let products = catalog::list_products(&db, "default-channel", None, 100).await.unwrap();
    let vid: i32 = products.iter().flat_map(|p| &p.variants).next().unwrap().id.parse().unwrap();
    let pricing = catalog::checkout_pricing(&db, "default-channel", &[vid]).await.unwrap();
    checkout_store::add_line_row(&db, c, vid, 1, pricing[&vid].0.amount, &currency, None).await.unwrap();
    backdate(&db, c, 31 * 24).await;
    // D: anonymous + empty + 31d idle BUT holding authorized money -> guard survives.
    let d = checkout_store::create_checkout_row(&db, ch_id, &currency, "").await.unwrap();
    backdate(&db, d, 31 * 24).await;
    let t = Utc::now();
    payment_transactionitem::ActiveModel {
        token: Set(uuid::Uuid::new_v4()),
        created_at: Set(t.into()),
        modified_at: Set(t.into()),
        private_metadata: Set(serde_json::json!({})),
        metadata: Set(serde_json::json!({})),
        available_actions: Set(vec![]),
        currency: Set(currency.clone()),
        charged_value: Set(rust_decimal::Decimal::ZERO),
        authorized_value: Set(rust_decimal::Decimal::new(100, 0)),
        refunded_value: Set(rust_decimal::Decimal::ZERO),
        checkout_id: Set(Some(d)),
        order_id: Set(None),
        app_id: Set(None),
        app_identifier: Set(None),
        authorize_pending_value: Set(rust_decimal::Decimal::ZERO),
        cancel_pending_value: Set(rust_decimal::Decimal::ZERO),
        canceled_value: Set(rust_decimal::Decimal::ZERO),
        charge_pending_value: Set(rust_decimal::Decimal::ZERO),
        external_url: Set(None),
        message: Set(None),
        name: Set(Some("money guard".into())),
        psp_reference: Set(Some("guard-1".into())),
        refund_pending_value: Set(rust_decimal::Decimal::ZERO),
        user_id: Set(None),
        use_old_id: Set(false),
        last_refund_success: Set(false),
        idempotency_key: Set(None),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let n = checkout_store::sweep_expired_checkouts(&db).await.unwrap();
    assert!(n >= 1, "must drop checkout A");
    assert!(gone(&db, a).await, "anonymous empty 7h idle must be swept");
    assert!(!gone(&db, b).await, "fresh checkout must survive");
    assert!(!gone(&db, c).await, "31d user checkout must survive (90d bucket)");
    assert!(!gone(&db, d).await, "checkout holding authorized money must survive");
    for tok in [b, c, d] {
        checkout_store::delete_checkout_row(&db, tok).await.unwrap();
    }
    // TransactionItem must survive its checkout's... D was NOT swept, so the
    // item row is still there; delete it explicitly for a clean slate.
    payment_transactionitem::Entity::delete_many()
        .filter(payment_transactionitem::Column::CheckoutId.eq(d))
        .exec(&db)
        .await
        .unwrap();
}
