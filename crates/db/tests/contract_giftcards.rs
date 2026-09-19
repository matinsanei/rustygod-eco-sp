//! Gift-card contract vs Django's `giftcard_*` tables.
//! Mirrors `saleor/giftcard/tests/`: issue/attach/redeem/refund flows,
//! the `active(date)` manager filter, usage restrictions, and event rows.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rustygod_db::{catalog, checkout_store, database_url, giftcards};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn issue_input(balance: Decimal) -> giftcards::IssueInput {
    giftcards::IssueInput {
        initial_balance: balance,
        currency: "USD".to_string(),
        created_by_email: Some("staff@example.com".to_string()),
        expiry_date: None,
        is_active: true,
    }
}

async fn events_of(db: &DatabaseConnection, card_id: i32) -> Vec<String> {
    use rustygod_db::entities::giftcard_giftcardevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    giftcard_giftcardevent::Entity::find()
        .filter(giftcard_giftcardevent::Column::GiftCardId.eq(card_id))
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.r#type)
        .collect()
}

#[tokio::test]
async fn issue_mints_unique_dashed_code_and_issued_event() {
    let db = db().await;
    let card = giftcards::issue(&db, issue_input(dec!(100)), None)
        .await
        .unwrap();
    assert!(rustygod_core::giftcard::is_valid_code_shape(&card.code), "{}", card.code);
    assert_eq!(card.current_balance_amount, dec!(100));
    assert_eq!(card.initial_balance_amount, dec!(100));
    assert_eq!(card.currency, "USD");
    assert!(card.is_active);

    let again = giftcards::get_by_code(&db, &card.code).await.unwrap().unwrap();
    assert_eq!(again.id, card.id);

    let events = events_of(&db, card.id).await;
    assert_eq!(events, vec![giftcards::events::ISSUED]);
}

#[tokio::test]
async fn issue_rejects_negative_balance() {
    let db = db().await;
    let err = giftcards::issue(&db, issue_input(dec!(-1)), None).await.unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));
}

#[tokio::test]
async fn attach_detach_roundtrip_with_balance_sum() {
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "buyer@example.com")
        .await
        .unwrap();

    let c1 = giftcards::issue(&db, issue_input(dec!(60)), None).await.unwrap();
    let c2 = giftcards::issue(&db, issue_input(dec!(50)), None).await.unwrap();

    giftcards::attach_to_checkout(&db, token, &c1.code, &currency, None).await.unwrap();
    // Idempotent re-attach, like Django's m2m .add().
    giftcards::attach_to_checkout(&db, token, &c1.code, &currency, None).await.unwrap();
    giftcards::attach_to_checkout(&db, token, &c2.code, &currency, None).await.unwrap();

    let cards = giftcards::checkout_cards(&db, token).await.unwrap();
    assert_eq!(cards.len(), 2);
    assert_eq!(giftcards::checkout_balance(&db, token, &currency).await.unwrap(), dec!(110));

    giftcards::detach_from_checkout(&db, token, &c1.code).await.unwrap();
    assert_eq!(giftcards::checkout_balance(&db, token, &currency).await.unwrap(), dec!(50));

    let err = giftcards::detach_from_checkout(&db, token, &c1.code).await.unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));

    giftcards::detach_from_checkout(&db, token, &c2.code).await.unwrap();
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn attach_rejects_unknown_expired_foreign_and_restricted() {
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "").await.unwrap();

    // Unknown code → generic invalid (Django's InvalidPromoCode).
    let err = giftcards::attach_to_checkout(&db, token, "NOPE-0000-XXXX", &currency, None)
        .await
        .unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));

    // Wrong currency.
    let eur = giftcards::IssueInput { currency: "EUR".to_string(), ..issue_input(dec!(10)) };
    let eur_card = giftcards::issue(&db, eur, None).await.unwrap();
    let err = giftcards::attach_to_checkout(&db, token, &eur_card.code, &currency, None)
        .await
        .unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));

    // Expired card.
    let expired = giftcards::IssueInput {
        expiry_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        ..issue_input(dec!(10))
    };
    let exp_card = giftcards::issue(&db, expired, None).await.unwrap();
    let err = giftcards::attach_to_checkout(&db, token, &exp_card.code, &currency, None)
        .await
        .unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));

    // Deactivated card.
    let off = giftcards::issue(&db, issue_input(dec!(10)), None).await.unwrap();
    giftcards::set_active(&db, &off.code, false, None).await.unwrap();
    let err = giftcards::attach_to_checkout(&db, token, &off.code, &currency, None)
        .await
        .unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));

    // Card restricted to another customer (needs a real user id for the FK).
    use rustygod_db::entities::account_user;
    use sea_orm::{EntityTrait, QuerySelect};
    let staff: i32 = account_user::Entity::find()
        .select_only()
        .column(account_user::Column::Id)
        .into_tuple()
        .one(&db)
        .await
        .unwrap()
        .expect("populatedb must have a user");
    let owned = giftcards::issue(&db, issue_input(dec!(10)), None).await.unwrap();
    giftcards::assign(&db, &owned.code, staff, "owner@example.com", None).await.unwrap();
    let err = giftcards::attach_to_checkout(&db, token, &owned.code, &currency, Some(staff + 999_999))
        .await
        .unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));
    // ...but the owner can attach.
    giftcards::attach_to_checkout(&db, token, &owned.code, &currency, Some(staff))
        .await
        .unwrap();

    giftcards::detach_from_checkout(&db, token, &owned.code).await.unwrap();
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn redeem_clamps_spend_and_records_event() {
    let db = db().await;
    let card = giftcards::issue(&db, issue_input(dec!(100)), None).await.unwrap();

    let take = giftcards::redeem_for_order(&db, &card.code, None, dec!(30), None)
        .await
        .unwrap();
    assert_eq!(take, dec!(30));
    let mid = giftcards::get_by_code(&db, &card.code).await.unwrap().unwrap();
    assert_eq!(mid.current_balance_amount, dec!(70));
    assert!(mid.last_used_on.is_some());

    // Over-request takes only the remainder.
    let take = giftcards::redeem_for_order(&db, &card.code, None, dec!(999), None)
        .await
        .unwrap();
    assert_eq!(take, dec!(70));
    let empty = giftcards::get_by_code(&db, &card.code).await.unwrap().unwrap();
    assert_eq!(empty.current_balance_amount, dec!(0));

    // Empty card redeems zero, never negative.
    let take = giftcards::redeem_for_order(&db, &card.code, None, dec!(10), None)
        .await
        .unwrap();
    assert_eq!(take, dec!(0));

    let events = events_of(&db, card.id).await;
    assert_eq!(
        events.iter().filter(|e| *e == giftcards::events::USED_IN_ORDER).count(),
        3
    );
}

#[tokio::test]
async fn redeem_refuses_inactive_card() {
    let db = db().await;
    let card = giftcards::issue(&db, issue_input(dec!(10)), None).await.unwrap();
    giftcards::set_active(&db, &card.code, false, None).await.unwrap();
    let err = giftcards::redeem_for_order(&db, &card.code, None, dec!(5), None)
        .await
        .unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));
}

#[tokio::test]
async fn refund_restores_balance_with_event() {
    let db = db().await;
    let card = giftcards::issue(&db, issue_input(dec!(100)), None).await.unwrap();
    giftcards::redeem_for_order(&db, &card.code, None, dec!(40), None).await.unwrap();
    let back = giftcards::refund_to_card(&db, &card.code, dec!(25), None).await.unwrap();
    assert_eq!(back.current_balance_amount, dec!(85));
    let events = events_of(&db, card.id).await;
    assert!(events.contains(&giftcards::events::REFUNDED_IN_ORDER.to_string()));

    let err = giftcards::refund_to_card(&db, &card.code, dec!(-1), None).await.unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));
}

#[tokio::test]
async fn adjust_balance_keeps_audit_trail() {
    let db = db().await;
    let card = giftcards::issue(&db, issue_input(dec!(100)), None).await.unwrap();
    let adj = giftcards::adjust_balance(&db, &card.code, dec!(250), None).await.unwrap();
    assert_eq!(adj.current_balance_amount, dec!(250));

    use rustygod_db::entities::giftcard_giftcardevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let row = giftcard_giftcardevent::Entity::find()
        .filter(giftcard_giftcardevent::Column::GiftCardId.eq(card.id))
        .filter(giftcard_giftcardevent::Column::Type.eq(giftcards::events::BALANCE_ADJUSTED))
        .one(&db)
        .await
        .unwrap()
        .expect("BALANCE_ADJUSTED row must exist");
    assert_eq!(row.parameters["balance"]["old_current_balance"], "100.000");

    let err = giftcards::adjust_balance(&db, &card.code, dec!(-5), None).await.unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));
}

#[tokio::test]
async fn assign_refuses_spent_card() {
    let db = db().await;
    use rustygod_db::entities::account_user;
    use sea_orm::{EntityTrait, QuerySelect};
    let staff: i32 = account_user::Entity::find()
        .select_only()
        .column(account_user::Column::Id)
        .into_tuple()
        .one(&db)
        .await
        .unwrap()
        .expect("populatedb must have a user");

    let card = giftcards::issue(&db, issue_input(dec!(50)), None).await.unwrap();
    giftcards::redeem_for_order(&db, &card.code, None, dec!(10), None).await.unwrap();
    let err = giftcards::assign(&db, &card.code, staff, "late@example.com", None)
        .await
        .unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::GiftCardNotApplicable(_)));
}
