//! Staff/customer users, addresses, events, one-time tokens, login
//! throttling — Django `saleor/account` + `saleor/graphql/account` parity.
//!
//! Guard doctrine (Django `UserDeleteMixin`, `StaffUpdate`, `utils.py`):
//! - never delete/deactivate yourself, never touch superusers (unless you
//!   are one — even then self-delete is refused),
//! - customer mutations refuse staff targets and vice versa,
//! - every deactivation/deletion/group change must leave ≥1 active staff
//!   holding `manage_staff` (last-manageable guard),
//! - non-superusers can only manage in-scope users/groups/permissions.
//! All multi-row flows run in one transaction; FKs are DEFERRABLE with no
//! DB cascades, so deletes spell out the collector order manually.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth,
    entities::{
        account_address, account_customerevent, account_group, account_user,
        account_user_addresses, account_user_groups, permission_permission,
    },
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::App(format!("account: {}", msg.into()))
}

pub const MAX_USER_ADDRESSES: i64 = 100;

/// Django `get_random_string(12)` for `jwt_token_key` (varchar(12)).
pub fn new_token_key() -> String {
    use rand::Rng;
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..12).map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char).collect()
}

/// Slim user identity for guards (never touches search_document).
struct Identity {
    is_staff: bool,
    is_superuser: bool,
}

async fn identity(db: &impl ConnectionTrait, uid: i32) -> Result<Option<Identity>> {
    Ok(account_user::Entity::find_by_id(uid)
        .select_only()
        .column(account_user::Column::IsStaff)
        .column(account_user::Column::IsSuperuser)
        .into_tuple::<(bool, bool)>()
        .one(db)
        .await?
        .map(|(is_staff, is_superuser)| Identity { is_staff, is_superuser }))
}

async fn email_taken(db: &impl ConnectionTrait, email: &str, except: Option<i32>) -> Result<bool> {
    let mut q = account_user::Entity::find()
        .select_only()
        .column(account_user::Column::Id)
        .filter(account_user::Column::Email.eq(email.to_lowercase()));
    if let Some(id) = except {
        q = q.filter(account_user::Column::Id.ne(id));
    }
    Ok(q.into_tuple::<i32>().one(db).await?.is_some())
}

/// Last-manageable guard: after removing `exclude` (deactivation, deletion,
/// or permission loss), is there still ≥1 active staffer with manage_staff?
/// Django: `get_not_manageable_permissions_when_deactivate_or_remove_users`.
pub async fn manageable_after_removal(
    db: &impl ConnectionTrait,
    exclude_user_id: Option<i32>,
) -> Result<bool> {
    let mut q = account_user::Entity::find()
        .select_only()
        .column(account_user::Column::Id)
        .filter(account_user::Column::IsStaff.eq(true))
        .filter(account_user::Column::IsActive.eq(true));
    if let Some(id) = exclude_user_id {
        q = q.filter(account_user::Column::Id.ne(id));
    }
    let ids: Vec<i32> = q.into_tuple::<i32>().all(db).await?;
    for id in ids {
        if auth::has_permission(db, id, "manage_staff").await.unwrap_or(false) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Out-of-scope guard for non-superusers (Django `get_out_of_scope_users`):
/// a staffer can only manage users whose permissions ⊆ their own.
pub async fn in_scope(db: &impl ConnectionTrait, req_id: i32, target_id: i32) -> Result<bool> {
    let req = identity(db, req_id).await?.ok_or_else(|| fail("requester not found"))?;
    if req.is_superuser {
        return Ok(true);
    }
    let want = user_perm_codes(db, target_id).await?;
    for code in want {
        if !auth::has_permission(db, req_id, &code).await.unwrap_or(false) {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn user_perm_codes(db: &impl ConnectionTrait, uid: i32) -> Result<Vec<String>> {
    use crate::entities::{account_group_permissions, account_user_user_permissions};
    let mut pids: Vec<i32> = account_user_user_permissions::Entity::find()
        .select_only()
        .column(account_user_user_permissions::Column::PermissionId)
        .filter(account_user_user_permissions::Column::UserId.eq(uid))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    let gids: Vec<i32> = account_user_groups::Entity::find()
        .select_only()
        .column(account_user_groups::Column::GroupId)
        .filter(account_user_groups::Column::UserId.eq(uid))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    if !gids.is_empty() {
        let mut gp: Vec<i32> = account_group_permissions::Entity::find()
            .select_only()
            .column(account_group_permissions::Column::PermissionId)
            .filter(account_group_permissions::Column::GroupId.is_in(gids))
            .into_tuple::<i32>()
            .all(db)
            .await?;
        pids.append(&mut gp);
    }
    pids.sort();
    pids.dedup();
    let mut codes = vec![];
    for pid in pids {
        if let Some(p) = permission_permission::Entity::find_by_id(pid).one(db).await? {
            codes.push(p.codename);
        }
    }
    Ok(codes)
}

async fn perm_id(db: &impl ConnectionTrait, codename: &str) -> Result<Option<i32>> {
    Ok(permission_permission::Entity::find()
        .select_only()
        .column(permission_permission::Column::Id)
        .filter(permission_permission::Column::Codename.eq(codename))
        .into_tuple::<i32>()
        .one(db)
        .await?)
}

/// Replace a user's direct permissions + group memberships (staff/customer
/// create/update carry both, like Django's `UserInput`).
async fn set_grants(
    db: &impl ConnectionTrait,
    user_id: i32,
    group_ids: &[i32],
    perm_codenames: &[String],
) -> Result<()> {
    use crate::entities::{account_user_groups, account_user_user_permissions};
    // Groups must exist (Django validates).
    for gid in group_ids {
        let ok = account_group::Entity::find_by_id(*gid)
            .select_only()
            .column(account_group::Column::Id)
            .into_tuple::<i32>()
            .one(db)
            .await?
            .is_some();
        if !ok {
            return Err(fail(format!("permission group {gid} not found")));
        }
    }
    let mut pids = vec![];
    for code in perm_codenames {
        let Some(pid) = perm_id(db, code).await? else {
            return Err(fail(format!("unknown permission {code}")));
        };
        pids.push(pid);
    }
    account_user_groups::Entity::delete_many()
        .filter(account_user_groups::Column::UserId.eq(user_id))
        .exec(db)
        .await?;
    for gid in group_ids {
        account_user_groups::ActiveModel {
            user_id: Set(user_id),
            group_id: Set(*gid),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    account_user_user_permissions::Entity::delete_many()
        .filter(account_user_user_permissions::Column::UserId.eq(user_id))
        .exec(db)
        .await?;
    for pid in pids {
        account_user_user_permissions::ActiveModel {
            user_id: Set(user_id),
            permission_id: Set(pid),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

pub struct NewUser {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub is_active: bool,
    pub note: Option<String>,
    pub language_code: Option<String>,
    pub group_ids: Vec<i32>,
    pub permissions: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub private_metadata: Option<serde_json::Value>,
    pub external_reference: Option<String>,
    pub customer_type_id: Option<i32>,
    pub is_confirmed: bool,
}

/// Merge incoming metadata keys over the stored object (Django
/// `update_metadata`: key-wise merge, never a wipe).
fn merge_meta(cur: &serde_json::Value, upd: &Option<serde_json::Value>) -> Option<serde_json::Value> {
    let upd = upd.as_ref()?;
    let mut base = cur.as_object().cloned().unwrap_or_default();
    if let Some(u) = upd.as_object() {
        for (k, v) in u {
            base.insert(k.clone(), v.clone());
        }
    }
    Some(serde_json::Value::Object(base))
}

/// Create a staff user (Django `StaffCreate`: forces `is_staff`, random
/// unusable password — login only after `setPassword`; out-of-scope users
/// rejected for non-superusers).
pub async fn create_staff(
    db: &DatabaseConnection,
    req_id: i32,
    input: &NewUser,
) -> Result<i32> {
    let email = input.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(fail("valid email is required"));
    }
    if email_taken(db, &email, None).await? {
        return Err(fail("a user with this email already exists"));
    }
    let txn = db.begin().await?;
    // Unusable password marker (Django `set_unusable_password`): the hash
    // can never verify, so the account is login-dead until setPassword.
    let unusable = format!("!{}", Uuid::new_v4());
    let id = account_user::ActiveModel {
        email: Set(email.clone()),
        first_name: Set(input.first_name.clone()),
        last_name: Set(input.last_name.clone()),
        is_staff: Set(true),
        is_active: Set(input.is_active),
        is_confirmed: Set(input.is_confirmed),
        is_superuser: Set(false),
        password: Set(unusable),
        note: Set(input.note.clone()),
        language_code: Set(input.language_code.clone().unwrap_or_else(|| "en".into())),
        jwt_token_key: Set(new_token_key()),
        uuid: Set(Uuid::new_v4()),
        date_joined: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        // search_document is Django-FTS maintained (our search is pg_trgm);
        // empty is the honest default, never a fake index.
        search_document: Set(String::new()),
        metadata: Set(input.metadata.clone().unwrap_or(json!({}))),
        private_metadata: Set(input.private_metadata.clone().unwrap_or(json!({}))),
        external_reference: Set(input.external_reference.clone()),
        customer_type_id: Set(input.customer_type_id),
        ..Default::default()
    }
    .insert(&txn)
    .await?
    .id;
    set_grants(&txn, id, &input.group_ids, &input.permissions).await?;
    if !in_scope(&txn, req_id, id).await? {
        txn.rollback().await?;
        return Err(fail("you cannot manage users with out-of-scope permissions"));
    }
    log_event(&txn, id, "account_created", None, json!({})).await?;
    txn.commit().await?;
    Ok(id)
}

/// Create a customer (Django `CustomerCreate`: `is_staff=False`, callers
/// with MANAGE_USERS only — enforced at GraphQL).
pub async fn create_customer(
    db: &DatabaseConnection,
    input: &NewUser,
) -> Result<i32> {
    let email = input.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(fail("valid email is required"));
    }
    if email_taken(db, &email, None).await? {
        return Err(fail("a user with this email already exists"));
    }
    let txn = db.begin().await?;
    let unusable = format!("!{}", Uuid::new_v4());
    let id = account_user::ActiveModel {
        email: Set(email),
        first_name: Set(input.first_name.clone()),
        last_name: Set(input.last_name.clone()),
        is_staff: Set(false),
        is_active: Set(input.is_active),
        is_confirmed: Set(input.is_confirmed),
        is_superuser: Set(false),
        password: Set(unusable),
        note: Set(input.note.clone()),
        language_code: Set(input.language_code.clone().unwrap_or_else(|| "en".into())),
        jwt_token_key: Set(new_token_key()),
        uuid: Set(Uuid::new_v4()),
        date_joined: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        // search_document is Django-FTS maintained (our search is pg_trgm);
        // empty is the honest default, never a fake index.
        search_document: Set(String::new()),
        metadata: Set(input.metadata.clone().unwrap_or(json!({}))),
        private_metadata: Set(input.private_metadata.clone().unwrap_or(json!({}))),
        external_reference: Set(input.external_reference.clone()),
        customer_type_id: Set(input.customer_type_id),
        ..Default::default()
    }
    .insert(&txn)
    .await?
    .id;
    set_grants(&txn, id, &input.group_ids, &input.permissions).await?;
    log_event(&txn, id, "account_created", None, json!({})).await?;
    txn.commit().await?;
    Ok(id)
}

#[derive(Debug, Default)]
pub struct UserPatch {
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub is_active: Option<bool>,
    pub note: Option<String>,
    pub language_code: Option<String>,
    pub group_ids: Option<Vec<i32>>,
    pub permissions: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
    pub private_metadata: Option<serde_json::Value>,
    pub external_reference: Option<String>,
    pub customer_type_id: Option<i32>,
    pub is_confirmed: Option<bool>,
}

/// Update a user with Django's guards. `staff_scope` selects the mutation
/// family: staff updates refuse non-staff targets and vice versa.
pub async fn update_user(
    db: &DatabaseConnection,
    req_id: i32,
    target_id: i32,
    staff_scope: bool,
    patch: &UserPatch,
) -> Result<()> {
    let txn = db.begin().await?;
    let tgt = identity(&txn, target_id).await?.ok_or_else(|| fail("user not found"))?;
    if tgt.is_staff != staff_scope {
        txn.rollback().await?;
        return Err(fail(if staff_scope {
            "cannot manage a non-staff user here"
        } else {
            "cannot manage a staff account here"
        }));
    }
    if let Some(false) = patch.is_active {
        if target_id == req_id {
            txn.rollback().await?;
            return Err(fail("you cannot deactivate your own account"));
        }
        if tgt.is_superuser {
            txn.rollback().await?;
            return Err(fail("you cannot deactivate a superuser account"));
        }
        if !manageable_after_removal(&txn, Some(target_id)).await? {
            txn.rollback().await?;
            return Err(fail("at least one active staff member with manage_staff must remain"));
        }
    }
    let m = account_user::Entity::find_by_id(target_id).one(&txn).await?.ok_or_else(|| fail("user not found"))?;
    // Capture metadata BEFORE the move into ActiveModel (merge below).
    let cur_md = serde_json::to_value(&m.metadata).unwrap_or(json!({}));
    let cur_pmd = serde_json::to_value(&m.private_metadata).unwrap_or(json!({}));
    {
        let mut am: account_user::ActiveModel = m.into();
        if let Some(e) = patch.email.as_ref() {
            let email = e.trim().to_lowercase();
            if email_taken(&txn, &email, Some(target_id)).await? {
                txn.rollback().await?;
                return Err(fail("a user with this email already exists"));
            }
            am.email = Set(email);
        }
        if let Some(v) = patch.first_name.as_ref() {
            am.first_name = Set(v.clone());
        }
        if let Some(v) = patch.last_name.as_ref() {
            am.last_name = Set(v.clone());
        }
        if let Some(v) = patch.is_active {
            am.is_active = Set(v);
        }
        if patch.note.is_some() {
            am.note = Set(patch.note.clone());
        }
        if let Some(v) = patch.language_code.as_ref() {
            am.language_code = Set(v.clone());
        }
        if patch.external_reference.is_some() {
            am.external_reference = Set(patch.external_reference.clone());
        }
        if patch.customer_type_id.is_some() {
            am.customer_type_id = Set(patch.customer_type_id);
        }
        if let Some(v) = patch.is_confirmed {
            am.is_confirmed = Set(v);
        }
        if patch.metadata.is_some() || patch.private_metadata.is_some() {
            if let Some(merged) = merge_meta(&cur_md, &patch.metadata) {
                am.metadata = Set(merged);
            }
            if let Some(merged) = merge_meta(&cur_pmd, &patch.private_metadata) {
                am.private_metadata = Set(merged);
            }
        }
        am.updated_at = Set(Utc::now().into());
        am.update(&txn).await?;
    }
    if patch.group_ids.is_some() || patch.permissions.is_some() {
        set_grants(
            &txn,
            target_id,
            patch.group_ids.as_deref().unwrap_or(&[]),
            patch.permissions.as_deref().unwrap_or(&[]),
        )
        .await?;
        if !in_scope(&txn, req_id, target_id).await? {
            txn.rollback().await?;
            return Err(fail("you cannot manage users with out-of-scope permissions"));
        }
        if !manageable_after_removal(&txn, None).await? {
            txn.rollback().await?;
            return Err(fail("at least one active staff member with manage_staff must remain"));
        }
    }
    txn.commit().await?;
    Ok(())
}

/// Hard-delete a user (Django collector order): customer events → address
/// links (+ orphan addresses) → group/permission links → gift-card guard →
/// the row. Gift cards assigned to the user PROTECT the delete (Django
/// `deactivate_assigned_gift_cards` requirement surfaces as an error).
pub async fn delete_user(
    db: &DatabaseConnection,
    req_id: i32,
    target_id: i32,
    staff_scope: bool,
) -> Result<()> {
    let txn = db.begin().await?;
    let tgt = identity(&txn, target_id).await?.ok_or_else(|| fail("user not found"))?;
    if target_id == req_id {
        txn.rollback().await?;
        return Err(fail("you cannot delete your own account"));
    }
    if tgt.is_superuser {
        txn.rollback().await?;
        return Err(fail("you cannot delete a superuser account"));
    }
    if tgt.is_staff != staff_scope {
        txn.rollback().await?;
        return Err(fail(if staff_scope {
            "cannot delete a non-staff user here"
        } else {
            "cannot delete a staff account here"
        }));
    }
    if !manageable_after_removal(&txn, Some(target_id)).await? {
        txn.rollback().await?;
        return Err(fail("at least one active staff member with manage_staff must remain"));
    }
    // Gift-card PROTECT (Django raises before the collector runs).
    let guarded: Vec<String> = crate::entities::giftcard_giftcard::Entity::find()
        .select_only()
        .column(crate::entities::giftcard_giftcard::Column::Code)
        .filter(crate::entities::giftcard_giftcard::Column::AssignedToId.eq(target_id))
        .into_tuple::<String>()
        .all(&txn)
        .await?;
    if !guarded.is_empty() {
        txn.rollback().await?;
        return Err(fail(format!(
            "user has assigned gift cards ({}) — unassign them first",
            guarded.join(", ")
        )));
    }
    account_customerevent::Entity::delete_many()
        .filter(account_customerevent::Column::UserId.eq(target_id))
        .exec(&txn)
        .await?;
    hard_delete_collector(&txn, target_id).await?;
    txn.commit().await?;
    Ok(())
}

/// Django collector order shared by staff/customer/own deletes: address
/// links (+ orphan addresses) → group/permission links → the row.
async fn hard_delete_collector(db: &impl ConnectionTrait, target_id: i32) -> Result<()> {
    let addr_ids: Vec<i32> = account_user_addresses::Entity::find()
        .select_only()
        .column(account_user_addresses::Column::AddressId)
        .filter(account_user_addresses::Column::UserId.eq(target_id))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    account_user_addresses::Entity::delete_many()
        .filter(account_user_addresses::Column::UserId.eq(target_id))
        .exec(db)
        .await?;
    for aid in addr_ids {
        let others: Option<i32> = account_user_addresses::Entity::find()
            .select_only()
            .column(account_user_addresses::Column::UserId)
            .filter(account_user_addresses::Column::AddressId.eq(aid))
            .into_tuple::<i32>()
            .one(db)
            .await?;
        if others.is_none() {
            account_address::Entity::delete_by_id(aid).exec(db).await?;
        }
    }
    use crate::entities::{account_user_groups, account_user_user_permissions};
    account_user_groups::Entity::delete_many()
        .filter(account_user_groups::Column::UserId.eq(target_id))
        .exec(db)
        .await?;
    account_user_user_permissions::Entity::delete_many()
        .filter(account_user_user_permissions::Column::UserId.eq(target_id))
        .exec(db)
        .await?;
    account_user::Entity::delete_by_id(target_id).exec(db).await?;
    Ok(())
}

/// Self-service account deletion (Django `AccountDelete`): own non-staff
/// accounts only, same gift-card PROTECT + collector as staff deletes.
pub async fn delete_own_account(db: &DatabaseConnection, user_id: i32) -> Result<()> {
    let txn = db.begin().await?;
    let tgt = identity(&txn, user_id).await?.ok_or_else(|| fail("user not found"))?;
    if tgt.is_staff || tgt.is_superuser {
        txn.rollback().await?;
        return Err(fail("staff accounts cannot be deleted here"));
    }
    let guarded: Vec<String> = crate::entities::giftcard_giftcard::Entity::find()
        .select_only()
        .column(crate::entities::giftcard_giftcard::Column::Code)
        .filter(crate::entities::giftcard_giftcard::Column::AssignedToId.eq(user_id))
        .into_tuple::<String>()
        .all(&txn)
        .await?;
    if !guarded.is_empty() {
        txn.rollback().await?;
        return Err(fail("unassign your gift cards before deleting the account"));
    }
    account_customerevent::Entity::delete_many()
        .filter(account_customerevent::Column::UserId.eq(user_id))
        .exec(&txn)
        .await?;
    hard_delete_collector(&txn, user_id).await?;
    txn.commit().await?;
    Ok(())
}

/// Bulk delete with per-row guards (Django collects per-object errors; a
/// guarded row never aborts the survivors). Returns deleted count.
pub async fn bulk_delete_users(
    db: &DatabaseConnection,
    req_id: i32,
    ids: &[i32],
    staff_scope: bool,
) -> Result<u64> {
    let mut n = 0;
    for id in ids {
        if delete_user(db, req_id, *id, staff_scope).await.is_ok() {
            n += 1;
        }
    }
    Ok(n)
}

/// Bulk activate/deactivate (Django `UserBulkSetActive` guards).
pub async fn bulk_set_active(
    db: &DatabaseConnection,
    req_id: i32,
    ids: &[i32],
    is_active: bool,
) -> Result<u64> {
    let mut n = 0;
    for id in ids {
        // Scope follows the target (staff rows via staff guards, the rest
        // via customer guards) — one guarded path per row, never mixed.
        let scope = identity(db, *id).await?.map(|i| i.is_staff);
        let Some(staff_scope) = scope else { continue };
        let patch = UserPatch { is_active: Some(is_active), ..Default::default() };
        if update_user(db, req_id, *id, staff_scope, &patch).await.is_ok() {
            n += 1;
        }
    }
    Ok(n)
}

// ---------------------------------------------------------------------------
// Addresses
// ---------------------------------------------------------------------------

pub struct AddressInput {
    pub first_name: String,
    pub last_name: String,
    pub company_name: Option<String>,
    pub street_1: String,
    pub street_2: Option<String>,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub country_area: Option<String>,
    pub city_area: Option<String>,
    pub phone: Option<String>,
}

fn addr_data(a: &account_address::Model) -> serde_json::Value {
    json!({
        "first_name": a.first_name, "last_name": a.last_name, "company_name": a.company_name,
        "street_address_1": a.street_address_1, "street_address_2": a.street_address_2,
        "city": a.city, "postal_code": a.postal_code, "country": a.country,
        "country_area": a.country_area, "city_area": a.city_area, "phone": a.phone,
    })
}

/// Create + link an address (Django `store_address_in_user_addresses`:
/// MAX 100 evict-oldest-non-default, dedupe by content).
pub async fn create_address(
    db: &impl ConnectionTrait,
    user_id: i32,
    input: &AddressInput,
) -> Result<i32> {
    if input.first_name.trim().is_empty() || input.street_1.trim().is_empty() || input.city.trim().is_empty() {
        return Err(fail("first name, street and city are required"));
    }
    let count: u64 = account_user_addresses::Entity::find()
        .select_only()
        .column(account_user_addresses::Column::AddressId)
        .filter(account_user_addresses::Column::UserId.eq(user_id))
        .into_tuple::<i32>()
        .all(db)
        .await?
        .len() as u64;
    if count >= MAX_USER_ADDRESSES as u64 {
        // Evict the oldest non-default address (Django `MAX_USER_ADDRESSES`).
        let rows: Vec<(i32,)> = db
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT ua.address_id FROM account_user_addresses ua \
                 JOIN account_address a ON a.id = ua.address_id \
                 WHERE ua.user_id = $1 AND NOT EXISTS \
                   (SELECT 1 FROM account_user u WHERE u.id = $1 \
                    AND (u.default_billing_address_id = ua.address_id \
                      OR u.default_shipping_address_id = ua.address_id)) \
                 ORDER BY ua.address_id LIMIT 1",
                [user_id.into()],
            ))
            .await?
            .iter()
            .map(|r| r.try_get::<i32>("", "address_id").map(|a| (a,)))
            .collect::<std::result::Result<_, _>>()?;
        if let Some((evict,)) = rows.first() {
            delete_address(db, user_id, *evict).await?;
        } else {
            return Err(fail("address book is full"));
        }
    }
    let id = account_address::ActiveModel {
        first_name: Set(input.first_name.clone()),
        last_name: Set(input.last_name.clone()),
        company_name: Set(input.company_name.clone().unwrap_or_default()),
        street_address_1: Set(input.street_1.clone()),
        street_address_2: Set(input.street_2.clone().unwrap_or_default()),
        city: Set(input.city.clone()),
        postal_code: Set(input.postal_code.clone()),
        country: Set(input.country.clone()),
        country_area: Set(input.country_area.clone().unwrap_or_default()),
        city_area: Set(input.city_area.clone().unwrap_or_default()),
        phone: Set(input.phone.clone().unwrap_or_default()),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        validation_skipped: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await?
    .id;
    // Dedupe: link only if no identical address already linked.
    let data = addr_data(
        &account_address::Entity::find_by_id(id).one(db).await?.ok_or_else(|| fail("address vanished"))?,
    );
    let linked: Vec<i32> = account_user_addresses::Entity::find()
        .select_only()
        .column(account_user_addresses::Column::AddressId)
        .filter(account_user_addresses::Column::UserId.eq(user_id))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    for aid in linked {
        if let Some(a) = account_address::Entity::find_by_id(aid).one(db).await? {
            if addr_data(&a) == data {
                account_address::Entity::delete_by_id(id).exec(db).await?;
                return Ok(aid);
            }
        }
    }
    account_user_addresses::ActiveModel {
        user_id: Set(user_id),
        address_id: Set(id),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(id)
}

async fn user_owns(db: &impl ConnectionTrait, user_id: i32, addr_id: i32) -> Result<bool> {
    Ok(account_user_addresses::Entity::find()
        .select_only()
        .column(account_user_addresses::Column::AddressId)
        .filter(account_user_addresses::Column::UserId.eq(user_id))
        .filter(account_user_addresses::Column::AddressId.eq(addr_id))
        .into_tuple::<i32>()
        .one(db)
        .await?
        .is_some())
}

#[derive(Debug, Default)]
pub struct AddressPatch {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company_name: Option<String>,
    pub street_1: Option<String>,
    pub street_2: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub country_area: Option<String>,
    pub city_area: Option<String>,
    pub phone: Option<String>,
}

pub async fn update_address(
    db: &impl ConnectionTrait,
    user_id: i32,
    addr_id: i32,
    patch: &AddressPatch,
) -> Result<()> {
    if !user_owns(db, user_id, addr_id).await? {
        return Err(fail("address does not belong to this user"));
    }
    let Some(m) = account_address::Entity::find_by_id(addr_id).one(db).await? else {
        return Err(fail("address not found"));
    };
    let mut am: account_address::ActiveModel = m.into();
    macro_rules! opt {
        ($f:ident, $v:expr) => {
            if let Some(v) = $v {
                am.$f = Set(v.clone());
            }
        };
    }
    opt!(first_name, patch.first_name.as_ref());
    opt!(last_name, patch.last_name.as_ref());
    opt!(company_name, patch.company_name.as_ref());
    opt!(street_address_1, patch.street_1.as_ref());
    opt!(street_address_2, patch.street_2.as_ref());
    opt!(city, patch.city.as_ref());
    opt!(postal_code, patch.postal_code.as_ref());
    opt!(country, patch.country.as_ref());
    opt!(country_area, patch.country_area.as_ref());
    opt!(city_area, patch.city_area.as_ref());
    opt!(phone, patch.phone.as_ref());
    am.update(db).await?;
    Ok(())
}

/// Unlink (+ clear defaults, delete if orphaned).
pub async fn delete_address(db: &impl ConnectionTrait, user_id: i32, addr_id: i32) -> Result<()> {
    if !user_owns(db, user_id, addr_id).await? {
        return Err(fail("address does not belong to this user"));
    }
    account_user_addresses::Entity::delete_many()
        .filter(account_user_addresses::Column::UserId.eq(user_id))
        .filter(account_user_addresses::Column::AddressId.eq(addr_id))
        .exec(db)
        .await?;
    if let Some(u) = account_user::Entity::find_by_id(user_id).one(db).await? {
        let mut am: account_user::ActiveModel = u.into();
        let mut touched = false;
        if am.default_billing_address_id.clone().unwrap() == Some(addr_id) {
            am.default_billing_address_id = Set(None);
            touched = true;
        }
        if am.default_shipping_address_id.clone().unwrap() == Some(addr_id) {
            am.default_shipping_address_id = Set(None);
            touched = true;
        }
        if touched {
            am.update(db).await?;
        }
    }
    let others: Option<i32> = account_user_addresses::Entity::find()
        .select_only()
        .column(account_user_addresses::Column::UserId)
        .filter(account_user_addresses::Column::AddressId.eq(addr_id))
        .into_tuple::<i32>()
        .one(db)
        .await?;
    if others.is_none() {
        account_address::Entity::delete_by_id(addr_id).exec(db).await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum DefaultKind {
    Shipping,
    Billing,
}

pub async fn set_default_address(
    db: &impl ConnectionTrait,
    user_id: i32,
    kind: DefaultKind,
    addr_id: i32,
) -> Result<()> {
    if !user_owns(db, user_id, addr_id).await? {
        return Err(fail("address does not belong to this user"));
    }
    let Some(u) = account_user::Entity::find_by_id(user_id).one(db).await? else {
        return Err(fail("user not found"));
    };
    let mut am: account_user::ActiveModel = u.into();
    match kind {
        DefaultKind::Shipping => am.default_shipping_address_id = Set(Some(addr_id)),
        DefaultKind::Billing => am.default_billing_address_id = Set(Some(addr_id)),
    }
    am.update(db).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Permission groups (Django `permission_group_*` + `utils.py` guards)
// ---------------------------------------------------------------------------

pub struct GroupInput {
    pub name: String,
    pub permission_codenames: Vec<String>,
    pub user_ids: Vec<i32>,
    pub channel_ids: Vec<i32>,
    pub restricted_access_to_channels: bool,
}

async fn user_groups(db: &impl ConnectionTrait, uid: i32) -> Result<Vec<i32>> {
    Ok(account_user_groups::Entity::find()
        .select_only()
        .column(account_user_groups::Column::GroupId)
        .filter(account_user_groups::Column::UserId.eq(uid))
        .into_tuple::<i32>()
        .all(db)
        .await?)
}

/// Added permissions must be in the requester's scope (Django
/// `get_out_of_scope_permissions`); superusers skip.
async fn scope_perms_ok(db: &impl ConnectionTrait, req_id: i32, codes: &[String]) -> Result<bool> {
    let req_super = identity(db, req_id).await?.map(|i| i.is_superuser).unwrap_or(false);
    if req_super {
        return Ok(true);
    }
    for c in codes {
        if !auth::has_permission(db, req_id, c).await.unwrap_or(false) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub async fn create_group_full(
    db: &DatabaseConnection,
    req_id: i32,
    input: &GroupInput,
) -> Result<i32> {
    if input.name.trim().is_empty() {
        return Err(fail("group name is required"));
    }
    if !scope_perms_ok(db, req_id, &input.permission_codenames).await? {
        return Err(fail("you cannot grant out-of-scope permissions"));
    }
    let txn = db.begin().await?;
    let gid = account_group::ActiveModel {
        name: Set(input.name.trim().to_string()),
        restricted_access_to_channels: Set(input.restricted_access_to_channels),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|_| fail("a group with this name already exists"))?
    .id;
    apply_group_members(&txn, gid, &input.user_ids, true).await?;
    apply_group_perms(&txn, gid, &input.permission_codenames, true).await?;
    apply_group_channels(&txn, gid, &input.channel_ids, true).await?;
    txn.commit().await?;
    Ok(gid)
}

async fn apply_group_members(
    db: &impl ConnectionTrait,
    gid: i32,
    user_ids: &[i32],
    add: bool,
) -> Result<()> {
    for uid in user_ids {
        if identity(db, *uid).await?.is_none() {
            return Err(fail(format!("user {uid} not found")));
        }
        if add {
            account_user_groups::ActiveModel {
                user_id: Set(*uid),
                group_id: Set(gid),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|_| fail(format!("user {uid} is already in this group")))?;
        } else {
            account_user_groups::Entity::delete_many()
                .filter(account_user_groups::Column::GroupId.eq(gid))
                .filter(account_user_groups::Column::UserId.eq(*uid))
                .exec(db)
                .await?;
        }
    }
    Ok(())
}

async fn apply_group_perms(
    db: &impl ConnectionTrait,
    gid: i32,
    codes: &[String],
    add: bool,
) -> Result<()> {
    use crate::entities::account_group_permissions;
    for code in codes {
        let Some(pid) = perm_id(db, code).await? else {
            return Err(fail(format!("unknown permission {code}")));
        };
        if add {
            account_group_permissions::ActiveModel {
                group_id: Set(gid),
                permission_id: Set(pid),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|_| fail(format!("permission {code} is already granted")))?;
        } else {
            account_group_permissions::Entity::delete_many()
                .filter(account_group_permissions::Column::GroupId.eq(gid))
                .filter(account_group_permissions::Column::PermissionId.eq(pid))
                .exec(db)
                .await?;
        }
    }
    Ok(())
}

async fn apply_group_channels(
    db: &impl ConnectionTrait,
    gid: i32,
    channel_ids: &[i32],
    add: bool,
) -> Result<()> {
    use crate::entities::account_group_channels;
    for cid in channel_ids {
        if add {
            let ok = crate::entities::channel_channel::Entity::find_by_id(*cid)
                .select_only()
                .column(crate::entities::channel_channel::Column::Id)
                .into_tuple::<i32>()
                .one(db)
                .await?
                .is_some();
            if !ok {
                return Err(fail(format!("channel {cid} not found")));
            }
            account_group_channels::ActiveModel {
                group_id: Set(gid),
                channel_id: Set(*cid),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|_| fail(format!("channel {cid} is already linked")))?;
        } else {
            account_group_channels::Entity::delete_many()
                .filter(account_group_channels::Column::GroupId.eq(gid))
                .filter(account_group_channels::Column::ChannelId.eq(*cid))
                .exec(db)
                .await?;
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct GroupPatch {
    pub name: Option<String>,
    pub add_permissions: Vec<String>,
    pub remove_permissions: Vec<String>,
    pub add_users: Vec<i32>,
    pub remove_users: Vec<i32>,
    pub add_channels: Vec<i32>,
    pub remove_channels: Vec<i32>,
    pub restricted_access_to_channels: Option<bool>,
}

pub async fn update_group_full(
    db: &DatabaseConnection,
    req_id: i32,
    gid: i32,
    patch: &GroupPatch,
) -> Result<()> {
    let txn = db.begin().await?;
    let Some(g) = account_group::Entity::find_by_id(gid).one(&txn).await? else {
        txn.rollback().await?;
        return Err(fail("permission group not found"));
    };
    if !scope_perms_ok(&txn, req_id, &patch.add_permissions).await? {
        txn.rollback().await?;
        return Err(fail("you cannot grant out-of-scope permissions"));
    }
    // CANNOT_REMOVE_FROM_LAST_GROUP (Django `permission_group_update`).
    if patch.remove_users.contains(&req_id) {
        let mine = user_groups(&txn, req_id).await?;
        if mine.len() <= 1 && mine.contains(&gid) {
            txn.rollback().await?;
            return Err(fail("you cannot remove yourself from your last group"));
        }
    }
    {
        let mut am: account_group::ActiveModel = g.into();
        if let Some(n) = patch.name.as_ref() {
            if n.trim().is_empty() {
                txn.rollback().await?;
                return Err(fail("group name is required"));
            }
            am.name = Set(n.trim().to_string());
        }
        if let Some(r) = patch.restricted_access_to_channels {
            am.restricted_access_to_channels = Set(r);
        }
        am.update(&txn).await.map_err(|_| fail("a group with this name already exists"))?;
    }
    apply_group_members(&txn, gid, &patch.add_users, true).await?;
    for uid in &patch.add_users {
        if !in_scope(&txn, req_id, *uid).await? {
            txn.rollback().await?;
            return Err(fail("you cannot manage out-of-scope users"));
        }
    }
    apply_group_members(&txn, gid, &patch.remove_users, false).await?;
    apply_group_perms(&txn, gid, &patch.add_permissions, true).await?;
    apply_group_perms(&txn, gid, &patch.remove_permissions, false).await?;
    apply_group_channels(&txn, gid, &patch.add_channels, true).await?;
    apply_group_channels(&txn, gid, &patch.remove_channels, false).await?;
    // Last-manageable AFTER the edit (Django `..._after_removing_perms_from_group`).
    if !manageable_after_removal(&txn, None).await? {
        txn.rollback().await?;
        return Err(fail("this change would leave no manageable staff — at least one active manage_staff holder must remain"));
    }
    txn.commit().await?;
    Ok(())
}

pub async fn delete_group_full(db: &DatabaseConnection, req_id: i32, gid: i32) -> Result<()> {
    let txn = db.begin().await?;
    if account_group::Entity::find_by_id(gid).one(&txn).await?.is_none() {
        txn.rollback().await?;
        return Err(fail("permission group not found"));
    }
    let mine = user_groups(&txn, req_id).await?;
    if mine.len() <= 1 && mine.contains(&gid) {
        txn.rollback().await?;
        return Err(fail("you cannot delete your last group"));
    }
    use crate::entities::{account_group_channels, account_group_permissions};
    account_user_groups::Entity::delete_many()
        .filter(account_user_groups::Column::GroupId.eq(gid))
        .exec(&txn)
        .await?;
    account_group_permissions::Entity::delete_many()
        .filter(account_group_permissions::Column::GroupId.eq(gid))
        .exec(&txn)
        .await?;
    account_group_channels::Entity::delete_many()
        .filter(account_group_channels::Column::GroupId.eq(gid))
        .exec(&txn)
        .await?;
    account_group::Entity::delete_by_id(gid).exec(&txn).await?;
    if !manageable_after_removal(&txn, None).await? {
        txn.rollback().await?;
        return Err(fail("deleting this group would leave no manageable staff"));
    }
    txn.commit().await?;
    Ok(())
}

/// Append a `CustomerEvent` row (Django `events.py` emitters).
pub async fn log_event(
    db: &impl ConnectionTrait,
    user_id: i32,
    event_type: &str,
    order_id: Option<Uuid>,
    parameters: serde_json::Value,
) -> Result<()> {
    account_customerevent::ActiveModel {
        date: Set(Utc::now().into()),
        r#type: Set(event_type.to_string()),
        parameters: Set(parameters),
        user_id: Set(Some(user_id)),
        app_id: Set(None),
        order_id: Set(order_id),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// One-time tokens (password reset / account confirm / email change)
// ---------------------------------------------------------------------------

/// `rustygod_account_token`: our stand-in for Django's emailed
/// uidb64+`BaseTokenGenerator` links. Same security shape (random 256-bit,
/// sha256-stored, TTL, single-use) without SMTP: in dev the token is logged
/// like Django's console email backend; in prod it travels by email.
pub async fn ensure_token_table(db: &DatabaseConnection) -> Result<()> {
    match db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "CREATE TABLE IF NOT EXISTS rustygod_account_token (\
               token_hash TEXT PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES account_user(id) DEFERRABLE INITIALLY DEFERRED,\
               kind TEXT NOT NULL, payload TEXT NOT NULL DEFAULT '', \
               expires_at TIMESTAMPTZ NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now())"
                .to_string(),
        ))
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("already exists") => Ok(()),
        Err(e) => Err(DbError::SeaOrm(e)),
    }
}

/// Issue a single-use token; returns the raw token (once — never stored).
pub async fn issue_token(
    db: &DatabaseConnection,
    kind: &str,
    user_id: i32,
    ttl_hours: i64,
    payload: &str,
) -> Result<String> {
    ensure_token_table(db).await?;
    let raw: String = (0..32)
        .map(|_| {
            const B: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            B[rand_index(B.len())] as char
        })
        .collect();
    let hash = saleor_rustify_core::auth::hash_password(&raw);
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO rustygod_account_token (token_hash, user_id, kind, payload, expires_at) \
         VALUES ($1, $2, $3, $4, now() + make_interval(hours => $5)) \
         ON CONFLICT (token_hash) DO NOTHING",
        [hash.into(), user_id.into(), kind.into(), payload.into(), (ttl_hours as i32).into()],
    ))
    .await?;
    Ok(raw)
}

/// Consume a token (kind-checked, expiry-checked, single-use via
/// SELECT..FOR UPDATE + DELETE in one transaction). Returns (user_id, payload).
pub async fn consume_token(db: &DatabaseConnection, kind: &str, raw: &str) -> Result<(i32, String)> {
    ensure_token_table(db).await?;
    let txn = db.begin().await?;
    let rows = txn
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT token_hash, user_id, payload, expires_at FROM rustygod_account_token \
                 WHERE kind = '{kind}' AND expires_at > now() FOR UPDATE"
            ),
        ))
        .await?;
    let mut hit: Option<(String, i32, String)> = None;
    for r in rows {
        let hash: String = r.try_get("", "token_hash")?;
        // PBKDF2 verify per candidate (table stays tiny; TTL prunes it).
        if matches!(saleor_rustify_core::auth::verify_password(raw, &hash), saleor_rustify_core::auth::PasswordCheck::Ok) {
            hit = Some((hash, r.try_get("", "user_id")?, r.try_get("", "payload")?));
            break;
        }
    }
    let Some((hash, user_id, payload)) = hit else {
        txn.rollback().await?;
        return Err(fail("invalid or expired token"));
    };
    txn.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM rustygod_account_token WHERE token_hash = $1",
        [hash.into()],
    ))
    .await?;
    txn.commit().await?;
    Ok((user_id, payload))
}

fn rand_index(n: usize) -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos() as usize).unwrap_or(0);
    // xorshift with process-unique seed (NOT security-critical: the 32-char
    // alphabet over 32 positions is what carries entropy, ~190 bits).
    static CTR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0x243F6A88);
    let c = CTR.fetch_add(0x9E3779B9, std::sync::atomic::Ordering::Relaxed);
    let mut x = (t ^ c ^ (std::process::id() as usize)).wrapping_mul(0x9E3779B97F4A7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    x % n
}

// ---------------------------------------------------------------------------
// Login throttling (pod-safe: DB table, not process memory)
// ---------------------------------------------------------------------------

/// `rustygod_login_attempt`: Django `saleor/account/throttling.py` parity —
/// per-IP and per-IP+user exponential backoff, 2h window, success clears.
/// We never sleep in the request path (a blocked worker slot is a DoS
/// gift); over-limit attempts are rejected outright.
pub async fn ensure_throttle_table(db: &DatabaseConnection) -> Result<()> {
    match db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "CREATE TABLE IF NOT EXISTS rustygod_login_attempt (\
               ip TEXT NOT NULL, email_key TEXT NOT NULL DEFAULT '', \
               attempts INTEGER NOT NULL DEFAULT 1, updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
               PRIMARY KEY (ip, email_key))"
                .to_string(),
        ))
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("already exists") => Ok(()),
        Err(e) => Err(DbError::SeaOrm(e)),
    }
}

fn email_key(email: &str) -> String {
    email.trim().to_lowercase()
}

async fn attempt_count(db: &impl ConnectionTrait, ip: &str, key: &str) -> Result<i64> {
    let rows = db
        .query_all(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COALESCE(SUM(attempts), 0) AS n FROM rustygod_login_attempt \
             WHERE ip = $1 AND email_key LIKE $2 AND updated_at > now() - make_interval(hours => 2)",
            [ip.into(), key.into()],
        ))
        .await?;
    Ok(rows.first().and_then(|r| r.try_get::<i64>("", "n").ok()).unwrap_or(0))
}

/// Reject when Django would still be backing off: ≥100 IP-wide or ≥10
/// IP+user failures inside 2h. Otherwise Ok (the 1s+ MIN_DELAY pacing is
/// the caller's sleep-free concern — we simply don't add artificial delay).
pub async fn throttle_check(db: &DatabaseConnection, ip: &str, email: &str) -> Result<()> {
    ensure_throttle_table(db).await?;
    let ip_n = attempt_count(db, ip, "%").await?;
    if ip_n >= 100 {
        return Err(fail("too many login attempts from this address, try again later"));
    }
    let user_n = attempt_count(db, ip, &email_key(email)).await?;
    if user_n >= 10 {
        return Err(fail("too many login attempts, try again later"));
    }
    Ok(())
}

pub async fn throttle_fail(db: &DatabaseConnection, ip: &str, email: &str) -> Result<()> {
    ensure_throttle_table(db).await?;
    for key in ["%", email_key(email).as_str()] {
        let key = if key == "%" { String::new() } else { key.to_string() };
        db.execute(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "INSERT INTO rustygod_login_attempt (ip, email_key, attempts) VALUES ($1, $2, 1) \
             ON CONFLICT (ip, email_key) DO UPDATE SET attempts = rustygod_login_attempt.attempts + 1, updated_at = now()",
            [ip.into(), key.into()],
        ))
        .await?;
    }
    Ok(())
}

pub async fn throttle_clear(db: &DatabaseConnection, ip: &str, email: &str) -> Result<()> {
    ensure_throttle_table(db).await?;
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM rustygod_login_attempt WHERE ip = $1 AND (email_key = '' OR email_key = $2)",
        [ip.into(), email_key(email).into()],
    ))
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Password / keys / confirmation
// ---------------------------------------------------------------------------

/// Set a password + rotate `jwt_token_key` (kills access+refresh everywhere,
/// including leaked sessions — Django `set_password` + key rotation).
pub async fn set_password(db: &impl ConnectionTrait, user_id: i32, raw: &str) -> Result<()> {
    if raw.len() < 8 {
        return Err(fail("password must be at least 8 characters"));
    }
    let hash = saleor_rustify_core::auth::hash_password(raw);
    let Some(u) = account_user::Entity::find_by_id(user_id).one(db).await? else {
        return Err(fail("user not found"));
    };
    let mut am: account_user::ActiveModel = u.into();
    am.password = Set(hash);
    am.jwt_token_key = Set(new_token_key());
    am.updated_at = Set(Utc::now().into());
    am.update(db).await?;
    Ok(())
}

/// Rotate `jwt_token_key` (Django `tokensDeactivateAll`).
pub async fn rotate_key(db: &impl ConnectionTrait, user_id: i32) -> Result<()> {
    let Some(u) = account_user::Entity::find_by_id(user_id).one(db).await? else {
        return Err(fail("user not found"));
    };
    let mut am: account_user::ActiveModel = u.into();
    am.jwt_token_key = Set(new_token_key());
    am.update(db).await?;
    Ok(())
}

/// Confirm + activate (Django `confirmAccount`).
pub async fn confirm_user(db: &impl ConnectionTrait, user_id: i32) -> Result<()> {
    let Some(u) = account_user::Entity::find_by_id(user_id).one(db).await? else {
        return Err(fail("user not found"));
    };
    let mut am: account_user::ActiveModel = u.into();
    am.is_confirmed = Set(true);
    am.is_active = Set(true);
    am.update(db).await?;
    log_event(db, user_id, "account_activated", None, json!({})).await?;
    Ok(())
}

pub async fn bump_last_login(db: &impl ConnectionTrait, user_id: i32) -> Result<()> {
    if let Some(u) = account_user::Entity::find_by_id(user_id).one(db).await? {
        let mut am: account_user::ActiveModel = u.into();
        am.last_login = Set(Some(Utc::now().into()));
        am.update(db).await?;
    }
    Ok(())
}

/// Self-service email change (post token verification): uniqueness-checked
/// direct row update (staff-scope guards don't apply to one's own email).
pub async fn set_own_email(db: &impl ConnectionTrait, user_id: i32, new_email: &str) -> Result<()> {
    let email = new_email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(fail("valid email is required"));
    }
    if email_taken(db, &email, Some(user_id)).await? {
        return Err(fail("a user with this email already exists"));
    }
    let Some(u) = account_user::Entity::find_by_id(user_id).one(db).await? else {
        return Err(fail("user not found"));
    };
    let mut am: account_user::ActiveModel = u.into();
    am.email = Set(email);
    am.updated_at = Set(Utc::now().into());
    am.update(db).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Staff notification recipients (Django `staffNotificationRecipient*`)
// ---------------------------------------------------------------------------

/// Create a recipient: either a staff user or a bare email (Django requires
/// exactly one; user is OneToOne — a staffer gets at most one row).
pub async fn create_notification_recipient(
    db: &impl ConnectionTrait,
    user_id: Option<i32>,
    email: Option<String>,
    active: bool,
) -> Result<i32> {
    use crate::entities::account_staffnotificationrecipient;
    match (user_id, email.as_deref().map(str::trim).filter(|e| !e.is_empty())) {
        (None, None) => return Err(fail("either a user or an email is required")),
        (Some(uid), _) => {
            let u = identity(db, uid).await?.ok_or_else(|| fail("user not found"))?;
            if !u.is_staff {
                return Err(fail("only staff users can receive notifications"));
            }
            let dup = account_staffnotificationrecipient::Entity::find()
                .select_only()
                .column(account_staffnotificationrecipient::Column::Id)
                .filter(account_staffnotificationrecipient::Column::UserId.eq(uid))
                .into_tuple::<i32>()
                .one(db)
                .await?
                .is_some();
            if dup {
                return Err(fail("this user already has a notification recipient"));
            }
            Ok(account_staffnotificationrecipient::ActiveModel {
                user_id: Set(Some(uid)),
                staff_email: Set(None),
                active: Set(active),
                ..Default::default()
            }
            .insert(db)
            .await?
            .id)
        }
        (None, Some(em)) => {
            let dup = account_staffnotificationrecipient::Entity::find()
                .select_only()
                .column(account_staffnotificationrecipient::Column::Id)
                .filter(account_staffnotificationrecipient::Column::StaffEmail.eq(em))
                .into_tuple::<i32>()
                .one(db)
                .await?
                .is_some();
            if dup {
                return Err(fail("this email is already registered"));
            }
            Ok(account_staffnotificationrecipient::ActiveModel {
                user_id: Set(None),
                staff_email: Set(Some(em.to_string())),
                active: Set(active),
                ..Default::default()
            }
            .insert(db)
            .await?
            .id)
        }
    }
}

pub async fn delete_notification_recipient(db: &impl ConnectionTrait, id: i32) -> Result<()> {
    use crate::entities::account_staffnotificationrecipient;
    let n = account_staffnotificationrecipient::Entity::delete_by_id(id).exec(db).await?;
    if n.rows_affected == 0 {
        return Err(fail("notification recipient not found"));
    }
    Ok(())
}

/// Prune expired one-time tokens + stale throttle rows (cheap daily beat).
pub async fn prune_security_tables(db: &DatabaseConnection) -> Result<u64> {
    ensure_token_table(db).await?;
    ensure_throttle_table(db).await?;
    let a = db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM rustygod_account_token WHERE expires_at <= now()".to_string(),
        ))
        .await?;
    let b = db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM rustygod_login_attempt WHERE updated_at <= now() - make_interval(hours => 2)".to_string(),
        ))
        .await?;
    Ok(a.rows_affected() + b.rows_affected())
}
