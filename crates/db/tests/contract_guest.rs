use rustygod_db::{catalog, checkout_store, complete, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url()).await.unwrap()
}

#[tokio::test]
async fn guest_email_links_to_existing_user_on_complete() {
    let db = db().await;
    // Create checkout as guest with admin email (existing user).
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "admin@example.com").await.unwrap();
    // Add a stocked line.
    let products = catalog::list_products(&db, "default-channel", None, 100).await.unwrap();
    let vid: i32 = products.iter().flat_map(|p| &p.variants).find(|v| v.quantity_available >= 1).unwrap().id.parse().unwrap();
    let pricing = catalog::checkout_pricing(&db, "default-channel", &[vid]).await.unwrap();
    checkout_store::add_line_row(&db, token, vid, 1, pricing[&vid].0.amount, &currency, None).await.unwrap();
    let out = complete::complete_checkout(&db, token).await.unwrap();
    let (header, _) = rustygod_db::order_store::get_order_rows(&db, out.order_id).await.unwrap().unwrap();
    // The order should have user_id of admin (id 1-ish) – at least not None, and email matches.
    assert_eq!(header.user_email, "admin@example.com");
    // Check order's user_id is linked (if DB has admin user).
    let order_user: Option<i32> = {
        use sea_orm::{EntityTrait, QuerySelect};
        rustygod_db::entities::order_order::Entity::find_by_id(out.order_id)
            .select_only().column(rustygod_db::entities::order_order::Column::UserId)
            .into_tuple::<Option<i32>>().one(&db).await.unwrap().flatten()
    };
    assert!(order_user.is_some(), "guest with existing email should link to user");
}

#[tokio::test]
async fn authenticated_checkout_links_user_and_addresses() {
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "auth@example.com").await.unwrap();
    // Set checkout user_id to admin (simulate authenticated checkout).
    {
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
        let co = rustygod_db::entities::checkout_checkout::Entity::find_by_id(token).one(&db).await.unwrap().unwrap();
        let mut am: rustygod_db::entities::checkout_checkout::ActiveModel = co.into();
        // Find admin user id
        let admin = rustygod_db::entities::account_user::Entity::find()
            .filter(rustygod_db::entities::account_user::Column::Email.eq("admin@example.com"))
            .one(&db).await.unwrap().unwrap();
        am.user_id = Set(Some(admin.id));
        // Create an address and set as billing
        let addr_id = rustygod_db::commerce::create_address(&db, &rustygod_db::commerce::NewAddress {
            first_name: "Test".into(), last_name: "User".into(), street_address_1: "123 St".into(),
            city: "Warsaw".into(), postal_code: "00-001".into(), country: "PL".into(), phone: "".into(),
        }).await.unwrap();
        am.billing_address_id = Set(Some(addr_id));
        am.update(&db).await.unwrap();
    }
    let products = catalog::list_products(&db, "default-channel", None, 100).await.unwrap();
    let vid: i32 = products.iter().flat_map(|p| &p.variants).find(|v| v.quantity_available >= 1).unwrap().id.parse().unwrap();
    let pricing = catalog::checkout_pricing(&db, "default-channel", &[vid]).await.unwrap();
    checkout_store::add_line_row(&db, token, vid, 1, pricing[&vid].0.amount, &currency, None).await.unwrap();
    let out = complete::complete_checkout(&db, token).await.unwrap();
    let order_user: Option<i32> = {
        use sea_orm::{EntityTrait, QuerySelect};
        rustygod_db::entities::order_order::Entity::find_by_id(out.order_id)
            .select_only().column(rustygod_db::entities::order_order::Column::UserId)
            .into_tuple::<Option<i32>>().one(&db).await.unwrap().flatten()
    };
    assert!(order_user.is_some(), "authenticated checkout should link user");
    let order_billing: Option<i32> = {
        use sea_orm::{EntityTrait, QuerySelect};
        rustygod_db::entities::order_order::Entity::find_by_id(out.order_id)
            .select_only().column(rustygod_db::entities::order_order::Column::BillingAddressId)
            .into_tuple::<Option<i32>>().one(&db).await.unwrap().flatten()
    };
    assert!(order_billing.is_some(), "billing address should be carried");
}
