//! Order extras, warehouse writes, giftcard adjust, menu delete, tax exemption.

use saleor_rustify_db::{database_url, order_store};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn an_order(db: &DatabaseConnection) -> Uuid {
    use saleor_rustify_db::entities::order_order;
    use sea_orm::{EntityTrait, QuerySelect};
    order_order::Entity::find()
        .select_only()
        .column(order_order::Column::Id)
        .into_tuple::<Uuid>()
        .one(db)
        .await
        .unwrap()
        .expect("seed must contain orders")
}

#[tokio::test]
async fn order_note_roundtrip() {
    let db = db().await;
    let oid = an_order(&db).await;
    order_store::add_order_note(&db, oid, None, "smoke note").await.unwrap();
    use saleor_rustify_db::entities::order_orderevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let ev = order_orderevent::Entity::find()
        .filter(order_orderevent::Column::OrderId.eq(oid))
        .filter(order_orderevent::Column::Type.eq("note_added"))
        .one(&db)
        .await
        .unwrap();
    assert!(ev.is_some(), "NOTE_ADDED event must exist");
    assert!(order_store::add_order_note(&db, Uuid::new_v4(), None, "x").await.is_err());
}

#[tokio::test]
async fn stock_bulk_paths_and_zone_links() {
    let db = db().await;
    // Resolve a real variant + warehouse.
    let (vid, wid): (i32, Uuid) = {
        use saleor_rustify_db::entities::warehouse_stock;
        use sea_orm::{EntityTrait, QuerySelect};
        warehouse_stock::Entity::find()
            .select_only()
            .column(warehouse_stock::Column::ProductVariantId)
            .column(warehouse_stock::Column::WarehouseId)
            .into_tuple::<(i32, Uuid)>()
            .one(&db)
            .await
            .unwrap()
            .expect("seed must have stocks")
    };
    let before: i32 = {
        use saleor_rustify_db::entities::warehouse_stock;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        warehouse_stock::Entity::find()
            .select_only()
            .column(warehouse_stock::Column::Quantity)
            .filter(warehouse_stock::Column::ProductVariantId.eq(vid))
            .filter(warehouse_stock::Column::WarehouseId.eq(wid))
            .into_tuple::<i32>()
            .one(&db)
            .await
            .unwrap()
            .unwrap()
    };
    saleor_rustify_db::catalog_writes::set_variant_stock(&db, vid, wid, before + 1).await.unwrap();
    saleor_rustify_db::catalog_writes::set_variant_stock(&db, vid, wid, before).await.unwrap();

    // Zone link + unlink roundtrip.
    use saleor_rustify_db::entities::shipping_shippingzone;
    use sea_orm::{EntityTrait, QuerySelect};
    let zid: i32 = shipping_shippingzone::Entity::find()
        .select_only()
        .column(shipping_shippingzone::Column::Id)
        .into_tuple::<i32>()
        .one(&db)
        .await
        .unwrap()
        .expect("seed must have zones");
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO warehouse_warehouse_shipping_zones (warehouse_id, shippingzone_id) VALUES ($1::uuid, $2) ON CONFLICT DO NOTHING",
        [wid.to_string().into(), zid.into()],
    ))
    .await
    .unwrap();
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM warehouse_warehouse_shipping_zones WHERE warehouse_id = $1::uuid AND shippingzone_id = $2",
        [wid.to_string().into(), zid.into()],
    ))
    .await
    .unwrap();
}

#[tokio::test]
async fn giftcard_adjust_and_menu_delete() {
    let db = db().await;
    use saleor_rustify_db::entities::giftcard_giftcard;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let (code, bal): (String, rust_decimal::Decimal) = giftcard_giftcard::Entity::find()
        .select_only()
        .column(giftcard_giftcard::Column::Code)
        .column(giftcard_giftcard::Column::CurrentBalanceAmount)
        .into_tuple::<(String, rust_decimal::Decimal)>()
        .one(&db)
        .await
        .unwrap()
        .expect("seed must have gift cards");
    saleor_rustify_db::giftcards::adjust_balance(&db, &code, bal, None).await.unwrap();

    // Menu item subtree delete on a scratch branch (never seed rows).
    use saleor_rustify_db::entities::{menu_menu, menu_menuitem};
    let mid: i32 = menu_menu::Entity::find()
        .select_only()
        .column(menu_menu::Column::Id)
        .into_tuple::<i32>()
        .one(&db)
        .await
        .unwrap()
        .expect("seed must have menus");
    let root = menu_menuitem::ActiveModel {
        menu_id: sea_orm::Set(mid),
        name: sea_orm::Set(format!("scratch-{}", &uuid::Uuid::new_v4().to_string()[..8])),
        lft: sea_orm::Set(900000),
        rght: sea_orm::Set(900001),
        tree_id: sea_orm::Set(999999),
        level: sea_orm::Set(0),
        ..Default::default()
    };
    use sea_orm::ActiveModelTrait;
    let rid = root.insert(&db).await.unwrap().id;
    menu_menuitem::Entity::delete_many()
        .filter(menu_menuitem::Column::TreeId.eq(999999))
        .exec(&db)
        .await
        .unwrap();
    assert!(menu_menuitem::Entity::find_by_id(rid).one(&db).await.unwrap().is_none());
}

#[tokio::test]
async fn tax_exemption_toggles_both_tables() {
    let db = db().await;
    use saleor_rustify_db::entities::{checkout_checkout, order_order};
    use sea_orm::{ActiveModelTrait, EntityTrait, QuerySelect, Set};
    // Checkout row flip + restore.
    let ctok: Option<Uuid> = checkout_checkout::Entity::find()
        .select_only()
        .column(checkout_checkout::Column::Token)
        .into_tuple::<Uuid>()
        .one(&db)
        .await
        .unwrap();
    if let Some(t) = ctok {
        let m = checkout_checkout::Entity::find_by_id(t).one(&db).await.unwrap().unwrap();
        let orig = m.tax_exemption;
        let mut am: checkout_checkout::ActiveModel = m.into();
        am.tax_exemption = Set(!orig);
        am.update(&db).await.unwrap();
        let back = checkout_checkout::Entity::find_by_id(t).one(&db).await.unwrap().unwrap();
        assert_eq!(back.tax_exemption, !orig);
        let mut am2: checkout_checkout::ActiveModel = back.into();
        am2.tax_exemption = Set(orig);
        am2.update(&db).await.unwrap();
    }
    // Order row flip + restore.
    let oid = an_order(&db).await;
    let m = order_order::Entity::find_by_id(oid).one(&db).await.unwrap().unwrap();
    let orig = m.tax_exemption;
    let mut am: order_order::ActiveModel = m.into();
    am.tax_exemption = Set(!orig);
    am.update(&db).await.unwrap();
    let back = order_order::Entity::find_by_id(oid).one(&db).await.unwrap().unwrap();
    assert_eq!(back.tax_exemption, !orig);
    let mut am2: order_order::ActiveModel = back.into();
    am2.tax_exemption = Set(orig);
    am2.update(&db).await.unwrap();
}
