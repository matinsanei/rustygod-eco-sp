//! Staff permission groups on Django's `account_group*` tables.
//!
//! Mirrors `saleor/graphql/account/mutations/staff/` group flows:
//! groups bundle permission codenames; users gain them through membership
//! (the same resolution `auth::has_permission` already honors).
//! - names are unique (Django enforces it — clean error here, not a 500);
//! - deleting a group only drops links, never users or permissions;
//! - granting an unknown codename is rejected (no silent no-ops).

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, SelectorTrait, Set, TransactionTrait,
};

use crate::{
    entities::{
        account_group, account_group_permissions, account_user, account_user_groups,
        permission_permission,
    },
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Group(msg.into())
}

#[derive(Debug)]
pub struct GroupView {
    pub id: i32,
    pub name: String,
    pub permissions: Vec<String>,
    pub member_ids: Vec<i32>,
}

async fn perm_id(db: &impl ConnectionTrait, codename: &str) -> Result<i32> {
    permission_permission::Entity::find()
        .filter(permission_permission::Column::Codename.eq(codename))
        .one(db)
        .await?
        .map(|p| p.id)
        .ok_or_else(|| fail(format!("unknown permission: {codename}")))
}

async fn view_of(db: &impl ConnectionTrait, group_id: i32) -> Result<GroupView> {
    use sea_orm::{QuerySelect, SelectorTrait};
    let name: String = account_group::Entity::find_by_id(group_id)
        .select_only()
        .column(account_group::Column::Name)
        .into_tuple()
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("group {group_id} not found")))?;
    let perm_ids: Vec<i32> = account_group_permissions::Entity::find()
        .select_only()
        .column(account_group_permissions::Column::PermissionId)
        .filter(account_group_permissions::Column::GroupId.eq(group_id))
        .into_tuple()
        .all(db)
        .await?;
    let permissions: Vec<String> = if perm_ids.is_empty() {
        vec![]
    } else {
        permission_permission::Entity::find()
            .select_only()
            .column(permission_permission::Column::Codename)
            .filter(permission_permission::Column::Id.is_in(perm_ids))
            .into_tuple()
            .all(db)
            .await?
    };
    let member_ids: Vec<i32> = account_user_groups::Entity::find()
        .select_only()
        .column(account_user_groups::Column::UserId)
        .filter(account_user_groups::Column::GroupId.eq(group_id))
        .into_tuple()
        .all(db)
        .await?;
    Ok(GroupView { id: group_id, name, permissions, member_ids })
}

pub async fn create_group(
    db: &DatabaseConnection,
    name: &str,
    permissions: &[String],
) -> Result<GroupView> {
    if name.trim().is_empty() {
        return Err(fail("group name cannot be empty"));
    }
    let txn = db.begin().await?;
    if account_group::Entity::find()
        .select_only()
        .column(account_group::Column::Id)
        .filter(account_group::Column::Name.eq(name))
        .into_tuple::<i32>()
        .one(&txn)
        .await?
        .is_some()
    {
        return Err(fail(format!("group '{name}' already exists")));
    }
    let row = account_group::ActiveModel {
        name: Set(name.to_string()),
        restricted_access_to_channels: Set(false),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    for codename in permissions {
        let pid = perm_id(&txn, codename).await?;
        account_group_permissions::ActiveModel {
            group_id: Set(row.id),
            permission_id: Set(pid),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    txn.commit().await?;
    view_of(db, row.id).await
}

pub async fn list_groups(db: &impl ConnectionTrait) -> Result<Vec<GroupView>> {
    use sea_orm::{QuerySelect, SelectorTrait};
    let ids: Vec<i32> = account_group::Entity::find()
        .select_only()
        .column(account_group::Column::Id)
        .order_by_asc(account_group::Column::Name)
        .into_tuple()
        .all(db)
        .await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(view_of(db, id).await?);
    }
    Ok(out)
}

pub async fn rename_group(
    db: &DatabaseConnection,
    group_id: i32,
    name: &str,
) -> Result<GroupView> {
    if name.trim().is_empty() {
        return Err(fail("group name cannot be empty"));
    }
    let txn = db.begin().await?;
    account_group::Entity::find_by_id(group_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("group {group_id} not found")))?;
    let clash: bool = account_group::Entity::find()
        .select_only()
        .column(account_group::Column::Id)
        .filter(account_group::Column::Name.eq(name))
        .filter(account_group::Column::Id.ne(group_id))
        .into_tuple::<i32>()
        .one(&txn)
        .await?
        .is_some();
    if clash {
        return Err(fail(format!("group '{name}' already exists")));
    }
    account_group::Entity::update_many()
        .col_expr(account_group::Column::Name, sea_orm::sea_query::Expr::value(name.to_string()))
        .filter(account_group::Column::Id.eq(group_id))
        .exec(&txn)
        .await?;
    txn.commit().await?;
    view_of(db, group_id).await
}

/// Delete a group: links go, users and permissions stay (Django semantics).
pub async fn delete_group(db: &DatabaseConnection, group_id: i32) -> Result<()> {
    let txn = db.begin().await?;
    account_group::Entity::find_by_id(group_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("group {group_id} not found")))?;
    account_group_permissions::Entity::delete_many()
        .filter(account_group_permissions::Column::GroupId.eq(group_id))
        .exec(&txn)
        .await?;
    account_user_groups::Entity::delete_many()
        .filter(account_user_groups::Column::GroupId.eq(group_id))
        .exec(&txn)
        .await?;
    account_group::Entity::delete_by_id(group_id).exec(&txn).await?;
    txn.commit().await?;
    Ok(())
}

/// Add users to a group. Unknown users are rejected (no silent no-ops).
pub async fn add_members(
    db: &DatabaseConnection,
    group_id: i32,
    user_ids: &[i32],
) -> Result<GroupView> {
    let txn = db.begin().await?;
    account_group::Entity::find_by_id(group_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("group {group_id} not found")))?;
    for uid in user_ids {
        let exists = account_user::Entity::find_by_id(*uid)
            .select_only()
            .column(account_user::Column::Id)
            .into_tuple::<i32>()
            .one(&txn)
            .await?
            .is_some();
        if !exists {
            return Err(fail(format!("user {uid} not found")));
        }
        let linked = account_user_groups::Entity::find()
            .filter(account_user_groups::Column::GroupId.eq(group_id))
            .filter(account_user_groups::Column::UserId.eq(*uid))
            .one(&txn)
            .await?;
        if linked.is_none() {
            account_user_groups::ActiveModel {
                group_id: Set(group_id),
                user_id: Set(*uid),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }
    txn.commit().await?;
    view_of(db, group_id).await
}

pub async fn remove_members(
    db: &DatabaseConnection,
    group_id: i32,
    user_ids: &[i32],
) -> Result<GroupView> {
    let txn = db.begin().await?;
    account_group::Entity::find_by_id(group_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("group {group_id} not found")))?;
    account_user_groups::Entity::delete_many()
        .filter(account_user_groups::Column::GroupId.eq(group_id))
        .filter(account_user_groups::Column::UserId.is_in(user_ids.to_vec()))
        .exec(&txn)
        .await?;
    txn.commit().await?;
    view_of(db, group_id).await
}

/// Grant codenames to a group (idempotent; unknown codenames rejected).
pub async fn grant_permissions(
    db: &DatabaseConnection,
    group_id: i32,
    codenames: &[String],
) -> Result<GroupView> {
    let txn = db.begin().await?;
    account_group::Entity::find_by_id(group_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("group {group_id} not found")))?;
    for codename in codenames {
        let pid = perm_id(&txn, codename).await?;
        let linked = account_group_permissions::Entity::find()
            .filter(account_group_permissions::Column::GroupId.eq(group_id))
            .filter(account_group_permissions::Column::PermissionId.eq(pid))
            .one(&txn)
            .await?;
        if linked.is_none() {
            account_group_permissions::ActiveModel {
                group_id: Set(group_id),
                permission_id: Set(pid),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }
    txn.commit().await?;
    view_of(db, group_id).await
}

pub async fn revoke_permissions(
    db: &DatabaseConnection,
    group_id: i32,
    codenames: &[String],
) -> Result<GroupView> {
    let txn = db.begin().await?;
    account_group::Entity::find_by_id(group_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("group {group_id} not found")))?;
    let mut pids = Vec::with_capacity(codenames.len());
    for codename in codenames {
        pids.push(perm_id(&txn, codename).await?);
    }
    account_group_permissions::Entity::delete_many()
        .filter(account_group_permissions::Column::GroupId.eq(group_id))
        .filter(account_group_permissions::Column::PermissionId.is_in(pids))
        .exec(&txn)
        .await?;
    txn.commit().await?;
    view_of(db, group_id).await
}
