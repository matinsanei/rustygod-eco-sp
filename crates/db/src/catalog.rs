//! Catalog reads over the Django Saleor schema.
//!
//! Price semantics mirror `saleor/product/models.py::ProductVariant.get_price`
//! and `saleor/graphql/product/` channel-aware resolution:
//! a variant's price is resolved through
//! `product_productvariantchannellisting` for the requested channel, and a
//! product is visible only when its `product_productchannellisting` row for
//! that channel has `is_published = true`.
//!
//! Mapping rule: never `SELECT *` on codegen'd entities — codegen mistypes
//! exotic Postgres columns (`tsvector`, `INTERVAL`), and decoding them fails.
//! We always project the columns we need.

use rust_decimal::Decimal;
use saleor_rustify_core::{
    money::Money,
    product::{Category, Product, ProductVariant},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
};

use crate::{
    entities::{
        channel_channel, product_category, product_product,
        product_productchannellisting, product_productvariant,
        product_productvariantchannellisting,
    },
    DbError, Result,
};

/// Slim product row: (id, name, slug, description_plaintext, product_type_id, category_id).
type ProductRow = (i32, String, String, String, i32, Option<i32>);

fn product_select() -> sea_orm::Select<product_product::Entity> {
    product_product::Entity::find()
        .select_only()
        .column(product_product::Column::Id)
        .column(product_product::Column::Name)
        .column(product_product::Column::Slug)
        .column(product_product::Column::DescriptionPlaintext)
        .column(product_product::Column::ProductTypeId)
        .column(product_product::Column::CategoryId)
}

async fn channel_id(db: &impl sea_orm::ConnectionTrait, slug: &str) -> Result<(i32, String)> {
    channel_info(db, slug).await
}

/// Public channel resolution: (channel_id, currency_code).
pub async fn channel_info(db: &impl sea_orm::ConnectionTrait, slug: &str) -> Result<(i32, String)> {    channel_channel::Entity::find()
        .select_only()
        .column(channel_channel::Column::Id)
        .column(channel_channel::Column::CurrencyCode)
        .filter(channel_channel::Column::Slug.eq(slug))
        .into_tuple::<(i32, String)>()
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(slug.to_string())))
}

/// Reverse lookup: slug for a channel id (slim select, no INTERVAL mistype).
pub async fn channel_slug_for_id(
    db: &impl sea_orm::ConnectionTrait,
    channel_id: i32,
) -> Result<String> {
    channel_channel::Entity::find_by_id(channel_id)
        .select_only()
        .column(channel_channel::Column::Slug)
        .into_tuple()
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(channel_id.to_string())))
}

fn published_filter(
    q: sea_orm::Select<product_product::Entity>,
    ch_id: i32,
) -> sea_orm::Select<product_product::Entity> {
    q.filter(
        product_product::Column::Id.in_subquery(
            sea_orm::sea_query::Query::select()
                .column(product_productchannellisting::Column::ProductId)
                .from(product_productchannellisting::Entity)
                .and_where(product_productchannellisting::Column::ChannelId.eq(ch_id))
                .and_where(product_productchannellisting::Column::IsPublished.eq(true))
                .to_owned(),
        ),
    )
}

pub async fn list_products(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
    category_id: Option<i32>,
    limit: u64,
) -> Result<Vec<Product>> {
    let (ch_id, currency) = channel_id(db, channel_slug).await?;

    let mut q = published_filter(product_select(), ch_id);
    if let Some(cat) = category_id {
        q = q.filter(product_product::Column::CategoryId.eq(cat));
    }
    let rows: Vec<ProductRow> = q.limit(limit).into_tuple().all(db).await?;

    load_products_batch(db, rows, ch_id, &currency).await
}

/// Batch assembly: 3 round-trips total (variants, listings, stocks) no
/// matter how many products — the dataloader fix for the N+1 reads.
async fn load_products_batch(
    db: &impl sea_orm::ConnectionTrait,
    rows: Vec<ProductRow>,
    ch_id: i32,
    currency: &str,
) -> Result<Vec<Product>> {
    use std::collections::HashMap;
    use crate::entities::warehouse_stock;

    let pids: Vec<i32> = rows.iter().map(|r| r.0).collect();
    let variants = product_productvariant::Entity::find()
        .filter(product_productvariant::Column::ProductId.is_in(pids))
        .all(db)
        .await?;
    let vids: Vec<i32> = variants.iter().map(|v| v.id).collect();

    let listings = product_productvariantchannellisting::Entity::find()
        .filter(product_productvariantchannellisting::Column::VariantId.is_in(vids.clone()))
        .filter(product_productvariantchannellisting::Column::ChannelId.eq(ch_id))
        .all(db)
        .await?;
    let listing_map: HashMap<i32, Decimal> = listings
        .into_iter()
        .map(|l| (l.variant_id, l.price_amount.unwrap_or(Decimal::ZERO)))
        .collect();

    let stocks = warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::ProductVariantId.is_in(vids))
        .all(db)
        .await?;
    let mut stock_map: HashMap<i32, i32> = HashMap::new();
    for s in stocks {
        *stock_map.entry(s.product_variant_id).or_insert(0) +=
            (s.quantity - s.quantity_allocated).max(0);
    }

    let mut by_product: HashMap<i32, Vec<ProductVariant>> = HashMap::new();
    for v in variants {
        let Some(price) = listing_map.get(&v.id) else { continue };
        let pv = ProductVariant {
            id: v.id.to_string(),
            product_id: v.product_id.to_string(),
            name: v.name,
            sku: v.sku.unwrap_or_default(),
            price: Money::new(*price, currency),
            quantity_available: stock_map.get(&v.id).copied().unwrap_or(0),
        };
        by_product.entry(v.product_id).or_default().push(pv);
    }

    Ok(rows
        .into_iter()
        .map(|(id, name, slug, description, product_type_id, category_id)| {
            let variants = by_product.remove(&id).unwrap_or_default();
            let default_price = variants
                .iter()
                .map(|v| v.price.clone())
                .min_by(|a, b| a.amount.cmp(&b.amount))
                .unwrap_or_else(|| Money::zero(currency));
            Product {
                id: id.to_string(),
                name,
                slug,
                description,
                product_type_id: product_type_id.to_string(),
                category_id: category_id.map(|c| c.to_string()),
                is_published: true,
                default_price,
                variants,
            }
        })
        .collect())
}

/// Products by ids with the same batched assembly (used by chat grounding
/// instead of full-catalog scans).
pub async fn products_by_ids(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
    ids: &[i32],
) -> Result<Vec<Product>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let (ch_id, currency) = channel_id(db, channel_slug).await?;
    let rows: Vec<ProductRow> = published_filter(product_select(), ch_id)
        .filter(product_product::Column::Id.is_in(ids.to_vec()))
        .into_tuple()
        .all(db)
        .await?;
    load_products_batch(db, rows, ch_id, &currency).await
}

pub async fn get_product(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
    product_id: i32,
) -> Result<Option<Product>> {    let (ch_id, currency) = channel_id(db, channel_slug).await?;
    let row: Option<ProductRow> = product_select()
        .filter(product_product::Column::Id.eq(product_id))
        .into_tuple()
        .one(db)
        .await?;
    let Some(row) = row else { return Ok(None) };

    let published = product_productchannellisting::Entity::find()
        .filter(product_productchannellisting::Column::ProductId.eq(product_id))
        .filter(product_productchannellisting::Column::ChannelId.eq(ch_id))
        .filter(product_productchannellisting::Column::IsPublished.eq(true))
        .one(db)
        .await?
        .is_some();
    if !published {
        return Ok(None);
    }
    let mut out = load_products_batch(db, vec![row], ch_id, &currency).await?;
    Ok(out.pop())
}

/// Product by id IGNORING channel publication (Saleor `product(id)` without
/// a published listing still returns the row — critical for just-created
/// products the dashboard navigates to before any listing exists).
/// Variants carry zero prices until listings are added.
pub async fn get_product_unlisted(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
    product_id: i32,
) -> Result<Option<Product>> {
    let (ch_id, currency) = channel_id(db, channel_slug).await?;
    let row: Option<ProductRow> = product_select()
        .filter(product_product::Column::Id.eq(product_id))
        .into_tuple()
        .one(db)
        .await?;
    let Some(row) = row else { return Ok(None) };
    let mut out = load_products_batch(db, vec![row], ch_id, &currency).await?;
    Ok(out.pop())
}

/// Batch price+name resolution for checkout lines, mirroring
/// `saleor/checkout/fetch.py::fetch_checkout_lines`: one round-trip for all
/// variants on the channel. Returns map variant_id -> (price, product name).
pub async fn checkout_pricing(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
    variant_ids: &[i32],
) -> Result<std::collections::HashMap<i32, (Money, String)>> {
    use std::collections::HashMap;
    let (ch_id, currency) = channel_id(db, channel_slug).await?;
    if variant_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let variants = product_productvariant::Entity::find()
        .filter(product_productvariant::Column::Id.is_in(variant_ids.to_vec()))
        .all(db)
        .await?;
    let listings = product_productvariantchannellisting::Entity::find()
        .filter(product_productvariantchannellisting::Column::VariantId.is_in(variant_ids.to_vec()))
        .filter(product_productvariantchannellisting::Column::ChannelId.eq(ch_id))
        .all(db)
        .await?;
    let prices: HashMap<i32, Decimal> = listings
        .into_iter()
        .map(|l| (l.variant_id, l.price_amount.unwrap_or(Decimal::ZERO)))
        .collect();

    // Product names for order lines (mirrors order line `product_name`).
    let product_ids: Vec<i32> = variants.iter().map(|v| v.product_id).collect();
    let products = product_select()
        .filter(product_product::Column::Id.is_in(product_ids))
        .into_tuple::<ProductRow>()
        .all(db)
        .await?;
    let names: HashMap<i32, String> =
        products.into_iter().map(|r| (r.0, r.1)).collect();

    let mut out = HashMap::new();
    for v in variants {
        if let Some(amount) = prices.get(&v.id) {
            let name = names
                .get(&v.product_id)
                .cloned()
                .unwrap_or_else(|| format!("variant {}", v.id));
            out.insert(v.id, (Money::new(*amount, &currency), name));
        }
    }
    Ok(out)
}

pub async fn list_categories(db: &DatabaseConnection) -> Result<Vec<Category>> {
    Ok(product_category::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|c| Category {
            id: c.id.to_string(),
            name: c.name,
            slug: c.slug,
            parent_id: None,
        })
        .collect())
}

/// Stock mirrors `saleor/warehouse`: sum of `stock.quantity - allocated`
/// across all warehouses. Negative clamp matches Django behavior of
/// `available_quantity` never going below zero at the API edge.
///
/// Currently no read path uses it (availability comes from the materialized
/// search view); kept as the canonical helper for future stock reads.
#[allow(dead_code)]
async fn stock_for(db: &impl sea_orm::ConnectionTrait, variant_id: i32) -> Result<i32> {
    use crate::entities::warehouse_stock;
    let rows = warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::ProductVariantId.eq(variant_id))
        .all(db)
        .await?;
    let qty: i32 = rows
        .iter()
        .map(|s| s.quantity - s.quantity_allocated)
        .sum();
    Ok(qty.max(0))
}


// ---------------------------------------------------------------------------
// Server-side filtering (Saleor filter/where semantics, dashboard-driven).
// ---------------------------------------------------------------------------

/// Parse a dashboard ID: plain int PK (what our API mints) or Saleor global
/// ID (`UHJvZHVjdDoyOA==` → `Product:28`).
pub fn parse_gid(s: &str) -> Option<i32> {
    let s = s.trim();
    if let Ok(n) = s.parse::<i32>() {
        return Some(n);
    }
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s)
        .ok()?;
    let t = String::from_utf8(bytes).ok()?;
    t.split_once(':')?.1.parse::<i32>().ok()
}

/// Flat product constraints mirroring the fields of Saleor's
/// `ProductFilterInput` / `ProductWhereInput` that the Dashboard sends.
/// NOT covered (accepted-ignored upstream): price/minimalPrice ranges,
/// attributes, stocks, dates, metadata, giftCard.
#[derive(Default, Clone)]
pub struct ProductListFilter {
    pub search: Option<String>,
    pub ids: Vec<i32>,
    pub slugs: Vec<String>,
    pub name_eq: Option<String>,
    pub name_one_of: Vec<String>,
    pub slug_eq: Option<String>,
    pub slug_one_of: Vec<String>,
    pub product_type_ids: Vec<i32>,
    /// Raw category ids; MPTT subtree (self + descendants, like Saleor's
    /// `where_filter_by_categories`) is resolved inside.
    pub category_ids: Vec<i32>,
    pub collection_ids: Vec<i32>,
    /// Saleor `isPublished` = channel-listing flag for the queried channel.
    pub is_published: Option<bool>,
    pub has_category: Option<bool>,
    /// `Some(true)` = ORDER BY name ASC, `Some(false)` = DESC.
    pub order_name_asc: Option<bool>,
}

/// MPTT subtree (self + descendants) for the given category ids.
async fn category_subtree_ids(
    db: &impl sea_orm::ConnectionTrait,
    ids: &[i32],
) -> Result<Vec<i32>> {
    use crate::entities::product_category::{Column as CatCol, Entity as Cat};
    use sea_orm::Condition;
    let bounds: Vec<(i32, i32, i32)> = Cat::find()
        .select_only()
        .column(CatCol::TreeId)
        .column(CatCol::Lft)
        .column(CatCol::Rght)
        .filter(CatCol::Id.is_in(ids.to_vec()))
        .into_tuple()
        .all(db)
        .await?;
    if bounds.is_empty() {
        return Ok(vec![]);
    }
    let mut cond = Condition::any();
    for (tree, lft, rght) in bounds {
        cond = cond.add(
            Condition::all()
                .add(CatCol::TreeId.eq(tree))
                .add(CatCol::Lft.gte(lft))
                .add(CatCol::Rght.lte(rght)),
        );
    }
    Ok(Cat::find()
        .select_only()
        .column(CatCol::Id)
        .filter(cond)
        .into_tuple()
        .all(db)
        .await?)
}

async fn filtered_select(
    db: &impl sea_orm::ConnectionTrait,
    ch_id: i32,
    f: &ProductListFilter,
) -> Result<sea_orm::Select<product_product::Entity>> {
    use product_product::Column as P;
    use sea_orm::{Condition, QueryOrder};
    // Publication: default = listed in channel (matches list_products).
    let mut q = match f.is_published {
        Some(false) => product_select().filter(
            product_product::Column::Id.in_subquery(
                sea_orm::sea_query::Query::select()
                    .column(product_productchannellisting::Column::ProductId)
                    .from(product_productchannellisting::Entity)
                    .and_where(product_productchannellisting::Column::ChannelId.eq(ch_id))
                    .and_where(product_productchannellisting::Column::IsPublished.eq(false))
                    .to_owned(),
            ),
        ),
        _ => published_filter(product_select(), ch_id),
    };
    if !f.ids.is_empty() {
        q = q.filter(P::Id.is_in(f.ids.clone()));
    }
    if !f.slugs.is_empty() {
        q = q.filter(P::Slug.is_in(f.slugs.clone()));
    }
    if let Some(n) = &f.name_eq {
        q = q.filter(P::Name.eq(n.clone()));
    }
    if !f.name_one_of.is_empty() {
        q = q.filter(P::Name.is_in(f.name_one_of.clone()));
    }
    if let Some(s) = &f.slug_eq {
        q = q.filter(P::Slug.eq(s.clone()));
    }
    if !f.slug_one_of.is_empty() {
        q = q.filter(P::Slug.is_in(f.slug_one_of.clone()));
    }
    if !f.product_type_ids.is_empty() {
        q = q.filter(P::ProductTypeId.is_in(f.product_type_ids.clone()));
    }
    if !f.category_ids.is_empty() {
        let sub = category_subtree_ids(db, &f.category_ids).await?;
        q = q.filter(P::CategoryId.is_in(sub));
    }
    if !f.collection_ids.is_empty() {
        q = q.filter(P::Id.in_subquery(
            sea_orm::sea_query::Query::select()
                .column(crate::entities::product_collectionproduct::Column::ProductId)
                .from(crate::entities::product_collectionproduct::Entity)
                .and_where(
                    crate::entities::product_collectionproduct::Column::CollectionId
                        .is_in(f.collection_ids.clone()),
                )
                .to_owned(),
        ));
    }
    if let Some(hc) = f.has_category {
        q = if hc {
            q.filter(P::CategoryId.is_not_null())
        } else {
            q.filter(P::CategoryId.is_null())
        };
    }
    if let Some(s) = f.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        // Admin search: trigram-style ILIKE on name/slug plus a tsvector
        // match (WHERE-only, so no decode hazard). Superset of Saleor's
        // rank-based search — better recall for an admin box.
        let like = format!("%{s}%");
        q = q.filter(
            Condition::any()
                .add(P::Name.like(like.clone()))
                .add(P::Slug.like(like))
                .add(sea_orm::sea_query::Expr::cust_with_values(
                    "product_product.search_vector @@ plainto_tsquery('english', $1)",
                    [s.to_string()],
                )),
        );
    }
    if let Some(asc) = f.order_name_asc {
        q = if asc {
            q.order_by_asc(P::Name)
        } else {
            q.order_by_desc(P::Name)
        };
    }
    Ok(q)
}

/// Product ids matching the filter (fast path for AND/OR branch resolution).
pub async fn product_ids_filtered(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
    f: &ProductListFilter,
) -> Result<Vec<i32>> {
    let (ch_id, _) = channel_id(db, channel_slug).await?;
    Ok(filtered_select(db, ch_id, f)
        .await?
        .select_only()
        .column(product_product::Column::Id)
        .into_tuple()
        .all(db)
        .await?)
}

/// Filtered product list: same batched assembly as `list_products`.
pub async fn list_products_filtered(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
    f: &ProductListFilter,
    limit: u64,
) -> Result<Vec<Product>> {
    let (ch_id, currency) = channel_id(db, channel_slug).await?;
    let rows: Vec<ProductRow> = filtered_select(db, ch_id, f)
        .await?
        .limit(limit)
        .into_tuple()
        .all(db)
        .await?;
    load_products_batch(db, rows, ch_id, &currency).await
}
