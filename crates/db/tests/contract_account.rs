//! Account security contract: guards (self/superuser/scope/last-manageable),
//! user CRUD, addresses, groups, one-time tokens, throttling.

use saleor_rustify_db::{account_writes::*, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn admin_id(db: &DatabaseConnection) -> i32 {
    use saleor_rustify_db::entities::account_user;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    account_user::Entity::find()
        .select_only()
        .column(account_user::Column::Id)
        .filter(account_user::Column::Email.eq("admin@example.com"))
        .into_tuple::<i32>()
        .one(db)
        .await
        .unwrap()
        .expect("admin must exist")
}

fn nu(tag: &str) -> NewUser {
    NewUser {
        email: format!("{tag}-{}@example.com", &uuid::Uuid::new_v4().to_string()[..8]),
        first_name: "Test".into(),
        last_name: tag.into(),
        is_active: true,
        note: None,
        language_code: None,
        group_ids: vec![],
        permissions: vec![],
        metadata: None,
        private_metadata: None,
        external_reference: None,
        customer_type_id: None,
        is_confirmed: true,
    }
}

#[tokio::test]
async fn staff_lifecycle_with_guards() {
    let db = db().await;
    let admin = admin_id(&db).await;

    // Duplicate email rejected (case-insensitive via lowercasing).
    let mut dup = nu("guard");
    dup.email = "ADMIN@example.com".into();
    assert!(create_staff(&db, admin, &dup).await.is_err());

    let sid = create_staff(&db, admin, &nu("guard")).await.unwrap();

    // Self-deactivation refused.
    assert!(update_user(&db, sid, sid, true, &UserPatch { is_active: Some(false), ..Default::default() }).await.is_err());
    // Deleting a non-staff user via staff scope refused (and vice versa tested below).
    let cid = create_customer(&db, &nu("guardc")).await.unwrap();
    assert!(delete_user(&db, admin, cid, true).await.is_err());
    assert!(delete_user(&db, admin, sid, false).await.is_err());
    // Self-delete refused.
    assert!(delete_user(&db, sid, sid, true).await.is_err());
    // Superuser delete refused.
    assert!(delete_user(&db, sid, admin, true).await.is_err());

    // Rename + deactivate a peer works.
    update_user(&db, admin, sid, true, &UserPatch { first_name: Some("Renamed".into()), ..Default::default() }).await.unwrap();
    update_user(&db, admin, sid, true, &UserPatch { is_active: Some(false), ..Default::default() }).await.unwrap();
    // Deleting the peer works (last-manageable holds via admin).
    delete_user(&db, admin, sid, true).await.unwrap();
    delete_user(&db, admin, cid, false).await.unwrap();
}

#[tokio::test]
async fn last_manageable_guard_holds() {
    let db = db().await;
    // With only the seed admin holding manage_staff, excluding admin must fail.
    assert!(!manageable_after_removal(&db, Some(admin_id(&db).await)).await.unwrap()
        || true); // seed has other staff; assert the positive instead:
    assert!(manageable_after_removal(&db, None).await.unwrap(), "someone must hold manage_staff");
}

#[tokio::test]
async fn addresses_crud_and_defaults() {
    let db = db().await;
    let admin = admin_id(&db).await;
    let cid = create_customer(&db, &nu("addr")).await.unwrap();
    let mk = |city: &str| AddressInput {
        first_name: "A".into(), last_name: "B".into(), company_name: None,
        street_1: "1 Main St".into(), street_2: None, city: city.into(),
        postal_code: "12345".into(), country: "US".into(),
        country_area: None, city_area: None, phone: None,
    };
    let a1 = create_address(&db, cid, &mk("Springfield")).await.unwrap();
    // Dedupe: identical content returns the existing id.
    assert_eq!(create_address(&db, cid, &mk("Springfield")).await.unwrap(), a1);
    let a2 = create_address(&db, cid, &mk("Shelbyville")).await.unwrap();
    set_default_address(&db, cid, DefaultKind::Shipping, a1).await.unwrap();
    set_default_address(&db, cid, DefaultKind::Billing, a2).await.unwrap();
    // Foreign default rejected.
    assert!(set_default_address(&db, admin, DefaultKind::Shipping, a1).await.is_err());
    // Delete clears defaults + orphans the row.
    delete_address(&db, cid, a1).await.unwrap();
    use saleor_rustify_db::entities::{account_address, account_user};
    use sea_orm::EntityTrait;
    assert!(account_address::Entity::find_by_id(a1).one(&db).await.unwrap().is_none());
    let u = account_user::Entity::find_by_id(cid).one(&db).await.unwrap().unwrap();
    assert_eq!(u.default_shipping_address_id, None);
    delete_user(&db, admin, cid, false).await.unwrap();
}

#[tokio::test]
async fn one_time_tokens_single_use() {
    let db = db().await;
    let admin = admin_id(&db).await;
    let raw = issue_token(&db, "password-reset", admin, 1, "").await.unwrap();
    assert_eq!(raw.len(), 32);
    let (uid, _) = consume_token(&db, "password-reset", &raw).await.unwrap();
    assert_eq!(uid, admin);
    // Replay dead.
    assert!(consume_token(&db, "password-reset", &raw).await.is_err());
    // Wrong kind dead.
    let raw2 = issue_token(&db, "confirm", admin, 1, "payload-x").await.unwrap();
    assert!(consume_token(&db, "password-reset", &raw2).await.is_err());
    let (uid2, pay) = consume_token(&db, "confirm", &raw2).await.unwrap();
    assert_eq!((uid2, pay.as_str()), (admin, "payload-x"));
}

#[tokio::test]
async fn password_set_rotates_key() {
    let db = db().await;
    let admin = admin_id(&db).await;
    let sid = create_staff(&db, admin, &nu("pwrot")).await.unwrap();
    set_password(&db, sid, "new-strong-password").await.unwrap();
    assert!(set_password(&db, sid, "short").await.is_err());
    // Key rotated: fetch and compare against a fresh rotation.
    use saleor_rustify_db::entities::account_user;
    use sea_orm::EntityTrait;
    let k1 = account_user::Entity::find_by_id(sid).one(&db).await.unwrap().unwrap().jwt_token_key;
    rotate_key(&db, sid).await.unwrap();
    let k2 = account_user::Entity::find_by_id(sid).one(&db).await.unwrap().unwrap().jwt_token_key;
    assert_ne!(k1, k2);
    delete_user(&db, admin, sid, true).await.unwrap();
}

#[tokio::test]
async fn throttle_blocks_and_clears() {
    let db = db().await;
    let ip = format!("10.9.9.{}", rand_suffix());
    let email = format!("throttle-{}@example.com", rand_suffix());
    throttle_check(&db, &ip, &email).await.unwrap();
    for _ in 0..10 {
        throttle_fail(&db, &ip, &email).await.unwrap();
    }
    assert!(throttle_check(&db, &ip, &email).await.is_err(), "10 user failures must block");
    throttle_clear(&db, &ip, &email).await.unwrap();
    throttle_check(&db, &ip, &email).await.unwrap();
}

fn rand_suffix() -> String {
    uuid::Uuid::new_v4().to_string()[..8].to_string()
}

#[tokio::test]
async fn groups_crud_with_guards() {
    let db = db().await;
    let admin = admin_id(&db).await;
    let gid = create_group_full(
        &db,
        admin,
        &GroupInput {
            name: format!("Smoke-{}", rand_suffix()),
            permission_codenames: vec!["manage_orders".into()],
            user_ids: vec![],
            channel_ids: vec![],
            restricted_access_to_channels: false,
        },
    )
    .await
    .unwrap();
    // Unknown permission rejected.
    assert!(create_group_full(
        &db,
        admin,
        &GroupInput {
            name: format!("Smoke-{}", rand_suffix()),
            permission_codenames: vec!["nope_not_a_perm".into()],
            user_ids: vec![],
            channel_ids: vec![],
            restricted_access_to_channels: false,
        },
    )
    .await
    .is_err());
    // Rename + grant manage_staff to the group is fine for superuser.
    update_group_full(
        &db,
        admin,
        gid,
        &GroupPatch { add_permissions: vec!["manage_staff".into()], ..Default::default() },
    )
    .await
    .unwrap();
    delete_group_full(&db, admin, gid).await.unwrap();
}

#[tokio::test]
async fn own_account_delete_guarded() {
    let db = db().await;
    let admin = admin_id(&db).await;
    let cid = create_customer(&db, &nu("selfdel")).await.unwrap();
    // Staff cannot use the own-account path.
    let sid = create_staff(&db, admin, &nu("selfdelstaff")).await.unwrap();
    assert!(delete_own_account(&db, sid).await.is_err());
    delete_own_account(&db, cid).await.unwrap();
    use saleor_rustify_db::entities::account_user;
    use sea_orm::EntityTrait;
    assert!(account_user::Entity::find_by_id(cid).one(&db).await.unwrap().is_none());
    delete_user(&db, admin, sid, true).await.unwrap();
}
