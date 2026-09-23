//! Gift-card lifecycle: issue/update/delete/activate/tags/notes/settings.

use rust_decimal::Decimal;
use saleor_rustify_db::{database_url, giftcards};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn issue_update_note_delete_cycle() {
    let db = db().await;
    let card = giftcards::issue(
        &db,
        giftcards::IssueInput {
            initial_balance: Decimal::new(100, 0),
            currency: "USD".into(),
            created_by_email: Some("staff@example.com".into()),
            expiry_date: None,
            is_active: true,
            custom_code: Some(format!("OP{:09}", chrono::Utc::now().timestamp_millis() % 1_000_000_000)),
        },
        None,
    )
    .await
    .unwrap();
    giftcards::add_tags(&db, card.id, &["vip".to_string(), "vip".to_string()]).await.unwrap();
    let upd = giftcards::update_card(
        &db,
        card.id,
        &giftcards::UpdateCard {
            add_tags: vec!["new".to_string()],
            remove_tags: vec!["vip".to_string()],
            expiry_date: None,
            balance_amount: Some(Decimal::new(150, 0)),
            is_active: None,
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(upd.current_balance_amount, Decimal::new(150, 0));
    let (_, eid) = giftcards::add_note(&db, card.id, "hello", None).await.unwrap();
    assert!(eid > 0);
    assert!(giftcards::add_note(&db, card.id, "   ", None).await.is_err());
    giftcards::set_active(&db, &card.code, false, None).await.unwrap();
    // Custom-code collision refused.
    assert!(giftcards::issue(
        &db,
        giftcards::IssueInput {
            initial_balance: Decimal::ONE,
            currency: "USD".into(),
            created_by_email: None,
            expiry_date: None,
            is_active: true,
            custom_code: Some(card.code.clone()),
        },
        None,
    )
    .await
    .is_err());
    giftcards::delete_card(&db, card.id).await.unwrap();
    use saleor_rustify_db::entities::giftcard_giftcard;
    use sea_orm::EntityTrait;
    assert!(giftcard_giftcard::Entity::find_by_id(card.id).one(&db).await.unwrap().is_none());
}

#[tokio::test]
async fn bulk_issue_and_settings_guards() {
    let db = db().await;
    let cards = giftcards::bulk_issue(&db, 3, Decimal::new(10, 0), "USD", &["bulk".to_string()], None, true, None)
        .await
        .unwrap();
    assert_eq!(cards.len(), 3);
    assert!(giftcards::bulk_issue(&db, 0, Decimal::ONE, "USD", &[], None, true, None).await.is_err());
    assert!(giftcards::update_settings(&db, Some("bogus".into()), None).await.is_err());
    giftcards::update_settings(&db, Some("expiry_period".into()), Some(30)).await.unwrap();
    giftcards::update_settings(&db, Some("never_expire".into()), None).await.unwrap();
    for c in cards {
        giftcards::delete_card(&db, c.id).await.unwrap();
    }
}

#[tokio::test]
async fn assign_unassign_cycle() {
    let db = db().await;
    let card = giftcards::issue(
        &db,
        giftcards::IssueInput {
            initial_balance: Decimal::new(20, 0),
            currency: "USD".into(),
            created_by_email: None,
            expiry_date: None,
            is_active: true,
            custom_code: None,
        },
        None,
    )
    .await
    .unwrap();
    assert!(giftcards::unassign(&db, &card.code, None).await.is_err());
    giftcards::assign(&db, &card.code, 1, "someone@example.com", None).await.unwrap();
    let u = giftcards::unassign(&db, &card.code, None).await.unwrap();
    assert!(u.assigned_to_email.is_none());
    giftcards::delete_card(&db, card.id).await.unwrap();
}
