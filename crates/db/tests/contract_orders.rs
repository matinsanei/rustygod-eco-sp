//! Order persistence contract: Rust-minted orders must live in Django's
//! tables with Django's values. Mirrors
//! `saleor/order/tests/test_order.py` (creation/numbers) and
//! `saleor/checkout/tests/test_order_from_checkout.py`.
//!
//! Requires the Saleor database (`RUSTIFY_DATABASE_URL`).

use rust_decimal::Decimal;
use saleor_rustify_db::{catalog, checkout_store, database_url, order_store};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn checkout_with_line(
    db: &DatabaseConnection,
) -> saleor_rustify_core::checkout::Checkout {
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    let products = catalog::list_products(db, "default-channel", None, 10_000)
        .await
        .unwrap();
    let v = products.iter().flat_map(|p| &p.variants).next().unwrap();
    let vid: i32 = v.id.parse().unwrap();

    let token = checkout_store::create_checkout_row(db, ch_id, &currency, "buyer@example.com")
        .await
        .unwrap();
    checkout_store::add_line_row(db, token, vid, 2, v.price.amount, &currency, None)
        .await
        .unwrap();
    let (co, lines) = checkout_store::load_checkout(db, token).await.unwrap().unwrap();
    let domain = checkout_store::to_domain(&co, &lines, "default-channel");
    checkout_store::delete_checkout_row(db, token).await.unwrap();
    domain
}

#[tokio::test]
async fn minted_order_lives_in_django_tables() {
    let db = db().await;
    let co = checkout_with_line(&db).await;
    let (ch_id, _) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let pricing = catalog::checkout_pricing(
        &db,
        "default-channel",
        &co.lines.iter().filter_map(|l| l.variant_id.parse().ok()).collect::<Vec<i32>>(),
    )
    .await
    .unwrap();

    let order = order_store::mint_from_checkout(&db, &co, ch_id, "default-channel", &pricing)
        .await
        .unwrap();

    // Django-visible assertions straight from its tables.
    let oid: uuid::Uuid = order.id.parse().unwrap();
    let (header, lines) = order_store::get_order_rows(&db, oid)
        .await
        .unwrap()
        .expect("order row must exist");
    assert_eq!(header.status, "unfulfilled");
    assert_eq!(header.currency, co.currency);
    assert_eq!(header.total_gross_amount, co.total().amount);
    assert_eq!(lines.len(), co.lines.len());
    assert_eq!(lines[0].quantity, 2);
    assert_eq!(lines[0].quantity_fulfilled, 0);
    assert_eq!(lines[0].unit_price_net_amount, lines[0].unit_price_gross_amount);
    // Human number comes from Django's own sequence.
    assert!(header.number > 0);
    assert_eq!(order.number, header.number.to_string());

    // Cleanup (test rows only).
    use saleor_rustify_db::entities::{order_order, order_orderline};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    order_orderline::Entity::delete_many()
        .filter(order_orderline::Column::OrderId.eq(oid))
        .exec(&db)
        .await
        .unwrap();
    order_order::Entity::delete_by_id(oid).exec(&db).await.unwrap();
}

#[tokio::test]
async fn order_numbers_come_from_django_sequence() {
    // Mirrors get_order_number(): two mints get distinct increasing numbers.
    let db = db().await;
    let n1 = order_store::next_number(&db).await.unwrap();
    let n2 = order_store::next_number(&db).await.unwrap();
    assert!(n2 > n1, "sequence must advance like Django's");
}

#[tokio::test]
async fn django_status_strings_round_trip() {
    // "partially fulfilled" has a SPACE in Django — the mapping must hold.
    use saleor_rustify_core::order::OrderStatus;
    assert_eq!(OrderStatus::PartiallyFulfilled.as_str(), "partially fulfilled");
    assert_eq!(
        OrderStatus::from_str("partially fulfilled"),
        OrderStatus::PartiallyFulfilled
    );
    assert_eq!(OrderStatus::from_str("unfulfilled"), OrderStatus::Unfulfilled);
    let _ = Decimal::ZERO;
}
