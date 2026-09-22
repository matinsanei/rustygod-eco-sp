//! Catalog writes — Saleor dashboard mutations (`productUpdate/Delete`,
//! `productVariantCreate/Update/Delete`, channel listings, stocks,
//! category/collection CRUD) executed against the Django tables.
//!
//! Delete order mirrors Django's collector (FK-safe, no DB cascades):
//! order lines `SET NULL` their variant, checkout lines are deleted,
//! translations/listings/stocks/media/m2m rows go before their owners.
//! Reads stay in `catalog.rs`; this module only mutates.

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement, TransactionTrait};
use uuid::Uuid;

use crate::{DbError, Result};

fn now() -> sea_orm::prelude::DateTimeWithTimeZone {
    Utc::now().into()
}

async fn exec(db: &impl ConnectionTrait, sql: &str, params: Vec<sea_orm::Value>) -> Result<u64> {
    let st = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params);
    Ok(db.execute(st).await.map_err(DbError::SeaOrm)?.rows_affected())
}

/// Best-effort plaintext for EditorJS JSON descriptions (Saleor's
/// `description_plaintext` feeds search; exact JS sanitizer not ported).
pub fn plaintext_of(desc_json: &str) -> String {
    let mut out = String::new();
    let mut rest = desc_json;
    while let Some(i) = rest.find("\"text\"") {
        rest = &rest[i + 6..];
        let rest2 = rest.trim_start_matches([':', ' ', '"']);
        let mut val = String::new();
        let mut esc = false;
        let mut closed = false;
        for c in rest2.chars() {
            if esc {
                match c {
                    'n' => val.push('\n'),
                    't' => val.push('\t'),
                    other => val.push(other),
                }
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                closed = true;
                break;
            } else {
                val.push(c);
            }
        }
        if !closed {
            break;
        }
        if !val.trim().is_empty() {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(val.trim());
        }
        rest = &rest2[val.len()..];
    }
    out
}

async fn slug_taken(db: &impl ConnectionTrait, table: &str, slug: &str, not_id: Option<i32>) -> Result<bool> {
    let sql = match not_id {
        Some(_) => format!("SELECT 1 FROM {table} WHERE slug = $1 AND id <> $2 LIMIT 1"),
        None => format!("SELECT 1 FROM {table} WHERE slug = $1 LIMIT 1"),
    };
    let mut params: Vec<sea_orm::Value> = vec![slug.to_string().into()];
    if let Some(n) = not_id {
        params.push(n.into());
    }
    let st = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params);
    Ok(db.query_one(st).await.map_err(DbError::SeaOrm)?.is_some())
}

// ---------------------------------------------------------------------------
// products
// ---------------------------------------------------------------------------

/// Identity-field patch for `productUpdate` (dashboard `ProductInput`).
/// `attributes` are accepted-ignored (attribute-write parity is its own
/// milestone); everything else persists.
pub struct ProductPatch {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<Option<i32>>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub rating: Option<f64>,
    pub tax_class_id: Option<Option<i32>>,
    pub charge_taxes: Option<bool>,
    pub collections: Option<Vec<i32>>,
    pub metadata: Option<serde_json::Value>,
    pub private_metadata: Option<serde_json::Value>,
}

pub async fn update_product(db: &DatabaseConnection, id: i32, p: &ProductPatch) -> Result<()> {
    let txn = db.begin().await.map_err(DbError::SeaOrm)?;
    if let Some(slug) = p.slug.as_deref() {
        if slug_taken(&txn, "product_product", slug, Some(id)).await? {
            return Err(DbError::Catalog("slug already exists".into()));
        }
    }
    let mut sets: Vec<String> = vec!["updated_at = now()".into(), "search_index_dirty = TRUE".into()];
    let mut params: Vec<sea_orm::Value> = vec![];
    let mut push = |col: &str, v: sea_orm::Value| {
        params.push(v);
        sets.push(format!("{col} = ${}", params.len() + 1));
    };
    if let Some(v) = p.name.as_deref() {
        push("name", v.to_string().into());
    }
    if let Some(v) = p.slug.as_deref() {
        push("slug", v.to_string().into());
    }
    if let Some(v) = p.description.as_deref() {
        let json: serde_json::Value =
            serde_json::from_str(v).unwrap_or_else(|_| serde_json::Value::String(v.to_string()));
        push("description", json.to_string().into());
        push("description_plaintext", plaintext_of(v).into());
    }
    if let Some(v) = p.category_id {
        push("category_id", v.into());
    }
    if let Some(v) = p.seo_title.as_deref() {
        push("seo_title", v.to_string().into());
    }
    if let Some(v) = p.seo_description.as_deref() {
        push("seo_description", v.to_string().into());
    }
    if let Some(v) = p.rating {
        push("rating", v.into());
    }
    if let Some(v) = p.tax_class_id {
        push("tax_class_id", v.into());
    }
    if let Some(v) = p.charge_taxes {
        push("charge_taxes", v.into());
    }
    if let Some(v) = p.metadata.as_ref() {
        push("metadata", v.to_string().into());
    }
    if let Some(v) = p.private_metadata.as_ref() {
        push("private_metadata", v.to_string().into());
    }
    // $1 is the id; value placeholders start at $2 (params pushed after id).
    let mut all: Vec<sea_orm::Value> = vec![id.into()];
    all.extend(params);
    exec(
        &txn,
        &format!("UPDATE product_product SET {} WHERE id = $1", sets.join(", ")),
        all,
    )
    .await?;
    if let Some(colls) = p.collections.as_ref() {
        exec(&txn, "DELETE FROM product_collectionproduct WHERE product_id = $1", vec![id.into()]).await?;
        for c in colls {
            exec(
                &txn,
                "INSERT INTO product_collectionproduct (collection_id, product_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                vec![(*c).into(), id.into()],
            )
            .await?;
        }
    }
    txn.commit().await.map_err(DbError::SeaOrm)?;
    Ok(())
}

/// `productDelete` in Django-collector order. Order history survives with
/// `variant_id` nulled (`SET_NULL`); checkout lines cascade (deleted).
pub async fn delete_product(db: &DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await.map_err(DbError::SeaOrm)?;
    let st = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id FROM product_productvariant WHERE product_id = $1",
        [id.into()],
    );
    let vids: Vec<i32> = txn
        .query_all(st)
        .await
        .map_err(DbError::SeaOrm)?
        .into_iter()
        .filter_map(|r| r.try_get::<i32>("", "id").ok())
        .collect();
    delete_variants_tx(&txn, &vids).await?;
    exec(&txn, "UPDATE product_product SET default_variant_id = NULL WHERE id = $1", vec![id.into()]).await?;
    for (table, col) in [
        ("product_producttranslation", "product_id"),
        ("product_productchannellisting", "product_id"),
        ("product_productmedia", "product_id"),
        ("product_collectionproduct", "product_id"),
        ("attribute_assignedproductattributevalue", "product_id"),
    ] {
        exec(&txn, &format!("DELETE FROM {table} WHERE {col} = $1"), vec![id.into()]).await?;
    }
    let n = exec(&txn, "DELETE FROM product_product WHERE id = $1", vec![id.into()]).await?;
    if n == 0 {
        return Err(DbError::Catalog("product not found".into()));
    }
    txn.commit().await.map_err(DbError::SeaOrm)?;
    Ok(())
}

async fn delete_variants_tx(db: &impl ConnectionTrait, vids: &[i32]) -> Result<()> {
    if vids.is_empty() {
        return Ok(());
    }
    let params: Vec<sea_orm::Value> = vids.iter().map(|v| (*v).into()).collect();
    let list = (1..=vids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
    // Order history survives, detached (Django SET_NULL).
    exec(db, &format!("UPDATE order_orderline SET variant_id = NULL WHERE variant_id IN ({list})"), params.clone()).await?;
    for (table, col) in [
        ("checkout_checkoutline", "variant_id"),
        ("product_productvariantchannellisting", "variant_id"),
        ("warehouse_stock", "product_variant_id"),
        ("product_variantmedia", "variant_id"),
        ("product_productvarianttranslation", "product_variant_id"),
        ("discount_promotionrule_gifts", "productvariant_id"),
        ("discount_promotionrule_variants", "productvariant_id"),
        ("attribute_assignedvariantattributevalue", "variant_id"),
        ("attribute_assignedvariantattribute", "variant_id"),
    ] {
        exec(db, &format!("DELETE FROM {table} WHERE {col} IN ({list})"), params.clone()).await?;
    }
    exec(db, &format!("DELETE FROM product_productvariant WHERE id IN ({list})"), params).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// variants
// ---------------------------------------------------------------------------

pub async fn create_variant(
    db: &DatabaseConnection,
    product_id: i32,
    sku: Option<String>,
    name: Option<String>,
    track_inventory: bool,
    quantity_limit: Option<i32>,
    external_reference: Option<String>,
) -> Result<i32> {
    if let Some(s) = sku.as_deref() {
        if !s.is_empty() {
            let st = Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT 1 FROM product_productvariant WHERE sku = $1 LIMIT 1",
                [s.to_string().into()],
            );
            if db.query_one(st).await.map_err(DbError::SeaOrm)?.is_some() {
                return Err(DbError::Catalog("sku already exists".into()));
            }
        }
    }
    let t = now();
    let st = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO product_productvariant \
         (sku, name, product_id, track_inventory, quantity_limit_per_customer, \
          external_reference, metadata, private_metadata, is_preorder, \
          created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, '{}', '{}', FALSE, $7, $7) RETURNING id",
        [
            sku.unwrap_or_default().into(),
            name.unwrap_or_default().into(),
            product_id.into(),
            track_inventory.into(),
            quantity_limit.into(),
            external_reference.into(),
            t.into(),
        ],
    );
    let row = db.query_one(st).await.map_err(DbError::SeaOrm)?.ok_or_else(|| DbError::Catalog("variant insert failed".into()))?;
    Ok(row.try_get::<i32>("", "id").map_err(DbError::SeaOrm)?)
}

/// Identity-field patch for `productVariantUpdate`.
pub struct VariantPatch {
    pub sku: Option<Option<String>>,
    pub name: Option<String>,
    pub track_inventory: Option<bool>,
    pub quantity_limit_per_customer: Option<Option<i32>>,
    pub external_reference: Option<Option<String>>,
    pub metadata: Option<serde_json::Value>,
    pub private_metadata: Option<serde_json::Value>,
}

pub async fn update_variant(db: &DatabaseConnection, id: i32, p: &VariantPatch) -> Result<()> {
    if let Some(Some(s)) = p.sku.as_ref() {
        if !s.is_empty() {
            let st = Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT 1 FROM product_productvariant WHERE sku = $1 AND id <> $2 LIMIT 1",
                [s.clone().into(), id.into()],
            );
            if db.query_one(st).await.map_err(DbError::SeaOrm)?.is_some() {
                return Err(DbError::Catalog("sku already exists".into()));
            }
        }
    }
    let mut params: Vec<sea_orm::Value> = vec![id.into()];
    let mut sets = vec!["updated_at = now()".to_string()];
    let mut push = |col: &str, v: sea_orm::Value| {
        params.push(v);
        sets.push(format!("{col} = ${}", params.len()));
    };
    if let Some(v) = p.sku.as_ref() {
        push("sku", v.clone().into());
    }
    if let Some(v) = p.name.as_deref() {
        push("name", v.to_string().into());
    }
    if let Some(v) = p.track_inventory {
        push("track_inventory", v.into());
    }
    if let Some(v) = p.quantity_limit_per_customer {
        push("quantity_limit_per_customer", v.into());
    }
    if let Some(v) = p.external_reference.as_ref() {
        push("external_reference", v.clone().into());
    }
    if let Some(v) = p.metadata.as_ref() {
        push("metadata", v.to_string().into());
    }
    if let Some(v) = p.private_metadata.as_ref() {
        push("private_metadata", v.to_string().into());
    }
    let n = exec(
        db,
        &format!("UPDATE product_productvariant SET {} WHERE id = $1", sets.join(", ")),
        params,
    )
    .await?;
    if n == 0 {
        return Err(DbError::Catalog("variant not found".into()));
    }
    Ok(())
}

pub async fn delete_variant(db: &DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await.map_err(DbError::SeaOrm)?;
    delete_variants_tx(&txn, &[id]).await?;
    txn.commit().await.map_err(DbError::SeaOrm)?;
    Ok(())
}

/// Upsert a variant channel listing (dashboard
/// `productVariantChannelListingUpdate`: create + update paths).
pub async fn upsert_variant_listing(
    db: &DatabaseConnection,
    variant_id: i32,
    channel_id: i32,
    price: Decimal,
    cost: Option<Decimal>,
    prior: Option<Decimal>,
) -> Result<()> {
    let txn = db.begin().await.map_err(DbError::SeaOrm)?;
    let sel = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id FROM product_productvariantchannellisting WHERE variant_id = $1 AND channel_id = $2",
        [variant_id.into(), channel_id.into()],
    );
    let existing: Option<i32> = txn
        .query_one(sel)
        .await
        .map_err(DbError::SeaOrm)?
        .and_then(|r| r.try_get::<i32>("", "id").ok());
    // Saleor stores currency on the listing; channel currency is canonical.
    let cur: Option<String> = {
        let st = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT currency_code FROM channel_channel WHERE id = $1",
            [channel_id.into()],
        );
        txn.query_one(st)
            .await
            .map_err(DbError::SeaOrm)?
            .and_then(|r| r.try_get::<String>("", "currency_code").ok())
    };
    let currency = cur.unwrap_or_else(|| "USD".into());
    match existing {
        Some(lid) => {
            let mut params: Vec<sea_orm::Value> = vec![lid.into()];
            let mut sets = vec![];
            let mut push = |col: &str, v: sea_orm::Value| {
                params.push(v);
                sets.push(format!("{col} = ${}", params.len()));
            };
            push("price_amount", price.into());
            push("currency", currency.into());
            if cost.is_some() {
                push("cost_price_amount", cost.into());
            }
            if prior.is_some() {
                push("prior_price_amount", prior.into());
            }
            // Keep the denormalized discounted price in step (promotions
            // recompute it async in Saleor; dashboard reads this column).
            push("discounted_price_amount", price.into());
            exec(&txn, &format!("UPDATE product_productvariantchannellisting SET {} WHERE id = $1", sets.join(", ")), params).await?;
        }
        None => {
            exec(
                &txn,
                "INSERT INTO product_productvariantchannellisting \
                 (variant_id, channel_id, currency, price_amount, cost_price_amount, \
                  prior_price_amount, discounted_price_amount) \
                 VALUES ($1, $2, $3, $4, $5, $6, $4)",
                vec![variant_id.into(), channel_id.into(), currency.into(), price.into(), cost.into(), prior.into()],
            )
            .await?;
        }
    }
    txn.commit().await.map_err(DbError::SeaOrm)?;
    Ok(())
}

pub async fn delete_variant_listing(db: &DatabaseConnection, listing_id: i32) -> Result<()> {
    exec(db, "DELETE FROM product_productvariantchannellisting WHERE id = $1", vec![listing_id.into()]).await?;
    Ok(())
}

/// Upsert `warehouse_stock` (dashboard `productVariantStocksCreate/Update`).
pub async fn set_variant_stock(
    db: &DatabaseConnection,
    variant_id: i32,
    warehouse_id: Uuid,
    quantity: i32,
) -> Result<()> {
    let txn = db.begin().await.map_err(DbError::SeaOrm)?;
    let sel = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id FROM warehouse_stock WHERE product_variant_id = $1 AND warehouse_id = $2::uuid",
        [variant_id.into(), warehouse_id.to_string().into()],
    );
    let existing: Option<i32> = txn
        .query_one(sel)
        .await
        .map_err(DbError::SeaOrm)?
        .and_then(|r| r.try_get::<i32>("", "id").ok());
    match existing {
        Some(sid) => {
            exec(&txn, "UPDATE warehouse_stock SET quantity = $2 WHERE id = $1", vec![sid.into(), quantity.into()]).await?;
        }
        None => {
            exec(
                &txn,
                "INSERT INTO warehouse_stock (product_variant_id, warehouse_id, quantity, quantity_allocated) \
                 VALUES ($1, $2::uuid, $3, 0)",
                vec![variant_id.into(), warehouse_id.to_string().into(), quantity.into()],
            )
            .await?;
        }
    }
    txn.commit().await.map_err(DbError::SeaOrm)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// categories (MPTT) + collections
// ---------------------------------------------------------------------------

/// Insert a category honoring django-mptt positions (child of `parent_id`,
/// or a new root). Returns the new id.
pub async fn create_category(
    db: &DatabaseConnection,
    name: &str,
    slug: &str,
    description: Option<&str>,
    parent_id: Option<i32>,
) -> Result<i32> {
    if slug_taken(db, "product_category", slug, None).await? {
        return Err(DbError::Catalog("slug already exists".into()));
    }
    let txn = db.begin().await.map_err(DbError::SeaOrm)?;
    let (tree_id, lft, rght, level): (i32, i32, i32, i32) = match parent_id {
        Some(pid) => {
            let st = Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT tree_id, lft, rght, level FROM product_category WHERE id = $1",
                [pid.into()],
            );
            let r = txn.query_one(st).await.map_err(DbError::SeaOrm)?.ok_or_else(|| DbError::Catalog("parent category not found".into()))?;
            let (t, _l, pr, pl): (i32, i32, i32, i32) = (
                r.try_get("", "tree_id").map_err(DbError::SeaOrm)?,
                r.try_get("", "lft").map_err(DbError::SeaOrm)?,
                r.try_get("", "rght").map_err(DbError::SeaOrm)?,
                r.try_get("", "level").map_err(DbError::SeaOrm)?,
            );
            exec(&txn, "UPDATE product_category SET lft = lft + 2 WHERE tree_id = $1 AND lft >= $2", vec![t.into(), pr.into()]).await?;
            exec(&txn, "UPDATE product_category SET rght = rght + 2 WHERE tree_id = $1 AND rght >= $2", vec![t.into(), pr.into()]).await?;
            (t, pr, pr + 1, pl + 1)
        }
        None => {
            let st = Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT COALESCE(MAX(tree_id), 0) + 1 AS t FROM product_category",
                [],
            );
            let r = txn.query_one(st).await.map_err(DbError::SeaOrm)?.ok_or_else(|| DbError::Catalog("tree alloc failed".into()))?;
            (r.try_get("", "t").map_err(DbError::SeaOrm)?, 1, 2, 0)
        }
    };
    let desc = description.unwrap_or("");
    let desc_json: serde_json::Value =
        serde_json::from_str(desc).unwrap_or_else(|_| serde_json::Value::String(desc.to_string()));
    let st = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO product_category \
         (name, slug, description, description_plaintext, lft, rght, tree_id, level, \
          parent_id, background_image_alt, metadata, private_metadata) \
         VALUES ($1, $2, $3::jsonb, $4, $5, $6, $7, $8, $9, '', '{}', '{}') RETURNING id",
        [
            name.to_string().into(),
            slug.to_string().into(),
            desc_json.to_string().into(),
            plaintext_of(desc).into(),
            lft.into(),
            rght.into(),
            tree_id.into(),
            level.into(),
            parent_id.into(),
        ],
    );
    let row = txn.query_one(st).await.map_err(DbError::SeaOrm)?.ok_or_else(|| DbError::Catalog("category insert failed".into()))?;
    let id: i32 = row.try_get("", "id").map_err(DbError::SeaOrm)?;
    txn.commit().await.map_err(DbError::SeaOrm)?;
    Ok(id)
}

pub struct CategoryPatch {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
}

pub async fn update_category(db: &DatabaseConnection, id: i32, p: &CategoryPatch) -> Result<()> {
    if let Some(s) = p.slug.as_deref() {
        if slug_taken(db, "product_category", s, Some(id)).await? {
            return Err(DbError::Catalog("slug already exists".into()));
        }
    }
    let mut params: Vec<sea_orm::Value> = vec![id.into()];
    let mut sets = vec!["updated_at = now()".to_string()];
    let mut push = |col: &str, v: sea_orm::Value| {
        params.push(v);
        sets.push(format!("{col} = ${}", params.len()));
    };
    if let Some(v) = p.name.as_deref() {
        push("name", v.to_string().into());
    }
    if let Some(v) = p.slug.as_deref() {
        push("slug", v.to_string().into());
    }
    if let Some(v) = p.description.as_deref() {
        let json: serde_json::Value =
            serde_json::from_str(v).unwrap_or_else(|_| serde_json::Value::String(v.to_string()));
        push("description", json.to_string().into());
        push("description_plaintext", plaintext_of(v).into());
    }
    if let Some(v) = p.seo_title.as_deref() {
        push("seo_title", v.to_string().into());
    }
    if let Some(v) = p.seo_description.as_deref() {
        push("seo_description", v.to_string().into());
    }
    let n = exec(db, &format!("UPDATE product_category SET {} WHERE id = $1", sets.join(", ")), params).await?;
    if n == 0 {
        return Err(DbError::Catalog("category not found".into()));
    }
    Ok(())
}

/// Delete a category subtree (Django collector deletes descendants):
/// products `SET NULL`, translations + subtree rows removed, MPTT gap closed.
pub async fn delete_category(db: &DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await.map_err(DbError::SeaOrm)?;
    let st = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT tree_id, lft, rght FROM product_category WHERE id = $1",
        [id.into()],
    );
    let r = txn.query_one(st).await.map_err(DbError::SeaOrm)?.ok_or_else(|| DbError::Catalog("category not found".into()))?;
    let (t, l, rr): (i32, i32, i32) = (
        r.try_get("", "tree_id").map_err(DbError::SeaOrm)?,
        r.try_get("", "lft").map_err(DbError::SeaOrm)?,
        r.try_get("", "rght").map_err(DbError::SeaOrm)?,
    );
    let width = rr - l + 1;
    exec(&txn, "UPDATE product_product SET category_id = NULL WHERE category_id IN (SELECT id FROM product_category WHERE tree_id = $1 AND lft BETWEEN $2 AND $3)", vec![t.into(), l.into(), rr.into()]).await?;
    exec(&txn, "DELETE FROM product_categorytranslation WHERE category_id IN (SELECT id FROM product_category WHERE tree_id = $1 AND lft BETWEEN $2 AND $3)", vec![t.into(), l.into(), rr.into()]).await?;
    exec(&txn, "DELETE FROM product_category WHERE tree_id = $1 AND lft BETWEEN $2 AND $3", vec![t.into(), l.into(), rr.into()]).await?;
    exec(&txn, "UPDATE product_category SET lft = lft - $2 WHERE tree_id = $1 AND lft > $3", vec![t.into(), width.into(), rr.into()]).await?;
    exec(&txn, "UPDATE product_category SET rght = rght - $2 WHERE tree_id = $1 AND rght > $3", vec![t.into(), width.into(), rr.into()]).await?;
    txn.commit().await.map_err(DbError::SeaOrm)?;
    Ok(())
}

/// Collection publication lives on the per-channel listing (3.x), not the
/// collection row. Mirror the dashboard intent onto the default channel.
async fn set_collection_published(db: &impl ConnectionTrait, collection_id: i32, published: bool) -> Result<()> {
    let ch = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id FROM channel_channel WHERE slug = 'default-channel' LIMIT 1",
        [],
    );
    let ch_id: Option<i32> = db
        .query_one(ch)
        .await
        .map_err(DbError::SeaOrm)?
        .and_then(|r| r.try_get::<i32>("", "id").ok());
    let Some(ch_id) = ch_id else { return Ok(()) };
    let n = exec(
        db,
        "UPDATE product_collectionchannellisting SET is_published = $3 WHERE collection_id = $1 AND channel_id = $2",
        vec![collection_id.into(), ch_id.into(), published.into()],
    )
    .await?;
    if n == 0 && published {
        exec(
            db,
            "INSERT INTO product_collectionchannellisting (collection_id, channel_id, is_published) VALUES ($1, $2, TRUE)",
            vec![collection_id.into(), ch_id.into()],
        )
        .await?;
    }
    Ok(())
}

pub async fn create_collection(
    db: &DatabaseConnection,
    name: &str,
    slug: &str,
    description: Option<&str>,
    is_published: bool,
    products: &[i32],
) -> Result<i32> {
    if slug_taken(db, "product_collection", slug, None).await? {
        return Err(DbError::Catalog("slug already exists".into()));
    }
    let txn = db.begin().await.map_err(DbError::SeaOrm)?;
    let desc = description.unwrap_or("");
    let desc_json: serde_json::Value =
        serde_json::from_str(desc).unwrap_or_else(|_| serde_json::Value::String(desc.to_string()));
    let st = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO product_collection \
         (name, slug, description, background_image_alt, metadata, private_metadata) \
         VALUES ($1, $2, $3::jsonb, '', '{}', '{}') RETURNING id",
        [name.to_string().into(), slug.to_string().into(), desc_json.to_string().into()],
    );
    let row = txn.query_one(st).await.map_err(DbError::SeaOrm)?.ok_or_else(|| DbError::Catalog("collection insert failed".into()))?;
    let id: i32 = row.try_get("", "id").map_err(DbError::SeaOrm)?;
    set_collection_published(&txn, id, is_published).await?;
    for p in products {
        exec(&txn, "INSERT INTO product_collectionproduct (collection_id, product_id) VALUES ($1, $2) ON CONFLICT DO NOTHING", vec![id.into(), (*p).into()]).await?;
    }
    txn.commit().await.map_err(DbError::SeaOrm)?;
    Ok(id)
}

pub struct CollectionPatch {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub is_published: Option<bool>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub products: Option<Vec<i32>>,
}

pub async fn update_collection(db: &DatabaseConnection, id: i32, p: &CollectionPatch) -> Result<()> {
    if let Some(s) = p.slug.as_deref() {
        if slug_taken(db, "product_collection", s, Some(id)).await? {
            return Err(DbError::Catalog("slug already exists".into()));
        }
    }
    let txn = db.begin().await.map_err(DbError::SeaOrm)?;
    let mut params: Vec<sea_orm::Value> = vec![id.into()];
    let mut sets: Vec<String> = vec![];
    let mut push = |col: &str, v: sea_orm::Value| {
        params.push(v);
        sets.push(format!("{col} = ${}", params.len()));
    };
    if let Some(v) = p.name.as_deref() {
        push("name", v.to_string().into());
    }
    if let Some(v) = p.slug.as_deref() {
        push("slug", v.to_string().into());
    }
    if let Some(v) = p.description.as_deref() {
        let json: serde_json::Value =
            serde_json::from_str(v).unwrap_or_else(|_| serde_json::Value::String(v.to_string()));
        push("description", json.to_string().into());
    }
    if let Some(v) = p.seo_title.as_deref() {
        push("seo_title", v.to_string().into());
    }
    if let Some(v) = p.seo_description.as_deref() {
        push("seo_description", v.to_string().into());
    }
    if !sets.is_empty() {
        exec(&txn, &format!("UPDATE product_collection SET {} WHERE id = $1", sets.join(", ")), params).await?;
    }
    if let Some(pub_) = p.is_published {
        set_collection_published(&txn, id, pub_).await?;
    }
    if let Some(prods) = p.products.as_ref() {
        exec(&txn, "DELETE FROM product_collectionproduct WHERE collection_id = $1", vec![id.into()]).await?;
        for x in prods {
            exec(&txn, "INSERT INTO product_collectionproduct (collection_id, product_id) VALUES ($1, $2) ON CONFLICT DO NOTHING", vec![id.into(), (*x).into()]).await?;
        }
    }
    txn.commit().await.map_err(DbError::SeaOrm)?;
    Ok(())
}

pub async fn delete_collection(db: &DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await.map_err(DbError::SeaOrm)?;
    for (table, col) in [
        ("product_collectionproduct", "collection_id"),
        ("product_collectionchannellisting", "collection_id"),
        ("product_collectiontranslation", "collection_id"),
    ] {
        exec(&txn, &format!("DELETE FROM {table} WHERE {col} = $1"), vec![id.into()]).await?;
    }
    let n = exec(&txn, "DELETE FROM product_collection WHERE id = $1", vec![id.into()]).await?;
    if n == 0 {
        return Err(DbError::Catalog("collection not found".into()));
    }
    txn.commit().await.map_err(DbError::SeaOrm)?;
    Ok(())
}

pub async fn collection_add_products(db: &DatabaseConnection, collection_id: i32, product_ids: &[i32]) -> Result<()> {
    for p in product_ids {
        exec(db, "INSERT INTO product_collectionproduct (collection_id, product_id) VALUES ($1, $2) ON CONFLICT DO NOTHING", vec![collection_id.into(), (*p).into()]).await?;
    }
    Ok(())
}

pub async fn collection_remove_products(db: &DatabaseConnection, collection_id: i32, product_ids: &[i32]) -> Result<()> {
    if product_ids.is_empty() {
        return Ok(());
    }
    let mut params: Vec<sea_orm::Value> = vec![collection_id.into()];
    for p in product_ids {
        params.push((*p).into());
    }
    let list = (2..=params.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
    exec(db, &format!("DELETE FROM product_collectionproduct WHERE collection_id = $1 AND product_id IN ({list})"), params).await?;
    Ok(())
}
