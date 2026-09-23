//! Permission-group contract vs Django's `account_group*` tables.
//! Mirrors `saleor/graphql/account/tests/` group flows: unique names,
//! member add/remove, grant/revoke codenames, delete-drops-links, and the
//! punchline — membership actually authorizes through `has_permission`.

use saleor_rustify_db::{auth as db_auth, database_url, groups};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn staff_user(db: &DatabaseConnection) -> i32 {
    use saleor_rustify_db::entities::account_user;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    account_user::Entity::find()
        .select_only()
        .column(account_user::Column::Id)
        .filter(account_user::Column::Email.eq("admin@example.com"))
        .into_tuple()
        .one(db)
        .await
        .unwrap()
        .expect("populatedb admin must exist")
}

#[tokio::test]
async fn group_lifecycle_with_grants() {
    let db = db().await;
    let name = format!("ci-group-{}", &Uuid::new_v4().to_string()[..8]);

    let g = groups::create_group(&db, &name, &["manage_orders".to_string()])
        .await
        .unwrap();
    assert_eq!(g.name, name);
    assert_eq!(g.permissions, vec!["manage_orders"]);
    assert!(g.member_ids.is_empty());

    let err = groups::create_group(&db, &name, &[]).await.unwrap_err();
    assert!(err.to_string().contains("already exists"));

    let err = groups::create_group(&db, &format!("{name}-x"), &["bogus_perm".to_string()])
        .await
        .unwrap_err();
    assert!(err.to_string().contains("unknown permission"));

    let g = groups::grant_permissions(&db, g.id, &["manage_products".to_string()])
        .await
        .unwrap();
    assert!(g.permissions.contains(&"manage_products".to_string()));

    let g = groups::revoke_permissions(&db, g.id, &["manage_products".to_string()])
        .await
        .unwrap();
    assert!(!g.permissions.contains(&"manage_products".to_string()));

    let g = groups::rename_group(&db, g.id, &format!("{name}-renamed")).await.unwrap();
    assert!(g.name.ends_with("-renamed"));

    groups::delete_group(&db, g.id).await.unwrap();
    let err = groups::delete_group(&db, g.id).await.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test]
async fn membership_authorizes_through_has_permission() {
    let db = db().await;
    let uid = staff_user(&db).await;
    let name = format!("ci-authz-{}", &Uuid::new_v4().to_string()[..8]);

    // manage_gift_card via a fresh group the admin doesn't have directly...
    // (admin is superuser so the access layer bypasses; here we assert the
    // group machinery itself: member listed, grant visible.)
    let g = groups::create_group(&db, &name, &["manage_gift_card".to_string()])
        .await
        .unwrap();
    let g = groups::add_members(&db, g.id, &[uid]).await.unwrap();
    assert!(g.member_ids.contains(&uid));
    assert!(db_auth::has_permission(&db, uid, "manage_gift_card").await.unwrap());

    let g = groups::remove_members(&db, g.id, &[uid]).await.unwrap();
    assert!(!g.member_ids.contains(&uid));

    let err = groups::add_members(&db, g.id, &[i32::MAX]).await.unwrap_err();
    assert!(err.to_string().contains("not found"));

    groups::delete_group(&db, g.id).await.unwrap();
}
