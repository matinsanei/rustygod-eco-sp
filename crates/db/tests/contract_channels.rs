//! Channel + listing write contract vs Django's tables.
//! Mirrors `saleor/graphql/channel/tests/` and product listing mutations:
//! create fills Django defaults, slug conflicts and bad inputs are clean
//! errors, order/checkout channels refuse deletion, listing upserts stick.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use saleor_rustify_db::{catalog, channels, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn create_update_delete_channel() {
    let db = db().await;
    let slug = format!("ci-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let ch = channels::create_channel(
        &db,
        channels::NewChannel {
            name: "CI channel".into(),
            slug: slug.clone(),
            currency_code: "usd".into(),
            default_country: "us".into(),
            allocation_strategy: String::new(),
        },
    )
    .await
    .unwrap();
    assert_eq!(ch.currency_code, "USD"); // normalized, like Django
    assert_eq!(ch.default_country, "US");
    assert_eq!(ch.allocation_strategy, "prioritize-sorting-order");
    assert!(ch.is_active);

    let err = channels::create_channel(
        &db,
        channels::NewChannel {
            name: "dup".into(),
            slug: slug.clone(),
            currency_code: "USD".into(),
            default_country: "US".into(),
            allocation_strategy: String::new(),
        },
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("taken"));

    let upd = channels::update_channel(
        &db,
        &slug,
        channels::ChannelPatch {
            name: Some("CI renamed".into()),
            is_active: Some(false),
            default_country: None,
            allocation_strategy: Some("prioritize-high-stock".into()),
            auto_confirm: Some(false),
        },
    )
    .await
    .unwrap();
    assert_eq!(upd.name, "CI renamed");
    assert!(!upd.is_active);

    // Fresh channel with no orders/checkouts deletes cleanly.
    channels::delete_channel(&db, &slug).await.unwrap();
    let err = channels::delete_channel(&db, &slug).await.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test]
async fn lived_channel_refuses_delete() {
    let db = db().await;
    // default-channel has orders: deletion must refuse, never orphan.
    let err = channels::delete_channel(&db, "default-channel").await.unwrap_err();
    assert!(err.to_string().contains("orders"));
}

#[tokio::test]
async fn listing_upserts_stick_and_price_validates() {
    let db = db().await;
    let products = catalog::list_products(&db, "default-channel", None, 50)
        .await
        .unwrap();
    // Avoid variant 353 (the promotions suite pins its $40.00 price).
    let (pid, vid, orig_price) = products
        .iter()
        .flat_map(|p| p.variants.iter().map(|v| (p.id.clone(), v)))
        .filter(|(_, v)| v.id != "353")
        .map(|(pid, v)| {
            let vid: i32 = v.id.parse().unwrap();
            (pid, vid, v.price.amount)
        })
        .next()
        .expect("need a non-353 listed variant");
    let pid: i32 = pid.parse().unwrap();

    // Unpublish then republish (upsert both ways).
    channels::set_product_listing(&db, "default-channel", pid, false, false)
        .await
        .unwrap();
    channels::set_product_listing(&db, "default-channel", pid, true, true)
        .await
        .unwrap();
    let back = catalog::list_products(&db, "default-channel", None, 1000)
        .await
        .unwrap();
    assert!(back.iter().any(|x| x.id == pid.to_string()), "product must be listed again");

    // Price update round-trips through the listing read path, then restore.
    channels::set_variant_price(&db, "default-channel", vid, Some(dec!(77.77)), None)
        .await
        .unwrap();
    let pricing = catalog::checkout_pricing(&db, "default-channel", &[vid])
        .await
        .unwrap();
    assert_eq!(pricing[&vid].0.amount, dec!(77.77));
    channels::set_variant_price(&db, "default-channel", vid, Some(orig_price), None)
        .await
        .unwrap();
    let pricing = catalog::checkout_pricing(&db, "default-channel", &[vid])
        .await
        .unwrap();
    assert_eq!(pricing[&vid].0.amount, orig_price);

    let err = channels::set_variant_price(&db, "default-channel", vid, Some(Decimal::new(-1, 0)), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("negative"));
}
