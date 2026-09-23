//! Attribute + attribute-value + assignment writes on the Django tables.
//!
//! Django parity (`saleor/graphql/product/mutations/attributes.py`,
//! `saleor/graphql/attribute/mutations/`):
//! - assign = INSERT into `attribute_attributeproduct` (PRODUCT ops) or
//!   `attribute_attributevariant` (VARIANT ops, carries `variant_selection`);
//!   same validations (exists, `product-type` kind, not already assigned,
//!   no `is_variant_only` on PRODUCT ops, `variant_selection` only on
//!   VARIANT ops).
//! - unassign deletes from BOTH assignment tables (Django removes the
//!   attribute from the product type regardless of scope).
//! - assignment-update touches `variant_selection` on the variant row.
//! - attribute/values CRUD + reorder write `sort_order` like the dashboard
//!   drag-and-drop; value slugs are `{attribute_id}_{slug(name)}` deduplicated.
//! - attribute delete cascades manually (Django's FKs are DEFERRABLE with no
//!   DB cascade): assigned values → values → translations → all assignment
//!   tables → the attribute itself, in one transaction.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect,
    Set, TransactionTrait,
};
use serde_json::json;

use crate::{
    entities::{attribute_attribute, attribute_attributeproduct, attribute_attributevariant, attribute_attributevalue},
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Catalog(msg.into())
}

/// Django `slugify`: lowercase, non-alphanumeric runs become one hyphen.
pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut dash = false;
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() { "x".to_string() } else { out }
}

async fn product_type_exists(db: &impl ConnectionTrait, pt_id: i32) -> Result<bool> {
    Ok(crate::entities::product_producttype::Entity::find_by_id(pt_id)
        .select_only()
        .column(crate::entities::product_producttype::Column::Id)
        .into_tuple::<i32>()
        .one(db)
        .await?
        .is_some())
}

struct AttrRow {
    kind: String,
    is_variant_only: bool,
    input_type: String,
}

async fn attr_row(db: &impl ConnectionTrait, attr_id: i32) -> Result<Option<AttrRow>> {
    type A = attribute_attribute::Entity;
    use attribute_attribute::Column as ACol;
    Ok(A::find_by_id(attr_id)
        .select_only()
        .column(ACol::Type)
        .column(ACol::IsVariantOnly)
        .column(ACol::InputType)
        .into_tuple::<(String, bool, String)>()
        .one(db)
        .await?
        .map(|(kind, is_variant_only, input_type)| AttrRow { kind, is_variant_only, input_type }))
}

async fn assigned_elsewhere(
    db: &impl ConnectionTrait,
    pt_id: i32,
    attr_id: i32,
) -> Result<bool> {
    let in_p = attribute_attributeproduct::Entity::find()
        .select_only()
        .column(attribute_attributeproduct::Column::Id)
        .filter(attribute_attributeproduct::Column::ProductTypeId.eq(pt_id))
        .filter(attribute_attributeproduct::Column::AttributeId.eq(attr_id))
        .into_tuple::<i32>()
        .one(db)
        .await?
        .is_some();
    if in_p {
        return Ok(true);
    }
    Ok(attribute_attributevariant::Entity::find()
        .select_only()
        .column(attribute_attributevariant::Column::Id)
        .filter(attribute_attributevariant::Column::ProductTypeId.eq(pt_id))
        .filter(attribute_attributevariant::Column::AttributeId.eq(attr_id))
        .into_tuple::<i32>()
        .one(db)
        .await?
        .is_some())
}

async fn max_sort(
    db: &impl ConnectionTrait,
    table: &str,
    scope_col: &str,
    scope_id: i32,
) -> Result<i32> {
    use sea_orm::Statement;
    let row: Option<(Option<i32>,)> = db
        .query_one(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT MAX(sort_order) FROM {table} WHERE {scope_col} = $1"),
            [scope_id.into()],
        ))
        .await?
        .map(|r| r.try_get::<Option<i32>>("", "max").map(|m| (m,)))
        .transpose()
        .map_err(DbError::SeaOrm)?;
    Ok(row.and_then(|(m,)| m).unwrap_or(0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignKind {
    Product,
    Variant,
}

pub struct AssignOp {
    pub attr_id: i32,
    pub kind: AssignKind,
    pub variant_selection: bool,
}

/// Assign attributes to a product type (Django `ProductAttributeAssign`).
pub async fn assign_attributes(
    db: &sea_orm::DatabaseConnection,
    product_type_id: i32,
    ops: &[AssignOp],
) -> Result<()> {
    if !product_type_exists(db, product_type_id).await? {
        return Err(fail("product type not found"));
    }
    // Validate everything before writing anything (Django raises the full
    // error dict; we surface the first failure as errors[]).
    for op in ops {
        let Some(a) = attr_row(db, op.attr_id).await? else {
            return Err(fail(format!("attribute {} doesn't exist", op.attr_id)));
        };
        if a.kind != "product-type" {
            return Err(fail("only product attributes can be assigned"));
        }
        if op.kind == AssignKind::Product && a.is_variant_only {
            return Err(fail("cannot assign variant only attributes"));
        }
        if op.variant_selection && op.kind != AssignKind::Variant {
            return Err(fail(format!(
                "attribute {} is not VARIANT-scoped and cannot use variant_selection",
                op.attr_id
            )));
        }
        if assigned_elsewhere(db, product_type_id, op.attr_id).await? {
            return Err(fail(format!(
                "attribute {} has already been assigned to this product type",
                op.attr_id
            )));
        }
    }
    let txn = db.begin().await?;
    for op in ops {
        match op.kind {
            AssignKind::Product => {
                let so = max_sort(&txn, "attribute_attributeproduct", "product_type_id", product_type_id).await? + 1;
                attribute_attributeproduct::ActiveModel {
                    attribute_id: Set(op.attr_id),
                    product_type_id: Set(product_type_id),
                    sort_order: Set(Some(so)),
                    ..Default::default()
                }
                .insert(&txn)
                .await?;
            }
            AssignKind::Variant => {
                let so = max_sort(&txn, "attribute_attributevariant", "product_type_id", product_type_id).await? + 1;
                attribute_attributevariant::ActiveModel {
                    attribute_id: Set(op.attr_id),
                    product_type_id: Set(product_type_id),
                    sort_order: Set(Some(so)),
                    variant_selection: Set(op.variant_selection),
                    ..Default::default()
                }
                .insert(&txn)
                .await?;
            }
        }
    }
    txn.commit().await?;
    Ok(())
}

/// Unassign attributes from a product type (both scopes, like Django).
pub async fn unassign_attributes(
    db: &impl ConnectionTrait,
    product_type_id: i32,
    attr_ids: &[i32],
) -> Result<u64> {
    if !product_type_exists(db, product_type_id).await? {
        return Err(fail("product type not found"));
    }
    let a = attribute_attributeproduct::Entity::delete_many()
        .filter(attribute_attributeproduct::Column::ProductTypeId.eq(product_type_id))
        .filter(attribute_attributeproduct::Column::AttributeId.is_in(attr_ids.to_vec()))
        .exec(db)
        .await?;
    let b = attribute_attributevariant::Entity::delete_many()
        .filter(attribute_attributevariant::Column::ProductTypeId.eq(product_type_id))
        .filter(attribute_attributevariant::Column::AttributeId.is_in(attr_ids.to_vec()))
        .exec(db)
        .await?;
    Ok(a.rows_affected + b.rows_affected)
}

/// Update `variant_selection` on variant assignments
/// (Django `ProductAttributeAssignmentUpdate`).
pub async fn update_attribute_assignment(
    db: &impl ConnectionTrait,
    product_type_id: i32,
    ops: &[(i32, bool)],
) -> Result<()> {
    for (attr_id, variant_selection) in ops {
        let n = attribute_attributevariant::Entity::update_many()
            .col_expr(
                attribute_attributevariant::Column::VariantSelection,
                sea_orm::sea_query::Expr::value(*variant_selection),
            )
            .filter(attribute_attributevariant::Column::ProductTypeId.eq(product_type_id))
            .filter(attribute_attributevariant::Column::AttributeId.eq(*attr_id))
            .exec(db)
            .await?;
        if n.rows_affected == 0 {
            return Err(fail(format!(
                "attribute {attr_id} is not assigned as a variant attribute of this product type"
            )));
        }
    }
    Ok(())
}

pub struct AttributeCreate {
    pub name: String,
    pub slug: Option<String>,
    pub input_type: String,
    pub attr_type: String,
    pub entity_type: Option<String>,
    pub unit: Option<String>,
    pub value_required: bool,
    pub external_reference: Option<String>,
    /// Reference page/product type ids (stored when entity_type is set).
    pub reference_types: Vec<i32>,
}

/// Replace the reference-type rows for a REFERENCE attribute (Django
/// `AttributeCreate/Update.reference_types`).
async fn set_reference_types(
    db: &impl ConnectionTrait,
    attr_id: i32,
    entity_type: Option<&str>,
    type_ids: &[i32],
) -> Result<()> {
    use sea_orm::Statement;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM attribute_attribute_reference_page_types WHERE attribute_id = $1",
        [attr_id.into()],
    ))
    .await?;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM attribute_attribute_reference_product_types WHERE attribute_id = $1",
        [attr_id.into()],
    ))
    .await?;
    match entity_type {
        Some("Page") => {
            use crate::entities::attribute_attribute_reference_page_types as R;
            for tid in type_ids {
                R::ActiveModel {
                    attribute_id: Set(attr_id),
                    pagetype_id: Set(*tid),
                    ..Default::default()
                }
                .insert(db)
                .await?;
            }
        }
        Some("Product") => {
            use crate::entities::attribute_attribute_reference_product_types as R;
            for tid in type_ids {
                R::ActiveModel {
                    attribute_id: Set(attr_id),
                    producttype_id: Set(*tid),
                    ..Default::default()
                }
                .insert(db)
                .await?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Create an attribute (Django `AttributeCreate`).
pub async fn create_attribute(db: &impl ConnectionTrait, input: &AttributeCreate) -> Result<i32> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(fail("name is required"));
    }
    let slug = slugify(input.slug.as_deref().unwrap_or(name));
    let taken = attribute_attribute::Entity::find()
        .select_only()
        .column(attribute_attribute::Column::Id)
        .filter(attribute_attribute::Column::Slug.eq(&slug))
        .into_tuple::<i32>()
        .one(db)
        .await?
        .is_some();
    if taken {
        return Err(fail(format!("slug {slug} already exists")));
    }
    // Django field defaults (several are NOT NULL with no DB default).
    let id = attribute_attribute::ActiveModel {
        name: Set(name.to_string()),
        slug: Set(slug),
        input_type: Set(input.input_type.clone()),
        r#type: Set(input.attr_type.clone()),
        entity_type: Set(input.entity_type.clone()),
        unit: Set(input.unit.clone()),
        value_required: Set(input.value_required),
        external_reference: Set(input.external_reference.clone()),
        visible_in_storefront: Set(true),
        is_variant_only: Set(false),
        filterable_in_storefront: Set(false),
        filterable_in_dashboard: Set(false),
        available_in_grid: Set(false),
        storefront_search_position: Set(0),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        ..Default::default()
    }
    .insert(db)
    .await?
    .id;
    set_reference_types(db, id, input.entity_type.as_deref(), &input.reference_types).await?;
    Ok(id)
}

#[derive(Debug, Default)]
pub struct AttributePatch {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub unit: Option<String>,
    pub value_required: Option<bool>,
    pub is_variant_only: Option<bool>,
    pub visible_in_storefront: Option<bool>,
    pub filterable_in_storefront: Option<bool>,
    pub filterable_in_dashboard: Option<bool>,
    pub storefront_search_position: Option<i32>,
    pub available_in_grid: Option<bool>,
    pub external_reference: Option<String>,
    pub entity_type: Option<String>,
    /// `Some` replaces reference types; `None` leaves them alone.
    pub reference_types: Option<Vec<i32>>,
}

/// Update an attribute + its values (Django `AttributeUpdate`: scalar patch
/// plus `addValues` / `removeValues` in the same mutation).
pub async fn update_attribute(
    db: &sea_orm::DatabaseConnection,
    attr_id: i32,
    patch: &AttributePatch,
    add_values: &[ValueCreate],
    remove_value_ids: &[i32],
) -> Result<()> {
    let txn = db.begin().await?;
    let Some(m) = attribute_attribute::Entity::find_by_id(attr_id).one(&txn).await? else {
        txn.rollback().await?;
        return Err(fail("attribute not found"));
    };
    // Slug change must stay unique (Django validates).
    if let Some(s) = patch.slug.as_ref() {
        let slug = slugify(s);
        let clash = attribute_attribute::Entity::find()
            .select_only()
            .column(attribute_attribute::Column::Id)
            .filter(attribute_attribute::Column::Slug.eq(&slug))
            .filter(attribute_attribute::Column::Id.ne(attr_id))
            .into_tuple::<i32>()
            .one(&txn)
            .await?
            .is_some();
        if clash {
            txn.rollback().await?;
            return Err(fail(format!("slug {slug} already exists")));
        }
    }
    {
        let mut am: attribute_attribute::ActiveModel = m.into();
        if let Some(n) = patch.name.as_ref() {
            am.name = Set(n.clone());
        }
        if let Some(s) = patch.slug.as_ref() {
            am.slug = Set(slugify(s));
        }
        if let Some(u) = patch.unit.as_ref() {
            am.unit = Set(Some(u.clone()));
        }
        if let Some(v) = patch.value_required {
            am.value_required = Set(v);
        }
        if let Some(v) = patch.is_variant_only {
            am.is_variant_only = Set(v);
        }
        if let Some(v) = patch.visible_in_storefront {
            am.visible_in_storefront = Set(v);
        }
        if let Some(v) = patch.filterable_in_storefront {
            am.filterable_in_storefront = Set(v);
        }
        if let Some(v) = patch.filterable_in_dashboard {
            am.filterable_in_dashboard = Set(v);
        }
        if let Some(v) = patch.storefront_search_position {
            am.storefront_search_position = Set(v);
        }
        if let Some(v) = patch.available_in_grid {
            am.available_in_grid = Set(v);
        }
        if patch.external_reference.is_some() {
            am.external_reference = Set(patch.external_reference.clone());
        }
        if patch.entity_type.is_some() {
            am.entity_type = Set(patch.entity_type.clone());
        }
        am.update(&txn).await?;
    }
    if patch.reference_types.is_some() || patch.entity_type.is_some() {
        // Resolve the effective entity type for the reference rows.
        let et: Option<String> = attribute_attribute::Entity::find_by_id(attr_id)
            .select_only()
            .column(attribute_attribute::Column::EntityType)
            .into_tuple::<Option<String>>()
            .one(&txn)
            .await?
            .flatten();
        set_reference_types(
            &txn,
            attr_id,
            et.as_deref(),
            patch.reference_types.as_deref().unwrap_or(&[]),
        )
        .await?;
    }
    for vid in remove_value_ids {
        delete_attribute_value(&txn, *vid).await?;
    }
    // Django `AttributeUpdate.addValues`: always CREATE new values (the
    // update input carries no id — it is create-shaped).
    for v in add_values {
        create_attribute_value(&txn, attr_id, v).await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Delete an attribute with its full dependent tree (manual cascade — the
/// Django FKs are DEFERRABLE with no DB-level cascade).
pub async fn delete_attribute(db: &sea_orm::DatabaseConnection, attr_id: i32) -> Result<()> {
    let txn = db.begin().await?;
    let value_ids: Vec<i32> = attribute_attributevalue::Entity::find()
        .select_only()
        .column(attribute_attributevalue::Column::Id)
        .filter(attribute_attributevalue::Column::AttributeId.eq(attr_id))
        .into_tuple::<i32>()
        .all(&txn)
        .await?;
    if !value_ids.is_empty() {
        for (entity, col) in [
            ("attribute_assignedproductattributevalue", "value_id"),
            ("attribute_assignedvariantattributevalue", "value_id"),
            ("attribute_assignedpageattributevalue", "value_id"),
            ("attribute_assigneduserattributevalue", "value_id"),
        ] {
            txn.execute(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                format!("DELETE FROM {entity} WHERE {col} = ANY($1)"),
                [value_ids.clone().into()],
            ))
            .await?;
        }
        txn.execute(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM attribute_attributevaluetranslation WHERE attribute_value_id = ANY($1)",
            [value_ids.clone().into()],
        ))
        .await?;
        attribute_attributevalue::Entity::delete_many()
            .filter(attribute_attributevalue::Column::Id.is_in(value_ids))
            .exec(&txn)
            .await?;
    }
    txn.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM attribute_attributetranslation WHERE attribute_id = $1",
        [attr_id.into()],
    ))
    .await?;
    for (entity, col) in [
        ("attribute_attributeproduct", "attribute_id"),
        ("attribute_attributevariant", "attribute_id"),
        ("attribute_attributepage", "attribute_id"),
        ("attribute_attributecustomertype", "attribute_id"),
        ("attribute_attribute_reference_product_types", "attribute_id"),
        ("attribute_attribute_reference_page_types", "attribute_id"),
    ] {
        txn.execute(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            format!("DELETE FROM {entity} WHERE {col} = $1"),
            [attr_id.into()],
        ))
        .await?;
    }
    let n = attribute_attribute::Entity::delete_by_id(attr_id).exec(&txn).await?;
    if n.rows_affected == 0 {
        txn.rollback().await?;
        return Err(fail("attribute not found"));
    }
    txn.commit().await?;
    Ok(())
}

pub async fn bulk_delete_attributes(db: &sea_orm::DatabaseConnection, ids: &[i32]) -> Result<u64> {
    let mut n = 0;
    for id in ids {
        // Best-effort per row (Django collects per-object errors; a missing
        // id must not abort the surviving deletes).
        if delete_attribute(db, *id).await.is_ok() {
            n += 1;
        }
    }
    Ok(n)
}

pub struct ValueCreate {
    pub name: String,
    pub value: Option<String>,
    pub plain_text: Option<String>,
    pub rich_text: Option<serde_json::Value>,
    pub file_url: Option<String>,
    pub content_type: Option<String>,
    pub external_reference: Option<String>,
}

async fn unique_value_slug(db: &impl ConnectionTrait, attr_id: i32, name: &str) -> Result<String> {
    let base = format!("{attr_id}_{}", slugify(name));
    let mut slug = base.clone();
    let mut n = 2;
    loop {
        let taken = attribute_attributevalue::Entity::find()
            .select_only()
            .column(attribute_attributevalue::Column::Id)
            .filter(attribute_attributevalue::Column::Slug.eq(&slug))
            .into_tuple::<i32>()
            .one(db)
            .await?
            .is_some();
        if !taken {
            return Ok(slug);
        }
        slug = format!("{base}-{n}");
        n += 1;
    }
}

/// Create an attribute value (Django `AttributeValueCreate`).
pub async fn create_attribute_value(
    db: &impl ConnectionTrait,
    attr_id: i32,
    v: &ValueCreate,
) -> Result<i32> {
    let a = attr_row(db, attr_id).await?.ok_or_else(|| fail("attribute not found"))?;
    // Django `AttributeMixin.clean_values`: values only on choice types.
    if !matches!(a.input_type.as_str(), "dropdown" | "multiselect" | "swatch") {
        return Err(fail(format!(
            "values cannot be used with input type {}",
            a.input_type
        )));
    }
    let name = v.name.trim();
    if name.is_empty() {
        return Err(fail("value name is required"));
    }
    let slug = unique_value_slug(db, attr_id, name).await?;
    let so = max_sort(db, "attribute_attributevalue", "attribute_id", attr_id).await? + 1;
    let id = attribute_attributevalue::ActiveModel {
        attribute_id: Set(attr_id),
        name: Set(name.to_string()),
        slug: Set(slug),
        value: Set(v.value.clone().unwrap_or_default()),
        plain_text: Set(v.plain_text.clone()),
        rich_text: Set(v.rich_text.clone()),
        file_url: Set(v.file_url.clone()),
        content_type: Set(v.content_type.clone()),
        external_reference: Set(v.external_reference.clone()),
        sort_order: Set(Some(so)),
        ..Default::default()
    }
    .insert(db)
    .await?
    .id;
    Ok(id)
}

#[derive(Debug, Default)]
pub struct ValuePatch {
    pub name: Option<String>,
    pub value: Option<String>,
    pub plain_text: Option<String>,
    pub rich_text: Option<serde_json::Value>,
    pub file_url: Option<String>,
    pub content_type: Option<String>,
    pub external_reference: Option<String>,
}

/// Update an attribute value (Django `AttributeValueUpdate`).
pub async fn update_attribute_value(
    db: &impl ConnectionTrait,
    value_id: i32,
    patch: &ValuePatch,
) -> Result<()> {
    let Some(m) = attribute_attributevalue::Entity::find_by_id(value_id).one(db).await? else {
        return Err(fail("attribute value not found"));
    };
    let mut am: attribute_attributevalue::ActiveModel = m.into();
    if let Some(n) = patch.name.as_ref() {
        am.name = Set(n.clone());
    }
    if let Some(v) = patch.value.as_ref() {
        am.value = Set(v.clone());
    }
    if patch.plain_text.is_some() {
        am.plain_text = Set(patch.plain_text.clone());
    }
    if let Some(r) = patch.rich_text.as_ref() {
        am.rich_text = Set(Some(r.clone()));
    }
    if patch.file_url.is_some() {
        am.file_url = Set(patch.file_url.clone());
    }
    if patch.content_type.is_some() {
        am.content_type = Set(patch.content_type.clone());
    }
    if patch.external_reference.is_some() {
        am.external_reference = Set(patch.external_reference.clone());
    }
    am.update(db).await?;
    Ok(())
}

/// Delete one attribute value with its assignments.
pub async fn delete_attribute_value(db: &impl ConnectionTrait, value_id: i32) -> Result<()> {
    for (entity, col) in [
        ("attribute_assignedproductattributevalue", "value_id"),
        ("attribute_assignedvariantattributevalue", "value_id"),
        ("attribute_assignedpageattributevalue", "value_id"),
        ("attribute_assigneduserattributevalue", "value_id"),
        ("attribute_attributevaluetranslation", "attribute_value_id"),
    ] {
        db.execute(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            format!("DELETE FROM {entity} WHERE {col} = $1"),
            [value_id.into()],
        ))
        .await?;
    }
    let n = attribute_attributevalue::Entity::delete_by_id(value_id).exec(db).await?;
    if n.rows_affected == 0 {
        return Err(fail("attribute value not found"));
    }
    Ok(())
}

pub async fn bulk_delete_attribute_values(
    db: &impl ConnectionTrait,
    ids: &[i32],
) -> Result<u64> {
    let mut n = 0;
    for id in ids {
        if delete_attribute_value(db, *id).await.is_ok() {
            n += 1;
        }
    }
    Ok(n)
}

/// Reorder values (`moves: [{id, sortOrder}]`, Django `AttributeReorderValues`).
pub async fn reorder_attribute_values(
    db: &impl ConnectionTrait,
    attr_id: i32,
    moves: &[(i32, i32)],
) -> Result<()> {
    for (value_id, sort_order) in moves {
        let n = attribute_attributevalue::Entity::update_many()
            .col_expr(
                attribute_attributevalue::Column::SortOrder,
                sea_orm::sea_query::Expr::value(Some(*sort_order)),
            )
            .filter(attribute_attributevalue::Column::Id.eq(*value_id))
            .filter(attribute_attributevalue::Column::AttributeId.eq(attr_id))
            .exec(db)
            .await?;
        if n.rows_affected == 0 {
            return Err(fail(format!("value {value_id} does not belong to attribute {attr_id}")));
        }
    }
    Ok(())
}

/// Reorder products inside a collection
/// (Django `CollectionReorderProducts`).
pub async fn reorder_collection_products(
    db: &impl ConnectionTrait,
    collection_id: i32,
    moves: &[(i32, i32)],
) -> Result<()> {
    use crate::entities::product_collectionproduct;
    for (product_id, sort_order) in moves {
        let n = product_collectionproduct::Entity::update_many()
            .col_expr(
                product_collectionproduct::Column::SortOrder,
                sea_orm::sea_query::Expr::value(Some(*sort_order)),
            )
            .filter(product_collectionproduct::Column::CollectionId.eq(collection_id))
            .filter(product_collectionproduct::Column::ProductId.eq(*product_id))
            .exec(db)
            .await?;
        if n.rows_affected == 0 {
            return Err(fail(format!(
                "product {product_id} is not in collection {collection_id}"
            )));
        }
    }
    Ok(())
}
