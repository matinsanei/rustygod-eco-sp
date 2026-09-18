//! App (extension) contract vs Django's `app_*` tables.
//! Mirrors `saleor/graphql/app/tests/` token flows: creation returns the raw
//! token once (only hash + last-4 stored), `AppTokenVerify` semantics,
//! revocation, and permission codename resolution.

use rustygod_db::{apps, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn active_app(db: &DatabaseConnection) -> i32 {
    apps::create_app(db, "ci-test-app", &["manage_orders", "manage_gift_card"])
        .await
        .expect("must create test app")
}

#[tokio::test]
async fn token_lifecycle_hash_verify_revoke() {
    let db = db().await;
    let app_id = active_app(&db).await;

    let (tid, raw) = apps::create_app_token(&db, app_id, "ci-token").await.unwrap();
    assert!(raw.len() >= 20);
    assert_eq!(raw[raw.len() - 4..].len(), 4);

    // Stored hashed: the row must not contain the raw token.
    use rustygod_db::entities::app_apptoken;
    use sea_orm::EntityTrait;
    let row = app_apptoken::Entity::find_by_id(tid).one(&db).await.unwrap().unwrap();
    assert_ne!(row.auth_token, raw);
    assert!(row.auth_token.starts_with("pbkdf2_sha256$"));
    assert_eq!(row.token_last_4, raw[raw.len() - 4..]);

    // Verify roundtrips (AppTokenVerify semantics).
    let view = apps::verify_app_token(&db, &raw).await.unwrap().expect("must verify");
    assert_eq!(view.token_id, tid);
    assert_eq!(view.app_id, app_id);

    // Wrong token fails closed.
    assert!(apps::verify_app_token(&db, "nope-nope-nope-nope-nope-0000").await.unwrap().is_none());
    // Tampered last-4 fails closed.
    let mut bad = raw.clone();
    bad.replace_range(raw.len() - 4.., "ZZZZ");
    assert!(apps::verify_app_token(&db, &bad).await.unwrap().is_none());

    // Revoke.
    assert!(apps::revoke_app_token(&db, tid).await.unwrap());
    assert!(apps::verify_app_token(&db, &raw).await.unwrap().is_none());
    assert!(!apps::revoke_app_token(&db, tid).await.unwrap());

    apps::delete_app(&db, app_id).await.unwrap();
}

#[tokio::test]
async fn create_rejects_unknown_or_inactive_app() {
    let db = db().await;
    let err = apps::create_app_token(&db, i32::MAX, "x").await.unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::App(_)));
}

#[tokio::test]
async fn app_permissions_resolve_codenames() {
    let db = db().await;
    let app_id = active_app(&db).await;
    let mut perms = apps::app_permissions(&db, app_id).await.unwrap();
    perms.sort();
    assert_eq!(perms, vec!["manage_gift_card", "manage_orders"]);
    apps::delete_app(&db, app_id).await.unwrap();
}
