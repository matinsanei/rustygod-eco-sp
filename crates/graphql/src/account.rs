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

fn aerr(code: &str, field: Option<String>, message: String) -> GqlAccountError {
    GqlAccountError { field, message, code: code.into(), address_type: None, attributes: None }
}

fn serr(field: Option<String>, message: String) -> gen::StaffError {
    gen::StaffError { field, message: Some(message), code: None }
}

fn gerr(field: Option<String>, message: String) -> gen::PermissionGroupError {
    gen::PermissionGroupError { field, message: Some(message), code: None }
}

fn cterr(field: Option<String>, message: String) -> gen::CustomerTypeCreateError {
    gen::CustomerTypeCreateError { field, message: Some(message), code: None }
}

trait FromCreate {
    fn from_create(field: Option<String>, message: String) -> Self;
}

macro_rules! impl_from_create {
    ($($ty:ty),*) => {
        $(impl FromCreate for $ty {
            fn from_create(field: Option<String>, message: String) -> Self {
                Self { field, message: Some(message), code: None }
            }
        })*
    };
}

macro_rules! impl_from_create_attrs {
    ($($ty:ty),*) => {
        $(impl FromCreate for $ty {
            fn from_create(field: Option<String>, message: String) -> Self {
                Self { field, message: Some(message), code: None, attributes: vec![] }
            }
        })*
    };
}

impl_from_create!(
    gen::CustomerTypeUpdateError,
    gen::CustomerTypeDeleteError,
    gen::CustomerTypeUnassignAttributesError
);

impl_from_create_attrs!(
    gen::CustomerTypeAssignAttributesError,
    gen::CustomerTypeReorderAttributesError
);

fn cterr_u(field: Option<String>, message: String) -> gen::CustomerTypeUpdateError {
    gen::CustomerTypeUpdateError::from_create(field, message)
}
fn cterr_d(field: Option<String>, message: String) -> gen::CustomerTypeDeleteError {
    gen::CustomerTypeDeleteError::from_create(field, message)
}
fn cterr_a(field: Option<String>, message: String) -> gen::CustomerTypeAssignAttributesError {
    gen::CustomerTypeAssignAttributesError::from_create(field, message)
}
fn cterr_un(field: Option<String>, message: String) -> gen::CustomerTypeUnassignAttributesError {
    gen::CustomerTypeUnassignAttributesError::from_create(field, message)
}
fn cterr_r(field: Option<String>, message: String) -> gen::CustomerTypeReorderAttributesError {
    gen::CustomerTypeReorderAttributesError::from_create(field, message)
}

fn shop_err(message: String) -> gen::ShopError {
    gen::ShopError { field: None, message: Some(message), code: None }
}

/// Customer-type assembly with ordered attributes (modeling pages).
async fn assemble_customer_type(db: &sea_orm::DatabaseConnection, ct: i32) -> Result<Option<gen::CustomerType>, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
    let Some(m) = rustygod_db::entities::account_customertype::Entity::find_by_id(ct)
        .one(db).await.map_err(|e| e.to_string())? else { return Ok(None) };
    let aids: Vec<i32> = rustygod_db::entities::attribute_attributecustomertype::Entity::find()
        .select_only().column(rustygod_db::entities::attribute_attributecustomertype::Column::AttributeId)
        .filter(rustygod_db::entities::attribute_attributecustomertype::Column::CustomerTypeId.eq(ct))
        .order_by_asc(rustygod_db::entities::attribute_attributecustomertype::Column::SortOrder)
        .into_tuple::<i32>().all(db).await.map_err(|e| e.to_string())?;
    let mut attributes = vec![];
    for aid in aids {
        if let Some(attr) = crate::catalog::assemble_attribute(db, aid).await.map_err(|e| e.to_string())? {
            attributes.push(attr);
        }
    }
    Ok(Some(gen::CustomerType {
        id: Some(ID(crate::common::gid("CustomerType", ct))),
        metadata: crate::common::json_to_metadata_items(&serde_json::to_value(&m.metadata).unwrap_or(serde_json::Value::Null)),
        name: Some(m.name),
        slug: Some(m.slug),
        is_default: Some(m.is_default),
        attributes,
    }))
}

async fn assemble_recipient(db: &sea_orm::DatabaseConnection, rid: i32) -> Result<Option<gen::StaffNotificationRecipient>, String> {
    let row = rustygod_db::entities::account_staffnotificationrecipient::Entity::find_by_id(rid)
        .one(db).await.map_err(|e| e.to_string())?;
    let Some(r) = row else { return Ok(None) };
    let user = match r.user_id {
        Some(uid) => slim_user_row(db, uid).await.map_err(|e| e.to_string())?,
        None => None,
    };
    Ok(Some(gen::StaffNotificationRecipient {
        id: Some(ID(crate::common::gid("StaffNotificationRecipient", rid))),
        user,
        email: r.staff_email,
        active: Some(r.active),
    }))
}

fn bearer_token(ctx: &Context<'_>) -> Result<String> {
    ctx.data_opt::<Bearer>().map(|b| b.0.as_str().to_string())
        .or_else(|| ctx.data_opt::<GqlContext>().and_then(|g| g.bearer.clone()))
        .ok_or_else(|| Error::new("authentication required"))
}

/// Authenticated requester: valid access JWT + active user + matching token
/// key (rotation revokes). Returns (user id, claims user id).
pub(crate) async fn requester(ctx: &Context<'_>, db: &sea_orm::DatabaseConnection) -> Result<(i32, String)> {
    let token = bearer_token(ctx)?;
    let claims = rustygod_core::auth::decode(&token).map_err(|e| Error::new(format!("auth: {e}")))?;
    if claims.token_type != rustygod_core::auth::TOKEN_TYPE_ACCESS {
        return Err(Error::new("authentication required"));
    }
    let uid = rustygod_core::auth::parse_user_global_id(&claims.user_id).unwrap_or(0);
    let user = rustygod_db::entities::account_user::Entity::find_by_id(uid)
        .one(db).await.map_err(|e| Error::new(e.to_string()))?
        .ok_or_else(|| Error::new("user not found"))?;
    if !user.is_active || user.jwt_token_key != claims.token {
        return Err(Error::new("authentication required"));
    }
    Ok((uid, claims.user_id))
}

/// Fine-grained permission gate (Django `permission_required`): superusers
/// bypass, everyone else needs the codename (direct or via groups).
pub async fn require_perm(ctx: &Context<'_>, codename: &str) -> Result<i32> {
    let g = ctx.data::<GqlContext>()?;
    let db = g.db()?;
    let (uid, _) = requester(ctx, db).await?;
    let user = rustygod_db::entities::account_user::Entity::find_by_id(uid)
        .one(db).await.map_err(|e| Error::new(e.to_string()))?
        .ok_or_else(|| Error::new("user not found"))?;
    if user.is_superuser {
        return Ok(uid);
    }
    if rustygod_db::auth::has_permission(db, uid, codename).await.unwrap_or(false) {
        return Ok(uid);
    }
    Err(Error::new(format!("permission denied: {codename} required")))
}

/// Any-of gate for queries readable by several roles (Django OR-perms).
pub(crate) async fn require_any_perm(ctx: &Context<'_>, codenames: &[&str]) -> Result<i32> {
    let g = ctx.data::<GqlContext>()?;
    let db = g.db()?;
    let (uid, _) = requester(ctx, db).await?;
    let user = rustygod_db::entities::account_user::Entity::find_by_id(uid)
        .one(db).await.map_err(|e| Error::new(e.to_string()))?
        .ok_or_else(|| Error::new("user not found"))?;
    if user.is_superuser {
        return Ok(uid);
    }
    for c in codenames {
        if rustygod_db::auth::has_permission(db, uid, c).await.unwrap_or(false) {
            return Ok(uid);
        }
    }
    Err(Error::new(format!("permission denied: one of {} required", codenames.join("/"))))
}

fn lite_channel(c: &rustygod_db::commerce::ChannelView) -> gen::Channel {
    gen::Channel {
        id: Some(ID(crate::common::gid("Channel", &c.id))),
        private_metadata: vec![],
        metadata: vec![],
        slug: Some(c.slug.clone()),
        name: Some(c.slug.clone()),
        is_active: Some(c.is_active),
        currency_code: Some(c.currency_code.clone()),
        has_orders: None,
        default_country: Some(crate::common::GqlCountryDisplay { code: "US".into(), country: "United States".into() }),
        warehouses: vec![],
        stock_settings: None,
        order_settings: None,
        checkout_settings: None,
        payment_settings: None,
        tax_configuration: None,
    }
}

pub fn to_gql_address(m: &rustygod_db::entities::account_address::Model) -> crate::order::GqlAddress {
    crate::order::GqlAddress {
        id: ID(crate::common::gid("Address", m.id)),
        city: m.city.clone(),
        city_area: m.city_area.clone(),
        company_name: m.company_name.clone(),
        country: crate::common::GqlCountryDisplay { code: m.country.clone(), country: m.country.clone() },
        country_area: m.country_area.clone(),
        first_name: m.first_name.clone(),
        last_name: m.last_name.clone(),
        phone: Some(m.phone.clone()),
        postal_code: m.postal_code.clone(),
        street_address_1: m.street_address_1.clone(),
        street_address_2: m.street_address_2.clone(),
        metadata: crate::common::json_to_metadata_items(&serde_json::to_value(&m.metadata).unwrap_or(serde_json::Value::Null)),
        private_metadata: crate::common::json_to_metadata_items(&serde_json::to_value(&m.private_metadata).unwrap_or(serde_json::Value::Null)),
    }
}

async fn user_addresses(db: &sea_orm::DatabaseConnection, uid: i32) -> Result<Vec<crate::order::GqlAddress>, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let aids: Vec<i32> = rustygod_db::entities::account_user_addresses::Entity::find()
        .select_only().column(rustygod_db::entities::account_user_addresses::Column::AddressId)
        .filter(rustygod_db::entities::account_user_addresses::Column::UserId.eq(uid))
        .into_tuple::<i32>().all(db).await.map_err(|e| format!("{e:?}"))?;
    let mut out = vec![];
    for aid in aids {
        if let Some(a) = rustygod_db::entities::account_address::Entity::find_by_id(aid)
            .one(db).await.map_err(|e| format!("{e:?}"))? {
            out.push(to_gql_address(&a));
        }
    }
    Ok(out)
}

async fn user_group_ids(db: &sea_orm::DatabaseConnection, uid: i32) -> Result<Vec<i32>, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    rustygod_db::entities::account_user_groups::Entity::find()
        .select_only().column(rustygod_db::entities::account_user_groups::Column::GroupId)
        .filter(rustygod_db::entities::account_user_groups::Column::UserId.eq(uid))
        .into_tuple::<i32>().all(db).await.map_err(|e| format!("{e:?}"))
}

/// Full user assembly: identity + addresses + defaults + groups +
/// permissions + channels (dashboard details pages read all of these).
async fn assemble_user(db: &sea_orm::DatabaseConnection, uid: i32, claims_user_id: &str) -> Result<gen::User, String> {
    let user = rustygod_db::entities::account_user::Entity::find_by_id(uid)
        .one(db).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "user not found".to_string())?;
    let perms = collect_permissions(db, uid).await.map_err(|e| e.to_string())?;
    let channels = rustygod_db::commerce::list_channels(db).await.unwrap_or_default();
    let addresses = user_addresses(db, uid).await.map_err(|e| e.to_string())?;
    let gids = user_group_ids(db, uid).await.map_err(|e| e.to_string())?;
    let mut groups = vec![];
    for gid in &gids {
        if let Some(g) = assemble_group(db, *gid, true).await.map_err(|e| e.to_string())? {
            groups.push(g);
        }
    }
    let find_addr = |id: Option<i32>| -> Option<crate::order::GqlAddress> {
        id.and_then(|w| addresses.iter().find(|a| a.id.0 == crate::common::gid("Address", w)).cloned())
    };
    let default_shipping = find_addr(user.default_shipping_address_id);
    let default_billing = find_addr(user.default_billing_address_id);
    Ok(gen::User {
        id: Some(ID(crate::common::gid("User", claims_user_id))),
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
        addresses,
        note: user.note.clone(),
        user_permissions: perms.into_iter().map(|code| GqlUserPermission { code: code.clone(), name: code }).collect(),
        permission_groups: groups.clone(),
        editable_groups: groups,
        accessible_channels: channels.iter().map(lite_channel).collect(),
        restricted_access_to_channels: Some(false),
        default_shipping_address: default_shipping,
        default_billing_address: default_billing,
        external_reference: user.external_reference.clone(),
        customer_type: None,
        last_login: user.last_login.map(|t| t.into()),
        date_joined: Some(user.date_joined.into()),
    })
}

/// Permission-group assembly (dashboard group pages + `User.permissionGroups`).
async fn assemble_group(db: &sea_orm::DatabaseConnection, gid: i32, can_manage: bool) -> Result<Option<gen::Group>, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let Some(g) = rustygod_db::entities::account_group::Entity::find_by_id(gid)
        .one(db).await.map_err(|e| e.to_string())? else { return Ok(None) };
    let member_ids: Vec<i32> = rustygod_db::entities::account_user_groups::Entity::find()
        .select_only().column(rustygod_db::entities::account_user_groups::Column::UserId)
        .filter(rustygod_db::entities::account_user_groups::Column::GroupId.eq(gid))
        .into_tuple::<i32>().all(db).await.map_err(|e| e.to_string())?;
    let mut users = vec![];
    for uid in member_ids {
        // Slim rows break the User->Group->User cycle (dashboard refetches).
        if let Some(row) = slim_user_row(db, uid).await.map_err(|e| e.to_string())? {
            users.push(Box::new(row));
        }
    }
    let perm_ids: Vec<i32> = rustygod_db::entities::account_group_permissions::Entity::find()
        .select_only().column(rustygod_db::entities::account_group_permissions::Column::PermissionId)
        .filter(rustygod_db::entities::account_group_permissions::Column::GroupId.eq(gid))
        .into_tuple::<i32>().all(db).await.map_err(|e| e.to_string())?;
    let mut permissions = vec![];
    for pid in perm_ids {
        if let Some(p) = rustygod_db::entities::permission_permission::Entity::find_by_id(pid)
            .one(db).await.map_err(|e| e.to_string())? {
            permissions.push(crate::commerce::GqlPermission {
                code: crate::common::permission_enum_code(&p.codename),
                name: p.name,
            });
        }
    }
    let channel_ids: Vec<i32> = rustygod_db::entities::account_group_channels::Entity::find()
        .select_only().column(rustygod_db::entities::account_group_channels::Column::ChannelId)
        .filter(rustygod_db::entities::account_group_channels::Column::GroupId.eq(gid))
        .into_tuple::<i32>().all(db).await.map_err(|e| e.to_string())?;
    let all_channels = rustygod_db::commerce::list_channels(db).await.unwrap_or_default();
    Ok(Some(gen::Group {
        id: Some(ID(crate::common::gid("Group", gid))),
        name: Some(g.name),
        users,
        permissions,
        user_can_manage: Some(can_manage),
        accessible_channels: all_channels.iter()
            .filter(|c| channel_ids.contains(&c.id))
            .map(lite_channel).collect(),
        restricted_access_to_channels: Some(g.restricted_access_to_channels),
    }))
}

async fn slim_user_row(db: &sea_orm::DatabaseConnection, uid: i32) -> Result<Option<gen::User>, String> {
    use sea_orm::{EntityTrait, QuerySelect};
    use rustygod_db::entities::account_user::{Column as UCol, Entity as UEnt};
    let row: Option<(i32, String, String, String, bool, bool, chrono::DateTime<chrono::Utc>)> = UEnt::find_by_id(uid)
        .select_only()
        .column(UCol::Id).column(UCol::Email).column(UCol::FirstName).column(UCol::LastName)
        .column(UCol::IsStaff).column(UCol::IsActive).column(UCol::DateJoined)
        .into_tuple().one(db).await.map_err(|e| e.to_string())?;
    Ok(row.map(|(id, email, first, last, staff, active, joined)| slim_list_user(id, email, first, last, staff, active, joined)))
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
        let _user = rustygod_db::entities::account_user::Entity::find_by_id(uid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("user not found"))?;
        // Permissions: collect from direct + group grants (like server::access).
        assemble_user(db, uid, &claims.user_id).await.map(Some).map_err(Error::new)
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
        require_any_perm(ctx, &["manage_users", "manage_orders"]).await?;
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
        require_any_perm(ctx, &["manage_staff", "manage_users", "manage_orders"]).await?;
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
                assemble_user(db, uid, &claims_id).await.map(Some).map_err(Error::new)
            }
        }
    }

    /// Dashboard staff list (Django `resolve_staff_users` = `is_staff=True`).
    async fn staff_users(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        filter: Option<gen::StaffUserInput>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::UserSortingInput>,
    ) -> Result<Option<gen::UserCountableConnection>> {
        crate::account::require_perm(ctx, "manage_staff").await?;
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use rustygod_db::entities::account_user::{Column as UCol, Entity as UEnt};
        use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
        let mut cond = Condition::all().add(UCol::IsStaff.eq(true));
        if let Some(f) = filter.as_ref() {
            let ids = crate::catalog::gid_vec(f.ids.clone());
            if !ids.is_empty() { cond = cond.add(UCol::Id.is_in(ids)); }
            if let Some(s) = f.search.as_ref().filter(|s| !s.trim().is_empty()) {
                let like = format!("%{s}%");
                cond = cond.add(Condition::any()
                    .add(UCol::Email.like(like.clone()))
                    .add(UCol::FirstName.like(like.clone()))
                    .add(UCol::LastName.like(like)));
            }
            match f.status.as_ref().map(|s| format!("{s:?}")).as_deref() {
                Some("ACTIVE") => { cond = cond.add(UCol::IsActive.eq(true)); }
                Some("DEACTIVATED") => { cond = cond.add(UCol::IsActive.eq(false)); }
                _ => {}
            }
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
        let rows: Vec<(i32, String, String, String, bool, bool, chrono::DateTime<chrono::Utc>)> = q
            .select_only()
            .column(UCol::Id).column(UCol::Email).column(UCol::FirstName).column(UCol::LastName)
            .column(UCol::IsStaff).column(UCol::IsActive).column(UCol::DateJoined)
            .into_tuple().all(db).await.map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len() as i32;
        let mut edges = vec![];
        for (i, (uid, email, first, last, staff, active, joined)) in rows.into_iter().skip(off).take(lim).enumerate() {
            edges.push(gen::UserCountableEdge {
                node: Some(slim_list_user(uid, email, first, last, staff, active, joined)),
                cursor: Some(crate::common::encode_cursor(off + i)),
            });
        }
        Ok(Some(gen::UserCountableConnection {
            page_info: Some(crate::common::PageInfo { has_next_page: off + lim < total as usize, has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
            edges,
            total_count: Some(total),
        }))
    }

    /// Dashboard permission-group list (Django `resolve_permission_groups`).
    async fn permission_groups(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        filter: Option<gen::PermissionGroupFilterInput>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::PermissionGroupSortingInput>,
    ) -> Result<Option<gen::GroupCountableConnection>> {
        crate::account::require_perm(ctx, "manage_staff").await?;
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use rustygod_db::entities::account_group::{Column as GCol, Entity as GEnt};
        use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
        let mut cond = Condition::all();
        if let Some(f) = filter.as_ref() {
            let ids = crate::catalog::gid_vec(f.ids.clone());
            if !ids.is_empty() { cond = cond.add(GCol::Id.is_in(ids)); }
            if let Some(s) = f.search.as_ref().filter(|s| !s.trim().is_empty()) {
                cond = cond.add(GCol::Name.like(format!("%{s}%")));
            }
        }
        let mut q = GEnt::find().filter(cond);
        let desc = sort_by.as_ref().map(|s| s.direction == gen::OrderDirection::DESC).unwrap_or(false);
        q = if desc { q.order_by_desc(GCol::Name) } else { q.order_by_asc(GCol::Name) };
        let rows: Vec<(i32,)> = q.select_only().column(GCol::Id)
            .into_tuple().all(db).await.map_err(|e| Error::new(e.to_string()))?;
        let mut edges = vec![];
        for (i, (gid,)) in rows.iter().skip(off).take(lim).enumerate() {
            if let Some(node) = assemble_group(db, *gid, true).await.map_err(Error::new)? {
                edges.push(gen::GroupCountableEdge { node: Some(node) });
                let _ = off + i;
            }
        }
        Ok(Some(gen::GroupCountableConnection {
            page_info: Some(crate::common::PageInfo { has_next_page: off + lim < rows.len(), has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
            edges,
        }))
    }

    /// Dashboard permission-group details.
    async fn permission_group(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::Group>> {
        crate::account::require_perm(ctx, "manage_staff").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(gid) = rustygod_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        assemble_group(db, gid, true).await.map_err(Error::new)
    }

    /// Single address (Django: MANAGE_USERS sees any, otherwise only owned).
    async fn address(&self, ctx: &Context<'_>, id: ID) -> Result<Option<crate::order::GqlAddress>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(aid) = rustygod_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        let addr = rustygod_db::entities::account_address::Entity::find_by_id(aid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?;
        let Some(a) = addr else { return Ok(None) };
        if crate::account::require_any_perm(ctx, &["manage_users", "manage_staff", "manage_orders"]).await.is_ok() {
            return Ok(Some(to_gql_address(&a)));
        }
        // Owner path: bearer must own the address.
        let (uid, _) = requester(ctx, db).await?;
        if address_owner(db, aid).await?.is_some_and(|o| o == uid) {
            return Ok(Some(to_gql_address(&a)));
        }
        Err(Error::new("permission denied"))
    }

    // ------------------------------------------------------------------
    // Customer types (dashboard modeling section)
    // ------------------------------------------------------------------

    async fn customer_type(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::CustomerType>> {
        require_any_perm(ctx, &["manage_staff", "manage_users"]).await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(ct) = rustygod_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        assemble_customer_type(db, ct).await.map_err(Error::new)
    }

    async fn customer_types(
        &self, ctx: &Context<'_>,
        #[graphql(name = "where")] where_input: Option<gen::CustomerTypeWhereInput>,
        search: Option<String>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::CustomerTypeSortingInput>,
        before: Option<String>, after: Option<String>, first: Option<i32>, last: Option<i32>,
    ) -> Result<Option<gen::CustomerTypeCountableConnection>> {
        require_any_perm(ctx, &["manage_staff", "manage_users"]).await?;
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use rustygod_db::entities::account_customertype::{Column as CCol, Entity as CEnt};
        use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
        fn apply_where(cond: Condition, w: &gen::CustomerTypeWhereInput) -> Condition {
            let mut c = cond;
            let ids = crate::catalog::gid_vec(w.ids.clone());
            if !ids.is_empty() { c = c.add(CCol::Id.is_in(ids)); }
            let (eq, one) = crate::catalog::str_filter(w.name.clone());
            if let Some(e) = eq { c = c.add(CCol::Name.eq(e)); }
            if !one.is_empty() { c = c.add(CCol::Name.is_in(one)); }
            let (seq, sone) = crate::catalog::str_filter(w.slug.clone());
            if let Some(e) = seq { c = c.add(CCol::Slug.eq(e)); }
            if !sone.is_empty() { c = c.add(CCol::Slug.is_in(sone)); }
            if let Some(d) = w.is_default { c = c.add(CCol::IsDefault.eq(d)); }
            for sub in w.and.as_ref().map(|v| v.as_slice()).unwrap_or(&[]) {
                c = c.add(apply_where(Condition::all(), sub));
            }
            if let Some(orw) = w.or.as_ref().filter(|v| !v.is_empty()) {
                let mut any = Condition::any();
                for sub in orw { any = any.add(apply_where(Condition::all(), sub)); }
                c = c.add(any);
            }
            c
        }
        let mut cond = Condition::all();
        if let Some(w) = where_input.as_ref() { cond = apply_where(cond, w); }
        if let Some(s) = search.as_ref().filter(|s| !s.trim().is_empty()) {
            let like = format!("%{s}%");
            cond = cond.add(Condition::any().add(CCol::Name.like(like.clone())).add(CCol::Slug.like(like)));
        }
        let mut q = CEnt::find().filter(cond);
        let desc = sort_by.as_ref().map(|s| s.direction == gen::OrderDirection::DESC).unwrap_or(false);
        q = match sort_by.as_ref().map(|s| &s.field) {
            Some(gen::CustomerTypeSortField::SLUG) => if desc { q.order_by_desc(CCol::Slug) } else { q.order_by_asc(CCol::Slug) },
            _ => if desc { q.order_by_desc(CCol::Name) } else { q.order_by_asc(CCol::Name) },
        };
        let rows: Vec<i32> = q.select_only().column(CCol::Id)
            .into_tuple::<i32>().all(db).await.map_err(|e| Error::new(e.to_string()))?;
        let mut edges = vec![];
        for (i, ct) in rows.iter().skip(off).take(lim).enumerate() {
            if let Some(node) = assemble_customer_type(db, *ct).await.map_err(Error::new)? {
                edges.push(gen::CustomerTypeCountableEdge { node: Some(node) });
                let _ = i;
            }
        }
        Ok(Some(gen::CustomerTypeCountableConnection {
            page_info: Some(crate::common::PageInfo { has_next_page: off + lim < rows.len(), has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
            edges,
        }))
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

/// Custom (non-dashboard-schema) auth payloads — same error shape as
/// Saleor, exposed for API completeness (storefront + tooling).
#[derive(SimpleObject, Clone)]
pub struct GqlTokenVerify {
    #[graphql(name = "isValid")]
    pub is_valid: bool,
    pub errors: Vec<GqlAccountError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlDeactivateAll {
    pub errors: Vec<GqlAccountError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlBulkResult {
    pub count: Option<i32>,
    pub errors: Vec<GqlAccountError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlAccountRegister {
    pub user: Option<gen::User>,
    pub errors: Vec<GqlAccountError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlConfirmResult {
    pub user: Option<gen::User>,
    pub errors: Vec<GqlAccountError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlAccountDelete {
    pub errors: Vec<GqlAccountError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlEmailChange {
    pub errors: Vec<GqlAccountError>,
}

async fn resolve_user_id(db: &sea_orm::DatabaseConnection, id: Option<ID>, ext: Option<String>) -> Result<Option<i32>> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    use rustygod_db::entities::account_user::{Column as UCol, Entity as UEnt};
    if let Some(i) = id {
        return Ok(rustygod_db::catalog::parse_gid(&i.0));
    }
    if let Some(x) = ext {
        return Ok(UEnt::find().select_only().column(UCol::Id)
            .filter(UCol::ExternalReference.eq(x))
            .into_tuple::<i32>().one(db).await.map_err(|e| Error::new(e.to_string()))?);
    }
    Ok(None)
}

async fn address_owner(db: &sea_orm::DatabaseConnection, addr_id: i32) -> Result<Option<i32>> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    rustygod_db::entities::account_user_addresses::Entity::find()
        .select_only().column(rustygod_db::entities::account_user_addresses::Column::UserId)
        .filter(rustygod_db::entities::account_user_addresses::Column::AddressId.eq(addr_id))
        .into_tuple::<i32>().one(db).await.map_err(|e| Error::new(e.to_string()))
}

fn address_input_of(input: &gen::AddressInput) -> rustygod_db::account_writes::AddressInput {
    rustygod_db::account_writes::AddressInput {
        first_name: input.first_name.clone().unwrap_or_default(),
        last_name: input.last_name.clone().unwrap_or_default(),
        company_name: input.company_name.clone(),
        street_1: input.street_address1.clone().unwrap_or_default(),
        street_2: input.street_address2.clone(),
        city: input.city.clone().unwrap_or_default(),
        postal_code: input.postal_code.clone().unwrap_or_default(),
        country: input.country.as_ref().map(|c| format!("{c:?}")).unwrap_or_default(),
        country_area: input.country_area.clone(),
        city_area: input.city_area.clone(),
        phone: input.phone.clone(),
    }
}

/// Default addresses riding on customer create/update (Django CustomerInput).
async fn apply_default_addresses(
    db: &sea_orm::DatabaseConnection,
    uid: i32,
    shipping: Option<&gen::AddressInput>,
    billing: Option<&gen::AddressInput>,
) -> Result<(), String> {
    if let Some(s) = shipping {
        let aid = rustygod_db::account_writes::create_address(db, uid, &address_input_of(s)).await.map_err(|e| e.to_string())?;
        rustygod_db::account_writes::set_default_address(db, uid, rustygod_db::account_writes::DefaultKind::Shipping, aid).await.map_err(|e| e.to_string())?;
    }
    if let Some(b) = billing {
        let aid = rustygod_db::account_writes::create_address(db, uid, &address_input_of(b)).await.map_err(|e| e.to_string())?;
        rustygod_db::account_writes::set_default_address(db, uid, rustygod_db::account_writes::DefaultKind::Billing, aid).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Default)]
pub struct AccountMutation;

#[Object]
impl AccountMutation {
    /// Dashboard login: mirrors `saleor/account` tokenCreate (email+password → RS256 JWTs).
    /// Uses the same `RSA_PRIVATE_KEY` and `account_user` rows Django uses.
    async fn token_create(&self, ctx: &Context<'_>, email: String, password: String) -> Result<GqlTokenCreate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        // Throttle first (Django checks BEFORE password work; unknown emails
        // get a dummy verify so timing doesn't leak existence).
        let ip = "unknown";
        let bad = |field: Option<String>, code: &str, message: &str| GqlTokenCreate {
            token: String::new(), refresh_token: String::new(), user: None,
            errors: vec![GqlAccountError { address_type: None, attributes: None, field, message: message.into(), code: code.into() }],
        };
        if rustygod_db::account_writes::throttle_check(db, ip, &email).await.is_err() {
            return Ok(bad(None, "LOGIN_THROTTLED", "Too many login attempts, try again later"));
        }
        let fail = || async {
            let _ = rustygod_db::account_writes::throttle_fail(db, ip, &email).await;
            // Dummy verify: equalize timing for unknown emails (Django throttling).
            let _ = rustygod_core::auth::verify_password(&password, "pbkdf2_sha256$600000$dummy$dummy");
        };
        let email_lc = email.trim().to_lowercase();
        let Some((user, hash)) = rustygod_db::auth::find_for_login(db, &email_lc).await.map_err(|e| Error::new(e.to_string()))? else {
            fail().await;
            return Ok(bad(Some("email".into()), "INVALID_CREDENTIALS", "Invalid credentials"));
        };
        if !user.is_active {
            fail().await;
            return Ok(bad(None, "INACTIVE", "User is inactive"));
        }
        // Load confirmation state (Django ACCOUNT_NOT_CONFIRMED).
        let confirmed: bool = rustygod_db::entities::account_user::Entity::find_by_id(user.id)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .map(|u| u.is_confirmed).unwrap_or(true);
        if !confirmed {
            fail().await;
            return Ok(bad(None, "ACCOUNT_NOT_CONFIRMED", "Account needs confirmation"));
        }
        let ok = matches!(rustygod_core::auth::verify_password(&password, &hash), rustygod_core::auth::PasswordCheck::Ok);
        if !ok {
            fail().await;
            return Ok(bad(Some("password".into()), "INVALID_CREDENTIALS", "Invalid credentials"));
        }
        let _ = rustygod_db::account_writes::throttle_clear(db, ip, &email).await;
        let _ = rustygod_db::account_writes::bump_last_login(db, user.id).await;
        let pair = rustygod_core::auth::mint_tokens("http://localhost:8000/graphql/", &user.email, user.id, user.is_staff, &user.jwt_token_key).map_err(|e| Error::new(format!("mint: {e}")))?;
        let claims = rustygod_core::auth::decode(&pair.access).map_err(|e| Error::new(format!("decode: {e}")))?;
        let gql_user = assemble_user(db, user.id, &claims.user_id).await.map_err(Error::new)?;
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
        let gql_user = assemble_user(db, uid, &claims.user_id).await.map_err(Error::new)?;
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
        let gql_user = assemble_user(db, user.id, &claims.user_id).await.map_err(Error::new)?;
        Ok(GqlTokenCreate { token: pair.access, refresh_token: pair.refresh, user: Some(gql_user), errors: vec![] })
    }

    // ------------------------------------------------------------------
    // Staff / customer management (MANAGE_STAFF / MANAGE_USERS)
    // ------------------------------------------------------------------

    async fn staff_create(&self, ctx: &Context<'_>, input: gen::StaffCreateInput) -> Result<gen::StaffCreate> {
        let req = require_perm(ctx, "manage_staff").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::StaffCreate { user: None, errors: vec![serr(None, m)] };
        let Some(email) = input.email.clone().filter(|e| !e.trim().is_empty()) else {
            return Ok(err("email is required".into()));
        };
        let nu = rustygod_db::account_writes::NewUser {
            email,
            first_name: input.first_name.clone().unwrap_or_default(),
            last_name: input.last_name.clone().unwrap_or_default(),
            is_active: input.is_active.unwrap_or(true),
            note: input.note.clone(),
            language_code: None,
            group_ids: crate::catalog::gid_vec(input.add_groups.clone()),
            permissions: vec![],
            metadata: input.metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            private_metadata: input.private_metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            external_reference: None,
            customer_type_id: None,
            is_confirmed: true,
        };
        match rustygod_db::account_writes::create_staff(db, req, &nu).await {
            Ok(uid) => Ok(gen::StaffCreate {
                user: Some(assemble_user(db, uid, &uid.to_string()).await.map_err(Error::new)?),
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn staff_update(&self, ctx: &Context<'_>, id: ID, input: gen::StaffUpdateInput) -> Result<gen::StaffUpdate> {
        let req = require_perm(ctx, "manage_staff").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::StaffUpdate { user: None, errors: vec![serr(None, m)] };
        let Some(uid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(err("bad user id".into()));
        };
        // add/remove group semantics (Django StaffUpdateInput).
        let mut group_ids = if input.add_groups.is_some() || input.remove_groups.is_some() {
            let mut cur = user_group_ids(db, uid).await.map_err(Error::new)?;
            for a in crate::catalog::gid_vec(input.add_groups.clone()) {
                if !cur.contains(&a) { cur.push(a); }
            }
            for r in crate::catalog::gid_vec(input.remove_groups.clone()) {
                cur.retain(|x| x != &r);
            }
            Some(cur)
        } else { None };
        let _ = &mut group_ids;
        let patch = rustygod_db::account_writes::UserPatch {
            email: input.email.clone(),
            first_name: input.first_name.clone(),
            last_name: input.last_name.clone(),
            is_active: input.is_active,
            note: input.note.clone(),
            language_code: None,
            group_ids,
            permissions: None,
            metadata: input.metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            private_metadata: input.private_metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            external_reference: None,
            customer_type_id: None,
            is_confirmed: None,
        };
        match rustygod_db::account_writes::update_user(db, req, uid, true, &patch).await {
            Ok(()) => Ok(gen::StaffUpdate {
                user: Some(assemble_user(db, uid, &uid.to_string()).await.map_err(Error::new)?),
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn staff_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::StaffDelete> {
        let req = require_perm(ctx, "manage_staff").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(uid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::StaffDelete { errors: vec![serr(Some("id".into()), "bad user id".into())] });
        };
        match rustygod_db::account_writes::delete_user(db, req, uid, true).await {
            Ok(()) => Ok(gen::StaffDelete { errors: vec![] }),
            Err(e) => Ok(gen::StaffDelete { errors: vec![serr(None, e.to_string())] }),
        }
    }

    async fn staff_bulk_delete(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<GqlBulkResult> {
        let req = require_perm(ctx, "manage_staff").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let uids: Vec<i32> = ids.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect();
        match rustygod_db::account_writes::bulk_delete_users(db, req, &uids, true).await {
            Ok(n) => Ok(GqlBulkResult { count: Some(n as i32), errors: vec![] }),
            Err(e) => Ok(GqlBulkResult { count: None, errors: vec![aerr("INVALID", None, e.to_string())] }),
        }
    }

    async fn customer_create(&self, ctx: &Context<'_>, input: gen::UserCreateInput) -> Result<gen::CustomerCreate> {
        let _req = require_perm(ctx, "manage_users").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::CustomerCreate { user: None, errors: vec![aerr("INVALID", None, m)] };
        let Some(email) = input.email.clone().filter(|e| !e.trim().is_empty()) else {
            return Ok(err("email is required".into()));
        };
        let nu = rustygod_db::account_writes::NewUser {
            email,
            first_name: input.first_name.clone().unwrap_or_default(),
            last_name: input.last_name.clone().unwrap_or_default(),
            is_active: input.is_active.unwrap_or(true),
            note: input.note.clone(),
            language_code: input.language_code.as_ref().map(|l| crate::gen::language_code_value(l).to_string()),
            group_ids: vec![],
            permissions: vec![],
            metadata: input.metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            private_metadata: input.private_metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            external_reference: input.external_reference.clone(),
            customer_type_id: input.customer_type.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)),
            is_confirmed: input.is_confirmed.unwrap_or(true),
        };
        match rustygod_db::account_writes::create_customer(db, &nu).await {
            Ok(uid) => {
                // Default addresses ride on the create (Django CustomerInput).
                if let Err(e) = apply_default_addresses(db, uid, input.default_shipping_address.as_ref(), input.default_billing_address.as_ref()).await {
                    return Ok(err(e));
                }
                Ok(gen::CustomerCreate {
                    user: Some(assemble_user(db, uid, &uid.to_string()).await.map_err(Error::new)?),
                    errors: vec![],
                })
            }
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn customer_update(
        &self, ctx: &Context<'_>,
        #[graphql(name = "externalReference")] external_reference: Option<String>,
        id: Option<ID>, input: gen::CustomerInput,
    ) -> Result<gen::CustomerUpdate> {
        let _req = require_perm(ctx, "manage_users").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::CustomerUpdate { user: None, errors: vec![aerr("INVALID", None, m)] };
        let Some(uid) = resolve_user_id(db, id, external_reference).await? else {
            return Ok(gen::CustomerUpdate { user: None, errors: vec![aerr("NOT_FOUND", Some("id".into()), "user not found".into())] });
        };
        let patch = rustygod_db::account_writes::UserPatch {
            email: input.email.clone(),
            first_name: input.first_name.clone(),
            last_name: input.last_name.clone(),
            is_active: input.is_active,
            note: input.note.clone(),
            language_code: input.language_code.as_ref().map(|l| crate::gen::language_code_value(l).to_string()),
            group_ids: None,
            permissions: None,
            metadata: input.metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            private_metadata: input.private_metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            external_reference: input.external_reference.clone(),
            customer_type_id: input.customer_type.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)),
            is_confirmed: input.is_confirmed,
        };
        match rustygod_db::account_writes::update_user(db, _req, uid, false, &patch).await {
            Ok(()) => {
                if let Err(e) = apply_default_addresses(db, uid, input.default_shipping_address.as_ref(), input.default_billing_address.as_ref()).await {
                    return Ok(err(e));
                }
                Ok(gen::CustomerUpdate {
                    user: Some(assemble_user(db, uid, &uid.to_string()).await.map_err(Error::new)?),
                    errors: vec![],
                })
            }
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn customer_delete(
        &self, ctx: &Context<'_>,
        #[graphql(name = "externalReference")] external_reference: Option<String>,
        id: Option<ID>,
    ) -> Result<gen::CustomerDelete> {
        let req = require_perm(ctx, "manage_users").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(uid) = resolve_user_id(db, id, external_reference).await? else {
            return Ok(gen::CustomerDelete { errors: vec![aerr("NOT_FOUND", Some("id".into()), "user not found".into())] });
        };
        match rustygod_db::account_writes::delete_user(db, req, uid, false).await {
            Ok(()) => {
                let _ = rustygod_db::account_writes::log_event(db, req, "customer_deleted", None, serde_json::json!({"user_id": uid})).await;
                Ok(gen::CustomerDelete { errors: vec![] })
            }
            Err(e) => Ok(gen::CustomerDelete { errors: vec![aerr("INVALID", None, e.to_string())] }),
        }
    }

    async fn customer_bulk_delete(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<gen::CustomerBulkDelete> {
        let req = require_perm(ctx, "manage_users").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let uids: Vec<i32> = ids.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect();
        match rustygod_db::account_writes::bulk_delete_users(db, req, &uids, false).await {
            Ok(_) => Ok(gen::CustomerBulkDelete { errors: vec![] }),
            Err(e) => Ok(gen::CustomerBulkDelete { errors: vec![aerr("INVALID", None, e.to_string())] }),
        }
    }

    async fn user_bulk_set_active(&self, ctx: &Context<'_>, ids: Vec<ID>, #[graphql(name = "isActive")] is_active: bool) -> Result<GqlBulkResult> {
        let req = require_perm(ctx, "manage_users").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let uids: Vec<i32> = ids.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect();
        match rustygod_db::account_writes::bulk_set_active(db, req, &uids, is_active).await {
            Ok(n) => Ok(GqlBulkResult { count: Some(n as i32), errors: vec![] }),
            Err(e) => Ok(GqlBulkResult { count: None, errors: vec![aerr("INVALID", None, e.to_string())] }),
        }
    }

    // ------------------------------------------------------------------
    // Staff-managed addresses (dashboard customer/staff details pages)
    // ------------------------------------------------------------------

    async fn address_create(&self, ctx: &Context<'_>, input: gen::AddressInput, #[graphql(name = "userId")] user_id: ID) -> Result<gen::AddressCreate> {
        let _req = require_perm(ctx, "manage_users").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::AddressCreate { user: None, address: None, errors: vec![aerr("INVALID", None, m)] };
        let Some(uid) = rustygod_db::catalog::parse_gid(&user_id.0) else {
            return Ok(err("bad user id".into()));
        };
        let ai = address_input_of(&input);
        match rustygod_db::account_writes::create_address(db, uid, &ai).await {
            Ok(aid) => {
                let addr = rustygod_db::entities::account_address::Entity::find_by_id(aid)
                    .one(db).await.map_err(|e| Error::new(e.to_string()))?
                    .ok_or_else(|| Error::new("address vanished"))?;
                Ok(gen::AddressCreate {
                    user: Some(assemble_user(db, uid, &uid.to_string()).await.map_err(Error::new)?),
                    address: Some(to_gql_address(&addr)),
                    errors: vec![],
                })
            }
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn address_update(&self, ctx: &Context<'_>, id: ID, input: gen::AddressInput) -> Result<gen::AddressUpdate> {
        let _req = require_perm(ctx, "manage_users").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::AddressUpdate { address: None, errors: vec![aerr("INVALID", None, m)] };
        let Some(aid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(err("bad address id".into()));
        };
        let Some(owner) = address_owner(db, aid).await? else {
            return Ok(gen::AddressUpdate { address: None, errors: vec![aerr("NOT_FOUND", Some("id".into()), "address not found".into())] });
        };
        let patch = rustygod_db::account_writes::AddressPatch {
            first_name: input.first_name.clone(), last_name: input.last_name.clone(),
            company_name: input.company_name.clone(), street_1: input.street_address1.clone(),
            street_2: input.street_address2.clone(), city: input.city.clone(),
            postal_code: input.postal_code.clone(),
            country: input.country.as_ref().map(|c| format!("{c:?}")),
            country_area: input.country_area.clone(), city_area: input.city_area.clone(),
            phone: input.phone.clone(),
        };
        match rustygod_db::account_writes::update_address(db, owner, aid, &patch).await {
            Ok(()) => {
                let addr = rustygod_db::entities::account_address::Entity::find_by_id(aid)
                    .one(db).await.map_err(|e| Error::new(e.to_string()))?
                    .ok_or_else(|| Error::new("address vanished"))?;
                Ok(gen::AddressUpdate { address: Some(to_gql_address(&addr)), errors: vec![] })
            }
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn address_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::AddressDelete> {
        let _req = require_perm(ctx, "manage_users").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::AddressDelete { user: None, errors: vec![aerr("INVALID", None, m)] };
        let Some(aid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(err("bad address id".into()));
        };
        let Some(owner) = address_owner(db, aid).await? else {
            return Ok(gen::AddressDelete { user: None, errors: vec![aerr("NOT_FOUND", Some("id".into()), "address not found".into())] });
        };
        match rustygod_db::account_writes::delete_address(db, owner, aid).await {
            Ok(()) => Ok(gen::AddressDelete {
                user: Some(assemble_user(db, owner, &owner.to_string()).await.map_err(Error::new)?),
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn address_set_default(
        &self, ctx: &Context<'_>,
        #[graphql(name = "addressId")] address_id: ID,
        #[graphql(name = "type")] address_type: gen::AddressTypeEnum,
        #[graphql(name = "userId")] user_id: ID,
    ) -> Result<gen::AddressSetDefault> {
        let _req = require_perm(ctx, "manage_users").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::AddressSetDefault { user: None, errors: vec![aerr("INVALID", None, m)] };
        let (Some(aid), Some(uid)) = (rustygod_db::catalog::parse_gid(&address_id.0), rustygod_db::catalog::parse_gid(&user_id.0)) else {
            return Ok(err("bad id".into()));
        };
        let kind = match format!("{address_type:?}").as_str() {
            "BILLING" => rustygod_db::account_writes::DefaultKind::Billing,
            _ => rustygod_db::account_writes::DefaultKind::Shipping,
        };
        match rustygod_db::account_writes::set_default_address(db, uid, kind, aid).await {
            Ok(()) => Ok(gen::AddressSetDefault {
                user: Some(assemble_user(db, uid, &uid.to_string()).await.map_err(Error::new)?),
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    // ------------------------------------------------------------------
    // Permission groups (MANAGE_STAFF)
    // ------------------------------------------------------------------

    async fn permission_group_create(&self, ctx: &Context<'_>, input: gen::PermissionGroupCreateInput) -> Result<gen::PermissionGroupCreate> {
        let req = require_perm(ctx, "manage_staff").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::PermissionGroupCreate { group: None, errors: vec![gerr(None, m)] };
        let gi = rustygod_db::account_writes::GroupInput {
            name: input.name.clone(),
            permission_codenames: input.add_permissions.as_ref().map(|v| v.as_slice()).unwrap_or(&[])
                .iter().map(|p| format!("{p:?}").to_lowercase()).collect(),
            user_ids: crate::catalog::gid_vec(input.add_users.clone()),
            channel_ids: crate::catalog::gid_vec(input.add_channels.clone()),
            restricted_access_to_channels: input.restricted_access_to_channels.unwrap_or(false),
        };
        match rustygod_db::account_writes::create_group_full(db, req, &gi).await {
            Ok(gid) => Ok(gen::PermissionGroupCreate {
                group: assemble_group(db, gid, true).await.map_err(Error::new)?,
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn permission_group_update(&self, ctx: &Context<'_>, id: ID, input: gen::PermissionGroupUpdateInput) -> Result<gen::PermissionGroupUpdate> {
        let req = require_perm(ctx, "manage_staff").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::PermissionGroupUpdate { group: None, errors: vec![gerr(None, m)] };
        let Some(gid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(err("bad group id".into()));
        };
        let codes = |v: Option<Vec<gen::PermissionEnum>>| v.as_ref().map(|x| x.as_slice()).unwrap_or(&[])
            .iter().map(|p| format!("{p:?}").to_lowercase()).collect::<Vec<_>>();
        let patch = rustygod_db::account_writes::GroupPatch {
            name: input.name.clone(),
            add_permissions: codes(input.add_permissions.clone()),
            remove_permissions: codes(input.remove_permissions.clone()),
            add_users: crate::catalog::gid_vec(input.add_users.clone()),
            remove_users: crate::catalog::gid_vec(input.remove_users.clone()),
            add_channels: crate::catalog::gid_vec(input.add_channels.clone()),
            remove_channels: crate::catalog::gid_vec(input.remove_channels.clone()),
            restricted_access_to_channels: input.restricted_access_to_channels,
        };
        match rustygod_db::account_writes::update_group_full(db, req, gid, &patch).await {
            Ok(()) => Ok(gen::PermissionGroupUpdate {
                group: assemble_group(db, gid, true).await.map_err(Error::new)?,
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn permission_group_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::PermissionGroupDelete> {
        let req = require_perm(ctx, "manage_staff").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(gid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::PermissionGroupDelete { errors: vec![gerr(Some("id".into()), "bad group id".into())] });
        };
        match rustygod_db::account_writes::delete_group_full(db, req, gid).await {
            Ok(()) => Ok(gen::PermissionGroupDelete { errors: vec![] }),
            Err(e) => Ok(gen::PermissionGroupDelete { errors: vec![gerr(None, e.to_string())] }),
        }
    }

    // ------------------------------------------------------------------
    // Password / registration / self-service (custom roots + dashboard)
    // ------------------------------------------------------------------

    async fn password_change(&self, ctx: &Context<'_>, #[graphql(name = "newPassword")] new_password: String, #[graphql(name = "oldPassword")] old_password: Option<String>) -> Result<gen::PasswordChange> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::PasswordChange { errors: vec![aerr("INVALID", None, m)] };
        let (uid, _) = match requester(ctx, db).await {
            Ok(v) => v,
            Err(_) => return Ok(err("authentication required".into())),
        };
        // Old password verified unless the account has an unusable one.
        let hash: Option<String> = rustygod_db::entities::account_user::Entity::find_by_id(uid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?.map(|u| u.password);
        if let Some(h) = hash {
            if !h.starts_with('!') {
                let Some(old) = old_password.filter(|o| !o.is_empty()) else {
                    return Ok(err("old password is required".into()));
                };
                if !matches!(rustygod_core::auth::verify_password(&old, &h), rustygod_core::auth::PasswordCheck::Ok) {
                    return Ok(err("old password is incorrect".into()));
                }
            }
        }
        match rustygod_db::account_writes::set_password(db, uid, &new_password).await {
            Ok(()) => {
                let _ = rustygod_db::account_writes::log_event(db, uid, "password_changed", None, serde_json::json!({})).await;
                Ok(gen::PasswordChange { errors: vec![] })
            }
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn request_password_reset(
        &self, ctx: &Context<'_>,
        channel: Option<String>, email: String, #[graphql(name = "redirectUrl")] redirect_url: String,
    ) -> Result<gen::RequestPasswordReset> {
        let _ = (channel, redirect_url);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        // Never leak existence (Django returns success for unknown emails).
        if let Some((user, _)) = rustygod_db::auth::find_for_login(db, &email.trim().to_lowercase()).await.map_err(|e| Error::new(e.to_string()))? {
            if let Ok(raw) = rustygod_db::account_writes::issue_token(db, "password-reset", user.id, 1, "").await {
                let _ = rustygod_db::account_writes::log_event(db, user.id, "password_reset_link_sent", None, serde_json::json!({})).await;
                // Console-backend parity: the token is logged, not emailed
                // (no SMTP wired; see checklist). Grep the log in dev.
                tracing::info!("password-reset token for {}: {}", user.email, raw);
            }
        }
        Ok(gen::RequestPasswordReset { errors: vec![] })
    }

    async fn set_password(&self, ctx: &Context<'_>, email: String, password: String, token: String) -> Result<gen::SetPassword> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::SetPassword { token: None, refresh_token: None, user: None, errors: vec![aerr("INVALID", None, m)] };
        let (uid, _) = match rustygod_db::account_writes::consume_token(db, "password-reset", &token).await {
            Ok(v) => v,
            Err(e) => return Ok(err(e.to_string())),
        };
        // The token is bound to the email it was issued for.
        let user = rustygod_db::entities::account_user::Entity::find_by_id(uid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("user not found"))?;
        if user.email.to_lowercase() != email.trim().to_lowercase() {
            return Ok(err("token does not match this email".into()));
        }
        if let Err(e) = rustygod_db::account_writes::set_password(db, uid, &password).await {
            return Ok(err(e.to_string()));
        }
        let _ = rustygod_db::account_writes::log_event(db, uid, "password_reset", None, serde_json::json!({})).await;
        if !user.is_active {
            return Ok(gen::SetPassword { token: None, refresh_token: None, user: None, errors: vec![aerr("INACTIVE", None, "User is inactive".into())] });
        }
        // set_password rotated jwt_token_key: re-read so the fresh claims
        // carry the CURRENT key (else the new tokens are born revoked).
        let fresh = rustygod_db::entities::account_user::Entity::find_by_id(uid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("user not found"))?;
        let pair = rustygod_core::auth::mint_tokens("http://localhost:8000/graphql/", &fresh.email, fresh.id, fresh.is_staff, &fresh.jwt_token_key).map_err(|e| Error::new(format!("mint: {e}")))?;
        let gql_user = assemble_user(db, uid, &uid.to_string()).await.map_err(Error::new)?;
        Ok(gen::SetPassword { token: Some(pair.access), refresh_token: Some(pair.refresh), user: Some(gql_user), errors: vec![] })
    }

    // ------------------------------------------------------------------
    // Custom auth roots (Saleor API surface absent from dashboard schema)
    // ------------------------------------------------------------------

    async fn token_verify(&self, ctx: &Context<'_>, token: String) -> Result<GqlTokenVerify> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlTokenVerify { is_valid: false, errors: vec![aerr("INVALID", None, m)] };
        let claims = match rustygod_core::auth::decode(&token) {
            Ok(c) => c,
            Err(e) => return Ok(err(format!("auth: {e}"))),
        };
        if claims.token_type != rustygod_core::auth::TOKEN_TYPE_ACCESS
            && claims.token_type != rustygod_core::auth::TOKEN_TYPE_REFRESH {
            return Ok(err("not an access or refresh token".into()));
        }
        let uid = rustygod_core::auth::parse_user_global_id(&claims.user_id).unwrap_or(0);
        let user = rustygod_db::entities::account_user::Entity::find_by_id(uid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?;
        match user {
            Some(u) if u.is_active && u.jwt_token_key == claims.token => Ok(GqlTokenVerify { is_valid: true, errors: vec![] }),
            Some(_) => Ok(err("token revoked or user inactive".into())),
            None => Ok(err("user not found".into())),
        }
    }

    async fn tokens_deactivate_all(&self, ctx: &Context<'_>) -> Result<GqlDeactivateAll> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let (uid, _) = match requester(ctx, db).await {
            Ok(v) => v,
            Err(_) => return Ok(GqlDeactivateAll { errors: vec![aerr("AUTHENTICATION_REQUIRED", None, "authentication required".into())] }),
        };
        match rustygod_db::account_writes::rotate_key(db, uid).await {
            Ok(()) => Ok(GqlDeactivateAll { errors: vec![] }),
            Err(e) => Ok(GqlDeactivateAll { errors: vec![aerr("INVALID", None, e.to_string())] }),
        }
    }

    async fn account_register(&self, ctx: &Context<'_>, input: gen::AccountRegisterInput) -> Result<GqlAccountRegister> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlAccountRegister { user: None, errors: vec![aerr("INVALID", None, m)] };
        if input.password.len() < 8 {
            return Ok(err("password must be at least 8 characters".into()));
        }
        let nu = rustygod_db::account_writes::NewUser {
            email: input.email.clone(),
            first_name: input.first_name.clone().unwrap_or_default(),
            last_name: input.last_name.clone().unwrap_or_default(),
            is_active: false,
            note: None,
            language_code: input.language_code.as_ref().map(|l| crate::gen::language_code_value(l).to_string()),
            group_ids: vec![],
            permissions: vec![],
            metadata: input.metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            private_metadata: None,
            external_reference: None,
            customer_type_id: None,
            is_confirmed: false,
        };
        let uid = match rustygod_db::account_writes::create_customer(db, &nu).await {
            Ok(u) => u,
            Err(e) => return Ok(err(e.to_string())),
        };
        // Usable password from day one (Django AccountRegister sets it).
        if let Err(e) = rustygod_db::account_writes::set_password(db, uid, &input.password).await {
            return Ok(err(e.to_string()));
        }
        let token = rustygod_db::account_writes::issue_token(db, "confirm", uid, 24 * 7, "").await.map_err(|e| Error::new(e.to_string()))?;
        tracing::info!("account-confirm token for {}: {}", nu.email, token);
        Ok(GqlAccountRegister {
            user: Some(assemble_user(db, uid, &uid.to_string()).await.map_err(Error::new)?),
            errors: vec![],
        })
    }

    async fn confirm_account(&self, ctx: &Context<'_>, token: String) -> Result<GqlConfirmResult> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlConfirmResult { user: None, errors: vec![aerr("INVALID", None, m)] };
        let (uid, _) = match rustygod_db::account_writes::consume_token(db, "confirm", &token).await {
            Ok(v) => v,
            Err(e) => return Ok(err(e.to_string())),
        };
        if let Err(e) = rustygod_db::account_writes::confirm_user(db, uid).await {
            return Ok(err(e.to_string()));
        }
        Ok(GqlConfirmResult {
            user: Some(assemble_user(db, uid, &uid.to_string()).await.map_err(Error::new)?),
            errors: vec![],
        })
    }

    async fn account_request_deletion(&self, ctx: &Context<'_>, #[graphql(name = "redirectUrl")] redirect_url: Option<String>) -> Result<GqlAccountDelete> {
        let _ = redirect_url;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let (uid, _) = match requester(ctx, db).await {
            Ok(v) => v,
            Err(_) => return Ok(GqlAccountDelete { errors: vec![aerr("AUTHENTICATION_REQUIRED", None, "authentication required".into())] }),
        };
        match rustygod_db::account_writes::issue_token(db, "account-delete", uid, 24, "").await {
            Ok(raw) => {
                tracing::info!("account-delete token for user {uid}: {raw}");
                Ok(GqlAccountDelete { errors: vec![] })
            }
            Err(e) => Ok(GqlAccountDelete { errors: vec![aerr("INVALID", None, e.to_string())] }),
        }
    }

    async fn account_delete(&self, ctx: &Context<'_>, token: String) -> Result<GqlAccountDelete> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlAccountDelete { errors: vec![aerr("INVALID", None, m)] };
        let (uid, _) = match rustygod_db::account_writes::consume_token(db, "account-delete", &token).await {
            Ok(v) => v,
            Err(e) => return Ok(err(e.to_string())),
        };
        match rustygod_db::account_writes::delete_own_account(db, uid).await {
            Ok(()) => Ok(GqlAccountDelete { errors: vec![] }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn request_email_change(
        &self, ctx: &Context<'_>,
        password: String, #[graphql(name = "newEmail")] new_email: String, #[graphql(name = "redirectUrl")] redirect_url: String,
    ) -> Result<GqlEmailChange> {
        let _ = redirect_url;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlEmailChange { errors: vec![aerr("INVALID", None, m)] };
        let (uid, _) = match requester(ctx, db).await {
            Ok(v) => v,
            Err(_) => return Ok(err("authentication required".into())),
        };
        let user = rustygod_db::entities::account_user::Entity::find_by_id(uid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("user not found"))?;
        if !user.password.starts_with('!')
            && !matches!(rustygod_core::auth::verify_password(&password, &user.password), rustygod_core::auth::PasswordCheck::Ok) {
            return Ok(err("incorrect password".into()));
        }
        let email = new_email.trim().to_lowercase();
        if email.is_empty() || !email.contains('@') {
            return Ok(err("valid email is required".into()));
        }
        match rustygod_db::account_writes::issue_token(db, "email-change", uid, 1, &email).await {
            Ok(raw) => {
                let _ = rustygod_db::account_writes::log_event(db, uid, "email_changed_request", None, serde_json::json!({"new_email": email})).await;
                tracing::info!("email-change token for user {uid}: {raw}");
                Ok(GqlEmailChange { errors: vec![] })
            }
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn confirm_email_change(&self, ctx: &Context<'_>, token: String) -> Result<GqlEmailChange> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlEmailChange { errors: vec![aerr("INVALID", None, m)] };
        let (uid, new_email) = match rustygod_db::account_writes::consume_token(db, "email-change", &token).await {
            Ok(v) => v,
            Err(e) => return Ok(err(e.to_string())),
        };
        match rustygod_db::account_writes::set_own_email(db, uid, &new_email).await {
            Ok(()) => {
                let _ = rustygod_db::account_writes::log_event(db, uid, "email_changed", None, serde_json::json!({"new_email": new_email})).await;
                Ok(GqlEmailChange { errors: vec![] })
            }
            Err(e) => Ok(err(e.to_string())),
        }
    }

    // ------------------------------------------------------------------
    // Customer-type / bulk / notification mutations
    // ------------------------------------------------------------------

    async fn customer_type_create(&self, ctx: &Context<'_>, input: gen::CustomerTypeCreateInput) -> Result<gen::CustomerTypeCreate> {
        require_perm(ctx, "manage_customer_types_and_attributes").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::CustomerTypeCreate { customer_type: None, errors: vec![cterr(None, m)] };
        let c = rustygod_db::customer_types::CustomerTypeCreate {
            name: input.name.clone(), slug: input.slug.clone(), is_default: input.is_default.unwrap_or(false),
        };
        match rustygod_db::customer_types::create_customer_type(db, &c).await {
            Ok(ct) => Ok(gen::CustomerTypeCreate {
                customer_type: assemble_customer_type(db, ct).await.map_err(Error::new)?,
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn customer_type_update(&self, ctx: &Context<'_>, id: ID, input: gen::CustomerTypeUpdateInput) -> Result<gen::CustomerTypeUpdate> {
        require_perm(ctx, "manage_customer_types_and_attributes").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::CustomerTypeUpdate { customer_type: None, errors: vec![cterr_u(None, m)] };
        let Some(ct) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(err("bad customer type id".into()));
        };
        let patch = rustygod_db::customer_types::CustomerTypePatch {
            name: input.name.clone(), slug: input.slug.clone(), is_default: input.is_default,
        };
        match rustygod_db::customer_types::update_customer_type(db, ct, &patch).await {
            Ok(()) => Ok(gen::CustomerTypeUpdate {
                customer_type: assemble_customer_type(db, ct).await.map_err(Error::new)?,
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn customer_type_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::CustomerTypeDelete> {
        require_perm(ctx, "manage_customer_types_and_attributes").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::CustomerTypeDelete { customer_type: None, errors: vec![cterr_d(None, m)] };
        let Some(ct) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(err("bad customer type id".into()));
        };
        match rustygod_db::customer_types::delete_customer_type(db, ct).await {
            Ok(()) => Ok(gen::CustomerTypeDelete { customer_type: None, errors: vec![] }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn customer_type_assign_attributes(
        &self, ctx: &Context<'_>,
        #[graphql(name = "attributeIds")] attribute_ids: Vec<ID>,
        #[graphql(name = "customerTypeId")] customer_type_id: ID,
    ) -> Result<gen::CustomerTypeAssignAttributes> {
        require_perm(ctx, "manage_customer_types_and_attributes").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::CustomerTypeAssignAttributes { customer_type: None, errors: vec![cterr_a(None, m)] };
        let Some(ct) = rustygod_db::catalog::parse_gid(&customer_type_id.0) else {
            return Ok(err("bad customer type id".into()));
        };
        let aids: Vec<i32> = attribute_ids.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect();
        match rustygod_db::customer_types::assign_attributes(db, ct, &aids).await {
            Ok(()) => Ok(gen::CustomerTypeAssignAttributes {
                customer_type: assemble_customer_type(db, ct).await.map_err(Error::new)?,
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn customer_type_unassign_attributes(
        &self, ctx: &Context<'_>,
        #[graphql(name = "attributeIds")] attribute_ids: Vec<ID>,
        #[graphql(name = "customerTypeId")] customer_type_id: ID,
    ) -> Result<gen::CustomerTypeUnassignAttributes> {
        require_perm(ctx, "manage_customer_types_and_attributes").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::CustomerTypeUnassignAttributes { customer_type: None, errors: vec![cterr_un(None, m)] };
        let Some(ct) = rustygod_db::catalog::parse_gid(&customer_type_id.0) else {
            return Ok(err("bad customer type id".into()));
        };
        let aids: Vec<i32> = attribute_ids.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect();
        match rustygod_db::customer_types::unassign_attributes(db, ct, &aids).await {
            Ok(_) => Ok(gen::CustomerTypeUnassignAttributes {
                customer_type: assemble_customer_type(db, ct).await.map_err(Error::new)?,
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn customer_type_reorder_attributes(
        &self, ctx: &Context<'_>,
        #[graphql(name = "customerTypeId")] customer_type_id: ID,
        moves: Vec<gen::ReorderInput>,
    ) -> Result<gen::CustomerTypeReorderAttributes> {
        require_perm(ctx, "manage_customer_types_and_attributes").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::CustomerTypeReorderAttributes { customer_type: None, errors: vec![cterr_r(None, m)] };
        let Some(ct) = rustygod_db::catalog::parse_gid(&customer_type_id.0) else {
            return Ok(err("bad customer type id".into()));
        };
        let mv: Vec<(i32, i32)> = moves.iter()
            .filter_map(|m| rustygod_db::catalog::parse_gid(&m.id.0).map(|v| (v, m.sort_order.unwrap_or(0))))
            .collect();
        match rustygod_db::customer_types::reorder_attributes(db, ct, &mv).await {
            Ok(()) => Ok(gen::CustomerTypeReorderAttributes {
                customer_type: assemble_customer_type(db, ct).await.map_err(Error::new)?,
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn customer_bulk_update(&self, ctx: &Context<'_>, ids: Vec<ID>, input: gen::CustomerInput) -> Result<GqlBulkResult> {
        let req = require_perm(ctx, "manage_users").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let patch = rustygod_db::account_writes::UserPatch {
            email: None,
            first_name: input.first_name.clone(),
            last_name: input.last_name.clone(),
            is_active: input.is_active,
            note: input.note.clone(),
            language_code: input.language_code.as_ref().map(|l| crate::gen::language_code_value(l).to_string()),
            group_ids: None,
            permissions: None,
            metadata: input.metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            private_metadata: input.private_metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            external_reference: input.external_reference.clone(),
            customer_type_id: input.customer_type.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)),
            is_confirmed: input.is_confirmed,
        };
        let mut n = 0;
        for i in &ids {
            if let Some(uid) = rustygod_db::catalog::parse_gid(&i.0) {
                if rustygod_db::account_writes::update_user(db, req, uid, false, &patch).await.is_ok() {
                    n += 1;
                }
            }
        }
        Ok(GqlBulkResult { count: Some(n), errors: vec![] })
    }

    // ------------------------------------------------------------------
    // Staff notification recipients (MANAGE_SETTINGS)
    // ------------------------------------------------------------------

    async fn staff_notification_recipient_create(&self, ctx: &Context<'_>, input: gen::StaffNotificationRecipientInput) -> Result<gen::StaffNotificationRecipientCreate> {
        require_perm(ctx, "manage_settings").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::StaffNotificationRecipientCreate { staff_notification_recipient: None, errors: vec![shop_err(m)] };
        let uid = input.user.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0));
        match rustygod_db::account_writes::create_notification_recipient(db, uid, input.email.clone(), input.active.unwrap_or(true)).await {
            Ok(rid) => Ok(gen::StaffNotificationRecipientCreate {
                staff_notification_recipient: assemble_recipient(db, rid).await.map_err(Error::new)?,
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn staff_notification_recipient_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::StaffNotificationRecipientDelete> {
        require_perm(ctx, "manage_settings").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(rid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::StaffNotificationRecipientDelete { errors: vec![shop_err("bad id".into())] });
        };
        match rustygod_db::account_writes::delete_notification_recipient(db, rid).await {
            Ok(()) => Ok(gen::StaffNotificationRecipientDelete { errors: vec![] }),
            Err(e) => Ok(gen::StaffNotificationRecipientDelete { errors: vec![shop_err(e.to_string())] }),
        }
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
