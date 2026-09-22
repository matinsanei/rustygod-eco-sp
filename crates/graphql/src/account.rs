//! Account / Auth GraphQL: `me { ...User }` — the boot-time query the
//! Dashboard blocks on. Thin wrapper over the same JWT + DB the gRPC
//! `AuthService` uses (RSA_PRIVATE_KEY, `account_user`).

use async_graphql::*;

use sea_orm::EntityTrait;
use crate::context::{Bearer, GqlContext};
use crate::gen;

#[derive(SimpleObject, Clone)]
#[graphql(name = "UserPermission", complex)]
pub struct GqlUserPermission { pub code: String, pub name: String }

#[ComplexObject]
impl GqlUserPermission {
    /// Saleor `UserPermission.sourcePermissionGroups(userId)` — Dashboard
    /// permission-group pages. No group membership index in this backend
    /// yet → empty (documented gap, zero-cost).
    #[graphql(name = "sourcePermissionGroups")]
    async fn source_permission_groups(&self, user_id: Option<ID>) -> Vec<crate::gen::Group> {
        let _ = user_id;
        vec![]
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "Image")]
pub struct GqlImage { pub url: String, pub alt: Option<String> }

/// Shared user assembly: real identity/permissions/channels from the same
/// `account_user` rows Django uses; `None`/`[]` elsewhere (stubs).
fn to_gen_user(
    user: rustygod_db::entities::account_user::Model,
    claims_user_id: String,
    perms: Vec<String>,
    channels: Vec<rustygod_db::commerce::ChannelView>,
) -> gen::User {
    gen::User {
        id: Some(ID(crate::common::gid("User", &claims_user_id))),
        private_metadata: vec![],
        metadata: crate::common::json_to_metadata_items(
            &serde_json::to_value(&user.metadata).unwrap_or(serde_json::Value::Null),
        ),
        email: Some(user.email),
        first_name: Some(user.first_name),
        last_name: Some(user.last_name),
        is_staff: Some(user.is_staff),
        is_active: Some(user.is_active),
        is_confirmed: Some(user.is_confirmed),
        addresses: vec![],
        note: user.note.clone(),
        user_permissions: perms.into_iter().map(|code| GqlUserPermission { code: code.clone(), name: code }).collect(),
        permission_groups: vec![],
        editable_groups: vec![],
        accessible_channels: channels.into_iter().map(|c| gen::Channel {
            id: Some(ID(crate::common::gid("Channel", &c.id))),
            private_metadata: vec![],
            metadata: vec![],
            slug: Some(c.slug.clone()),
            name: Some(c.slug),
            is_active: Some(c.is_active),
            currency_code: Some(c.currency_code),
            has_orders: None,
            default_country: Some(crate::common::GqlCountryDisplay { code: "US".into(), country: "United States".into() }),
            warehouses: vec![],
            stock_settings: Some(crate::common::GqlStockSettings { allocation_strategy: "prioritize-sorting-order".into() }),
            order_settings: None,
            checkout_settings: None,
            payment_settings: None,
            tax_configuration: None,
        }).collect(),
        restricted_access_to_channels: Some(false),
        default_shipping_address: None,
        default_billing_address: None,
        external_reference: user.external_reference.clone(),
        customer_type: None,
        last_login: user.last_login.map(|t| t.into()),
        date_joined: Some(user.date_joined.into()),
    }
}

#[derive(Default)]
pub struct AccountQuery;

#[Object]
impl AccountQuery {
    async fn me(&self, ctx: &Context<'_>) -> Result<Option<gen::User>> {
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
        Ok(Some(to_gen_user(user, claims.user_id, perms, channels)))
    }

    /// Dashboard customer list: non-staff users with search/filter/where/sort.
    /// Saleor `customers` = `is_staff=False`; per-row `orders { totalCount }`
    /// resolves via the real `User.orders` method.
    async fn customers(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        filter: Option<gen::CustomerFilterInput>,
        #[graphql(name = "where")] where_input: Option<gen::CustomerWhereInput>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::UserSortingInput>,
        search: Option<String>,
    ) -> Result<gen::UserCountableConnection> {
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use rustygod_db::entities::account_user::{Column as UCol, Entity as UEnt};
        use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
        let mut cond = Condition::all().add(UCol::IsStaff.eq(false));
        let mut search_terms: Vec<String> = vec![];
        if let Some(s) = search.clone() { search_terms.push(s); }
        if let Some(flt) = filter.as_ref() {
            let ids = crate::catalog::gid_vec(flt.ids.clone());
            if !ids.is_empty() { cond = cond.add(UCol::Id.is_in(ids)); }
            if let Some(s) = flt.search.as_ref() { search_terms.push(s.clone()); }
        }
        if let Some(w) = where_input.as_ref() {
            let ids = crate::catalog::gid_vec(w.ids.clone());
            if !ids.is_empty() { cond = cond.add(UCol::Id.is_in(ids)); }
            let (eq, one) = crate::catalog::str_filter(w.email.clone());
            if let Some(e) = eq { cond = cond.add(UCol::Email.eq(e)); }
            if !one.is_empty() { cond = cond.add(UCol::Email.is_in(one)); }
            let (feq, fone) = crate::catalog::str_filter(w.first_name.clone());
            if let Some(e) = feq { cond = cond.add(UCol::FirstName.eq(e)); }
            if !fone.is_empty() { cond = cond.add(UCol::FirstName.is_in(fone)); }
            let (leq, lone) = crate::catalog::str_filter(w.last_name.clone());
            if let Some(e) = leq { cond = cond.add(UCol::LastName.eq(e)); }
            if !lone.is_empty() { cond = cond.add(UCol::LastName.is_in(lone)); }
            if let Some(a) = w.is_active { cond = cond.add(UCol::IsActive.eq(a)); }
            if let Some(r) = w.date_joined.as_ref() {
                if let Some(gte) = r.gte { cond = cond.add(UCol::DateJoined.gte(gte)); }
                if let Some(lte) = r.lte { cond = cond.add(UCol::DateJoined.lte(lte)); }
            }
        }
        for s in search_terms.iter().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            let like = format!("%{s}%");
            cond = cond.add(Condition::any()
                .add(UCol::Email.like(like.clone()))
                .add(UCol::FirstName.like(like.clone()))
                .add(UCol::LastName.like(like)));
        }
        let mut q = UEnt::find().filter(cond);
        let asc = sort_by.as_ref().map(|s| matches!(s.direction, gen::OrderDirection::ASC)).unwrap_or(true);
        q = match sort_by.as_ref().map(|s| &s.field) {
            Some(gen::UserSortField::EMAIL) => if asc { q.order_by_asc(UCol::Email) } else { q.order_by_desc(UCol::Email) },
            Some(gen::UserSortField::FIRSTNAME) => if asc { q.order_by_asc(UCol::FirstName) } else { q.order_by_desc(UCol::FirstName) },
            Some(gen::UserSortField::LASTNAME) => if asc { q.order_by_asc(UCol::LastName) } else { q.order_by_desc(UCol::LastName) },
            Some(gen::UserSortField::CREATEDAT) => if asc { q.order_by_asc(UCol::DateJoined) } else { q.order_by_desc(UCol::DateJoined) },
            _ => q.order_by_asc(UCol::Email),
        };
        // Slim select: full-model decode crashes on search_vector (tsvector).
        let rows: Vec<(i32, String, String, String, bool, bool, chrono::DateTime<chrono::Utc>)> = q
            .select_only()
            .column(UCol::Id)
            .column(UCol::Email)
            .column(UCol::FirstName)
            .column(UCol::LastName)
            .column(UCol::IsStaff)
            .column(UCol::IsActive)
            .column(UCol::DateJoined)
            .into_tuple()
            .all(db)
            .await
            .map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len() as i32;
        let mut edges = vec![];
        for (i, (uid, email, first, last, staff, active, joined)) in rows.into_iter().skip(off).take(lim).enumerate() {
            // Slim row: the list fragment needs identity fields only, and
            // per-row orders resolve via the real User.orders method.
            let node = slim_list_user(uid, email, first, last, staff, active, joined);
            edges.push(gen::UserCountableEdge {
                node: Some(node),
                cursor: Some(crate::common::encode_cursor(off + i)),
            });
        }
        Ok(gen::UserCountableConnection {
            page_info: Some(crate::common::PageInfo { has_next_page: off + lim < total as usize, has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
            edges,
            total_count: Some(total),
        })
    }

    /// Saleor `user(id, email, externalReference)` (dashboard customer details).
    async fn user(
        &self, ctx: &Context<'_>,
        id: Option<ID>, email: Option<String>, external_reference: Option<String>,
    ) -> Result<Option<gen::User>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use rustygod_db::entities::account_user::{Column as UCol, Entity as UEnt};
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        let uid: Option<i32> = if let Some(i) = id {
            rustygod_db::catalog::parse_gid(&i.0)
        } else if let Some(e) = email {
            UEnt::find().select_only().column(UCol::Id)
                .filter(UCol::Email.eq(e))
                .into_tuple::<i32>().one(db).await.map_err(|e| Error::new(e.to_string()))?
        } else if let Some(x) = external_reference {
            UEnt::find().select_only().column(UCol::Id)
                .filter(UCol::ExternalReference.eq(x))
                .into_tuple::<i32>().one(db).await.map_err(|e| Error::new(e.to_string()))?
        } else { None };
        match uid {
            None => Ok(None),
            Some(uid) => {
                let claims_id = uid.to_string();
                load_gql_user(db, uid, &claims_id).await.map(Some).map_err(|e| Error::new(e.to_string()))
            }
        }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "CreateToken")]
pub struct GqlTokenCreate {
    pub token: String,
    #[graphql(name = "refreshToken")]
    pub refresh_token: String,
    pub user: Option<gen::User>,
    pub errors: Vec<GqlAccountError>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "AccountError")]
pub struct GqlAccountError {
    pub field: Option<String>,
    pub message: String,
    pub code: String,
    #[graphql(name = "addressType")]
    pub address_type: Option<String>,
    pub attributes: Option<Vec<String>>,
}

#[derive(InputObject, Clone, Debug)]
pub struct AccountInput {
    #[graphql(name = "firstName")]
    pub first_name: Option<String>,
    #[graphql(name = "lastName")]
    pub last_name: Option<String>,
    #[graphql(name = "languageCode")]
    pub language_code: Option<String>,
    pub metadata: Option<Vec<crate::common::MetadataInput>>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "UpdateAccount")]
pub struct GqlAccountUpdate {
    pub user: Option<gen::User>,
    pub errors: Vec<GqlAccountError>,
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
            return Ok(GqlTokenCreate { token: String::new(), refresh_token: String::new(), user: None, errors: vec![GqlAccountError { address_type: None, attributes: None, field: Some("email".into()), message: "Invalid credentials".into(), code: "INVALID_CREDENTIALS".into() }] });
        };
        if !user.is_active {
            return Ok(GqlTokenCreate { token: String::new(), refresh_token: String::new(), user: None, errors: vec![GqlAccountError { address_type: None, attributes: None, field: None, message: "User is inactive".into(), code: "INACTIVE".into() }] });
        }
        let ok = match rustygod_core::auth::verify_password(&password, &hash) {
            rustygod_core::auth::PasswordCheck::Ok => true,
            _ => false,
        };
        if !ok {
            return Ok(GqlTokenCreate { token: String::new(), refresh_token: String::new(), user: None, errors: vec![GqlAccountError { address_type: None, attributes: None, field: Some("password".into()), message: "Invalid credentials".into(), code: "INVALID_CREDENTIALS".into() }] });
        }
        let pair = rustygod_core::auth::mint_tokens("http://localhost:8000/graphql/", &user.email, user.id, user.is_staff, &user.jwt_token_key).map_err(|e| Error::new(format!("mint: {e}")))?;
        let claims = rustygod_core::auth::decode(&pair.access).map_err(|e| Error::new(format!("decode: {e}")))?;
        let gql_user = load_gql_user(db, user.id, &claims.user_id).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlTokenCreate { token: pair.access, refresh_token: pair.refresh, user: Some(gql_user), errors: vec![] })
    }

    /// Dashboard profile + navigation pins (`UserAccountUpdate`,
    /// `UpdateUserNavigationPins`): merges names/metadata into the same
    /// `account_user` row Django uses, returns refreshed `user`.
    async fn account_update(&self, ctx: &Context<'_>, input: AccountInput) -> Result<GqlAccountUpdate> {
        let bearer = ctx.data_opt::<Bearer>().map(|b| b.0.as_str().to_string())
            .or_else(|| ctx.data_opt::<GqlContext>().and_then(|g| g.bearer.clone()));
        let Some(token) = bearer else {
            return Ok(err_update("AUTHENTICATION_REQUIRED", "Authentication required"));
        };
        let claims = rustygod_core::auth::decode(&token).map_err(|e| Error::new(format!("auth: {e}")))?;
        let uid = rustygod_core::auth::parse_user_global_id(&claims.user_id).unwrap_or(0);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let user = rustygod_db::entities::account_user::Entity::find_by_id(uid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("user not found"))?;
        {
            use sea_orm::{ActiveModelTrait, Set};
            let current_meta = serde_json::to_value(&user.metadata).unwrap_or(serde_json::Value::Null);
            let mut am: rustygod_db::entities::account_user::ActiveModel = user.into();
            if let Some(first) = input.first_name { am.first_name = Set(first); }
            if let Some(last) = input.last_name { am.last_name = Set(last); }
            if let Some(lang) = input.language_code { am.language_code = Set(lang); }
            if let Some(meta) = input.metadata {
                am.metadata = Set(crate::common::merge_metadata(&current_meta, &meta).into());
            }
            am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
        }
        let gql_user = load_gql_user(db, uid, &claims.user_id).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlAccountUpdate { user: Some(gql_user), errors: vec![] })
    }

    async fn token_refresh(&self, ctx: &Context<'_>, refresh_token: String) -> Result<GqlTokenCreate> {        let claims = rustygod_core::auth::decode(&refresh_token).map_err(|e| Error::new(format!("auth: {e}")))?;
        if claims.token_type != rustygod_core::auth::TOKEN_TYPE_REFRESH {
            return Ok(GqlTokenCreate { token: String::new(), refresh_token: String::new(), user: None, errors: vec![GqlAccountError { address_type: None, attributes: None, field: None, message: "Invalid refresh token".into(), code: "INVALID".into() }] });
        }
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let uid = rustygod_core::auth::parse_user_global_id(&claims.user_id).unwrap_or(0);
        let user = rustygod_db::entities::account_user::Entity::find_by_id(uid).one(db).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("user not found"))?;
        // Rotate check: Django invalidates refresh if jwt_token_key changed.
        if user.jwt_token_key != claims.token {
            return Ok(GqlTokenCreate { token: String::new(), refresh_token: String::new(), user: None, errors: vec![GqlAccountError { address_type: None, attributes: None, field: None, message: "Token expired".into(), code: "EXPIRED".into() }] });
        }
        let pair = rustygod_core::auth::mint_tokens("http://localhost:8000/graphql/", &user.email, user.id, user.is_staff, &user.jwt_token_key).map_err(|e| Error::new(format!("mint: {e}")))?;
        let new_claims = rustygod_core::auth::decode(&pair.access).map_err(|e| Error::new(format!("decode: {e}")))?;
        let gql_user = load_gql_user(db, user.id, &new_claims.user_id).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlTokenCreate { token: pair.access, refresh_token: pair.refresh, user: Some(gql_user), errors: vec![] })
    }
}

fn err_update(code: &str, message: &str) -> GqlAccountUpdate {
    GqlAccountUpdate {
        user: None,
        errors: vec![GqlAccountError { address_type: None, attributes: None, field: None, message: message.into(), code: code.into() }],
    }
}

/// Slim customer-list row (identity fields only; see `customers`).
/// Full gen::User constructor — permissions empty (list fragment doesn't
/// read them), orders resolve per-row via the real `User.orders` method.
fn slim_list_user(
    uid: i32,
    email: String,
    first_name: String,
    last_name: String,
    is_staff: bool,
    is_active: bool,
    date_joined: chrono::DateTime<chrono::Utc>,
) -> gen::User {
    gen::User {
        id: Some(ID(crate::common::gid("User", uid))),
        private_metadata: vec![],
        metadata: vec![],
        email: Some(email),
        first_name: Some(first_name),
        last_name: Some(last_name),
        is_staff: Some(is_staff),
        is_active: Some(is_active),
        is_confirmed: None,
        addresses: vec![],
        note: None,
        user_permissions: vec![],
        permission_groups: vec![],
        editable_groups: vec![],
        accessible_channels: vec![],
        restricted_access_to_channels: None,
        default_shipping_address: None,
        default_billing_address: None,
        external_reference: None,
        customer_type: None,
        last_login: None,
        date_joined: Some(date_joined.into()),
    }
}

async fn load_gql_user(db: &sea_orm::DatabaseConnection, uid: i32, claims_user_id: &str) -> Result<gen::User, sea_orm::DbErr> {
    let user = rustygod_db::entities::account_user::Entity::find_by_id(uid).one(db).await?.ok_or_else(|| sea_orm::DbErr::RecordNotFound("user".into()))?;
    let perms = collect_permissions(db, uid).await.unwrap_or_default();
    let channels = rustygod_db::commerce::list_channels(db).await.unwrap_or_default();
    Ok(to_gen_user(user, claims_user_id.to_string(), perms, channels))
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
        // Return ALL permissions so Dashboard menu renders (was 5, now all).
        let user = rustygod_db::entities::account_user::Entity::find_by_id(user_id).one(db).await?.unwrap();
        if user.is_superuser {
            let all: Vec<(i32, String)> = rustygod_db::entities::permission_permission::Entity::find()
                .select_only().column(rustygod_db::entities::permission_permission::Column::Id)
                .column(rustygod_db::entities::permission_permission::Column::Codename)
                .into_tuple::<(i32, String)>().all(db).await?.into_iter().collect();
            codes = all.into_iter().map(|(_, c)| c).collect();
            if codes.is_empty() {
                codes = vec!["manage_orders".into(), "manage_products".into(), "manage_channels".into(), "manage_staff".into(), "manage_apps".into(), "manage_discounts".into(), "manage_gift_card".into(), "manage_menus".into(), "manage_pages".into(), "manage_shipping".into(), "manage_taxes".into()];
            }
        }
    }
    codes.sort(); codes.dedup();
    // Saleor returns PermissionEnum names (MANAGE_PRODUCTS), not raw codenames.
    Ok(codes.into_iter().map(|c| crate::common::permission_enum_code(&c)).collect())
}
