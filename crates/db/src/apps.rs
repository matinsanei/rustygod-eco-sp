//! Saleor Apps (extensions) on Django's `app_*` tables.
//!
//! Mirrors `saleor/app/models.py` + `saleor/graphql/app/mutations/`:
//! - app tokens are shown once and stored **hashed** (`pbkdf2_sha256`,
//!   Django's `make_password`); verification filters by `token_last_4` and
//!   `check_password`s the candidates — exactly `AppTokenVerify`;
//! - only `is_active` apps whose `removed_at` is null authenticate;
//! - permissions resolve through `app_app_permissions` to codenames, the
//!   same strings staff checks use (`manage_orders`, `manage_gift_card`…).

use rand::Rng;
use rustygod_core::auth::{self, PasswordCheck};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set,
};
use uuid::Uuid;

use crate::{
    entities::{app_app, app_app_permissions, app_apptoken, permission_permission},
    DbError, Result,
};

/// oauthlib's `generate_token` default: 30 urlsafe-ish chars. We use
/// alphanumerics (same alphabet as Django's random strings).
fn generate_token() -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..30)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

#[derive(Debug)]
pub struct AppTokenView {
    pub token_id: i32,
    pub app_id: i32,
    pub app_name: String,
}

/// Register a minimal third-party app row (test/extension bootstrap).
/// Returns the app id. Callers own cleanup.
pub async fn create_app(
    db: &impl sea_orm::ConnectionTrait,
    name: &str,
    permissions: &[&str],
) -> Result<i32> {
    use chrono::Utc;
    use serde_json::json;
    let row = app_app::ActiveModel {
        private_metadata: Set(json!({})),
        metadata: Set(json!({})),
        name: Set(name.to_string()),
        created_at: Set(Utc::now().into()),
        is_active: Set(true),
        identifier: Set(format!("test.{}", Uuid::new_v4())),
        r#type: Set("thirdparty".to_string()),
        is_installed: Set(true),
        uuid: Set(Uuid::new_v4()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    for codename in permissions {
        if let Some(perm) = permission_permission::Entity::find()
            .filter(permission_permission::Column::Codename.eq(*codename))
            .one(db)
            .await?
        {
            app_app_permissions::ActiveModel {
                app_id: Set(row.id),
                permission_id: Set(perm.id),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
    }
    Ok(row.id)
}

/// Delete an app row with its tokens and permission grants (test cleanup).
pub async fn delete_app(db: &impl sea_orm::ConnectionTrait, app_id: i32) -> Result<()> {
    app_apptoken::Entity::delete_many()
        .filter(app_apptoken::Column::AppId.eq(app_id))
        .exec(db)
        .await?;
    app_app_permissions::Entity::delete_many()
        .filter(app_app_permissions::Column::AppId.eq(app_id))
        .exec(db)
        .await?;
    app_app::Entity::delete_by_id(app_id).exec(db).await?;
    Ok(())
}

/// Mint an app token. Returns `(token_id, raw_token)` — the raw value is
/// never stored (only its hash + last 4), so it must be shown once.
pub async fn create_app_token(
    db: &impl sea_orm::ConnectionTrait,
    app_id: i32,
    name: &str,
) -> Result<(i32, String)> {
    let app = app_app::Entity::find_by_id(app_id)
        .one(db)
        .await?
        .ok_or_else(|| DbError::App(format!("app {app_id} not found")))?;
    if !app.is_active || app.removed_at.is_some() {
        return Err(DbError::App(format!("app {app_id} is not active")));
    }
    let raw = generate_token();
    let row = app_apptoken::ActiveModel {
        name: Set(name.to_string()),
        auth_token: Set(auth::hash_password(&raw)),
        app_id: Set(app_id),
        token_last_4: Set(raw[raw.len() - 4..].to_string()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok((row.id, raw))
}

/// Verify a raw app token (`AppTokenVerify`).
pub async fn verify_app_token(
    db: &impl sea_orm::ConnectionTrait,
    raw: &str,
) -> Result<Option<AppTokenView>> {
    if raw.len() < 4 {
        return Ok(None);
    }
    let last4 = &raw[raw.len() - 4..];
    let candidates = app_apptoken::Entity::find()
        .filter(app_apptoken::Column::TokenLast4.eq(last4))
        .all(db)
        .await?;
    for tok in candidates {
        if !matches!(auth::verify_password(raw, &tok.auth_token), PasswordCheck::Ok) {
            continue;
        }
        let Some(app) = app_app::Entity::find_by_id(tok.app_id).one(db).await? else {
            continue;
        };
        if !app.is_active || app.removed_at.is_some() {
            continue;
        }
        return Ok(Some(AppTokenView {
            token_id: tok.id,
            app_id: app.id,
            app_name: app.name,
        }));
    }
    Ok(None)
}

/// Revoke (delete) an app token.
pub async fn revoke_app_token(
    db: &impl sea_orm::ConnectionTrait,
    token_id: i32,
) -> Result<bool> {
    let res = app_apptoken::Entity::delete_by_id(token_id).exec(db).await?;
    Ok(res.rows_affected > 0)
}

/// Permission codenames granted to an app.
pub async fn app_permissions(
    db: &impl sea_orm::ConnectionTrait,
    app_id: i32,
) -> Result<Vec<String>> {
    let perm_ids: Vec<i32> = app_app_permissions::Entity::find()
        .select_only()
        .column(app_app_permissions::Column::PermissionId)
        .filter(app_app_permissions::Column::AppId.eq(app_id))
        .into_tuple()
        .all(db)
        .await?;
    if perm_ids.is_empty() {
        return Ok(vec![]);
    }
    Ok(permission_permission::Entity::find()
        .select_only()
        .column(permission_permission::Column::Codename)
        .filter(permission_permission::Column::Id.is_in(perm_ids))
        .into_tuple()
        .all(db)
        .await?)
}

/// Staff-side view for the access layer: id, superuser/staff flags,
/// and the live `jwt_token_key` for revocation checks.
pub struct StaffIdentity {
    pub id: i32,
    pub email: String,
    pub is_staff: bool,
    pub is_superuser: bool,
    pub is_active: bool,
    pub jwt_token_key: String,
}

pub async fn staff_identity(
    db: &impl sea_orm::ConnectionTrait,
    user_id: i32,
) -> Result<Option<StaffIdentity>> {
    use crate::entities::account_user;
    let row: Option<(i32, String, bool, bool, bool, String)> =
        account_user::Entity::find_by_id(user_id)
            .select_only()
            .column(account_user::Column::Id)
            .column(account_user::Column::Email)
            .column(account_user::Column::IsStaff)
            .column(account_user::Column::IsSuperuser)
            .column(account_user::Column::IsActive)
            .column(account_user::Column::JwtTokenKey)
            .into_tuple()
            .one(db)
            .await?;
    Ok(row.map(|(id, email, is_staff, is_superuser, is_active, jwt_token_key)| {
        StaffIdentity { id, email, is_staff, is_superuser, is_active, jwt_token_key }
    }))
}
