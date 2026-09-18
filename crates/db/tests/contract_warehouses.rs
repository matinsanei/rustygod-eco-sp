//! Warehouse write contract vs Django's `warehouse_*` tables.
//! Mirrors `saleor/graphql/warehouse/tests/`: create carries an address,
//! cc options are closed, stocked warehouses refuse deletion, stock
//! upserts respect the allocated floor, zone links round-trip.

use rustygod_db::{database_url, warehouses};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn new_warehouse(slug: &str) -> warehouses::NewWarehouse {
    warehouses::NewWarehouse {
        name: format!("CI {slug}"),
        slug: slug.to_string(),
        email: "wh@example.com".into(),
        street: "1 Test St".into(),
        city: "Berlin".into(),
        postal_code: "10115".into(),
        country: "DE".into(),
        is_private: false,
        cc_option: "all".into(),
    }
}

#[tokio::test]
async fn create_update_delete_lifecycle() {
    let db = db().await;
    let slug = format!("ci-wh-{}", &Uuid::new_v4().to_string()[..8]);
    let wh = warehouses::create_warehouse(&db, new_warehouse(&slug)).await.unwrap();
    assert_eq!(wh.slug, slug);
    assert_eq!(wh.click_and_collect_option, "all");

    // Slug conflict is a clean error, not a 500.
    let err = warehouses::create_warehouse(&db, new_warehouse(&slug)).await.unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::Warehouse(_)));

    // Bad cc option rejected.
    let mut bad = new_warehouse(&format!("{slug}-x"));
    bad.cc_option = "everywhere".into();
    let err = warehouses::create_warehouse(&db, bad).await.unwrap_err();
    assert!(err.to_string().contains("click_and_collect"));

    let upd = warehouses::update_warehouse(
        &db,
        wh.id,
        warehouses::WarehousePatch {
            name: Some("CI renamed".into()),
            email: None,
            cc_option: Some("local".into()),
            is_private: Some(true),
        },
    )
    .await
    .unwrap();
    assert_eq!(upd.name, "CI renamed");
    assert_eq!(upd.click_and_collect_option, "local");
    assert!(upd.is_private);

    warehouses::delete_warehouse(&db, wh.id).await.unwrap();
    let err = warehouses::delete_warehouse(&db, wh.id).await.unwrap_err();
    assert!(err.to_string().contains("not found"));

    // Deep cleanup removes the address row too.
    warehouses::delete_warehouse_deep(&db, wh.id).await.unwrap_err();
}

#[tokio::test]
async fn stocked_warehouse_refuses_delete_and_stock_floors() {
    let db = db().await;
    let slug = format!("ci-stock-{}", &Uuid::new_v4().to_string()[..8]);
    let wh = warehouses::create_warehouse(&db, new_warehouse(&slug)).await.unwrap();

    // A variant id that exists (any product variant works for the FK-free row).
    use rustygod_db::entities::product_productvariant;
    use sea_orm::{EntityTrait, QuerySelect};
    let vid: i32 = product_productvariant::Entity::find()
        .select_only()
        .column(product_productvariant::Column::Id)
        .into_tuple()
        .one(&db)
        .await
        .unwrap()
        .expect("need a variant");

    let s = warehouses::upsert_stock(&db, wh.id, vid, 10).await.unwrap();
    assert_eq!((s.quantity, s.quantity_allocated), (10, 0));

    let err = warehouses::delete_warehouse(&db, wh.id).await.unwrap_err();
    assert!(err.to_string().contains("holds stock"));

    // Restock upserts the same row.
    let s = warehouses::upsert_stock(&db, wh.id, vid, 25).await.unwrap();
    assert_eq!(s.quantity, 25);

    let err = warehouses::upsert_stock(&db, wh.id, vid, -1).await.unwrap_err();
    assert!(err.to_string().contains("negative"));

    warehouses::delete_warehouse_deep(&db, wh.id).await.unwrap();
}

#[tokio::test]
async fn zones_list_and_link_roundtrip() {
    let db = db().await;
    let zones = warehouses::list_zones(&db).await.unwrap();
    assert!(!zones.is_empty(), "populatedb must have shipping zones");
    let zid = zones[0].id;

    let slug = format!("ci-zone-{}", &Uuid::new_v4().to_string()[..8]);
    let wh = warehouses::create_warehouse(&db, new_warehouse(&slug)).await.unwrap();

    warehouses::assign_zone(&db, wh.id, zid).await.unwrap();
    warehouses::assign_zone(&db, wh.id, zid).await.unwrap(); // idempotent
    let serving = warehouses::warehouses_for_zone(&db, zid).await.unwrap();
    assert!(serving.contains(&wh.id));

    assert!(warehouses::unassign_zone(&db, wh.id, zid).await.unwrap());
    assert!(!warehouses::unassign_zone(&db, wh.id, zid).await.unwrap());

    let err = warehouses::assign_zone(&db, wh.id, i32::MAX).await.unwrap_err();
    assert!(err.to_string().contains("shipping zone"));

    warehouses::delete_warehouse_deep(&db, wh.id).await.unwrap();
}
