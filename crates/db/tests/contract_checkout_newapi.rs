//! New-checkout-API row writers: lines update/delete, setters, promo remove.

use rustygod_db::{catalog, checkout_store, database_url};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn stocked(db: &DatabaseConnection) -> (Uuid, i32, rust_decimal::Decimal) {
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(db, ch_id, &currency, "newapi@example.com").await.unwrap();
    let products = catalog::list_products(db, "default-channel", None, 100).await.unwrap();
    let vid: i32 = products.iter().flat_map(|p| &p.variants)
        .find(|v| v.quantity_available >= 2).unwrap().id.parse().unwrap();
    let pricing = catalog::checkout_pricing(db, "default-channel", &[vid]).await.unwrap();
    let unit = pricing[&vid].0.amount;
    (token, vid, unit)
}

#[tokio::test]
async fn lines_update_delete_and_setters() {
    let db = db().await;
    let (token, vid, unit) = stocked(&db).await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    checkout_store::add_line_row(&db, token, vid, 2, unit, &currency, None).await.unwrap();
    let (_, lines) = checkout_store::load_checkout(&db, token).await.unwrap().unwrap();
    let lid = lines[0].id;

    // Update qty; zero deletes.
    checkout_store::update_line_quantity(&db, token, lid, 5).await.unwrap();
    let (_, lines) = checkout_store::load_checkout(&db, token).await.unwrap().unwrap();
    assert_eq!(lines.iter().find(|l| l.id == lid).unwrap().quantity, 5);
    checkout_store::update_line_quantity(&db, token, lid, 0).await.unwrap();
    let (_, lines) = checkout_store::load_checkout(&db, token).await.unwrap().unwrap();
    assert!(lines.iter().all(|l| l.id != lid));

    // Missing line update errors (unknown line).
    assert!(checkout_store::update_line_quantity(&db, token, Uuid::new_v4(), 3).await.is_err());
    // Delete is idempotent.
    checkout_store::delete_line(&db, token, Uuid::new_v4()).await.unwrap();

    // Scalar setters.
    checkout_store::set_email_opt(&db, token, Some("n@e.com".into())).await.unwrap();
    checkout_store::set_note_text(&db, token, "leave at door".into()).await.unwrap();
    checkout_store::set_language_code(&db, token, "de".into()).await.unwrap();
    checkout_store::set_customer_opt(&db, token, None).await.unwrap();
    checkout_store::set_shipping_method_opt(&db, token, None).await.unwrap();
    let (co, _) = checkout_store::load_checkout(&db, token).await.unwrap().unwrap();
    assert_eq!(co.email.as_deref(), Some("n@e.com"));
    assert_eq!(co.note, "leave at door");
    assert_eq!(co.language_code, "de");

    // Addresses.
    let aid = checkout_store::set_checkout_address(&db, token, false, "F", "L", "1 Main", "Town", "123", "US").await.unwrap();
    assert!(aid > 0);

    // Promo remove with no code: false, no error.
    assert!(!checkout_store::remove_voucher_code(&db, token, None).await.unwrap());
    let _ = (ch_id, currency);
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}
