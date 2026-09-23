//! Real-money contract: update-deltas, event-report idempotency,
//! request-action execution, legacy capture/refund/void guards.

use rust_decimal::Decimal;
use saleor_rustify_db::{database_url, payments::*};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn fresh_transaction(db: &DatabaseConnection) -> i32 {
    // A live checkout to hang the transaction on.
    use saleor_rustify_db::{catalog, checkout_store};
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(db, ch_id, &currency, "money@example.com").await.unwrap();
    let t = create_transaction(
        db,
        &NewTransaction {
            checkout_id: Some(token),
            order_id: None,
            currency,
            name: "money cover".into(),
            app_identifier: None,
            idempotency_key: Some(format!("cover-{}", uuid::Uuid::new_v4())),
            available_actions: vec!["charge".into(), "refund".into(), "cancel".into()],
        },
    )
    .await
    .unwrap();
    t.id
}

#[tokio::test]
async fn full_lifecycle_authorize_charge_refund() {
    let db = db().await;
    let tid = fresh_transaction(&db).await;
    let key = |s: &str| format!("lc-{}-{s}", uuid::Uuid::new_v4());

    // Authorize 100, charge 60, refund 20.
    let v = authorize(&db, tid, Decimal::new(100, 0), &key("a")).await.unwrap();
    assert_eq!(v.authorized, Decimal::new(100, 0));
    let v = charge(&db, tid, Decimal::new(60, 0), &key("c")).await.unwrap();
    assert_eq!(v.charged, Decimal::new(60, 0));
    let v = refund(&db, tid, Decimal::new(20, 0), &key("r")).await.unwrap();
    assert_eq!(v.refunded, Decimal::new(20, 0));

    // Over-charge / over-refund rejected by guards.
    assert!(charge(&db, tid, Decimal::new(61, 0), &key("c2")).await.is_err());
    assert!(refund(&db, tid, Decimal::new(41, 0), &key("r2")).await.is_err());

    // Update: raise authorized absolutely via ADJUSTMENT. Django-identical
    // math: the adjustment cuts the old success and assigns 150, but the
    // live charge(60) still moves out of authorized → 90.
    let v = apply_amount_targets(
        &db,
        tid,
        &AmountTargets { authorized: Some(Decimal::new(150, 0)), ..Default::default() },
        "USD",
        &key("u"),
    )
    .await
    .unwrap();
    assert_eq!(v.authorized, Decimal::new(90, 0));
    // Update: charged delta via SUCCESS event.
    let v = apply_amount_targets(
        &db,
        tid,
        &AmountTargets { charged: Some(Decimal::new(80, 0)), ..Default::default() },
        "USD",
        &key("u2"),
    )
    .await
    .unwrap();
    assert_eq!(v.charged, Decimal::new(80, 0));
    // Decreases rejected (use refund/void flows).
    assert!(apply_amount_targets(
        &db,
        tid,
        &AmountTargets { charged: Some(Decimal::new(10, 0)), ..Default::default() },
        "USD",
        &key("u3"),
    )
    .await
    .is_err());
    // Wrong currency rejected.
    assert!(apply_amount_targets(
        &db,
        tid,
        &AmountTargets { charged: Some(Decimal::new(90, 0)), ..Default::default() },
        "EUR",
        &key("u4"),
    )
    .await
    .is_err());
}

#[tokio::test]
async fn event_report_idempotent_and_recounting() {
    use saleor_rustify_core::psp::ManualPsp;
    let db = db().await;
    let tid = fresh_transaction(&db).await;
    authorize(&db, tid, Decimal::new(100, 0), &format!("rep-a-{}", uuid::Uuid::new_v4())).await.unwrap();
    // External PSP-style report: charge 42 with its own reference.
    let key = format!("rep-{}", uuid::Uuid::new_v4());
    let v = execute_via(
        &db,
        tid,
        saleor_rustify_core::psp::PspAction::Charge,
        Decimal::new(42, 0),
        &key,
        &ManualPsp,
        None,
        Some("{\"psp_reference\": \"psp-42\"}"),
    )
    .await
    .unwrap()
    .txn;
    assert_eq!(v.charged, Decimal::new(42, 0));
    // Same idempotency key replays without duplicating.
    let v2 = execute_via(
        &db,
        tid,
        saleor_rustify_core::psp::PspAction::Charge,
        Decimal::new(42, 0),
        &key,
        &ManualPsp,
        None,
        Some("{\"psp_reference\": \"psp-42\"}"),
    )
    .await
    .unwrap()
    .txn;
    assert_eq!(v2.charged, Decimal::new(42, 0));
}

#[tokio::test]
async fn request_action_routes_and_guards() {
    use saleor_rustify_core::psp::{ManualPsp, PspAction};
    let db = db().await;
    let tid = fresh_transaction(&db).await;
    let key = |s: &str| format!("ra-{}-{s}", uuid::Uuid::new_v4());
    authorize(&db, tid, Decimal::new(200, 0), &key("a")).await.unwrap();
    // Charge via request_action through the manual PSP.
    let v = request_action(&db, tid, PspAction::Charge, Decimal::new(50, 0), &key("c"), &ManualPsp, None).await.unwrap();
    assert_eq!(v.charged, Decimal::new(50, 0));
    // Refund more than charged fails.
    assert!(request_action(&db, tid, PspAction::Refund, Decimal::new(999, 0), &key("r"), &ManualPsp, None).await.is_err());
    // Cancel works (no amount needed).
    let v = request_action(&db, tid, PspAction::Cancel, Decimal::ZERO, &key("x"), &ManualPsp, Some("buyer changed mind".into())).await.unwrap();
    assert!(v.canceled >= Decimal::ZERO);
}

#[tokio::test]
async fn legacy_capture_refund_void_guards() {
    let db = db().await;
    // Seed a legacy payment row straight into payment_payment.
    use sea_orm::{ActiveModelTrait, Set};
    use saleor_rustify_db::entities::payment_payment;
    let pid = payment_payment::ActiveModel {
        gateway: Set("manual".into()),
        is_active: Set(true),
        charge_status: Set("not-charged".into()),
        currency: Set("USD".into()),
        total: Set(Decimal::new(100, 0)),
        captured_amount: Set(Decimal::ZERO),
        token: Set(uuid::Uuid::new_v4().to_string()),
        extra_data: Set("{}".into()),
        billing_first_name: Set("T".into()),
        billing_last_name: Set("E".into()),
        billing_company_name: Set(String::new()),
        billing_address_1: Set("s".into()),
        billing_address_2: Set(String::new()),
        billing_city: Set("c".into()),
        billing_city_area: Set(String::new()),
        billing_postal_code: Set("1".into()),
        billing_country_code: Set("US".into()),
        billing_country_area: Set(String::new()),
        billing_email: Set("t@e.com".into()),
        cc_brand: Set(String::new()),
        cc_first_digits: Set(String::new()),
        cc_last_digits: Set(String::new()),
        created_at: Set(chrono::Utc::now().into()),
        modified_at: Set(chrono::Utc::now().into()),
        to_confirm: Set(false),
        payment_method_type: Set(String::new()),
        store_payment_method: Set("NONE".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap()
    .id;
    // Over-capture rejected; exact capture works.
    assert!(capture_legacy_payment(&db, pid, Decimal::new(101, 0)).await.is_err());
    let v = capture_legacy_payment(&db, pid, Decimal::new(100, 0)).await.unwrap();
    assert_eq!(v.captured, Decimal::new(100, 0));
    assert_eq!(v.charge_status, "fully-charged");
    // Void after capture rejected; refund over captured rejected.
    assert!(void_legacy_payment(&db, pid).await.is_err());
    assert!(refund_legacy_payment(&db, pid, Decimal::new(101, 0)).await.is_err());
    let v = refund_legacy_payment(&db, pid, Decimal::new(30, 0)).await.unwrap();
    assert_eq!(v.refunded, Decimal::new(30, 0));
    assert_eq!(v.charge_status, "partially-refunded");
    let v = refund_legacy_payment(&db, pid, Decimal::new(70, 0)).await.unwrap();
    assert_eq!(v.charge_status, "fully-refunded");
    // Cumulative guard: 30+70=100 captured, one more must fail.
    assert!(refund_legacy_payment(&db, pid, Decimal::new(1, 0)).await.is_err());
    // Cleanup.
    use sea_orm::EntityTrait;
    payment_payment::Entity::delete_by_id(pid).exec(&db).await.unwrap();
}

#[tokio::test]
async fn scalar_patch_validates() {
    let db = db().await;
    let tid = fresh_transaction(&db).await;
    update_transaction_scalars(
        &db,
        tid,
        &ItemPatch { name: Some("renamed".into()), available_actions: Some(vec!["charge".into(), "charge".into()]), ..Default::default() },
    )
    .await
    .unwrap();
    // psp uniqueness enforced across items.
    let tid2 = fresh_transaction(&db).await;
    let uniq = format!("uniq-{}", uuid::Uuid::new_v4());
    update_transaction_scalars(&db, tid, &ItemPatch { psp_reference: Some(uniq.clone()), ..Default::default() }).await.unwrap();
    assert!(update_transaction_scalars(&db, tid2, &ItemPatch { psp_reference: Some(uniq), ..Default::default() }).await.is_err());
}

#[tokio::test]
async fn initialize_then_process_manual_flow() {
    use saleor_rustify_db::{catalog, checkout_store};
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "init@example.com").await.unwrap();
    let products = catalog::list_products(&db, "default-channel", None, 100).await.unwrap();
    let vid: i32 = products.iter().flat_map(|p| &p.variants)
        .find(|v| v.quantity_available >= 1).unwrap().id.parse().unwrap();
    let pricing = catalog::checkout_pricing(&db, "default-channel", &[vid]).await.unwrap();
    checkout_store::add_line_row(&db, token, vid, 1, pricing[&vid].0.amount, &currency, None).await.unwrap();
    checkout_store::refresh_totals(&db, token).await.unwrap();

    // Initialize: REQUEST event only, buckets untouched.
    let t = initialize_transaction(
        &db,
        &InitSpec {
            checkout_id: Some(token),
            order_id: None,
            currency: currency.clone(),
            name: "init cover".into(),
            app_identifier: Some("manual".into()),
            idempotency_key: Some(format!("init-{}", uuid::Uuid::new_v4())),
            available_actions: vec!["charge".into(), "cancel".into()],
            charge_flow: false,
        },
        pricing[&vid].0.amount,
    )
    .await
    .unwrap();
    assert_eq!(t.authorized, Decimal::ZERO);
    // Outstanding = total, next = authorize.
    let (rest, next, cur) = outstanding(&db, t.id).await.unwrap();
    assert_eq!(cur, currency);
    assert!(rest > Decimal::ZERO);
    assert!(matches!(next, saleor_rustify_core::psp::PspAction::Authorize));
    // Process the authorization through manual PSP.
    use saleor_rustify_core::psp::ManualPsp;
    let v = request_action(&db, t.id, saleor_rustify_core::psp::PspAction::Authorize, rest, &format!("proc-{}", uuid::Uuid::new_v4()), &ManualPsp, None).await.unwrap();
    assert_eq!(v.authorized, rest);
    // Then the charge remainder.
    let (rest2, next2, _) = outstanding(&db, t.id).await.unwrap();
    assert!(matches!(next2, saleor_rustify_core::psp::PspAction::Charge));
    let v = request_action(&db, t.id, saleor_rustify_core::psp::PspAction::Charge, rest2, &format!("proc2-{}", uuid::Uuid::new_v4()), &ManualPsp, None).await.unwrap();
    assert_eq!(v.charged, rest2);
    // Fully processed now.
    assert!(outstanding(&db, t.id).await.is_err());
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}
