//! Payment contract: idempotent creation/events, manual gateway flow,
//! guard rails, and order status refresh — against Django's tables.
//! Mirrors `saleor/payment/tests/test_transaction_item.py` flows.

use rust_decimal::Decimal;
use rustygod_db::{database_url, payments};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

async fn cleanup(db: &DatabaseConnection, id: i32) {
    use rustygod_db::entities::{payment_transactionevent, payment_transactionitem};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    payment_transactionevent::Entity::delete_many()
        .filter(payment_transactionevent::Column::TransactionId.eq(id))
        .exec(db)
        .await
        .unwrap();
    payment_transactionitem::Entity::delete_by_id(id).exec(db).await.unwrap();
}

#[tokio::test]
async fn create_is_idempotent_on_key() {
    let db = db().await;
    let new = payments::NewTransaction {
        checkout_id: None,
        order_id: None,
        currency: "USD".into(),
        name: "manual".into(),
        app_identifier: Some("rustygod-manual".into()),
        idempotency_key: Some(format!("audit-{}", uuid::Uuid::new_v4())),
        available_actions: vec!["authorize".into()],
    };
    let a = payments::create_transaction(&db, &new).await.unwrap();
    let b = payments::create_transaction(&db, &new).await.unwrap();
    assert_eq!(a.id, b.id, "same key must return the same transaction");
    cleanup(&db, a.id).await;
}

#[tokio::test]
async fn manual_authorize_charge_refund_flow() {
    let db = db().await;
    let key = uuid::Uuid::new_v4().to_string();
    let t = payments::create_transaction(
        &db,
        &payments::NewTransaction {
            checkout_id: None,
            order_id: None,
            currency: "USD".into(),
            name: "manual".into(),
            app_identifier: Some("rustygod-manual".into()),
            idempotency_key: Some(format!("flow-{key}")),
            available_actions: vec!["authorize".into()],
        },
    )
    .await
    .unwrap();

    let v = payments::authorize(&db, t.id, dec("100.00"), &format!("auth-{key}")).await.unwrap();
    assert_eq!(v.authorized, dec("100.00"));
    assert!(v.available_actions.contains(&"charge".to_string()));

    // Replay with same key: events dedupe, amounts unchanged.
    let v2 = payments::authorize(&db, t.id, dec("100.00"), &format!("auth-{key}")).await.unwrap();
    assert_eq!(v2.authorized, dec("100.00"));

    let v = payments::charge(&db, t.id, dec("60.00"), &format!("ch-{key}")).await.unwrap();
    assert_eq!(v.charged, dec("60.00"));
    assert_eq!(v.authorized, dec("40.00")); // previous-bucket move

    // Over-charge guarded.
    assert!(payments::charge(&db, t.id, dec("50.00"), &format!("ch2-{key}")).await.is_err());

    // Cancel the uncharged remainder (100 authorized - 60 charged).
    let v = payments::cancel(&db, t.id, dec("40.00"), &format!("cx-{key}")).await.unwrap();
    assert_eq!(v.canceled, dec("40.00"));
    assert_eq!(v.authorized, dec("0"));

    let v = payments::refund(&db, t.id, dec("20.00"), &format!("rf-{key}")).await.unwrap();
    assert_eq!(v.refunded, dec("20.00"));
    assert_eq!(v.charged, dec("40.00"));

    // Over-refund guarded.
    assert!(payments::refund(&db, t.id, dec("50.00"), &format!("rf2-{key}")).await.is_err());

    cleanup(&db, t.id).await;
}

#[tokio::test]
async fn order_statuses_refresh_from_coverage() {
    use rustygod_db::{catalog, checkout_store, order_store};

    let db = db().await;
    // Mint a real order through checkout, then pay it in full.
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "")
        .await
        .unwrap();
    let products = catalog::list_products(&db, "default-channel", None, 10).await.unwrap();
    let v = products.iter().flat_map(|p| &p.variants).next().unwrap();
    let vid: i32 = v.id.parse().unwrap();
    checkout_store::add_lines_tx(
        &db,
        token,
        ch_id,
        &currency,
        &[checkout_store::NewLine { variant_id: vid, quantity: 1, unit_price: v.price.amount, price_override: None }],
    )
    .await
    .unwrap();
    let (co, lines) = checkout_store::load_checkout(&db, token).await.unwrap().unwrap();
    let domain = checkout_store::to_domain(&co, &lines, "default-channel");
    let pricing = catalog::checkout_pricing(
        &db,
        "default-channel",
        &domain.lines.iter().filter_map(|l| l.variant_id.parse().ok()).collect::<Vec<i32>>(),
    )
    .await
    .unwrap();
    let order = order_store::mint_from_checkout(&db, &domain, ch_id, "default-channel", &pricing)
        .await
        .unwrap();
    checkout_store::delete_checkout_row(&db, token).await.unwrap();

    let oid: uuid::Uuid = order.id.parse().unwrap();
    let key = uuid::Uuid::new_v4().to_string();
    let t = payments::create_transaction(
        &db,
        &payments::NewTransaction {
            checkout_id: None,
            order_id: Some(oid),
            currency: order.currency.clone(),
            name: "manual".into(),
            app_identifier: Some("rustygod-manual".into()),
            idempotency_key: Some(format!("ord-{key}")),
            available_actions: vec!["authorize".into()],
        },
    )
    .await
    .unwrap();
    payments::authorize(&db, t.id, order.total.amount, &format!("a-{key}")).await.unwrap();
    payments::charge(&db, t.id, order.total.amount, &format!("c-{key}")).await.unwrap();

    // Re-read statuses with explicit projection (never SELECT * on tsvector tables).
    {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect, SelectorTrait};
        use rustygod_db::entities::order_order;
        let (auth, charge): (String, String) = order_order::Entity::find_by_id(oid)
            .select_only()
            .column(order_order::Column::AuthorizeStatus)
            .column(order_order::Column::ChargeStatus)
            .into_tuple()
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(auth, "full");
        assert_eq!(charge, "full");
    }

    cleanup(&db, t.id).await;
    use rustygod_db::entities::{order_orderline, order_order as oo};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    order_orderline::Entity::delete_many()
        .filter(order_orderline::Column::OrderId.eq(oid))
        .exec(&db)
        .await
        .unwrap();
    oo::Entity::delete_by_id(oid).exec(&db).await.unwrap();
}
