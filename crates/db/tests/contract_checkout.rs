//! Checkout persistence contract: rows Rust writes must be identical to
//! rows Django writes, and Django-visible behavior must hold.
//! Mirrors `saleor/checkout/tests/test_checkout.py` (creation/lines) and
//! `test_base_calculations.py` (price_override precedence).
//!
//! Requires the Saleor database (`RUSTYGOD_DATABASE_URL`).

use rust_decimal::Decimal;
use rustygod_db::{catalog, checkout_store, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn usd_variant(db: &DatabaseConnection) -> (i32, Decimal) {
    let products = catalog::list_products(db, "default-channel", None, 10_000)
        .await
        .unwrap();
    let v = products
        .iter()
        .flat_map(|p| &p.variants)
        .next()
        .expect("populatedb must have variants");
    (v.id.parse().unwrap(), v.price.amount)
}

#[tokio::test]
async fn created_row_carries_django_defaults() {
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();

    let token =
        checkout_store::create_checkout_row(&db, ch_id, &currency, "buyer@example.com")
            .await
            .unwrap();

    let (co, _) = checkout_store::load_checkout(&db, token)
        .await
        .unwrap()
        .expect("row must exist");
    assert_eq!(co.currency, "USD");
    assert_eq!(co.country, "US");
    assert_eq!(co.language_code, "en");
    assert_eq!(co.authorize_status, "none");
    assert_eq!(co.charge_status, "none");
    assert_eq!(co.email.as_deref(), Some("buyer@example.com"));

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn lines_persist_and_total_matches_listings() {
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let (vid, unit) = usd_variant(&db).await;

    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "")
        .await
        .unwrap();
    checkout_store::add_line_row(&db, token, vid, 3, unit, &currency, None)
        .await
        .unwrap();

    let (co, lines) = checkout_store::load_checkout(&db, token)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].variant_id, vid);
    assert_eq!(lines[0].quantity, 3);
    // Pre-tax parity: net == gross == 3 x unit.
    assert_eq!(lines[0].total_price_net_amount, unit * Decimal::from(3));
    assert_eq!(
        lines[0].total_price_gross_amount,
        lines[0].total_price_net_amount
    );

    let domain = checkout_store::to_domain(&co, &lines, "default-channel");
    assert_eq!(domain.total().amount, unit * Decimal::from(3));

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn price_override_wins_in_domain_total() {
    // Mirrors test_calculate_base_line_unit_price_with_custom_price.
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let (vid, _) = usd_variant(&db).await;

    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "")
        .await
        .unwrap();
    let override_price = Decimal::new(1222, 2);
    checkout_store::add_line_row(&db, token, vid, 1, Decimal::new(9999, 2), &currency, Some(override_price))
        .await
        .unwrap();

    let (co, lines) = checkout_store::load_checkout(&db, token)
        .await
        .unwrap()
        .unwrap();
    let domain = checkout_store::to_domain(&co, &lines, "default-channel");
    assert_eq!(domain.lines[0].unit_price.amount, override_price);
    assert_eq!(domain.total().amount, override_price);

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn zero_quantity_rejected_like_django_validator() {
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let (vid, unit) = usd_variant(&db).await;

    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "")
        .await
        .unwrap();
    assert!(
        checkout_store::add_line_row(&db, token, vid, 0, unit, &currency, None)
            .await
            .is_err()
    );

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn delete_removes_checkout_and_lines() {
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let (vid, unit) = usd_variant(&db).await;

    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "")
        .await
        .unwrap();
    checkout_store::add_line_row(&db, token, vid, 1, unit, &currency, None)
        .await
        .unwrap();
    checkout_store::delete_checkout_row(&db, token).await.unwrap();

    assert!(
        checkout_store::load_checkout(&db, token)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn same_variant_merges_into_one_row() {
    // Django get_line semantics: adding the same variant twice bumps
    // quantity on ONE row instead of duplicating lines.
    use rustygod_db::checkout_store::NewLine;

    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let (vid, unit) = usd_variant(&db).await;

    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "")
        .await
        .unwrap();
    let item = || NewLine { variant_id: vid, quantity: 1, unit_price: unit, price_override: None };
    checkout_store::add_lines_tx(&db, token, ch_id, &currency, &[item()])
        .await
        .unwrap();
    checkout_store::add_lines_tx(&db, token, ch_id, &currency, &[item(), item()])
        .await
        .unwrap();

    let (_, lines) = checkout_store::load_checkout(&db, token)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(lines.len(), 1, "same variant must merge into one row");
    assert_eq!(lines[0].quantity, 3);

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn denormalized_totals_refresh_for_django_readers() {
    use rustygod_db::checkout_store::NewLine;

    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let (vid, unit) = usd_variant(&db).await;

    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "")
        .await
        .unwrap();
    checkout_store::add_lines_tx(
        &db,
        token,
        ch_id,
        &currency,
        &[NewLine { variant_id: vid, quantity: 2, unit_price: unit, price_override: None }],
    )
    .await
    .unwrap();

    let (co, _) = checkout_store::load_checkout(&db, token)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(co.total_gross_amount, unit * Decimal::from(2));
    assert_eq!(co.total_net_amount, co.total_gross_amount);
    assert_eq!(co.subtotal_gross_amount, co.total_gross_amount);

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}
