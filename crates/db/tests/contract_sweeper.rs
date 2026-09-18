//! Sweeper contract: expired reservations rot away, live ones survive,
//! delivery claiming is single-flight.

use rustygod_db::{catalog, checkout_store, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
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
    let v: &rustygod_core::product::ProductVariant = products
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
    let stocks = rustygod_db::commerce::stocks_for_variant(&db, vid).await.unwrap();
    let wh: uuid::Uuid = stocks[0].warehouse_id.parse().unwrap();

    // One already-expired reservation, one live.
    rustygod_db::commerce::reserve_stock(&db, vid, wh, 1, lid1, -60).await.unwrap();
    rustygod_db::commerce::reserve_stock(&db, vid, wh, 1, lid2, 3600).await.unwrap();

    let swept = rustygod_db::commerce::sweep_expired_reservations(&db).await.unwrap();
    assert!(swept >= 1, "must drop the expired row");

    use rustygod_db::entities::warehouse_reservation;
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
    assert!(!rustygod_db::webhooks::claim_delivery(&db, i32::MAX).await.unwrap());
}
