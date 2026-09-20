//! Account / Auth GraphQL: `me { ...User }` — the boot-time query the
//! Dashboard blocks on. Thin wrapper over the same JWT + DB the gRPC
//! `AuthService` uses (RSA_PRIVATE_KEY, `account_user`).

use async_graphql::*;
use chrono::{DateTime, Utc};
use sea_orm::EntityTrait;

use crate::context::{Bearer, GqlContext};

#[derive(SimpleObject, Clone)]
pub struct GqlCountryDisplay { pub code: String, pub country: String }

#[derive(SimpleObject, Clone)]
pub struct GqlStockSettings { pub allocation_strategy: String }

#[derive(SimpleObject, Clone)]
pub struct GqlChannelForUser {
    pub id: ID,
    pub is_active: bool,
    pub name: String,
    pub slug: String,
    pub currency_code: String,
    pub default_country: GqlCountryDisplay,
    pub stock_settings: GqlStockSettings,
}

#[derive(SimpleObject, Clone)]
pub struct GqlUserPermission { pub code: String, pub name: String }

#[derive(SimpleObject, Clone)]
pub struct GqlUser {
    pub id: ID,
    pub email: String,
    #[graphql(name = "firstName")]
    pub first_name: String,
    #[graphql(name = "lastName")]
    pub last_name: String,
    #[graphql(name = "isActive")]
    pub is_active: bool,
    #[graphql(name = "isStaff")]
    pub is_staff: bool,
    #[graphql(name = "dateJoined")]
    pub date_joined: DateTime<Utc>,
    #[graphql(name = "restrictedAccessToChannels")]
    pub restricted_access_to_channels: bool,
    pub metadata: Vec<GqlMetadataItem>,
    #[graphql(name = "userPermissions")]
    pub user_permissions: Option<Vec<GqlUserPermission>>,
    pub avatar: Option<GqlImage>,
    #[graphql(name = "accessibleChannels")]
    pub accessible_channels: Option<Vec<GqlChannelForUser>>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlMetadataItem { pub key: String, pub value: String }

#[derive(SimpleObject, Clone)]
pub struct GqlImage { pub url: String }

#[derive(Default)]
pub struct AccountQuery;

#[Object]
impl AccountQuery {
    async fn me(&self, ctx: &Context<'_>) -> Result<Option<GqlUser>> {
        let bearer = ctx.data_opt::<Bearer>().map(|b| b.0.as_str().to_string())
            .or_else(|| ctx.data_opt::<GqlContext>().and_then(|g| g.bearer.clone()));
        let Some(token) = bearer else { return Ok(None) };
        let claims = rustygod_core::auth::decode(&token).map_err(|e| Error::new(format!("auth: {e}")))?;
        if claims.token_type != rustygod_core::auth::TOKEN_TYPE_ACCESS {
            return Ok(None);
        }
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let uid = rustygod_core::auth::parse_user_global_id(&claims.user_id).unwrap_or(0);
        let user = rustygod_db::entities::account_user::Entity::find_by_id(uid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("user not found"))?;
        // Permissions: collect from direct + group grants (like server::access).
        let perms = collect_permissions(db, uid).await.unwrap_or_default();
        let channels = rustygod_db::commerce::list_channels(db).await.unwrap_or_default();
        Ok(Some(GqlUser {
            id: ID(claims.user_id),
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
            is_active: user.is_active,
            is_staff: user.is_staff,
            date_joined: user.date_joined.into(),
            restricted_access_to_channels: false,
            metadata: vec![],
            user_permissions: Some(perms.into_iter().map(|code| GqlUserPermission { code: code.clone(), name: code }).collect()),
            avatar: None,
            accessible_channels: Some(channels.into_iter().map(|c| GqlChannelForUser {
                id: ID(format!("Channel:{}", c.id)),
                is_active: c.is_active,
                name: c.slug.clone(),
                slug: c.slug,
                currency_code: c.currency_code,
                default_country: GqlCountryDisplay { code: "US".into(), country: "United States".into() },
                stock_settings: GqlStockSettings { allocation_strategy: "prioritize-sorting-order".into() },
            }).collect()),
        }))
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlTokenCreate {
    pub token: String,
    #[graphql(name = "refreshToken")]
    pub refresh_token: String,
    pub errors: Vec<GqlAccountError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlAccountError {
    pub field: Option<String>,
    pub message: String,
    pub code: String,
}

#[derive(Default)]
pub struct AccountMutation;

#[Object]
impl AccountMutation {
    /// Dashboard login: mirrors `saleor/account` tokenCreate (email+password → RS256 JWTs).
    /// Uses the same `RSA_PRIVATE_KEY` and `account_user` rows Django uses.
    async fn token_create(&self, ctx: &Context<'_>, email: String, password: String) -> Result<GqlTokenCreate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some((user, hash)) = rustygod_db::auth::find_for_login(db, &email).await.map_err(|e| Error::new(e.to_string()))? else {
            return Ok(GqlTokenCreate { token: String::new(), refresh_token: String::new(), errors: vec![GqlAccountError { field: Some("email".into()), message: "Invalid credentials".into(), code: "INVALID_CREDENTIALS".into() }] });
        };
        if !user.is_active {
            return Ok(GqlTokenCreate { token: String::new(), refresh_token: String::new(), errors: vec![GqlAccountError { field: None, message: "User is inactive".into(), code: "INACTIVE".into() }] });
        }
        let ok = match rustygod_core::auth::verify_password(&password, &hash) {
            rustygod_core::auth::PasswordCheck::Ok => true,
            _ => false,
        };
        if !ok {
            return Ok(GqlTokenCreate { token: String::new(), refresh_token: String::new(), errors: vec![GqlAccountError { field: Some("password".into()), message: "Invalid credentials".into(), code: "INVALID_CREDENTIALS".into() }] });
        }
        let pair = rustygod_core::auth::mint_tokens("http://localhost:8000/graphql/", &user.email, user.id, user.is_staff, &user.jwt_token_key).map_err(|e| Error::new(format!("mint: {e}")))?;
        Ok(GqlTokenCreate { token: pair.access, refresh_token: pair.refresh, errors: vec![] })
    }

    async fn token_refresh(&self, ctx: &Context<'_>, refresh_token: String) -> Result<GqlTokenCreate> {
        let claims = rustygod_core::auth::decode(&refresh_token).map_err(|e| Error::new(format!("auth: {e}")))?;
        if claims.token_type != rustygod_core::auth::TOKEN_TYPE_REFRESH {
            return Ok(GqlTokenCreate { token: String::new(), refresh_token: String::new(), errors: vec![GqlAccountError { field: None, message: "Invalid refresh token".into(), code: "INVALID".into() }] });
        }
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let uid = rustygod_core::auth::parse_user_global_id(&claims.user_id).unwrap_or(0);
        let user = rustygod_db::entities::account_user::Entity::find_by_id(uid).one(db).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("user not found"))?;
        // Rotate check: Django invalidates refresh if jwt_token_key changed.
        if user.jwt_token_key != claims.token {
            return Ok(GqlTokenCreate { token: String::new(), refresh_token: String::new(), errors: vec![GqlAccountError { field: None, message: "Token expired".into(), code: "EXPIRED".into() }] });
        }
        let pair = rustygod_core::auth::mint_tokens("http://localhost:8000/graphql/", &user.email, user.id, user.is_staff, &user.jwt_token_key).map_err(|e| Error::new(format!("mint: {e}")))?;
        Ok(GqlTokenCreate { token: pair.access, refresh_token: pair.refresh, errors: vec![] })
    }
}

async fn collect_permissions(db: &sea_orm::DatabaseConnection, user_id: i32) -> Result<Vec<String>, sea_orm::DbErr> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    use rustygod_db::entities::{account_user_user_permissions, permission_permission, account_user_groups, account_group_permissions};
    let mut codes: Vec<String> = vec![];
    // Direct permissions
    let direct_pids: Vec<i32> = account_user_user_permissions::Entity::find()
        .select_only().column(account_user_user_permissions::Column::PermissionId)
        .filter(account_user_user_permissions::Column::UserId.eq(user_id))
        .into_tuple::<i32>().all(db).await?;
    for pid in direct_pids {
        if let Some(p) = permission_permission::Entity::find_by_id(pid).one(db).await? {
            codes.push(p.codename);
        }
    }
    // Group permissions
    let groups: Vec<i32> = account_user_groups::Entity::find()
        .select_only().column(account_user_groups::Column::GroupId)
        .filter(account_user_groups::Column::UserId.eq(user_id))
        .into_tuple::<i32>().all(db).await?;
    if !groups.is_empty() {
        let pids: Vec<i32> = account_group_permissions::Entity::find()
            .select_only().column(account_group_permissions::Column::PermissionId)
            .filter(account_group_permissions::Column::GroupId.is_in(groups))
            .into_tuple::<i32>().all(db).await?;
        for pid in pids {
            if let Some(p) = permission_permission::Entity::find_by_id(pid).one(db).await? {
                codes.push(p.codename);
            }
        }
    }
    if codes.is_empty() {
        // Fallback for populatedb admin (superuser bypass in server::access).
        // Return a minimal staff set so UI doesn't hide everything.
        let user = rustygod_db::entities::account_user::Entity::find_by_id(user_id).one(db).await?.unwrap();
        if user.is_superuser {
            codes = vec!["manage_orders".into(), "manage_products".into(), "manage_channels".into(), "manage_staff".into(), "manage_apps".into()];
        }
    }
    codes.sort(); codes.dedup();
    Ok(codes)
}
