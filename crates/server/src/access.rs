//! Request authorization for staff-only RPCs.
//!
//! Saleor semantics: mutating staff mutations require a permission codename
//! (`MANAGE_ORDERS`, `MANAGE_GIFT_CARD`…), carried by either a staff **user
//! JWT** (`Authorization: Bearer <access>`) or an **app token**. Superusers
//! bypass checks (Django's `is_superuser`); revoked JWTs (`jwt_token_key`
//! rotation) and inactive/removed apps never authorize.
//!
//! Customer-facing flows (checkout, attach/redeem gift cards) stay open —
//! exactly the mutations Django leaves permission-free.

use rustygod_core::auth as core_auth;
use rustygod_db::{apps, auth as db_auth};
use sea_orm::DatabaseConnection;
use tonic::{metadata::MetadataMap, Status};

/// Permission codenames — the DB side of Saleor's `permission/enums.py`
/// (`MANAGE_GIFT_CARD = "giftcard.manage_gift_card"` etc.).
pub const MANAGE_GIFT_CARD: &str = "manage_gift_card";
pub const MANAGE_ORDERS: &str = "manage_orders";
pub const MANAGE_APPS: &str = "manage_apps";
pub const MANAGE_PRODUCTS: &str = "manage_products";

#[derive(Debug)]
pub enum Requestor {
    Staff { id: i32, email: String, superuser: bool },
    App { id: i32, name: String },
}

/// Extract `Bearer <token>` from gRPC metadata (case-insensitive scheme).
pub fn bearer(metadata: &MetadataMap) -> Option<String> {
    let v = metadata.get("authorization")?;
    let s = v.to_str().ok()?;
    let (scheme, token) = s.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

async fn authorize_user(
    db: &DatabaseConnection,
    token: &str,
    codename: &str,
    decoding_pem: &str,
) -> Result<Requestor, Status> {
    let claims = core_auth::decode_with_key(token, decoding_pem)
        .map_err(|_| Status::unauthenticated("invalid or expired token"))?;
    if claims.token_type != core_auth::TOKEN_TYPE_ACCESS {
        return Err(Status::unauthenticated("access token required"));
    }
    let user_id = core_auth::parse_user_global_id(&claims.user_id)
        .ok_or_else(|| Status::unauthenticated("malformed token identity"))?;
    let ident = apps::staff_identity(db, user_id)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or_else(|| Status::unauthenticated("unknown user"))?;
    if !ident.is_active || !db_auth::token_key_valid(
        &db_auth::LoginUser {
            id: ident.id,
            email: ident.email.clone(),
            is_staff: ident.is_staff,
            is_active: ident.is_active,
            jwt_token_key: ident.jwt_token_key.clone(),
        },
        &claims.token,
    ) {
        return Err(Status::unauthenticated("token revoked or user inactive"));
    }
    if ident.is_superuser {
        return Ok(Requestor::Staff { id: ident.id, email: ident.email, superuser: true });
    }
    if ident.is_staff
        && db_auth::has_permission(db, user_id, codename)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
    {
        return Ok(Requestor::Staff { id: ident.id, email: ident.email, superuser: false });
    }
    Err(Status::permission_denied(format!(
        "missing permission: {codename}"
    )))
}

async fn authorize_app(
    db: &DatabaseConnection,
    token: &str,
    codename: &str,
) -> Result<Requestor, Status> {
    let view = apps::verify_app_token(db, token)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or_else(|| Status::unauthenticated("invalid app token"))?;
    let perms = apps::app_permissions(db, view.app_id)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
    if perms.iter().any(|p| p == codename) {
        Ok(Requestor::App { id: view.app_id, name: view.app_name })
    } else {
        Err(Status::permission_denied(format!(
            "app lacks permission: {codename}"
        )))
    }
}

/// Authorize a request against a permission codename.
/// JWTs (contain `.`) go the user path, opaque tokens the app path —
/// mirroring Django trying each auth backend in turn.
pub async fn authorize_with_key(
    db: &DatabaseConnection,
    metadata: &MetadataMap,
    codename: &str,
    decoding_pem: &str,
) -> Result<Requestor, Status> {
    let token = bearer(metadata).ok_or_else(|| Status::unauthenticated("authentication required"))?;
    if token.contains('.') {
        // JWT-shaped: user path only (a malformed JWT is not an app token).
        authorize_user(db, &token, codename, decoding_pem).await
    } else {
        authorize_app(db, &token, codename).await
    }
}

/// Service entry point: decoding key from the shared `RSA_PRIVATE_KEY`,
/// exactly like `AuthService` token verification.
pub async fn authorize(
    db: &DatabaseConnection,
    metadata: &MetadataMap,
    codename: &str,
) -> Result<Requestor, Status> {
    let pem = std::env::var("RSA_PRIVATE_KEY")
        .map_err(|_| Status::unavailable("RSA_PRIVATE_KEY not configured"))?;
    authorize_with_key(db, metadata, codename, &pem).await
}
