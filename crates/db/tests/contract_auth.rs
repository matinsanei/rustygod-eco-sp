//! Auth contract: real Django password hashes verify, RS256 tokens
//! round-trip, refresh/revocation follow Django semantics.
//! Mirrors `saleor/graphql/account/tests/mutations/authentication/`.
//!
//! Keys pass explicitly (no env races between parallel tests); the server
//! layer reads the shared `RSA_PRIVATE_KEY` like Django.

use rustygod_core::auth::{self, PasswordCheck};
use rustygod_db::{auth as db_auth, database_url};
use sea_orm::DatabaseConnection;

fn test_key() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/testdata/test_rsa.pem"
    ))
    .expect("test RSA key must exist")
}

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn django_admin_hash_verifies() {
    // admin@example.com / admin — the populatedb account, Django-hashed.
    let db = db().await;
    let (user, hash) = db_auth::find_for_login(&db, "admin@example.com")
        .await
        .unwrap()
        .expect("populatedb admin must exist");
    assert!(user.is_staff);
    assert!(matches!(auth::verify_password("admin", &hash), PasswordCheck::Ok));
    assert!(matches!(auth::verify_password("wrong", &hash), PasswordCheck::Wrong));
}

#[tokio::test]
async fn token_roundtrip_and_type_enforcement() {
    let pem = test_key();
    let db = db().await;
    let (user, _) = db_auth::find_for_login(&db, "admin@example.com").await.unwrap().unwrap();

    let pair =
        auth::mint_tokens_with_key(&pem, "test", &user.email, user.id, user.is_staff, &user.jwt_token_key)
            .unwrap();
    let access = auth::decode_with_key(&pair.access, &pem).unwrap();
    assert_eq!(access.token_type, "access");
    assert_eq!(access.email, "admin@example.com");
    assert_eq!(access.user_id, auth::user_global_id(user.id));
    assert!(access.exp - access.iat == 5 * 60);
    let refresh = auth::decode_with_key(&pair.refresh, &pem).unwrap();
    assert_eq!(refresh.token_type, "refresh");
    assert!(refresh.exp - refresh.iat == 30 * 24 * 3600);

    // Tampered payload rejected.
    let mut bad = pair.access.clone();
    bad.push('x');
    assert!(auth::decode_with_key(&bad, &pem).is_err());
    // Wrong key rejected.
    assert!(auth::decode_with_key(&pair.access, &pem.replace("A", "B")).is_err());
}

#[tokio::test]
async fn revocation_on_key_rotation() {
    let pem = test_key();
    let db = db().await;
    let (user, _) = db_auth::find_for_login(&db, "admin@example.com").await.unwrap().unwrap();
    let old_key = user.jwt_token_key.clone();

    let pair =
        auth::mint_tokens_with_key(&pem, "test", &user.email, user.id, user.is_staff, &old_key)
            .unwrap();
    let claims = auth::decode_with_key(&pair.access, &pem).unwrap();
    assert!(db_auth::token_key_valid(&user, &claims.token));

    // Rotate (like Django on password change): old token dies.
    let new_key = db_auth::rotate_token_key(&db, user.id).await.unwrap();
    assert_ne!(new_key, old_key);
    let (user2, _) = db_auth::find_for_login(&db, "admin@example.com").await.unwrap().unwrap();
    assert!(!db_auth::token_key_valid(&user2, &claims.token));

    // Restore (test hygiene on shared data).
    {
        use rustygod_db::entities::account_user;
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};
        let row = account_user::Entity::find_by_id(user.id).one(&db).await.unwrap().unwrap();
        let mut am: account_user::ActiveModel = row.into();
        am.jwt_token_key = Set(old_key);
        am.update(&db).await.unwrap();
    }
}

#[tokio::test]
async fn staff_permission_resolution() {
    let db = db().await;
    let (user, _) = db_auth::find_for_login(&db, "admin@example.com").await.unwrap().unwrap();
    // Superuser-style staff from populatedb carries checkout/order perms via groups.
    let can = db_auth::has_permission(&db, user.id, "manage_orders").await.unwrap();
    assert!(can, "populatedb admin must manage orders");
    assert!(!db_auth::has_permission(&db, user.id, "no_such_perm_xyz").await.unwrap());
    assert!(!db_auth::has_permission(&db, -1, "manage_orders").await.unwrap());
}
