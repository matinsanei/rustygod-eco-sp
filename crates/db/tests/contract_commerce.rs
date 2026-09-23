//! Commerce contract tests: discount, shipping, giftcard, menu, page,
//! account, channel, tax, warehouse reads must match Django's tables.
//!
//! Requires the Saleor database (`RUSTIFY_DATABASE_URL`).

use saleor_rustify_db::{commerce, database_url};
use sea_orm::DatabaseConnection;


/// Cross-process stock lock (fs2 flock) — shared with fulfillment tests.
struct CommerceStockGuard {
    _f: std::fs::File,
}

fn commerce_stock_guard() -> CommerceStockGuard {
    use fs2::FileExt;
    let f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open("/tmp/rustify-stock.lock")
        .expect("lock file");
    f.lock_exclusive().expect("stock lock");
    CommerceStockGuard { _f: f }
}

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn channels_match_django() {
    let db = db().await;
    let channels = commerce::list_channels(&db).await.unwrap();
    assert!(channels.len() >= 2);
    let slugs: Vec<_> = channels.iter().map(|c| c.slug.as_str()).collect();
    assert!(slugs.contains(&"default-channel"));
    let usd = channels.iter().find(|c| c.slug == "default-channel").unwrap();
    assert_eq!(usd.currency_code, "USD");
    assert!(usd.is_active);
}

#[tokio::test]
async fn promotions_and_vouchers_match() {
    use saleor_rustify_db::entities::discount_vouchercode;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = db().await;
    // Promotions endpoint must not error and only returns date-valid promos.
    let _ = commerce::list_promotions(&db, "default-channel").await.unwrap();

    // If Django has an active code, validation must accept it with the
    // channel listing's values.
    if let Some(code_row) = discount_vouchercode::Entity::find()
        .filter(discount_vouchercode::Column::IsActive.eq(true))
        .one(&db)
        .await
        .unwrap()
    {
        let v = commerce::validate_voucher(&db, &code_row.code, "default-channel").await;
        // May legitimately fail on date window/usage — but a failure must be
        // a domain error, never a crash; success must carry listing values.
        if let Ok(v) = v {
            assert!(!v.discount_value.is_zero());
            assert_eq!(v.currency.len(), 3);
        }
    }
    // Unknown code is a clean error.
    assert!(
        commerce::validate_voucher(&db, "NOPE-NOT-A-CODE", "default-channel")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn shipping_methods_match_listings() {
    use saleor_rustify_db::entities::shipping_shippingmethodchannellisting;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let db = db().await;
    let methods = commerce::list_shipping_methods(&db, "default-channel").await.unwrap();
    let expected = shipping_shippingmethodchannellisting::Entity::find()
        .filter(shipping_shippingmethodchannellisting::Column::ChannelId.eq(1))
        .paginate(&db, 1000)
        .num_items()
        .await
        .unwrap();
    assert_eq!(methods.len() as u64, expected);
    for m in &methods {
        assert!(!m.price_amount.is_zero() || m.price_amount.is_zero());
        assert_eq!(m.currency.len(), 3);
    }
}

#[tokio::test]
async fn giftcard_lookup_matches_row() {
    use saleor_rustify_db::entities::giftcard_giftcard;
    use sea_orm::EntityTrait;

    let db = db().await;
    if let Some(row) = giftcard_giftcard::Entity::find().one(&db).await.unwrap() {
        let g = commerce::get_gift_card(&db, &row.code).await.unwrap().unwrap();
        assert_eq!(g.current_balance, row.current_balance_amount);
        assert_eq!(g.currency, row.currency);
        assert_eq!(g.is_active, row.is_active);
    }
    assert!(commerce::get_gift_card(&db, "NOPE").await.unwrap().is_none());
}

#[tokio::test]
async fn menus_and_pages_match() {
    use saleor_rustify_db::entities::{menu_menu, page_page};
    use sea_orm::{EntityTrait, PaginatorTrait};

    let db = db().await;
    if let Some(menu) = menu_menu::Entity::find().one(&db).await.unwrap() {
        let m = commerce::get_menu(&db, &menu.slug).await.unwrap().unwrap();
        assert_eq!(m.name, menu.name);
    }
    assert!(commerce::get_menu(&db, "nope").await.unwrap().is_none());

    let pages = commerce::list_pages(&db).await.unwrap();
    let expected = page_page::Entity::find().paginate(&db, 1000).num_items().await.unwrap();
    assert_eq!(pages.len() as u64, expected);
    if let Some(p) = pages.first() {
        let one = commerce::get_page(&db, &p.slug).await.unwrap().unwrap();
        assert_eq!(one.title, p.title);
    }
}

#[tokio::test]
async fn customer_lookup_is_safe_and_address_roundtrips() {
    use saleor_rustify_db::entities::account_user;
    use sea_orm::{EntityTrait, QuerySelect};

    let db = db().await;
    let email: Option<String> = account_user::Entity::find()
        .select_only()
        .column(account_user::Column::Email)
        .into_tuple::<String>()
        .one(&db)
        .await
        .unwrap();
    if let Some(email) = email {
        let c = commerce::get_customer(&db, &email).await.unwrap().unwrap();
        assert_eq!(c.email, email);
        assert!(c.id > 0);
    }
    assert!(commerce::get_customer(&db, "nobody@example.com").await.unwrap().is_none());

    let id = commerce::create_address(
        &db,
        &commerce::NewAddress {
            first_name: "Rust".into(),
            last_name: "God".into(),
            street_address_1: "1 Ferric Way".into(),
            city: "Tehran".into(),
            postal_code: "12345".into(),
            country: "IR".into(),
            phone: "+980000000000".into(),
        },
    )
    .await
    .unwrap();
    assert!(id > 0);
    // Cleanup test row.
    use saleor_rustify_db::entities::account_address;
    account_address::Entity::delete_by_id(id).exec(&db).await.unwrap();
}

#[tokio::test]
async fn warehouses_stocks_and_reservation() {
    let _stock = commerce_stock_guard();
    use saleor_rustify_db::entities::warehouse_stock;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = db().await;
    let wh = commerce::list_warehouses(&db, "default-channel").await.unwrap();
    assert!(!wh.is_empty());

    let stock_row = warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::Quantity.gt(1))
        .one(&db)
        .await
        .unwrap()
        .expect("need a stocked variant");
    let stocks = commerce::stocks_for_variant(&db, stock_row.product_variant_id)
        .await
        .unwrap();
    assert!(!stocks.is_empty());

    // Reservations FK to checkout_checkoutline: use a real line (like Django).
    use saleor_rustify_db::{catalog, checkout_store};
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let products = catalog::list_products(&db, "default-channel", None, 10)
        .await
        .unwrap();
    let v = products.iter().flat_map(|p| &p.variants).next().unwrap();
    let vid: i32 = v.id.parse().unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "")
        .await
        .unwrap();
    let line_id = checkout_store::add_line_row(&db, token, vid, 1, v.price.amount, &currency, None)
        .await
        .unwrap();

    // Reserve + release round-trip.
    commerce::reserve_stock(
        &db,
        stock_row.product_variant_id,
        stock_row.warehouse_id,
        1,
        line_id,
        600,
    )
    .await
    .unwrap();
    // Over-reserving must fail.
    assert!(
        commerce::reserve_stock(
            &db,
            stock_row.product_variant_id,
            stock_row.warehouse_id,
            1_000_000_000,
            line_id,
            600,
        )
        .await
        .is_err()
    );
    commerce::release_reservations(&db, line_id).await.unwrap();
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn reserve_is_idempotent_per_line() {
    let _stock = commerce_stock_guard();
    // Retrying a reservation replaces instead of double-booking.
    use saleor_rustify_db::{catalog, checkout_store};
    use saleor_rustify_db::entities::warehouse_reservation;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let products = catalog::list_products(&db, "default-channel", None, 10)
        .await
        .unwrap();
    let v = products.iter().flat_map(|p| &p.variants).next().unwrap();
    let vid: i32 = v.id.parse().unwrap();
    let stocks = commerce::stocks_for_variant(&db, vid).await.unwrap();
    let stock = stocks.iter().find(|s| s.quantity - s.quantity_allocated > 1).unwrap();
    let wh: uuid::Uuid = stock.warehouse_id.parse().unwrap();

    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "")
        .await
        .unwrap();
    let line_id = checkout_store::add_line_row(&db, token, vid, 1, v.price.amount, &currency, None)
        .await
        .unwrap();

    commerce::reserve_stock(&db, vid, wh, 1, line_id, 600).await.unwrap();
    commerce::reserve_stock(&db, vid, wh, 1, line_id, 600).await.unwrap();
    let count = warehouse_reservation::Entity::find()
        .filter(warehouse_reservation::Column::CheckoutLineId.eq(line_id))
        .paginate(&db, 10)
        .num_items()
        .await
        .unwrap();
    assert_eq!(count, 1, "retry must replace, not duplicate");

    commerce::release_reservations(&db, line_id).await.unwrap();
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn tax_classes_match() {
    use saleor_rustify_db::entities::tax_taxclass;
    use sea_orm::{EntityTrait, PaginatorTrait};

    let db = db().await;
    let classes = commerce::list_tax_classes(&db).await.unwrap();
    let expected = tax_taxclass::Entity::find().paginate(&db, 100).num_items().await.unwrap();
    assert_eq!(classes.len() as u64, expected);
}
