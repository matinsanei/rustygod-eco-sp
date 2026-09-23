//! Shipping zones/methods + tax classes + country rates.

use rust_decimal::Decimal;
use saleor_rustify_db::{database_url, ship_tax_writes};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn zone_method_lifecycle() {
    let db = db().await;
    let zid = ship_tax_writes::create_zone(
        &db,
        &ship_tax_writes::NewZone {
            name: "Smoke Zone".into(),
            description: String::new(),
            countries: vec!["US".into()],
            default: false,
            warehouse_ids: vec![],
            channel_ids: vec![1],
        },
    )
    .await
    .unwrap();
    assert!(ship_tax_writes::create_zone(
        &db,
        &ship_tax_writes::NewZone {
            name: "  ".into(),
            description: String::new(),
            countries: vec![],
            default: false,
            warehouse_ids: vec![],
            channel_ids: vec![],
        },
    )
    .await
    .is_err());
    let mid = ship_tax_writes::create_method(
        &db,
        &ship_tax_writes::NewMethod {
            zone_id: zid,
            name: "Smoke Post".into(),
            description: None,
            method_type: "price".into(),
            min_weight: None,
            max_weight: None,
            min_days: Some(1),
            max_days: Some(5),
            tax_class_id: None,
            postal_rules: vec![("10001".into(), Some("10010".into()))],
            inclusion: Some("include".into()),
        },
    )
    .await
    .unwrap();
    ship_tax_writes::update_method_listings(
        &db,
        mid,
        &[ship_tax_writes::ListingPatch {
            channel_id: 1,
            price: Some(Decimal::new(999, 2)),
            min_price: None,
            max_price: None,
        }],
        &[],
        "USD",
    )
    .await
    .unwrap();
    ship_tax_writes::exclude_products(&db, mid, &[126]).await.unwrap();
    ship_tax_writes::include_products(&db, mid, &[126]).await.unwrap();
    ship_tax_writes::delete_method(&db, mid).await.unwrap();
    ship_tax_writes::delete_zone(&db, zid).await.unwrap();
    use saleor_rustify_db::entities::shipping_shippingzone;
    use sea_orm::EntityTrait;
    assert!(shipping_shippingzone::Entity::find_by_id(zid).one(&db).await.unwrap().is_none());
}

#[tokio::test]
async fn tax_class_and_country_rates() {
    let db = db().await;
    let tid = ship_tax_writes::create_tax_class(&db, "Smoke Class", &[("DE".into(), Decimal::new(19, 0))])
        .await
        .unwrap();
    assert!(ship_tax_writes::create_tax_class(&db, "  ", &[]).await.is_err());
    ship_tax_writes::update_tax_class(
        &db,
        tid,
        &ship_tax_writes::TaxClassPatch {
            name: Some("Smoke Class 2".into()),
            update_rates: vec![("FR".into(), Decimal::new(20, 0))],
            remove_countries: vec!["DE".into()],
        },
    )
    .await
    .unwrap();
    // Country config: class rate + default rate + delete.
    ship_tax_writes::update_country_rates(
        &db,
        "NL",
        &[(Some(tid), Some(Decimal::new(21, 0))), (None, Some(Decimal::new(9, 0)))],
    )
    .await
    .unwrap();
    ship_tax_writes::update_country_rates(&db, "NL", &[(Some(tid), None)]).await.unwrap();
    ship_tax_writes::delete_country_rates(&db, "NL").await.unwrap();
    use saleor_rustify_db::entities::tax_taxclasscountryrate;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    assert!(tax_taxclasscountryrate::Entity::find()
        .filter(tax_taxclasscountryrate::Column::Country.eq("NL"))
        .one(&db)
        .await
        .unwrap()
        .is_none());
    ship_tax_writes::delete_tax_class(&db, tid).await.unwrap();
}

#[tokio::test]
async fn channel_warehouse_links() {
    let db = db().await;
    // Reorder is a silent no-op for unknown pairs (never errors the page).
    ship_tax_writes::reorder_channel_warehouses(&db, 1, &[]).await.unwrap();
    let view = saleor_rustify_db::channels::update_channel_settings(
        &db,
        "default-channel",
        saleor_rustify_db::channels::ChannelSettingsPatch {
            allow_unpaid_orders: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(view.slug.contains("default-channel"));
}
