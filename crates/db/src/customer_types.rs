//! Customer-type CRUD + attribute assignment (Django
//! `saleor/graphql/account/mutations/customer_type/*` parity).
//!
//! Guards: unique name/slug, the default type cannot be deleted, only
//! `customer-type`-kind attributes assign.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::{
    entities::{
        account_customertype, attribute_attributecustomertype, attribute_attribute,
    },
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::App(format!("customer-type: {}", msg.into()))
}

fn slugify(s: &str) -> String {
    crate::attribute_writes::slugify(s)
}

pub struct CustomerTypeCreate {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub is_default: bool,
}

pub async fn create_customer_type(db: &impl ConnectionTrait, input: &CustomerTypeCreate) -> Result<i32> {
    let name = input.name.as_deref().map(str::trim).filter(|s| !s.is_empty())
        .ok_or_else(|| fail("name is required"))?;
    let slug = slugify(input.slug.as_deref().unwrap_or(name));
    if slug_taken(db, &slug, None).await? {
        return Err(fail(format!("slug {slug} already exists")));
    }
    if name_taken(db, name, None).await? {
        return Err(fail(format!("name {name} already exists")));
    }
    Ok(account_customertype::ActiveModel {
        name: Set(name.to_string()),
        slug: Set(slug),
        is_default: Set(input.is_default),
        ..Default::default()
    }
    .insert(db)
    .await?
    .id)
}

async fn slug_taken(db: &impl ConnectionTrait, slug: &str, except: Option<i32>) -> Result<bool> {
    let mut q = account_customertype::Entity::find()
        .select_only()
        .column(account_customertype::Column::Id)
        .filter(account_customertype::Column::Slug.eq(slug));
    if let Some(id) = except {
        q = q.filter(account_customertype::Column::Id.ne(id));
    }
    Ok(q.into_tuple::<i32>().one(db).await?.is_some())
}

async fn name_taken(db: &impl ConnectionTrait, name: &str, except: Option<i32>) -> Result<bool> {
    let mut q = account_customertype::Entity::find()
        .select_only()
        .column(account_customertype::Column::Id)
        .filter(account_customertype::Column::Name.eq(name));
    if let Some(id) = except {
        q = q.filter(account_customertype::Column::Id.ne(id));
    }
    Ok(q.into_tuple::<i32>().one(db).await?.is_some())
}

#[derive(Debug, Default)]
pub struct CustomerTypePatch {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub is_default: Option<bool>,
}

pub async fn update_customer_type(
    db: &impl ConnectionTrait,
    ct_id: i32,
    patch: &CustomerTypePatch,
) -> Result<()> {
    let Some(m) = account_customertype::Entity::find_by_id(ct_id).one(db).await? else {
        return Err(fail("customer type not found"));
    };
    if let Some(s) = patch.slug.as_ref() {
        let slug = slugify(s);
        if slug_taken(db, &slug, Some(ct_id)).await? {
            return Err(fail(format!("slug {slug} already exists")));
        }
    }
    if let Some(n) = patch.name.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if name_taken(db, n, Some(ct_id)).await? {
            return Err(fail(format!("name {n} already exists")));
        }
    }
    let mut am: account_customertype::ActiveModel = m.into();
    if let Some(n) = patch.name.as_ref() {
        am.name = Set(n.clone());
    }
    if let Some(s) = patch.slug.as_ref() {
        am.slug = Set(slugify(s));
    }
    if let Some(v) = patch.is_default {
        am.is_default = Set(v);
    }
    am.update(db).await?;
    Ok(())
}

/// Delete a customer type (Django blocks the default; users pointing at it
/// keep a dangling FK-free reference — Django SET_NULLs via collector, and
/// so do we).
pub async fn delete_customer_type(db: &DatabaseConnection, ct_id: i32) -> Result<()> {
    let txn = db.begin().await?;
    let Some(m) = account_customertype::Entity::find_by_id(ct_id).one(&txn).await? else {
        txn.rollback().await?;
        return Err(fail("customer type not found"));
    };
    if m.is_default {
        txn.rollback().await?;
        return Err(fail("the default customer type cannot be deleted"));
    }
    attribute_attributecustomertype::Entity::delete_many()
        .filter(attribute_attributecustomertype::Column::CustomerTypeId.eq(ct_id))
        .exec(&txn)
        .await?;
    // Users referencing the type fall back to NULL (Django collector SET_NULL).
    txn.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE account_user SET customer_type_id = NULL WHERE customer_type_id = $1",
        [ct_id.into()],
    ))
    .await?;
    account_customertype::Entity::delete_by_id(ct_id).exec(&txn).await?;
    txn.commit().await?;
    Ok(())
}

/// Assign attributes (Django: only `customer-type`-kind, not already assigned).
pub async fn assign_attributes(
    db: &impl ConnectionTrait,
    ct_id: i32,
    attr_ids: &[i32],
) -> Result<()> {
    if account_customertype::Entity::find_by_id(ct_id).one(db).await?.is_none() {
        return Err(fail("customer type not found"));
    }
    for aid in attr_ids {
        let kind: Option<String> = attribute_attribute::Entity::find_by_id(*aid)
            .select_only()
            .column(attribute_attribute::Column::Type)
            .into_tuple::<String>()
            .one(db)
            .await?;
        match kind.as_deref() {
            None => return Err(fail(format!("attribute {aid} doesn't exist"))),
            Some(k) if k != "customer-type" => {
                return Err(fail("only customer attributes can be assigned"))
            }
            _ => {}
        }
        let dup = attribute_attributecustomertype::Entity::find()
            .select_only()
            .column(attribute_attributecustomertype::Column::Id)
            .filter(attribute_attributecustomertype::Column::CustomerTypeId.eq(ct_id))
            .filter(attribute_attributecustomertype::Column::AttributeId.eq(*aid))
            .into_tuple::<i32>()
            .one(db)
            .await?
            .is_some();
        if dup {
            return Err(fail(format!("attribute {aid} is already assigned")));
        }
        let orders: Vec<Option<i32>> = attribute_attributecustomertype::Entity::find()
            .select_only()
            .column(attribute_attributecustomertype::Column::SortOrder)
            .filter(attribute_attributecustomertype::Column::CustomerTypeId.eq(ct_id))
            .into_tuple::<Option<i32>>()
            .all(db)
            .await?;
        let so = orders.into_iter().flatten().max().unwrap_or(-1);
        attribute_attributecustomertype::ActiveModel {
            attribute_id: Set(*aid),
            customer_type_id: Set(ct_id),
            sort_order: Set(Some(so + 1)),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

pub async fn unassign_attributes(
    db: &impl ConnectionTrait,
    ct_id: i32,
    attr_ids: &[i32],
) -> Result<u64> {
    Ok(attribute_attributecustomertype::Entity::delete_many()
        .filter(attribute_attributecustomertype::Column::CustomerTypeId.eq(ct_id))
        .filter(attribute_attributecustomertype::Column::AttributeId.is_in(attr_ids.to_vec()))
        .exec(db)
        .await?
        .rows_affected)
}

pub async fn reorder_attributes(
    db: &impl ConnectionTrait,
    ct_id: i32,
    moves: &[(i32, i32)],
) -> Result<()> {
    for (attr_id, sort_order) in moves {
        let n = attribute_attributecustomertype::Entity::update_many()
            .col_expr(
                attribute_attributecustomertype::Column::SortOrder,
                sea_orm::sea_query::Expr::value(Some(*sort_order)),
            )
            .filter(attribute_attributecustomertype::Column::CustomerTypeId.eq(ct_id))
            .filter(attribute_attributecustomertype::Column::AttributeId.eq(*attr_id))
            .exec(db)
            .await?;
        if n.rows_affected == 0 {
            return Err(fail(format!("attribute {attr_id} is not assigned to this customer type")));
        }
    }
    Ok(())
}
