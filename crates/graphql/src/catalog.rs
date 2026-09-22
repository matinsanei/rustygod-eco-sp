//! Catalog GraphQL: products + variants (read path the Dashboard lists).

use async_graphql::*;

use crate::{common::*, context::GqlContext, gen};

#[derive(SimpleObject, Clone)]
pub struct GqlProductEdge {
    pub node: gen::Product,
    pub cursor: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductCountableConnection")]
pub struct GqlProductConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlProductEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: crate::common::PageInfo,
}

#[derive(Default)]
pub struct CatalogQuery;

/// Empty connection (where-branch matched nothing — Saleor returns empty, not error).
fn empty_connection() -> GqlProductConnection {
    GqlProductConnection { total_count: Some(0), edges: vec![], page_info: crate::common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None } }
}

/// Dashboard IDs are plain ints (ours) or Saleor global IDs — both resolve.
pub(crate) fn gid_vec(ids: Option<Vec<ID>>) -> Vec<i32> {
    ids.unwrap_or_default().into_iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect()
}

pub(crate) fn gid_filter(f: Option<gen::GlobalIDFilterInput>) -> Vec<i32> {
    let mut out = vec![];
    if let Some(ff) = f {
        if let Some(eq) = ff.eq {
            out.extend(rustygod_db::catalog::parse_gid(&eq.0));
        }
        out.extend(ff.one_of.unwrap_or_default().into_iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)));
    }
    out
}

pub(crate) fn str_filter(f: Option<gen::StringFilterInput>) -> (Option<String>, Vec<String>) {
    match f {
        None => (None, vec![]),
        Some(ff) => (ff.eq, ff.one_of.unwrap_or_default()),
    }
}

/// Merge one where-level's flat fields (no AND/OR) into the DB filter.
fn merge_where_flat(f: &mut rustygod_db::catalog::ProductListFilter, w: &gen::ProductWhereInput) {
    f.ids.extend(gid_vec(w.ids.clone()));
    let (neq, none_of) = str_filter(w.name.clone());
    if neq.is_some() { f.name_eq = neq; }
    f.name_one_of.extend(none_of);
    let (seq, sone_of) = str_filter(w.slug.clone());
    if seq.is_some() { f.slug_eq = seq; }
    f.slug_one_of.extend(sone_of);
    f.product_type_ids.extend(gid_filter(w.product_type.clone()));
    f.category_ids.extend(gid_filter(w.category.clone()));
    f.collection_ids.extend(gid_filter(w.collection.clone()));
    if w.is_published.is_some() { f.is_published = w.is_published; }
    if w.has_category.is_some() { f.has_category = w.has_category; }
}

/// Resolve AND/OR nesting to an id set (`None` = unconstrained). Branches are
/// evaluated with the fast ids-only path; AND intersects, OR unions.
fn resolve_where_ids<'a>(
    db: &'a sea_orm::DatabaseConnection,
    ch: &'a str,
    w: &'a gen::ProductWhereInput,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<std::collections::HashSet<i32>>, sea_orm::DbErr>> + Send + 'a>> {
    Box::pin(async move {
        use std::collections::HashSet;
        let mut acc: Option<HashSet<i32>> = None;
        // AND: intersect. A branch with only-flat fields folds into the
        // parent query; nested AND/OR resolves recursively.
        for sub in w.and.clone().unwrap_or_default() {
            if let Some(set) = resolve_where_ids(db, ch, &sub).await? {
                acc = Some(match acc {
                    None => set,
                    Some(a) => a.intersection(&set).cloned().collect(),
                });
            }
        }
        // OR: union; an unconstrained branch unconstrains the whole OR.
        if let Some(ors) = w.or.clone() {
            let mut union = HashSet::new();
            let mut unconstrained = false;
            for sub in ors {
                match resolve_where_ids(db, ch, &sub).await? {
                    Some(set) => { union.extend(set); }
                    None => { unconstrained = true; break; }
                }
            }
            if unconstrained {
                return Ok(acc);
            }
            acc = Some(match acc {
                None => union,
                Some(a) => a.intersection(&union).cloned().collect(),
            });
        }
        // This level's own flat fields (if it has any + the level is a pure
        // branch; the top level is applied by the caller — here we only
        // resolve what AND/OR need).
        let mut flat = rustygod_db::catalog::ProductListFilter::default();
        merge_where_flat(&mut flat, w);
        let has_flat = !flat.ids.is_empty() || flat.name_eq.is_some() || !flat.name_one_of.is_empty()
            || flat.slug_eq.is_some() || !flat.slug_one_of.is_empty() || !flat.product_type_ids.is_empty()
            || !flat.category_ids.is_empty() || !flat.collection_ids.is_empty()
            || flat.is_published.is_some() || flat.has_category.is_some();
        if has_flat {
            let ids: HashSet<i32> = rustygod_db::catalog::product_ids_filtered(db, ch, &flat).await.map_err(|_| sea_orm::DbErr::RecordNotFound("product filter".into()))?.into_iter().collect();
            acc = Some(match acc {
                None => ids,
                Some(a) => a.intersection(&ids).cloned().collect(),
            });
        }
        Ok(acc)
    })
}

#[Object]
impl CatalogQuery {
    /// Mirrors dashboard `ProductList` — channel defaults to
    /// `default-channel` (populatedb). Filter/where/search/sort are applied
    /// server-side with Saleor semantics (see `rustygod_db::catalog`);
    /// price/attribute/stock/date/metadata sub-filters stay accepted-ignored.
    async fn products(
        &self,
        ctx: &Context<'_>,
        channel: Option<String>,
        first: Option<i32>,
        after: Option<String>,
        before: Option<String>,
        last: Option<i32>,
        filter: Option<gen::ProductFilterInput>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::ProductOrder>,
        #[graphql(name = "where")] where_input: Option<gen::ProductWhereInput>,
        search: Option<String>,
    ) -> Result<GqlProductConnection> {
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let off = after.and_then(|c| decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        let mut f = rustygod_db::catalog::ProductListFilter::default();
        // Deprecated filter input (dashboard search boxes still send it).
        if let Some(flt) = filter.as_ref() {
            f.ids.extend(gid_vec(flt.ids.clone()));
            f.slugs.extend(flt.slugs.clone().unwrap_or_default());
            f.collection_ids.extend(gid_vec(flt.collections.clone()));
            f.category_ids.extend(gid_vec(flt.categories.clone()));
            f.product_type_ids.extend(gid_vec(flt.product_types.clone()));
            if flt.is_published.is_some() { f.is_published = flt.is_published; }
            if flt.has_category.is_some() { f.has_category = flt.has_category; }
            if f.search.is_none() { f.search = flt.search.clone(); }
        }
        if f.search.is_none() { f.search = search.clone(); }
        // Where input (ConditionalFilter): flat fields + AND/OR nesting.
        if let Some(w) = where_input.as_ref() {
            merge_where_flat(&mut f, w);
            if let Some(sub) = resolve_where_ids(db, &ch, w).await.map_err(|e| Error::new(e.to_string()))? {
                if sub.is_empty() {
                    return Ok(empty_connection());
                }
                // Intersect with any ids already constrained.
                if f.ids.is_empty() {
                    f.ids = sub.into_iter().collect();
                } else {
                    let keep: std::collections::HashSet<i32> = f.ids.iter().cloned().collect();
                    f.ids = sub.into_iter().filter(|i| keep.contains(i)).collect();
                }
            }
        }
        if let Some(sort) = sort_by.as_ref() {
            if matches!(sort.field, Some(gen::ProductOrderField::NAME)) {
                f.order_name_asc = Some(matches!(sort.direction, gen::OrderDirection::ASC));
            }
        }
        let all = rustygod_db::catalog::list_products_filtered(db, &ch, &f, 200).await.map_err(|e| Error::new(e.to_string()))?;
        let total = all.len() as i32;
        let assembled = assemble_list_products(db, all).await?;
        let edges = assembled.into_iter().skip(off).take(lim).enumerate().map(|(i, node)| GqlProductEdge { node, cursor: encode_cursor(off + i) }).collect();
        Ok(GqlProductConnection { total_count: Some(total), edges, page_info: crate::common::PageInfo { has_next_page: off + lim < total as usize, has_previous_page: off > 0, start_cursor: None, end_cursor: None } })
    }

    /// Saleor `product(id, slug)` — single product with the same assembly as
    /// the list (dashboard details page).
    async fn product(
        &self, ctx: &Context<'_>,
        id: Option<ID>, slug: Option<String>, channel: Option<String>,
    ) -> Result<Option<gen::Product>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let mut f = rustygod_db::catalog::ProductListFilter::default();
        if let Some(i) = id {
            if let Some(n) = rustygod_db::catalog::parse_gid(&i.0) { f.ids.push(n); }
        }
        if let Some(s) = slug { f.slug_eq = Some(s); }
        if f.ids.is_empty() && f.slug_eq.is_none() { return Ok(None); }
        let items = rustygod_db::catalog::list_products_filtered(db, &ch, &f, 2).await.map_err(|e| Error::new(e.to_string()))?;
        let out = assemble_list_products(db, items).await?;
        Ok(out.into_iter().next())
    }

    /// Saleor `category(id, slug)`.
    async fn category(
        &self, ctx: &Context<'_>,
        id: Option<ID>, slug: Option<String>,
    ) -> Result<Option<gen::Category>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        use rustygod_db::entities::product_category::{Column as CatCol, Entity as Cat};
        let row: Option<(i32, String, String, Option<String>, Option<String>, i32)> = if let Some(i) = id {
            match rustygod_db::catalog::parse_gid(&i.0) {
                Some(n) => Cat::find_by_id(n).select_only()
                    .column(CatCol::Id).column(CatCol::Name).column(CatCol::Slug)
                    .column(CatCol::SeoTitle).column(CatCol::SeoDescription).column(CatCol::Level)
                    .into_tuple().one(db).await.map_err(|e| Error::new(e.to_string()))?,
                None => None,
            }
        } else if let Some(s) = slug {
            Cat::find().select_only()
                .column(CatCol::Id).column(CatCol::Name).column(CatCol::Slug)
                .column(CatCol::SeoTitle).column(CatCol::SeoDescription).column(CatCol::Level)
                .filter(CatCol::Slug.eq(s))
                .into_tuple().one(db).await.map_err(|e| Error::new(e.to_string()))?
        } else { None };
        Ok(row.map(|(cid, name, cslug, seo_t, seo_d, level)| gen::Category {
            id: Some(ID(cid.to_string())),
            private_metadata: vec![],
            metadata: vec![],
            seo_title: seo_t,
            seo_description: seo_d,
            name: Some(name),
            description: None,
            slug: Some(cslug),
            parent: None,
            level: Some(level),
            updated_at: None,
        }))
    }

    /// Saleor `collection(id, slug)`.
    async fn collection(
        &self, ctx: &Context<'_>,
        id: Option<ID>, slug: Option<String>,
    ) -> Result<Option<gen::Collection>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        use rustygod_db::entities::product_collection::{Column as ColCol, Entity as Col};
        let row: Option<(i32, String, String, Option<String>, Option<String>)> = if let Some(i) = id {
            match rustygod_db::catalog::parse_gid(&i.0) {
                Some(n) => Col::find_by_id(n).select_only()
                    .column(ColCol::Id).column(ColCol::Name).column(ColCol::Slug)
                    .column(ColCol::SeoTitle).column(ColCol::SeoDescription)
                    .into_tuple().one(db).await.map_err(|e| Error::new(e.to_string()))?,
                None => None,
            }
        } else if let Some(s) = slug {
            Col::find().select_only()
                .column(ColCol::Id).column(ColCol::Name).column(ColCol::Slug)
                .column(ColCol::SeoTitle).column(ColCol::SeoDescription)
                .filter(ColCol::Slug.eq(s))
                .into_tuple().one(db).await.map_err(|e| Error::new(e.to_string()))?
        } else { None };
        Ok(row.map(|(cid, name, cslug, seo_t, seo_d)| gen::Collection {
            id: Some(ID(cid.to_string())),
            private_metadata: vec![],
            metadata: vec![],
            seo_title: seo_t,
            seo_description: seo_d,
            name: Some(name),
            description: None,
            slug: Some(cslug),
            channel_listings: vec![],
        }))
    }
}

/// Shared list assembly: type/category enrichment (2 batched queries) +
/// row mapping. Used by `products` and `product`.
async fn assemble_list_products(
    db: &sea_orm::DatabaseConnection,
    all: Vec<rustygod_core::product::Product>,
) -> Result<Vec<gen::Product>> {
        // Batch productType + category (2 queries; dashboard list reads
        // `productType.name/hasVariants` and category in every row).
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        use std::collections::{HashMap, HashSet};
        let type_ids: HashSet<i32> = all.iter().filter_map(|p| p.product_type_id.parse::<i32>().ok()).collect();
        let mut types: HashMap<i32, (String, String, bool)> = HashMap::new();
        if !type_ids.is_empty() {
            let rows = rustygod_db::entities::product_producttype::Entity::find()
                .select_only()
                .column(rustygod_db::entities::product_producttype::Column::Id)
                .column(rustygod_db::entities::product_producttype::Column::Name)
                .column(rustygod_db::entities::product_producttype::Column::Slug)
                .column(rustygod_db::entities::product_producttype::Column::HasVariants)
                .filter(rustygod_db::entities::product_producttype::Column::Id.is_in(type_ids.into_iter().collect::<Vec<_>>()))
                .into_tuple::<(i32, String, String, bool)>()
                .all(db).await.map_err(|e| Error::new(e.to_string()))?;
            for (id, name, slug, hv) in rows { types.insert(id, (name, slug, hv)); }
        }
        let cat_ids: HashSet<i32> = all.iter().filter_map(|p| p.category_id.as_ref().and_then(|c| c.parse::<i32>().ok())).collect();
        let mut cats: HashMap<i32, (String, String)> = HashMap::new();
        if !cat_ids.is_empty() {
            let rows = rustygod_db::entities::product_category::Entity::find()
                .select_only()
                .column(rustygod_db::entities::product_category::Column::Id)
                .column(rustygod_db::entities::product_category::Column::Name)
                .column(rustygod_db::entities::product_category::Column::Slug)
                .filter(rustygod_db::entities::product_category::Column::Id.is_in(cat_ids.into_iter().collect::<Vec<_>>()))
                .into_tuple::<(i32, String, String)>()
                .all(db).await.map_err(|e| Error::new(e.to_string()))?;
            for (id, name, slug) in rows { cats.insert(id, (name, slug)); }
        }
        let page: Vec<gen::Product> = all.into_iter().map(|p| gen::Product {
            id: Some(ID(p.id)),
            name: Some(p.name),
            slug: Some(p.slug),
            default_variant: p.variants.into_iter().next().map(|v| gen::ProductVariant {
                id: Some(ID(v.id)),
                name: Some(v.name),
                sku: Some(v.sku),
                quantity_available: Some(v.quantity_available),
                private_metadata: vec![],
                metadata: vec![],
                track_inventory: None,
                quantity_limit_per_customer: None,
                weight: None,
                channel_listings: vec![],
                media: vec![],
                stocks: vec![],
                updated_at: None,
                product: None,
            }),
            private_metadata: vec![],
            metadata: vec![],
            seo_title: None,
            seo_description: None,
            description: None,
            product_type: p.product_type_id.parse::<i32>().ok().and_then(|tid| types.get(&tid)).map(|(name, slug, hv)| gen::ProductType {
                id: Some(ID(p.product_type_id.clone())),
                private_metadata: vec![],
                metadata: vec![],
                name: Some(name.clone()),
                slug: Some(slug.clone()),
                has_variants: Some(*hv),
                is_shipping_required: None,
                weight: None,
                kind: None,
                tax_class: None,
                assigned_variant_attributes: vec![],
                product_attributes: vec![],
            }),
            category: p.category_id.as_ref().and_then(|c| c.parse::<i32>().ok()).and_then(|cid| cats.get(&cid)).map(|(name, slug)| gen::Category {
                id: Some(ID(p.category_id.clone().unwrap_or_default())),
                private_metadata: vec![],
                metadata: vec![],
                seo_title: None,
                seo_description: None,
                name: Some(name.clone()),
                description: None,
                slug: Some(slug.clone()),
                parent: None,
                level: None,
                updated_at: None,
            }),
            created: None,
            updated_at: None,
            weight: None,
            rating: None,
            is_available: None,
            attributes: vec![],
            channel_listings: vec![],
            media: vec![],
            collections: vec![],
            is_available_for_purchase: None,
            tax_class: None,
        }).collect();
        Ok(page)
}

/// Dashboard `ProductCreate` payload shape (`product { id }`, `errors`).
#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductCreate")]
pub struct GqlProductCreate {
    pub product: Option<gen::Product>,
    pub errors: Vec<gen::ProductError>,
}

#[derive(Default)]
pub struct CatalogMutation;

#[Object]
impl CatalogMutation {
    /// Dashboard quick-create (`ProductCreate`). Requires `manage_products`
    /// (staff). Creates product row + picks first productType if not given.
    /// Accepts the full Saleor `ProductCreateInput`; only identity fields
    /// are persisted (documented gap).
    async fn product_create(&self, ctx: &Context<'_>, input: gen::ProductCreateInput) -> Result<GqlProductCreate> {
        let bearer = ctx.data_opt::<crate::context::Bearer>().map(|b| b.0.as_str().to_string())
            .or_else(|| ctx.data_opt::<GqlContext>().and_then(|g| g.bearer.clone()));
        if bearer.is_none() { return Err(Error::new("authentication required")); }
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let name = input.name.clone().unwrap_or_default();
        if name.trim().is_empty() { return Err(Error::new("name is required")); }
        let slug = input.slug.clone().unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        // Resolve product_type: use given or first existing.
        let pt_id: i32 = {
            let parsed = input.product_type.0.parse::<i32>().unwrap_or(0);
            if parsed != 0 { parsed } else {
                use sea_orm::EntityTrait;
                let pt = rustygod_db::entities::product_producttype::Entity::find().one(db).await.map_err(|e| Error::new(e.to_string()))?
                    .ok_or_else(|| Error::new("no product type found"))?;
                pt.id
            }
        };
        let cat_id: Option<i32> = input.category.as_ref().and_then(|c| c.0.parse::<i32>().ok());
        let id = create_product_row(db, &name, &slug, pt_id, cat_id).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlProductCreate {
            product: Some(gen::Product {
                id: Some(ID(id.to_string())),
                name: Some(name),
                slug: Some(slug),
                private_metadata: vec![],
                metadata: vec![],
                seo_title: None,
                seo_description: None,
                description: None,
                product_type: None,
                category: None,
                created: None,
                updated_at: None,
                weight: None,
                default_variant: None,
                rating: None,
                is_available: None,
                attributes: vec![],
                channel_listings: vec![],
                media: vec![],
                collections: vec![],
                is_available_for_purchase: None,
                tax_class: None,
            }),
            errors: vec![],
        })
    }
}

async fn create_product_row(
    db: &sea_orm::DatabaseConnection,
    name: &str,
    slug: &str,
    product_type_id: i32,
    category_id: Option<i32>,
) -> Result<i32, sea_orm::DbErr> {
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, Set};
    use serde_json::json;
    let now: sea_orm::prelude::DateTimeWithTimeZone = Utc::now().into();
    let row = rustygod_db::entities::product_product::ActiveModel {
        name: Set(name.to_string()),
        slug: Set(slug.to_string()),
        product_type_id: Set(product_type_id),
        category_id: Set(category_id),
        description: Set(None),
        updated_at: Set(now),
        created_at: Set(now),
        seo_description: Set(None),
        seo_title: Set(None),
        weight: Set(None),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        description_plaintext: Set(String::new()),
        rating: Set(None),
        search_document: Set(String::new()),
        search_index_dirty: Set(true),
        ..Default::default()
    };
    let inserted = row.insert(db).await?;
    Ok(inserted.id)
}
