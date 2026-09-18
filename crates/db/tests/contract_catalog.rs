//! Comparative contract tests: the Rust catalog must return **the same rows
//! Django wrote**. These run against the live Saleor PostgreSQL (`populatedb`
//! data) and assert behavioral parity with:
//! - `saleor/product/tests/test_product_availability.py` (published listings)
//! - `saleor/checkout/tests/test_base_calculations.py` (unit price == variant
//!   channel price)
//!
//! Requires the Saleor database: `RUSTYGOD_DATABASE_URL`
//! (default `postgres://saleor:saleor@localhost:5434/saleor`).

use rustygod_db::{catalog, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn published_count_matches_django_listings() {
    use rustygod_db::entities::product_productchannellisting;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let db = db().await;
    let products = catalog::list_products(&db, "default-channel", None, 10_000)
        .await
        .unwrap();

    // Ground truth straight from Django's tables (id-only select: the table
    // has an INTERVAL column codegen mistypes).
    let ch_id: i32 = {
        use sea_orm::{QuerySelect, SelectorTrait};
        rustygod_db::entities::channel_channel::Entity::find()
            .select_only()
            .column(rustygod_db::entities::channel_channel::Column::Id)
            .filter(
                rustygod_db::entities::channel_channel::Column::Slug.eq("default-channel"),
            )
            .into_tuple::<i32>()
            .one(&db)
            .await
            .unwrap()
            .unwrap()
    };
    let expected = product_productchannellisting::Entity::find()
        .filter(product_productchannellisting::Column::ChannelId.eq(ch_id))
        .filter(product_productchannellisting::Column::IsPublished.eq(true))
        .paginate(&db, 10_000)
        .num_items()
        .await
        .unwrap();

    assert_eq!(
        products.len() as u64,
        expected,
        "rust catalog must expose exactly the published listings Django has"
    );
    assert!(!products.is_empty(), "populatedb catalog must be non-empty");
}

#[tokio::test]
async fn variant_prices_match_channel_listings() {
    use rustygod_db::entities::product_productvariantchannellisting;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = db().await;
    let products = catalog::list_products(&db, "default-channel", None, 10_000)
        .await
        .unwrap();

    let mut checked = 0;
    for p in &products {
        for v in &p.variants {
            let vid: i32 = v.id.parse().unwrap();
            let row = product_productvariantchannellisting::Entity::find()
                .filter(product_productvariantchannellisting::Column::VariantId.eq(vid))
                .filter(
                    product_productvariantchannellisting::Column::Currency.eq("USD"),
                )
                .one(&db)
                .await
                .unwrap()
                .expect("variant must have a USD channel listing");
            // Mirrors: unit_price == variant.get_price(channel_listing)
            assert_eq!(
                v.price.amount,
                row.price_amount.unwrap(),
                "variant {vid} price must equal Django's channel listing"
            );
            assert_eq!(v.price.currency, "USD");
            checked += 1;
        }
    }
    assert!(checked > 0, "must verify at least one variant price");
}

#[tokio::test]
async fn unpublished_products_are_hidden() {
    use rustygod_db::entities::{product_product, product_productchannellisting};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = db().await;
    // Any product Django marked unpublished on default-channel...
    let hidden = product_productchannellisting::Entity::find()
        .filter(product_productchannellisting::Column::ChannelId.eq(1))
        .filter(product_productchannellisting::Column::IsPublished.eq(false))
        .one(&db)
        .await
        .unwrap();

    if let Some(row) = hidden {
        let got = catalog::get_product(&db, "default-channel", row.product_id).await.unwrap();
        assert!(
            got.is_none(),
            "unpublished product {} must be hidden like Saleor hides it",
            row.product_id
        );
    } else {
        // ...otherwise at least a missing id resolves to None (no panic, no row).
        assert!(
            catalog::get_product(&db, "default-channel", -1)
                .await
                .unwrap()
                .is_none()
        );
    }
    let _ = product_product::Entity;
}

#[tokio::test]
async fn stock_never_negative() {
    let db = db().await;
    let products = catalog::list_products(&db, "default-channel", None, 10_000)
        .await
        .unwrap();
    for p in &products {
        for v in &p.variants {
            assert!(
                v.quantity_available >= 0,
                "available stock must never go below zero"
            );
        }
    }
}
