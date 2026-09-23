//! Authentication against Django's `account_user` rows:
//! PBKDF2/bcrypt verification (no password resets), RS256 tokens from the
//! same `RSA_PRIVATE_KEY`, `jwt_token_key` revocation checks, and staff
//! permission resolution through groups + direct grants.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set,
};

use crate::{
    entities::{
        account_group_permissions, account_user, account_user_groups,
        account_user_user_permissions, permission_permission,
    },
    DbError, Result,
};

pub struct LoginUser {
    pub id: i32,
    pub email: String,
    pub is_staff: bool,
    pub is_active: bool,
    pub jwt_token_key: String,
}

pub async fn find_for_login(
    db: &impl sea_orm::ConnectionTrait,
    email: &str,
) -> Result<Option<(LoginUser, String)>> {
    let row: Option<(i32, String, bool, bool, String, String)> = account_user::Entity::find()
        .select_only()
        .column(account_user::Column::Id)
        .column(account_user::Column::Email)
        .column(account_user::Column::IsStaff)
        .column(account_user::Column::IsActive)
        .column(account_user::Column::JwtTokenKey)
        .column(account_user::Column::Password)
        .filter(account_user::Column::Email.eq(email))
        .into_tuple()
        .one(db)
        .await?;
    Ok(row.map(|(id, email, is_staff, is_active, jwt_token_key, password)| {
        (
            LoginUser { id, email, is_staff, is_active, jwt_token_key },
            password,
        )
    }))
}

/// Django semantics: inactive users cannot authenticate, and a token is
/// valid only while its `token` claim matches the row's `jwt_token_key`
/// (password change rotates the key and revokes everything).
pub fn token_key_valid(user: &LoginUser, claims_token: &str) -> bool {
    user.is_active && user.jwt_token_key == claims_token
}

/// Rotate `jwt_token_key`, revoking all outstanding tokens (Django does
/// this on password change). Django's key is `get_random_string(12)` —
/// 12 alphanumeric chars fitting `varchar(12)`.
pub async fn rotate_token_key(
    db: &impl sea_orm::ConnectionTrait,
    user_id: i32,
) -> Result<String> {
    use rand::Rng;
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let key: String = (0..12)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect();
    let row = account_user::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(user_id.to_string())))?;
    let mut am: account_user::ActiveModel = row.into();
    am.jwt_token_key = Set(key.clone());
    am.update(db).await?;
    Ok(key)
}

/// Staff permission check: direct user grant OR via any group —
/// mirrors Django's `user.has_perm` for Saleor's codenames.
pub async fn has_permission(
    db: &impl sea_orm::ConnectionTrait,
    user_id: i32,
    codename: &str,
) -> Result<bool> {
    let Some(perm) = permission_permission::Entity::find()
        .filter(permission_permission::Column::Codename.eq(codename))
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    if account_user_user_permissions::Entity::find()
        .filter(account_user_user_permissions::Column::UserId.eq(user_id))
        .filter(account_user_user_permissions::Column::PermissionId.eq(perm.id))
        .one(db)
        .await?
        .is_some()
    {
        return Ok(true);
    }
    let group_ids: Vec<i32> = account_user_groups::Entity::find()
        .select_only()
        .column(account_user_groups::Column::GroupId)
        .filter(account_user_groups::Column::UserId.eq(user_id))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    if group_ids.is_empty() {
        return Ok(false);
    }
    Ok(account_group_permissions::Entity::find()
        .filter(account_group_permissions::Column::GroupId.is_in(group_ids))
        .filter(account_group_permissions::Column::PermissionId.eq(perm.id))
        .one(db)
        .await?
        .is_some())
}

pub async fn issuer() -> String {
    std::env::var("RUSTIFY_JWT_ISSUER").unwrap_or_else(|_| "localhost".to_string())
}
