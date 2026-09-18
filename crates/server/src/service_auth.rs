//! AuthService: Django-compatible login (PBKDF2/bcrypt, no resets),
//! RS256 tokens from the shared RSA_PRIVATE_KEY, refresh + verify with
//! jwt_token_key revocation checks, staff permission resolution.

use rustygod_core::auth::{self, PasswordCheck};
use rustygod_db::{apps, auth as db_auth};
use rustygod_proto::auth::{
    auth_service_server::AuthService, CheckPermissionRequest, CheckPermissionResponse,
    CreateAppTokenRequest, CreateAppTokenResponse, LoginRequest, LoginResponse,
    RefreshTokenRequest, RefreshTokenResponse, RevokeAppTokenRequest, RevokeAppTokenResponse,
    VerifyAppTokenRequest, VerifyAppTokenResponse, VerifyTokenRequest, VerifyTokenResponse,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};

pub struct AuthServiceImpl {
    db: Option<DatabaseConnection>,
}

impl AuthServiceImpl {
    pub fn new(db: Option<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn db(&self) -> Result<&DatabaseConnection, Status> {
        self.db
            .as_ref()
            .ok_or_else(|| Status::unavailable("postgres unavailable"))
    }

    fn err(code: &str, message: String) -> rustygod_proto::common::Error {
        rustygod_proto::common::Error {
            code: code.into(),
            message,
            field: String::new(),
        }
    }

    fn invalid_login() -> Vec<rustygod_proto::common::Error> {
        // Django returns the same error for bad email and bad password
        // (no user enumeration).
        vec![Self::err(
            "INVALID_CREDENTIALS",
            "Please, enter valid credentials".into(),
        )]
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();
        let db = self.db()?;
        let fail = || {
            Ok(Response::new(LoginResponse {
                access_token: String::new(),
                refresh_token: String::new(),
                user_id: String::new(),
                is_staff: false,
                errors: Self::invalid_login(),
            }))
        };
        let Some((user, hash)) = db_auth::find_for_login(db, &req.email)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        else {
            return fail();
        };
        if !user.is_active {
            return fail();
        }
        if !matches!(auth::verify_password(&req.password, &hash), PasswordCheck::Ok) {
            return fail();
        }
        let issuer = db_auth::issuer().await;
        let pair = auth::mint_tokens(&issuer, &user.email, user.id, user.is_staff, &user.jwt_token_key)
            .map_err(|e| Status::internal(format!("token signing failed: {e}")))?;
        Ok(Response::new(LoginResponse {
            access_token: pair.access,
            refresh_token: pair.refresh,
            user_id: auth::user_global_id(user.id),
            is_staff: user.is_staff,
            errors: vec![],
        }))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        let db = self.db()?;
        let fail = || {
            Ok(Response::new(RefreshTokenResponse {
                access_token: String::new(),
                errors: vec![Self::err("INVALID_TOKEN", "Invalid refresh token".into())],
            }))
        };
        let claims = match auth::decode(&request.into_inner().refresh_token) {
            Ok(c) => c,
            Err(_) => return fail(),
        };
        if claims.token_type != auth::TOKEN_TYPE_REFRESH {
            return fail();
        }
        let Some(uid) = auth::parse_user_global_id(&claims.user_id) else {
            return fail();
        };
        let issuer = db_auth::issuer().await;
        // Revocation check: token claim must match the live row (Django semantics).
        let Some((user, _)) = db_auth::find_for_login(db, &claims.email)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        else {
            return fail();
        };
        if user.id != uid || !db_auth::token_key_valid(&user, &claims.token) {
            return fail();
        }
        let pair = auth::mint_tokens(&issuer, &user.email, user.id, user.is_staff, &user.jwt_token_key)
            .map_err(|e| Status::internal(format!("token signing failed: {e}")))?;
        Ok(Response::new(RefreshTokenResponse {
            access_token: pair.access,
            errors: vec![],
        }))
    }

    async fn verify_token(
        &self,
        request: Request<VerifyTokenRequest>,
    ) -> Result<Response<VerifyTokenResponse>, Status> {
        let db = self.db()?;
        let invalid = || {
            Ok(Response::new(VerifyTokenResponse {
                valid: false,
                user_id: String::new(),
                email: String::new(),
                token_type: String::new(),
                is_staff: false,
                errors: vec![Self::err("INVALID_TOKEN", "Invalid token".into())],
            }))
        };
        let claims = match auth::decode(&request.into_inner().token) {
            Ok(c) => c,
            Err(_) => return invalid(),
        };
        let Some((user, _)) = db_auth::find_for_login(db, &claims.email)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        else {
            return invalid();
        };
        if !db_auth::token_key_valid(&user, &claims.token) {
            return invalid();
        }
        Ok(Response::new(VerifyTokenResponse {
            valid: true,
            user_id: claims.user_id,
            email: claims.email,
            token_type: claims.token_type,
            is_staff: claims.is_staff,
            errors: vec![],
        }))
    }

    async fn check_permission(
        &self,
        request: Request<CheckPermissionRequest>,
    ) -> Result<Response<CheckPermissionResponse>, Status> {
        let req = request.into_inner();
        let uid = auth::parse_user_global_id(&req.user_id)
            .or_else(|| req.user_id.parse::<i32>().ok())
            .unwrap_or(-1);
        let allowed = db_auth::has_permission(self.db()?, uid, &req.codename)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(CheckPermissionResponse { allowed }))
    }

    async fn create_app_token(
        &self,
        request: Request<CreateAppTokenRequest>,
    ) -> Result<Response<CreateAppTokenResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_APPS).await?;
        let req = request.into_inner();
        let name = if req.name.is_empty() { "api-token".to_string() } else { req.name };
        match apps::create_app_token(db, req.app_id, &name).await {
            Ok((token_id, token)) => Ok(Response::new(CreateAppTokenResponse {
                token_id,
                token,
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(CreateAppTokenResponse {
                token_id: 0,
                token: String::new(),
                errors: vec![Self::err("REJECTED", e.to_string())],
            })),
        }
    }

    async fn verify_app_token(
        &self,
        request: Request<VerifyAppTokenRequest>,
    ) -> Result<Response<VerifyAppTokenResponse>, Status> {
        let db = self.db()?;
        match apps::verify_app_token(db, &request.into_inner().token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(v) => Ok(Response::new(VerifyAppTokenResponse {
                valid: true,
                app_id: v.app_id,
                app_name: v.app_name,
            })),
            None => Ok(Response::new(VerifyAppTokenResponse {
                valid: false,
                app_id: 0,
                app_name: String::new(),
            })),
        }
    }

    async fn revoke_app_token(
        &self,
        request: Request<RevokeAppTokenRequest>,
    ) -> Result<Response<RevokeAppTokenResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_APPS).await?;
        match apps::revoke_app_token(db, request.into_inner().token_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            true => Ok(Response::new(RevokeAppTokenResponse { revoked: true, errors: vec![] })),
            false => Ok(Response::new(RevokeAppTokenResponse {
                revoked: false,
                errors: vec![Self::err("NOT_FOUND", "app token not found".into())],
            })),
        }
    }
}
