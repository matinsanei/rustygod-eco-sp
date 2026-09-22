//! Catalog GraphQL: products + variants (read path the Dashboard lists).

use async_graphql::*;

use crate::{common::*, context::GqlContext, gen, metadata};

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
    /// Dashboard category list (roots only, `level: 0`): name/sort/search
    /// server-side; children/products counts per row (subtree, Saleor parity).
    async fn categories(
        &self,
        ctx: &Context<'_>,
        filter: Option<gen::CategoryFilterInput>,
        #[graphql(name = "where")] where_input: Option<gen::CategoryWhereInput>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::CategorySortingInput>,
        level: Option<i32>,
        before: Option<String>,
        after: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<Option<gen::CategoryCountableConnection>> {
        let _ = (where_input, before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{ConnectionTrait, Statement};
        let mut conds: Vec<String> = vec![];
        let mut params: Vec<sea_orm::Value> = vec![];
        if let Some(lv) = level {
            // Roots (dashboard passes level: 0): no parent. Deeper levels
            // match MPTT level exactly (same column Django filters).
            if lv == 0 {
                conds.push("parent_id IS NULL".into());
            } else {
                params.push(lv.into());
                conds.push(format!("level = ${}", params.len()));
            }
        }
        if let Some(f) = filter.as_ref() {
            if let Some(s) = f.search.as_ref().filter(|s| !s.trim().is_empty()) {
                params.push(format!("%{s}%").into());
                let p = params.len();
                conds.push(format!("(name ILIKE ${p} OR slug ILIKE ${p})"));
            }
            if let Some(ids) = f.ids.as_ref() {
                let list: Vec<i32> = ids.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect();
                if !list.is_empty() {
                    let base = params.len();
                    let ph = (1..=list.len()).map(|i| format!("${}", base + i)).collect::<Vec<_>>().join(", ");
                    conds.push(format!("id IN ({ph})"));
                    params.extend(list.into_iter().map(|i| i.into()));
                }
            }
            if let Some(slugs) = f.slugs.as_ref().filter(|s| !s.is_empty()) {
                let base = params.len();
                let ph = (1..=slugs.len()).map(|i| format!("${}", base + i)).collect::<Vec<_>>().join(", ");
                conds.push(format!("slug IN ({ph})"));
                params.extend(slugs.iter().map(|s| s.clone().into()));
            }
        }
        let order = match sort_by.as_ref().map(|s| (&s.field, &s.direction)) {
            Some((gen::CategorySortField::NAME, gen::OrderDirection::DESC)) => "name DESC, id",
            _ => "name ASC, id",
        };
        let where_sql = if conds.is_empty() { String::new() } else { format!("WHERE {}", conds.join(" AND ")) };
        let rows = db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT id, name, slug FROM product_category {where_sql} ORDER BY {order}"),
            params,
        )).await.map_err(|e| Error::new(e.to_string()))?;
        let all: Vec<(i32, String, String)> = rows.into_iter().filter_map(|r| {
            Some((r.try_get::<i32>("", "id").ok()?,
                  r.try_get::<String>("", "name").ok()?,
                  r.try_get::<String>("", "slug").ok()?))
        }).collect();
        let total = all.len() as i32;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        let edges = all.into_iter().skip(off).take(lim).map(|(id, name, slug)| {
            let mut c = metadata::lit_category(crate::common::gid("Category", id), vec![], vec![]);
            c.name = Some(name);
            c.slug = Some(slug);
            gen::CategoryCountableEdge { node: Some(Box::new(c)) }
        }).collect();
        Ok(Some(gen::CategoryCountableConnection {
            page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
            edges,
            total_count: Some(total),
        }))
    }

    /// Dashboard collection list: search/sort/channel server-side;
    /// channelListings + product counts per row.
    async fn collections(
        &self,
        ctx: &Context<'_>,
        filter: Option<gen::CollectionFilterInput>,
        #[graphql(name = "where")] where_input: Option<gen::CollectionWhereInput>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::CollectionSortingInput>,
        channel: Option<String>,
        before: Option<String>,
        after: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<Option<gen::CollectionCountableConnection>> {
        let _ = (where_input, before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{ConnectionTrait, Statement};
        use std::collections::HashMap;
        let mut conds: Vec<String> = vec![];
        let mut params: Vec<sea_orm::Value> = vec![];
        if let Some(f) = filter.as_ref() {
            if let Some(s) = f.search.as_ref().filter(|s| !s.trim().is_empty()) {
                params.push(format!("%{s}%").into());
                let p = params.len();
                conds.push(format!("(c.name ILIKE ${p} OR c.slug ILIKE ${p})"));
            }
            if let Some(ids) = f.ids.as_ref() {
                let list: Vec<i32> = ids.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect();
                if !list.is_empty() {
                    let base = params.len();
                    let ph = (1..=list.len()).map(|i| format!("${}", base + i)).collect::<Vec<_>>().join(", ");
                    conds.push(format!("c.id IN ({ph})"));
                    params.extend(list.into_iter().map(|i| i.into()));
                }
            }
            if let Some(slugs) = f.slugs.as_ref().filter(|s| !s.is_empty()) {
                let base = params.len();
                let ph = (1..=slugs.len()).map(|i| format!("${}", base + i)).collect::<Vec<_>>().join(", ");
                conds.push(format!("c.slug IN ({ph})"));
                params.extend(slugs.iter().map(|s| s.clone().into()));
            }
            if let Some(pub_) = f.published.as_ref() {
                match pub_ {
                    gen::CollectionPublished::PUBLISHED => conds.push("EXISTS(SELECT 1 FROM product_collectionchannellisting l WHERE l.collection_id = c.id AND l.is_published)".into()),
                    gen::CollectionPublished::HIDDEN => conds.push("NOT EXISTS(SELECT 1 FROM product_collectionchannellisting l WHERE l.collection_id = c.id AND l.is_published)".into()),
                }
            }
        }
        if let Some(ch) = channel.as_ref() {
            params.push(ch.clone().into());
            let p = params.len();
            conds.push(format!("EXISTS(SELECT 1 FROM product_collectionchannellisting l JOIN channel_channel c2 ON c2.id = l.channel_id WHERE l.collection_id = c.id AND c2.slug = ${p})"));
        }
        let desc = matches!(sort_by.as_ref().map(|s| &s.direction), Some(gen::OrderDirection::DESC));
        let order = if desc { "c.name DESC, c.id" } else { "c.name ASC, c.id" };
        let where_sql = if conds.is_empty() { String::new() } else { format!("WHERE {}", conds.join(" AND ")) };
        let rows = db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT c.id, c.name, c.slug FROM product_collection c {where_sql} ORDER BY {order}"),
            params,
        )).await.map_err(|e| Error::new(e.to_string()))?;
        let all: Vec<(i32, String, String)> = rows.into_iter().filter_map(|r| {
            Some((r.try_get::<i32>("", "id").ok()?,
                  r.try_get::<String>("", "name").ok()?,
                  r.try_get::<String>("", "slug").ok()?))
        }).collect();
        let total = all.len() as i32;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        // channel listings per collection, batched
        let page_ids: Vec<i32> = all.iter().map(|t| t.0).collect();
        let mut listings: HashMap<i32, Vec<gen::CollectionChannelListing>> = HashMap::new();
        if !page_ids.is_empty() {
            let list = (1..=page_ids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
            let lrows = db.query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT l.id, l.collection_id, l.is_published, l.published_at, c.id AS chid, c.slug, c.name FROM product_collectionchannellisting l JOIN channel_channel c ON c.id = l.channel_id WHERE l.collection_id IN ({list})"),
                page_ids.iter().map(|i| (*i).into()).collect::<Vec<sea_orm::Value>>(),
            )).await.map_err(|e| Error::new(e.to_string()))?;
            for r in lrows {
                if let (Ok(_lid), Ok(cid), Ok(ch)) = (r.try_get::<i32>("", "id"), r.try_get::<i32>("", "collection_id"), r.try_get::<i32>("", "chid")) {
                    let mut c = metadata::lit_channel(crate::common::gid("Channel", ch), vec![], vec![]);
                    c.slug = r.try_get::<String>("", "slug").ok();
                    c.name = r.try_get::<String>("", "name").ok();
                    listings.entry(cid).or_default().push(gen::CollectionChannelListing {
                        is_published: r.try_get::<bool>("", "is_published").ok(),
                        published_at: r.try_get::<Option<chrono::DateTime<chrono::Utc>>>("", "published_at").ok().flatten(),
                        channel: Some(c),
                    });
                }
            }
        }
        let edges = all.into_iter().skip(off).take(lim).map(|(id, name, slug)| {
            let mut c = metadata::lit_collection(crate::common::gid("Collection", id), vec![], vec![]);
            c.name = Some(name);
            c.slug = Some(slug);
            c.channel_listings = listings.get(&id).cloned().unwrap_or_default();
            gen::CollectionCountableEdge { node: Some(c) }
        }).collect();
        Ok(Some(gen::CollectionCountableConnection {
            page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
            edges,
            total_count: Some(total),
        }))
    }

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
        // Unlisted fallback: Saleor returns the row even with no published
        // listing (post-create details page depends on it).
        let items = if items.is_empty() && f.slug_eq.is_none() && f.ids.len() == 1 {
            rustygod_db::catalog::get_product_unlisted(db, &ch, f.ids[0]).await.map_err(|e| Error::new(e.to_string()))?.into_iter().collect()
        } else { items };
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
            id: Some(ID(crate::common::gid("Category", cid))),
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
            id: Some(ID(crate::common::gid("Collection", cid))),
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
/// Full product-page assembly (dashboard details parity). Populatedb rows
/// carry media, channel listings, stocks, collections and attributes — all
/// of it is exposed here in batched queries, never N+1.
struct VariantBatches {
    vbase: std::collections::HashMap<i32, (Option<String>, String, bool, Option<i32>)>,
    vlist: std::collections::HashMap<i32, Vec<(i32, i32, Option<rust_decimal::Decimal>, Option<rust_decimal::Decimal>, String)>>,
    channels: std::collections::HashMap<i32, gen::Channel>,
    stocks: std::collections::HashMap<i32, Vec<gen::Stock>>,
    variant_media: std::collections::HashMap<i32, Vec<gen::ProductMedia>>,
    vattrs: std::collections::HashMap<i32, Vec<gen::SelectedAttribute>>,
}

fn money(amount: rust_decimal::Decimal, currency: String) -> crate::common::Money {
    crate::common::Money { amount: amount.to_string(), currency, fraction_digits: None }
}

async fn load_variant_batches(
    db: &sea_orm::DatabaseConnection,
    vids: &[i32],
) -> Result<VariantBatches> {
    use sea_orm::{ConnectionTrait, Statement};
    use std::collections::{HashMap, HashSet};
    let mut b = VariantBatches {
        vbase: HashMap::new(), vlist: HashMap::new(), channels: HashMap::new(),
        stocks: HashMap::new(), variant_media: HashMap::new(), vattrs: HashMap::new(),
    };
    if vids.is_empty() {
        return Ok(b);
    }
    let in_list = (1..=vids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
    let params: Vec<sea_orm::Value> = vids.iter().map(|i| (*i).into()).collect();
    let q = |sql: String, params: Vec<sea_orm::Value>| async move {
        db.query_all(Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params)).await
    };
    for r in q(format!("SELECT id, sku, name, track_inventory, quantity_limit_per_customer FROM product_productvariant WHERE id IN ({in_list})"), params.clone())
        .await.map_err(|e| Error::new(e.to_string()))? {
        if let Ok(vid) = r.try_get::<i32>("", "id") {
            b.vbase.insert(vid, (
                r.try_get::<Option<String>>("", "sku").ok().flatten(),
                r.try_get::<String>("", "name").unwrap_or_default(),
                r.try_get::<bool>("", "track_inventory").unwrap_or(true),
                r.try_get::<Option<i32>>("", "quantity_limit_per_customer").ok().flatten()));
        }
    }
    let mut chan_ids: HashSet<i32> = HashSet::new();
    for r in q(format!("SELECT id, variant_id, channel_id, price_amount, cost_price_amount, currency FROM product_productvariantchannellisting WHERE variant_id IN ({in_list})"), params.clone())
        .await.map_err(|e| Error::new(e.to_string()))? {
        if let (Ok(lid), Ok(vid), Ok(ch)) = (r.try_get::<i32>("", "id"), r.try_get::<i32>("", "variant_id"), r.try_get::<i32>("", "channel_id")) {
            chan_ids.insert(ch);
            b.vlist.entry(vid).or_default().push((
                lid, ch,
                r.try_get::<Option<rust_decimal::Decimal>>("", "price_amount").ok().flatten(),
                r.try_get::<Option<rust_decimal::Decimal>>("", "cost_price_amount").ok().flatten(),
                r.try_get::<String>("", "currency").unwrap_or_else(|_| "USD".into())));
        }
    }
    if !chan_ids.is_empty() {
        let cl = (1..=chan_ids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
        for r in q(format!("SELECT id, slug, name, currency_code, is_active, default_country FROM channel_channel WHERE id IN ({cl})"),
            chan_ids.into_iter().map(|i| i.into()).collect()).await.map_err(|e| Error::new(e.to_string()))? {
            if let Ok(cid) = r.try_get::<i32>("", "id") {
                let dc = r.try_get::<String>("", "default_country").unwrap_or_else(|_| "US".into());
                let mut c = metadata::lit_channel(crate::common::gid("Channel", cid), vec![], vec![]);
                c.slug = r.try_get::<String>("", "slug").ok();
                c.name = r.try_get::<String>("", "name").ok();
                c.is_active = r.try_get::<bool>("", "is_active").ok();
                c.currency_code = r.try_get::<String>("", "currency_code").ok();
                c.default_country = Some(crate::common::GqlCountryDisplay { code: dc.clone(), country: dc });
                b.channels.insert(cid, c);
            }
        }
    }
    // stocks + warehouses
    let mut raw: Vec<(i32, i32, String, i32, i32)> = vec![];
    let mut wh_ids: HashSet<String> = HashSet::new();
    for r in q(format!("SELECT id, product_variant_id, warehouse_id::text AS warehouse_id, quantity, quantity_allocated FROM warehouse_stock WHERE product_variant_id IN ({in_list})"), params.clone())
        .await.map_err(|e| Error::new(e.to_string()))? {
        if let (Ok(sid), Ok(vid), Ok(wh), Ok(qty), Ok(alc)) = (
            r.try_get::<i32>("", "id"), r.try_get::<i32>("", "product_variant_id"),
            r.try_get::<String>("", "warehouse_id"), r.try_get::<i32>("", "quantity"),
            r.try_get::<i32>("", "quantity_allocated")) {
            wh_ids.insert(wh.clone());
            raw.push((sid, vid, wh, qty, alc));
        }
    }
    let mut wh_names: HashMap<String, (String, String)> = HashMap::new();
    if !wh_ids.is_empty() {
        let list = wh_ids.iter().enumerate().map(|(i, _)| format!("${}::uuid", i + 1)).collect::<Vec<_>>().join(", ");
        for w in q(format!("SELECT id::text AS id, name FROM warehouse_warehouse WHERE id IN ({list})"),
            wh_ids.into_iter().map(|s| s.into()).collect()).await.map_err(|e| Error::new(e.to_string()))? {
            if let (Ok(id), Ok(name)) = (w.try_get::<String>("", "id"), w.try_get::<String>("", "name")) {
                wh_names.insert(id.clone(), (id, name));
            }
        }
    }
    for (sid, vid, wh, qty, alc) in raw {
        let (wgid, wname) = wh_names.get(&wh).cloned().unwrap_or_else(|| (wh.clone(), wh.clone()));
        let mut w = metadata::lit_warehouse(crate::common::gid("Warehouse", &wgid), vec![], vec![]);
        w.name = Some(wname);
        b.stocks.entry(vid).or_default().push(gen::Stock {
            id: Some(ID(crate::common::gid("Stock", sid))),
            warehouse: Some(w),
            quantity: Some(qty),
            quantity_allocated: Some(alc),
        });
    }
    // variant media (through m2m, objects from product media rows)
    let mut media_ids: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut all_mids: HashSet<i32> = HashSet::new();
    for r in q(format!("SELECT variant_id, media_id FROM product_variantmedia WHERE variant_id IN ({in_list})"), params.clone())
        .await.map_err(|e| Error::new(e.to_string()))? {
        if let (Ok(v), Ok(m)) = (r.try_get::<i32>("", "variant_id"), r.try_get::<i32>("", "media_id")) {
            media_ids.entry(v).or_default().push(m);
            all_mids.insert(m);
        }
    }
    if !all_mids.is_empty() {
        let ml = (1..=all_mids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
        let mut by_id: HashMap<i32, gen::ProductMedia> = HashMap::new();
        for r in q(format!("SELECT id, alt, sort_order, \"type\" FROM product_productmedia WHERE id IN ({ml})"),
            all_mids.into_iter().map(|i| i.into()).collect()).await.map_err(|e| Error::new(e.to_string()))? {
            if let Ok(mid) = r.try_get::<i32>("", "id") {
                let mut m = metadata::lit_product_media(crate::common::gid("ProductMedia", mid), vec![], vec![]);
                m.alt = Some(r.try_get::<String>("", "alt").unwrap_or_default());
                m.sort_order = r.try_get::<Option<i32>>("", "sort_order").ok().flatten();
                m.r#type = r.try_get::<String>("", "type").ok();
                by_id.insert(mid, m);
            }
        }
        for (v, mids) in media_ids {
            b.variant_media.insert(v, mids.into_iter().filter_map(|m| by_id.get(&m).cloned()).collect());
        }
    }
    // variant attributes
    for r in q(format!("SELECT av.variant_id, v.id AS vid, v.name AS vname, v.slug AS vslug, a.id AS aid, a.name AS aname FROM attribute_assignedvariantattributevalue av JOIN attribute_attributevalue v ON v.id = av.value_id JOIN attribute_attribute a ON a.id = v.attribute_id WHERE av.variant_id IN ({in_list})"), params)
        .await.map_err(|e| Error::new(e.to_string()))? {
        if let (Ok(vid), Ok(vvid), Ok(aid)) = (r.try_get::<i32>("", "variant_id"), r.try_get::<i32>("", "vid"), r.try_get::<i32>("", "aid")) {
            let mut a = metadata::lit_attribute(crate::common::gid("Attribute", aid), vec![], vec![]);
            a.name = r.try_get::<String>("", "aname").ok();
            let mut aval = metadata::lit_attribute_value(crate::common::gid("AttributeValue", vvid), vec![], vec![]);
            aval.name = r.try_get::<String>("", "vname").ok();
            aval.slug = r.try_get::<String>("", "vslug").ok();
            b.vattrs.entry(vid).or_default().push(gen::SelectedAttribute { attribute: Some(a), values: vec![aval] });
        }
    }
    Ok(b)
}

fn build_variant(m: &VariantBatches, vid: i32, fb_name: String, fb_sku: String, fb_qty: i32) -> gen::ProductVariant {
    let (sku, name, track, limit) = m.vbase.get(&vid).cloned().unwrap_or((Some(fb_sku), fb_name, true, None));
    let listings: Vec<gen::ProductVariantChannelListing> = m.vlist.get(&vid).cloned().unwrap_or_default().into_iter().map(|(lid, ch, price, cost, cur)| {
        gen::ProductVariantChannelListing {
            id: Some(ID(crate::common::gid("ProductVariantChannelListing", lid))),
            channel: m.channels.get(&ch).cloned(),
            price: price.map(|p| money(p, cur.clone())),
            cost_price: cost.map(|c| money(c, cur.clone())),
        }
    }).collect();
    let stocks = m.stocks.get(&vid).cloned().unwrap_or_default();
    // Stock math is the truth for availability (core qty mirrors it on the
    // list path; the grid path synthesizes core rows with qty 0).
    let qty = stocks.iter().map(|s| s.quantity.unwrap_or(0) - s.quantity_allocated.unwrap_or(0)).sum::<i32>().max(0);
    let qty_avail = if stocks.is_empty() { fb_qty } else { qty };
    gen::ProductVariant {
        id: Some(ID(crate::common::gid("ProductVariant", vid))),
        name: Some(name),
        sku,
        quantity_available: Some(qty_avail),
        private_metadata: vec![],
        metadata: vec![],
        track_inventory: Some(track),
        quantity_limit_per_customer: limit,
        weight: None,
        channel_listings: listings,
        media: m.variant_media.get(&vid).cloned().unwrap_or_default(),
        stocks,
        updated_at: None,
        product: None,
    }
}

/// Variant attributes for `ProductVariant.attributes(variantSelection:)`
/// (selection scope accepted-ignored: assignments carry no scope column).
pub(crate) async fn variant_attributes(
    db: &sea_orm::DatabaseConnection,
    variant_gid: &str,
) -> Vec<gen::SelectedAttribute> {
    let Some(vid) = rustygod_db::catalog::parse_gid(variant_gid) else { return vec![] };
    load_variant_batches(db, &[vid]).await.map(|m| m.vattrs.get(&vid).cloned().unwrap_or_default()).unwrap_or_default()
}

async fn assemble_list_products(
    db: &sea_orm::DatabaseConnection,
    all: Vec<rustygod_core::product::Product>,
) -> Result<Vec<gen::Product>> {
        use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Statement};
        use std::collections::{HashMap, HashSet};
        let pids: Vec<i32> = all.iter().filter_map(|p| p.id.parse::<i32>().ok()).collect();
        let vids: Vec<i32> = all.iter().flat_map(|p| &p.variants)
            .filter_map(|v| v.id.parse::<i32>().ok()).collect();
        let in_list = |n: usize| (1..=n).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
        let qall = |sql: String, params: Vec<sea_orm::Value>| async move {
            db.query_all(Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params)).await
        };
        let vb = load_variant_batches(db, &vids).await?;
        // --- product extras (description/seo/rating/tax) ---
        let mut extras: HashMap<i32, (Option<String>, Option<String>, Option<String>, Option<f64>, Option<i32>)> = HashMap::new();
        if !pids.is_empty() {
            let rows = qall(format!("SELECT id, description::text, seo_title, seo_description, rating, tax_class_id FROM product_product WHERE id IN ({})", in_list(pids.len())),
                pids.iter().map(|i| (*i).into()).collect()).await.map_err(|e| Error::new(e.to_string()))?;
            for r in rows {
                extras.insert(
                    r.try_get::<i32>("", "id").map_err(|e| Error::new(e.to_string()))?,
                    (r.try_get::<Option<String>>("", "description").ok().flatten(),
                     r.try_get::<Option<String>>("", "seo_title").ok().flatten(),
                     r.try_get::<Option<String>>("", "seo_description").ok().flatten(),
                     r.try_get::<Option<f64>>("", "rating").ok().flatten(),
                     r.try_get::<Option<i32>>("", "tax_class_id").ok().flatten()));
            }
        }
        let tax_ids: HashSet<i32> = extras.values().filter_map(|e| e.4).collect();
        let mut tax_names: HashMap<i32, String> = HashMap::new();
        if !tax_ids.is_empty() {
            let rows = qall(format!("SELECT id, name FROM tax_taxclass WHERE id IN ({})", in_list(tax_ids.len())),
                tax_ids.into_iter().map(|i| i.into()).collect()).await.map_err(|e| Error::new(e.to_string()))?;
            for r in rows {
                if let (Ok(id), Ok(name)) = (r.try_get::<i32>("", "id"), r.try_get::<String>("", "name")) {
                    tax_names.insert(id, name);
                }
            }
        }
        // --- product media ---
        let mut media_by_product: HashMap<i32, Vec<gen::ProductMedia>> = HashMap::new();
        if !pids.is_empty() {
            let rows = qall(format!("SELECT id, product_id, alt, sort_order, \"type\" FROM product_productmedia WHERE product_id IN ({}) ORDER BY sort_order NULLS LAST, id", in_list(pids.len())),
                pids.iter().map(|i| (*i).into()).collect()).await.map_err(|e| Error::new(e.to_string()))?;
            for r in rows {
                let (Ok(mid), pid) = (r.try_get::<i32>("", "id"), r.try_get::<Option<i32>>("", "product_id").ok().flatten()) else { continue };
                let mut m = metadata::lit_product_media(crate::common::gid("ProductMedia", mid), vec![], vec![]);
                m.alt = Some(r.try_get::<String>("", "alt").unwrap_or_default());
                m.sort_order = r.try_get::<Option<i32>>("", "sort_order").ok().flatten();
                m.r#type = r.try_get::<String>("", "type").ok();
                if let Some(pid) = pid {
                    media_by_product.entry(pid).or_default().push(m);
                }
            }
        }
        // --- product listings ---
        let mut plist: HashMap<i32, Vec<(i32, i32, bool, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>, bool)>> = HashMap::new();
        let mut pchan: HashMap<i32, gen::Channel> = HashMap::new();
        if !pids.is_empty() {
            let rows = qall(format!("SELECT l.id, l.product_id, l.channel_id, l.is_published, l.published_at, l.available_for_purchase_at, l.visible_in_listings, c.slug, c.name, c.currency_code, c.is_active, c.default_country FROM product_productchannellisting l JOIN channel_channel c ON c.id = l.channel_id WHERE l.product_id IN ({})", in_list(pids.len())),
                pids.iter().map(|i| (*i).into()).collect()).await.map_err(|e| Error::new(e.to_string()))?;
            for r in rows {
                let (Ok(lid), Ok(pid), Ok(ch)) = (r.try_get::<i32>("", "id"), r.try_get::<i32>("", "product_id"), r.try_get::<i32>("", "channel_id")) else { continue };
                let dc = r.try_get::<String>("", "default_country").unwrap_or_else(|_| "US".into());
                let mut c = metadata::lit_channel(crate::common::gid("Channel", ch), vec![], vec![]);
                c.slug = r.try_get::<String>("", "slug").ok();
                c.name = r.try_get::<String>("", "name").ok();
                c.is_active = r.try_get::<bool>("", "is_active").ok();
                c.currency_code = r.try_get::<String>("", "currency_code").ok();
                c.default_country = Some(crate::common::GqlCountryDisplay { code: dc.clone(), country: dc });
                pchan.insert(ch, c);
                plist.entry(pid).or_default().push((
                    lid, ch,
                    r.try_get::<bool>("", "is_published").unwrap_or(false),
                    r.try_get::<Option<chrono::DateTime<chrono::Utc>>>("", "published_at").ok().flatten(),
                    r.try_get::<Option<chrono::DateTime<chrono::Utc>>>("", "available_for_purchase_at").ok().flatten(),
                    r.try_get::<bool>("", "visible_in_listings").unwrap_or(false)));
            }
        }
        // --- collections ---
        let mut colls: HashMap<i32, Vec<gen::Collection>> = HashMap::new();
        if !pids.is_empty() {
            let rows = qall(format!("SELECT cp.product_id, c.id, c.name, c.slug FROM product_collectionproduct cp JOIN product_collection c ON c.id = cp.collection_id WHERE cp.product_id IN ({})", in_list(pids.len())),
                pids.iter().map(|i| (*i).into()).collect()).await.map_err(|e| Error::new(e.to_string()))?;
            for r in rows {
                if let (Ok(pid), Ok(cid)) = (r.try_get::<i32>("", "product_id"), r.try_get::<i32>("", "id")) {
                    let mut c = metadata::lit_collection(crate::common::gid("Collection", cid), vec![], vec![]);
                    c.name = r.try_get::<String>("", "name").ok();
                    c.slug = r.try_get::<String>("", "slug").ok();
                    colls.entry(pid).or_default().push(c);
                }
            }
        }
        // --- product attributes ---
        let mut pattrs: HashMap<i32, Vec<gen::SelectedAttribute>> = HashMap::new();
        if !pids.is_empty() {
            let rows = qall(format!("SELECT ap.product_id, v.id AS vid, v.name AS vname, v.slug AS vslug, a.id AS aid, a.name AS aname FROM attribute_assignedproductattributevalue ap JOIN attribute_attributevalue v ON v.id = ap.value_id JOIN attribute_attribute a ON a.id = v.attribute_id WHERE ap.product_id IN ({})", in_list(pids.len())),
                pids.iter().map(|i| (*i).into()).collect()).await.map_err(|e| Error::new(e.to_string()))?;
            for r in rows {
                if let (Ok(pid), Ok(vid), Ok(aid)) = (r.try_get::<i32>("", "product_id"), r.try_get::<i32>("", "vid"), r.try_get::<i32>("", "aid")) {
                    let mut a = metadata::lit_attribute(crate::common::gid("Attribute", aid), vec![], vec![]);
                    a.name = r.try_get::<String>("", "aname").ok();
                    let mut av = metadata::lit_attribute_value(crate::common::gid("AttributeValue", vid), vec![], vec![]);
                    av.name = r.try_get::<String>("", "vname").ok();
                    av.slug = r.try_get::<String>("", "vslug").ok();
                    pattrs.entry(pid).or_default().push(gen::SelectedAttribute { attribute: Some(a), values: vec![av] });
                }
            }
        }
        // --- product rows (type + category batches kept) ---
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
        // price ranges per (product, channel) from variant listings.
        let mut ranges: HashMap<(i32, i32), (rust_decimal::Decimal, rust_decimal::Decimal, String)> = HashMap::new();
        for p in &all {
            if let Ok(pid) = p.id.parse::<i32>() {
                for v in &p.variants {
                    if let Ok(vid) = v.id.parse::<i32>() {
                        if let Some(ls) = vb.vlist.get(&vid) {
                            for (_, ch, price, _, cur) in ls {
                                if let Some(pr) = price {
                                    ranges.entry((pid, *ch)).and_modify(|e| {
                                        if *pr < e.0 { e.0 = *pr; }
                                        if *pr > e.1 { e.1 = *pr; }
                                    }).or_insert((*pr, *pr, cur.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
        let taxed = |amount: rust_decimal::Decimal, currency: String| crate::order::GqlTaxedMoney {
            currency: Some(currency.clone()),
            gross: money(amount, currency.clone()),
            net: money(amount, currency),
            tax: None,
        };
        // variant channels come from the SHARED batch (single source of truth
        // for listing channels); product listings carry their own join.
        let page: Vec<gen::Product> = all.into_iter().map(|p| {
            let pid: i32 = p.id.parse().unwrap_or(-1);
            let default_variant = p.variants.iter().next().and_then(|v| v.id.parse::<i32>().ok())
                .map(|vid| {
                    let core_v = p.variants.iter().find(|v| v.id.parse::<i32>().ok() == Some(vid)).unwrap();
                    build_variant(&vb, vid, core_v.name.clone(), core_v.sku.clone(), core_v.quantity_available)
                });
            let listings: Vec<gen::ProductChannelListing> = plist.get(&pid).cloned().unwrap_or_default().into_iter().map(|(lid, ch, pub_, pub_at, avail_at, vis)| {
                let pricing = ranges.get(&(pid, ch)).map(|(lo, hi, c)| gen::ProductPricingInfo {
                    price_range: Some(gen::TaxedMoneyRange {
                        start: Some(taxed(*lo, c.clone())),
                        stop: Some(taxed(*hi, c.clone())),
                    }),
                });
                gen::ProductChannelListing {
                    id: Some(ID(crate::common::gid("ProductChannelListing", lid))),
                    published_at: pub_at,
                    is_published: Some(pub_),
                    channel: pchan.get(&ch).cloned().or_else(|| vb.channels.get(&ch).cloned()),
                    visible_in_listings: Some(vis),
                    available_for_purchase_at: avail_at,
                    is_available_for_purchase: Some(pub_),
                    pricing,
                }
            }).collect();
            let any_pub = plist.get(&pid).map(|ls| ls.iter().any(|l| l.2)).unwrap_or(false);
            let (desc, seo_t, seo_d, rating, tax_id) = extras.get(&pid).cloned().unwrap_or((None, None, None, None, None));
            gen::Product {
            id: Some(ID(crate::common::gid("Product", &p.id))),
            name: Some(p.name),
            slug: Some(p.slug),
            default_variant,
            private_metadata: vec![],
            metadata: vec![],
            seo_title: seo_t,
            seo_description: seo_d,
            description: desc.map(gen::GenJSONString),
            product_type: p.product_type_id.parse::<i32>().ok().and_then(|tid| types.get(&tid)).map(|(name, slug, hv)| gen::ProductType {
                id: Some(ID(crate::common::gid("ProductType", &p.product_type_id))),
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
                id: Some(ID(crate::common::gid("Category", p.category_id.clone().unwrap_or_default()))),
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
            rating,
            is_available: Some(any_pub),
            attributes: pattrs.get(&pid).cloned().unwrap_or_default(),
            channel_listings: listings,
            media: media_by_product.get(&pid).cloned().unwrap_or_default(),
            collections: colls.get(&pid).cloned().unwrap_or_default(),
            is_available_for_purchase: Some(any_pub),
            tax_class: tax_id.and_then(|t| tax_names.get(&t)).map(|n| {
                let mut tc = metadata::lit_tax_class(crate::common::gid("TaxClass", tax_id.unwrap_or(0)), vec![], vec![]);
                tc.name = Some(n.clone());
                tc
            }),
        }}).collect();
        Ok(page)
}

/// Full variant assembly for arbitrary variant ids (grids, payloads).
pub(crate) async fn assemble_variants(
    db: &sea_orm::DatabaseConnection,
    variant_ids: Vec<i32>,
) -> Result<Vec<(i32, gen::ProductVariant)>> {
    if variant_ids.is_empty() {
        return Ok(vec![]);
    }
    let vb = load_variant_batches(db, &variant_ids).await?;
    let mut out: Vec<(i32, gen::ProductVariant)> = variant_ids.iter().map(|vid| {
        let (sku, name, _, _) = vb.vbase.get(vid).cloned().unwrap_or((None, String::new(), true, None));
        (*vid, build_variant(&vb, *vid, name, sku.unwrap_or_default(), 0))
    }).collect();
    out.sort_by_key(|t| t.0);
    Ok(out)
}

/// Variants grid (`product.productVariants`) with search + pagination.
pub(crate) async fn product_variants_page(
    db: &sea_orm::DatabaseConnection,
    product_id: i32,
    search: Option<String>,
    first: Option<i32>,
    after: Option<String>,
) -> Result<Option<gen::ProductVariantCountableConnection>> {
    use sea_orm::{ConnectionTrait, Statement};
    let mut sql = "SELECT id FROM product_productvariant WHERE product_id = $1".to_string();
    let mut params: Vec<sea_orm::Value> = vec![product_id.into()];
    if let Some(s) = search.as_ref().filter(|s| !s.trim().is_empty()) {
        params.push(format!("%{s}%").into());
        sql.push_str(" AND (name ILIKE $2 OR sku ILIKE $2)");
    }
    sql.push_str(" ORDER BY id");
    let rows = db.query_all(Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params))
        .await.map_err(|e| Error::new(e.to_string()))?;
    let ids: Vec<i32> = rows.into_iter().filter_map(|r| r.try_get::<i32>("", "id").ok()).collect();
    let total = ids.len() as i32;
    let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
    let lim = first.unwrap_or(20).clamp(1, 100) as usize;
    let page_ids: Vec<i32> = ids.into_iter().skip(off).take(lim).collect();
    let assembled = assemble_variants(db, page_ids).await?;
    let edges = assembled.into_iter().map(|(_, v)| gen::ProductVariantCountableEdge { node: Some(v) }).collect();
    Ok(Some(gen::ProductVariantCountableConnection {
        page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
        edges,
        total_count: Some(total),
    }))
}

/// Single media lookup (`product.media_by_id`).
pub(crate) async fn media_by_id(
    db: &sea_orm::DatabaseConnection,
    media_gid: &str,
) -> Result<Option<gen::ProductMedia>> {
    use sea_orm::{ConnectionTrait, Statement};
    let Some(mid) = rustygod_db::catalog::parse_gid(media_gid) else { return Ok(None) };
    let rows = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, alt, sort_order, \"type\" FROM product_productmedia WHERE id = $1",
        [mid.into()],
    )).await.map_err(|e| Error::new(e.to_string()))?;
    let Some(r) = rows.into_iter().next() else { return Ok(None) };
    let mut m = metadata::lit_product_media(crate::common::gid("ProductMedia", mid), vec![], vec![]);
    m.alt = Some(r.try_get::<String>("", "alt").unwrap_or_default());
    m.sort_order = r.try_get::<Option<i32>>("", "sort_order").ok().flatten();
    m.r#type = r.try_get::<String>("", "type").ok();
    Ok(Some(m))
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
            let parsed = rustygod_db::catalog::parse_gid(&input.product_type.0).unwrap_or(0);
            if parsed != 0 { parsed } else {
                use sea_orm::EntityTrait;
                let pt = rustygod_db::entities::product_producttype::Entity::find().one(db).await.map_err(|e| Error::new(e.to_string()))?
                    .ok_or_else(|| Error::new("no product type found"))?;
                pt.id
            }
        };
        let cat_id: Option<i32> = input.category.as_ref().and_then(|c| rustygod_db::catalog::parse_gid(&c.0));
        let id = create_product_row(db, &name, &slug, pt_id, cat_id).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlProductCreate {
            product: Some(gen::Product {
                id: Some(ID(crate::common::gid("Product", id))),
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

/// Dashboard write mutations (product/variant/category/collection CRUD).
/// Reads stay above; deletes mirror Django's collector via `catalog_writes`.
#[derive(Default)]
pub struct CatalogWriteMutation;

fn perr(field: Option<String>, message: String) -> gen::ProductError {
    gen::ProductError { field, message: Some(message), code: None, attributes: vec![] }
}

fn cerr(field: Option<String>, message: String) -> gen::CollectionError {
    gen::CollectionError { field, message: Some(message), code: None }
}

fn lerr(field: Option<String>, message: String) -> gen::ProductChannelListingError {
    gen::ProductChannelListingError { field, message: Some(message), code: None, channels: vec![] }
}

fn serr(field: Option<String>, message: String) -> gen::BulkStockError {
    gen::BulkStockError { field, message: Some(message), code: None, index: None }
}

fn require_staff(ctx: &Context<'_>) -> Result<()> {
    let bearer = ctx.data_opt::<crate::context::Bearer>().map(|b| b.0.as_str().to_string())
        .or_else(|| ctx.data_opt::<GqlContext>().and_then(|g| g.bearer.clone()));
    if bearer.is_none() { return Err(Error::new("authentication required")); }
    Ok(())
}

async fn resolve_product(db: &sea_orm::DatabaseConnection, id: Option<ID>, ext: Option<String>) -> Result<Option<i32>> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    if let Some(i) = id {
        return Ok(rustygod_db::catalog::parse_gid(&i.0));
    }
    if let Some(x) = ext {
        return Ok(rustygod_db::entities::product_product::Entity::find()
            .select_only().column(rustygod_db::entities::product_product::Column::Id)
            .filter(rustygod_db::entities::product_product::Column::ExternalReference.eq(x))
            .into_tuple::<i32>().one(db).await.map_err(|e| Error::new(e.to_string()))?);
    }
    Ok(None)
}

async fn resolve_variant(db: &sea_orm::DatabaseConnection, id: Option<ID>, ext: Option<String>, sku: Option<String>) -> Result<Option<i32>> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    type V = rustygod_db::entities::product_productvariant::Entity;
    use rustygod_db::entities::product_productvariant::Column as VCol;
    if let Some(i) = id {
        return Ok(rustygod_db::catalog::parse_gid(&i.0));
    }
    if let Some(x) = ext {
        return Ok(V::find().select_only().column(VCol::Id)
            .filter(VCol::ExternalReference.eq(x))
            .into_tuple::<i32>().one(db).await.map_err(|e| Error::new(e.to_string()))?);
    }
    if let Some(s) = sku {
        return Ok(V::find().select_only().column(VCol::Id)
            .filter(VCol::Sku.eq(s))
            .into_tuple::<i32>().one(db).await.map_err(|e| Error::new(e.to_string()))?);
    }
    Ok(None)
}

/// Read current metadata JSON for merge semantics (dashboard sends full
/// lists, but merge matches Saleor's `update_metadata` behavior on conflicts).
async fn read_meta(db: &sea_orm::DatabaseConnection, table: &str, id: i32) -> (serde_json::Value, serde_json::Value) {
    use sea_orm::ConnectionTrait;
    let st = sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        format!("SELECT metadata, private_metadata FROM {table} WHERE id = $1"),
        [id.into()],
    );
    let row: Option<sea_orm::QueryResult> = db.query_one(st).await.ok().flatten();
    let get = |c: &str| row.as_ref().and_then(|r| r.try_get::<serde_json::Value>("", c).ok()).unwrap_or(serde_json::Value::Null);
    (get("metadata"), get("private_metadata"))
}

fn merged(cur: serde_json::Value, input: Option<Vec<crate::common::MetadataInput>>) -> Option<serde_json::Value> {
    input.map(|v| crate::common::merge_metadata(&cur, &v))
}

#[Object]
impl CatalogWriteMutation {
    /// Dashboard `UpdateProduct`: identity fields + category + collections +
    /// SEO + metadata. Attributes accepted-ignored (own milestone).
    async fn product_update(&self, ctx: &Context<'_>, id: Option<ID>, #[graphql(name = "externalReference")] external_reference: Option<String>, input: gen::ProductInput) -> Result<gen::ProductUpdate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(pid) = resolve_product(db, id, external_reference).await? else {
            return Ok(gen::ProductUpdate { errors: vec![perr(Some("id".into()), "product not found".into())] });
        };
        let (cur_md, cur_pmd) = read_meta(db, "product_product", pid).await;
        let patch = rustygod_db::catalog_writes::ProductPatch {
            name: input.name.clone(),
            slug: input.slug.clone(),
            description: input.description.clone().map(|d| d.0.clone()),
            category_id: input.category.as_ref().map(|c| rustygod_db::catalog::parse_gid(&c.0)),
            seo_title: input.seo.clone().and_then(|s| s.title),
            seo_description: input.seo.clone().and_then(|s| s.description),
            rating: input.rating,
            tax_class_id: input.tax_class.as_ref().map(|t| rustygod_db::catalog::parse_gid(&t.0)),
            charge_taxes: input.charge_taxes,
            collections: input.collections.as_ref().map(|c| c.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect()),
            metadata: merged(cur_md, input.metadata.clone()),
            private_metadata: merged(cur_pmd, input.private_metadata.clone()),
        };
        match rustygod_db::catalog_writes::update_product(db, pid, &patch).await {
            Ok(()) => Ok(gen::ProductUpdate { errors: vec![] }),
            Err(e) => Ok(gen::ProductUpdate { errors: vec![perr(None, e.to_string())] }),
        }
    }

    async fn product_delete(&self, ctx: &Context<'_>, id: Option<ID>, #[graphql(name = "externalReference")] external_reference: Option<String>) -> Result<gen::ProductDelete> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(pid) = resolve_product(db, id, external_reference).await? else {
            return Ok(gen::ProductDelete { errors: vec![perr(Some("id".into()), "product not found".into())] });
        };
        match rustygod_db::catalog_writes::delete_product(db, pid).await {
            Ok(()) => Ok(gen::ProductDelete { errors: vec![] }),
            Err(e) => Ok(gen::ProductDelete { errors: vec![perr(None, e.to_string())] }),
        }
    }

    async fn product_variant_create(&self, ctx: &Context<'_>, input: gen::ProductVariantCreateInput) -> Result<gen::ProductVariantCreate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(pid) = rustygod_db::catalog::parse_gid(&input.product.0) else {
            return Ok(gen::ProductVariantCreate { product_variant: None, errors: vec![perr(Some("product".into()), "bad product id".into())] });
        };
        let vid = match rustygod_db::catalog_writes::create_variant(
            db, pid,
            input.sku.clone(),
            input.name.clone(),
            input.track_inventory.unwrap_or(true),
            input.quantity_limit_per_customer,
            input.external_reference.clone(),
        ).await {
            Ok(v) => v,
            Err(e) => return Ok(gen::ProductVariantCreate { product_variant: None, errors: vec![perr(None, e.to_string())] }),
        };
        for s in input.stocks.clone().unwrap_or_default() {
            match crate::common::parse_uuid_gid(&s.warehouse.0) {
                Some(wid) => if let Err(e) = rustygod_db::catalog_writes::set_variant_stock(db, vid, wid, s.quantity).await {
                    return Ok(gen::ProductVariantCreate { product_variant: None, errors: vec![perr(Some("stocks".into()), e.to_string())] });
                },
                None => return Ok(gen::ProductVariantCreate { product_variant: None, errors: vec![perr(Some("stocks".into()), "bad warehouse id".into())] }),
            }
        }
        Ok(gen::ProductVariantCreate {
            product_variant: Some(crate::metadata::lit_product_variant(crate::common::gid("ProductVariant", vid), vec![], vec![])),
            errors: vec![],
        })
    }

    async fn product_variant_update(&self, ctx: &Context<'_>, id: Option<ID>, #[graphql(name = "externalReference")] external_reference: Option<String>, input: gen::ProductVariantInput, sku: Option<String>) -> Result<gen::ProductVariantUpdate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(vid) = resolve_variant(db, id, external_reference, sku).await? else {
            return Ok(gen::ProductVariantUpdate { product_variant: None, errors: vec![perr(Some("id".into()), "variant not found".into())] });
        };
        let (cur_md, cur_pmd) = read_meta(db, "product_productvariant", vid).await;
        let patch = rustygod_db::catalog_writes::VariantPatch {
            sku: input.sku.clone().map(Some),
            name: input.name.clone(),
            track_inventory: input.track_inventory,
            quantity_limit_per_customer: input.quantity_limit_per_customer.map(Some),
            external_reference: input.external_reference.clone().map(Some),
            metadata: merged(cur_md, input.metadata.clone()),
            private_metadata: merged(cur_pmd, input.private_metadata.clone()),
        };
        match rustygod_db::catalog_writes::update_variant(db, vid, &patch).await {
            Ok(()) => Ok(gen::ProductVariantUpdate {
                product_variant: Some(crate::metadata::lit_product_variant(crate::common::gid("ProductVariant", vid), vec![], vec![])),
                errors: vec![],
            }),
            Err(e) => Ok(gen::ProductVariantUpdate { product_variant: None, errors: vec![perr(None, e.to_string())] }),
        }
    }

    async fn product_variant_delete(&self, ctx: &Context<'_>, id: Option<ID>, #[graphql(name = "externalReference")] external_reference: Option<String>, sku: Option<String>) -> Result<gen::ProductVariantDelete> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(vid) = resolve_variant(db, id, external_reference, sku).await? else {
            return Ok(gen::ProductVariantDelete { errors: vec![perr(Some("id".into()), "variant not found".into())] });
        };
        match rustygod_db::catalog_writes::delete_variant(db, vid).await {
            Ok(()) => Ok(gen::ProductVariantDelete { errors: vec![] }),
            Err(e) => Ok(gen::ProductVariantDelete { errors: vec![perr(None, e.to_string())] }),
        }
    }

    /// Dashboard ` productVariantChannelListingUpdate`: per-channel price /
    /// cost / prior upserts (id or sku locates the variant).
    async fn product_variant_channel_listing_update(&self, ctx: &Context<'_>, id: Option<ID>, input: Vec<gen::ProductVariantChannelListingAddInput>, sku: Option<String>) -> Result<gen::ProductVariantChannelListingUpdate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(vid) = resolve_variant(db, id, None, sku).await? else {
            return Ok(gen::ProductVariantChannelListingUpdate { variant: None, errors: vec![lerr(Some("id".into()), "variant not found".into())] });
        };
        for l in &input {
            let Some(ch) = rustygod_db::catalog::parse_gid(&l.channel_id.0) else {
                return Ok(gen::ProductVariantChannelListingUpdate { variant: None, errors: vec![lerr(Some("channelId".into()), "bad channel id".into())] });
            };
            let price: rust_decimal::Decimal = match l.price.0.parse() {
                Ok(p) => p,
                Err(_) => return Ok(gen::ProductVariantChannelListingUpdate { variant: None, errors: vec![lerr(Some("price".into()), "bad price".into())] }),
            };
            let cost: Option<rust_decimal::Decimal> = l.cost_price.as_ref().and_then(|c| c.0.parse().ok());
            let prior: Option<rust_decimal::Decimal> = l.prior_price.as_ref().and_then(|c| c.0.parse().ok());
            if let Err(e) = rustygod_db::catalog_writes::upsert_variant_listing(db, vid, ch, price, cost, prior).await {
                return Ok(gen::ProductVariantChannelListingUpdate { variant: None, errors: vec![lerr(None, e.to_string())] });
            }
        }
        Ok(gen::ProductVariantChannelListingUpdate { variant: None, errors: vec![] })
    }

    async fn product_variant_stocks_create(&self, ctx: &Context<'_>, stocks: Vec<gen::StockInput>, #[graphql(name = "variantId")] variant_id: ID) -> Result<gen::ProductVariantStocksCreate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(vid) = rustygod_db::catalog::parse_gid(&variant_id.0) else {
            return Ok(gen::ProductVariantStocksCreate { product_variant: None, errors: vec![serr(Some("variantId".into()), "bad variant id".into())] });
        };
        for s in &stocks {
            let Some(wid) = crate::common::parse_uuid_gid(&s.warehouse.0) else {
                return Ok(gen::ProductVariantStocksCreate { product_variant: None, errors: vec![serr(Some("warehouse".into()), "bad warehouse id".into())] });
            };
            if let Err(e) = rustygod_db::catalog_writes::set_variant_stock(db, vid, wid, s.quantity).await {
                return Ok(gen::ProductVariantStocksCreate { product_variant: None, errors: vec![serr(None, e.to_string())] });
            }
        }
        Ok(gen::ProductVariantStocksCreate { product_variant: None, errors: vec![] })
    }

    async fn product_variant_stocks_update(&self, ctx: &Context<'_>, sku: Option<String>, stocks: Vec<gen::StockInput>, #[graphql(name = "variantId")] variant_id: Option<ID>) -> Result<gen::ProductVariantStocksUpdate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let vid = match resolve_variant(db, variant_id, None, sku).await? {
            Some(v) => v,
            None => return Ok(gen::ProductVariantStocksUpdate { product_variant: None, errors: vec![serr(Some("variantId".into()), "variant not found".into())] }),
        };
        for s in &stocks {
            let Some(wid) = crate::common::parse_uuid_gid(&s.warehouse.0) else {
                return Ok(gen::ProductVariantStocksUpdate { product_variant: None, errors: vec![serr(Some("warehouse".into()), "bad warehouse id".into())] });
            };
            if let Err(e) = rustygod_db::catalog_writes::set_variant_stock(db, vid, wid, s.quantity).await {
                return Ok(gen::ProductVariantStocksUpdate { product_variant: None, errors: vec![serr(None, e.to_string())] });
            }
        }
        Ok(gen::ProductVariantStocksUpdate { product_variant: None, errors: vec![] })
    }

    async fn category_create(&self, ctx: &Context<'_>, input: gen::CategoryInput, parent: Option<ID>) -> Result<gen::CategoryCreate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let name = input.name.clone().unwrap_or_default();
        if name.trim().is_empty() {
            return Ok(gen::CategoryCreate { errors: vec![perr(Some("name".into()), "name is required".into())], category: None });
        }
        let slug = input.slug.clone().unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let pid = parent.map(|p| rustygod_db::catalog::parse_gid(&p.0)).unwrap_or(None);
        match rustygod_db::catalog_writes::create_category(db, &name, &slug, input.description.as_ref().map(|d| d.0.as_str()), pid).await {
            Ok(id) => Ok(gen::CategoryCreate { errors: vec![], category: Some(crate::metadata::lit_category(crate::common::gid("Category", id), vec![], vec![])) }),
            Err(e) => Ok(gen::CategoryCreate { errors: vec![perr(None, e.to_string())], category: None }),
        }
    }

    async fn category_update(&self, ctx: &Context<'_>, id: ID, input: gen::CategoryInput) -> Result<gen::CategoryUpdate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(cid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::CategoryUpdate { errors: vec![perr(Some("id".into()), "bad category id".into())], category: None });
        };
        let patch = rustygod_db::catalog_writes::CategoryPatch {
            name: input.name.clone(),
            slug: input.slug.clone(),
            description: input.description.clone().map(|d| d.0.clone()),
            seo_title: input.seo.clone().and_then(|s| s.title),
            seo_description: input.seo.clone().and_then(|s| s.description),
        };
        match rustygod_db::catalog_writes::update_category(db, cid, &patch).await {
            Ok(()) => Ok(gen::CategoryUpdate { errors: vec![], category: None }),
            Err(e) => Ok(gen::CategoryUpdate { errors: vec![perr(None, e.to_string())], category: None }),
        }
    }

    async fn category_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::CategoryDelete> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(cid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::CategoryDelete { errors: vec![perr(Some("id".into()), "bad category id".into())] });
        };
        match rustygod_db::catalog_writes::delete_category(db, cid).await {
            Ok(()) => Ok(gen::CategoryDelete { errors: vec![] }),
            Err(e) => Ok(gen::CategoryDelete { errors: vec![perr(None, e.to_string())] }),
        }
    }

    async fn collection_create(&self, ctx: &Context<'_>, input: gen::CollectionCreateInput) -> Result<gen::CollectionCreate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let name = input.name.clone().unwrap_or_default();
        if name.trim().is_empty() {
            return Ok(gen::CollectionCreate { errors: vec![cerr(Some("name".into()), "name is required".into())], collection: None });
        }
        let slug = input.slug.clone().unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let prods: Vec<i32> = input.products.as_ref().map(|p| p.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect()).unwrap_or_default();
        match rustygod_db::catalog_writes::create_collection(db, &name, &slug, input.description.as_ref().map(|d| d.0.as_str()), input.is_published.unwrap_or(false), &prods).await {
            Ok(id) => Ok(gen::CollectionCreate { errors: vec![], collection: Some(crate::metadata::lit_collection(crate::common::gid("Collection", id), vec![], vec![])) }),
            Err(e) => Ok(gen::CollectionCreate { errors: vec![cerr(None, e.to_string())], collection: None }),
        }
    }

    async fn collection_update(&self, ctx: &Context<'_>, id: ID, input: gen::CollectionInput) -> Result<gen::CollectionUpdate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(cid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::CollectionUpdate { errors: vec![cerr(Some("id".into()), "bad collection id".into())], collection: None });
        };
        let patch = rustygod_db::catalog_writes::CollectionPatch {
            name: input.name.clone(),
            slug: input.slug.clone(),
            description: input.description.clone().map(|d| d.0.clone()),
            is_published: input.is_published,
            seo_title: input.seo.clone().and_then(|s| s.title),
            seo_description: input.seo.clone().and_then(|s| s.description),
            // CollectionInput carries no products (dashboard manages
            // membership via collectionAdd/RemoveProducts).
            products: None,
        };
        match rustygod_db::catalog_writes::update_collection(db, cid, &patch).await {
            Ok(()) => Ok(gen::CollectionUpdate { errors: vec![], collection: None }),
            Err(e) => Ok(gen::CollectionUpdate { errors: vec![cerr(None, e.to_string())], collection: None }),
        }
    }

    async fn collection_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::CollectionDelete> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(cid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::CollectionDelete { errors: vec![cerr(Some("id".into()), "bad collection id".into())] });
        };
        match rustygod_db::catalog_writes::delete_collection(db, cid).await {
            Ok(()) => Ok(gen::CollectionDelete { errors: vec![] }),
            Err(e) => Ok(gen::CollectionDelete { errors: vec![cerr(None, e.to_string())] }),
        }
    }

    async fn collection_add_products(&self, ctx: &Context<'_>, #[graphql(name = "collectionId")] collection_id: ID, products: Vec<ID>) -> Result<gen::CollectionAddProducts> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let (Some(cid), prods) = (rustygod_db::catalog::parse_gid(&collection_id.0), products.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect::<Vec<_>>()) else {
            return Ok(gen::CollectionAddProducts { errors: vec![cerr(Some("collectionId".into()), "bad collection id".into())] });
        };
        match rustygod_db::catalog_writes::collection_add_products(db, cid, &prods).await {
            Ok(()) => Ok(gen::CollectionAddProducts { errors: vec![] }),
            Err(e) => Ok(gen::CollectionAddProducts { errors: vec![cerr(None, e.to_string())] }),
        }
    }

    async fn collection_remove_products(&self, ctx: &Context<'_>, #[graphql(name = "collectionId")] collection_id: ID, products: Vec<ID>) -> Result<gen::CollectionRemoveProducts> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let (Some(cid), prods) = (rustygod_db::catalog::parse_gid(&collection_id.0), products.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect::<Vec<_>>()) else {
            return Ok(gen::CollectionRemoveProducts { collection: None, errors: vec![cerr(Some("collectionId".into()), "bad collection id".into())] });
        };
        match rustygod_db::catalog_writes::collection_remove_products(db, cid, &prods).await {
            Ok(()) => Ok(gen::CollectionRemoveProducts { collection: None, errors: vec![] }),
            Err(e) => Ok(gen::CollectionRemoveProducts { collection: None, errors: vec![cerr(None, e.to_string())] }),
        }
    }

    /// Dashboard product save: bulk variant updates with nested stocks +
    /// channel listings. Per-variant results (Saleor parity); attributes
    /// accepted-ignored like the singular path.
    async fn product_variant_bulk_update(
        &self, ctx: &Context<'_>,
        #[graphql(name = "errorPolicy")] error_policy: Option<gen::ErrorPolicyEnum>,
        product: ID, variants: Vec<gen::ProductVariantBulkUpdateInput>,
    ) -> Result<gen::ProductVariantBulkUpdate> {
        let _ = (error_policy, product);
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let berr = |m: String| gen::ProductVariantBulkError { field: None, message: Some(m), code: None, attributes: vec![], values: vec![], warehouses: vec![], channels: vec![] };
        let mut results = vec![];
        for v in &variants {
            let vid = match resolve_variant(db, Some(v.id.clone()), None, None).await? {
                Some(id) => id,
                None => {
                    results.push(gen::ProductVariantBulkResult { product_variant: None, errors: vec![berr("variant not found".into())] });
                    continue;
                }
            };
            let (cur_md, cur_pmd) = read_meta(db, "product_productvariant", vid).await;
            let patch = rustygod_db::catalog_writes::VariantPatch {
                sku: v.sku.clone().map(Some),
                name: v.name.clone(),
                track_inventory: v.track_inventory,
                quantity_limit_per_customer: v.quantity_limit_per_customer.map(Some),
                external_reference: v.external_reference.clone().map(Some),
                metadata: merged(cur_md, v.metadata.clone()),
                private_metadata: merged(cur_pmd, v.private_metadata.clone()),
            };
            let mut errs: Vec<gen::ProductVariantBulkError> = vec![];
            if let Err(e) = rustygod_db::catalog_writes::update_variant(db, vid, &patch).await {
                errs.push(berr(e.to_string()));
            }
            for e in apply_variant_sub(db, vid, v.stocks.clone(), v.channel_listings.clone()).await {
                errs.push(berr(e));
            }
            let pv = if errs.is_empty() {
                let b = load_variant_batches(db, &[vid]).await?;
                let (sku, name, _, _) = b.vbase.get(&vid).cloned().unwrap_or((None, String::new(), true, None));
                Some(build_variant(&b, vid, name, sku.unwrap_or_default(), 0))
            } else { None };
            results.push(gen::ProductVariantBulkResult { product_variant: pv, errors: errs });
        }
        Ok(gen::ProductVariantBulkUpdate { results, errors: vec![] })
    }

    async fn product_variant_bulk_create(
        &self, ctx: &Context<'_>,
        #[graphql(name = "errorPolicy")] error_policy: Option<gen::ErrorPolicyEnum>,
        product: ID, variants: Vec<gen::ProductVariantBulkCreateInput>,
    ) -> Result<gen::ProductVariantBulkCreate> {
        let _ = error_policy;
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(pid) = rustygod_db::catalog::parse_gid(&product.0) else {
            return Ok(gen::ProductVariantBulkCreate { product_variants: vec![], results: vec![], errors: vec![gen::BulkProductError { field: Some("product".into()), message: Some("bad product id".into()), code: None, index: None, channels: vec![] }] });
        };
        let mut pvs = vec![];
        let mut results = vec![];
        for (idx, v) in variants.iter().enumerate() {
            let berr = |m: String| gen::ProductVariantBulkError { field: None, message: Some(m), code: None, attributes: vec![], values: vec![], warehouses: vec![], channels: vec![] };
            let vid = match rustygod_db::catalog_writes::create_variant(
                db, pid, v.sku.clone(), v.name.clone(),
                v.track_inventory.unwrap_or(true), v.quantity_limit_per_customer, v.external_reference.clone(),
            ).await {
                Ok(id) => id,
                Err(e) => {
                    results.push(gen::ProductVariantBulkResult { product_variant: None, errors: vec![berr(e.to_string())] });
                    continue;
                }
            };
            let mut errs: Vec<gen::ProductVariantBulkError> = vec![];
            for e in apply_variant_sub(db, vid, v.stocks.clone().map(|s| gen::ProductVariantStocksUpdateInput { create: Some(s), update: None, remove: None }), v.channel_listings.clone().map(|c| gen::ProductVariantChannelListingUpdateInput { create: Some(c), update: None, remove: None })).await {
                errs.push(berr(e));
            }
            let _ = idx;
            let pv = if errs.is_empty() {
                let b = load_variant_batches(db, &[vid]).await?;
                let (sku, name, _, _) = b.vbase.get(&vid).cloned().unwrap_or((None, String::new(), true, None));
                Some(build_variant(&b, vid, name, sku.unwrap_or_default(), 0))
            } else { None };
            if let Some(ref p) = pv { pvs.push(p.clone()); }
            results.push(gen::ProductVariantBulkResult { product_variant: pv, errors: errs });
        }
        Ok(gen::ProductVariantBulkCreate { product_variants: pvs, results, errors: vec![] })
    }

    async fn product_variant_bulk_delete(
        &self, ctx: &Context<'_>,
        ids: Option<Vec<ID>>, skus: Option<Vec<String>>,
    ) -> Result<gen::ProductVariantBulkDelete> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let mut errs = vec![];
        for i in ids.unwrap_or_default() {
            match rustygod_db::catalog::parse_gid(&i.0) {
                Some(vid) => if let Err(e) = rustygod_db::catalog_writes::delete_variant(db, vid).await {
                    errs.push(perr(None, e.to_string()));
                },
                None => errs.push(perr(Some("ids".into()), "bad variant id".into())),
            }
        }
        // skus resolve via the variant table
        for s in skus.unwrap_or_default() {
            match resolve_variant(db, None, None, Some(s)).await? {
                Some(vid) => if let Err(e) = rustygod_db::catalog_writes::delete_variant(db, vid).await {
                    errs.push(perr(None, e.to_string()));
                },
                None => errs.push(perr(Some("skus".into()), "variant not found".into())),
            }
        }
        Ok(gen::ProductVariantBulkDelete { errors: errs })
    }

    /// Dashboard publish switches (product availability per channel).
    async fn product_channel_listing_update(
        &self, ctx: &Context<'_>, id: ID, input: gen::ProductChannelListingUpdateInput,
    ) -> Result<gen::ProductChannelListingUpdate> {
        require_staff(ctx)?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let lerr = |m: String| gen::ProductChannelListingError { field: None, message: Some(m), code: None, channels: vec![] };
        let Some(pid) = rustygod_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::ProductChannelListingUpdate { product: None, errors: vec![lerr("bad product id".into())] });
        };
        for c in input.update_channels.clone().unwrap_or_default() {
            match rustygod_db::catalog::parse_gid(&c.channel_id.0) {
                Some(ch) => {
                    if let Err(e) = rustygod_db::catalog_writes::upsert_product_listing(
                        db, pid, ch, c.is_published,
                        c.published_at.or(c.publication_date), c.visible_in_listings,
                        c.available_for_purchase_at.or(c.available_for_purchase_date),
                    ).await {
                        return Ok(gen::ProductChannelListingUpdate { product: None, errors: vec![lerr(e.to_string())] });
                    }
                }
                None => return Ok(gen::ProductChannelListingUpdate { product: None, errors: vec![lerr("bad channel id".into())] }),
            }
        }
        for r in input.remove_channels.clone().unwrap_or_default() {
            match rustygod_db::catalog::parse_gid(&r.0) {
                Some(ch) => if let Err(e) = rustygod_db::catalog_writes::delete_product_listing(db, pid, ch).await {
                    return Ok(gen::ProductChannelListingUpdate { product: None, errors: vec![lerr(e.to_string())] });
                },
                None => return Ok(gen::ProductChannelListingUpdate { product: None, errors: vec![lerr("bad channel id".into())] }),
            }
        }
        Ok(gen::ProductChannelListingUpdate { product: None, errors: vec![] })
    }
}

/// Category node resolvers (list counts + details page relations).
/// Counts are single-row queries; the list page reads only totalCounts.
pub(crate) async fn category_children(
    db: &sea_orm::DatabaseConnection,
    cid_gid: &str,
    first: Option<i32>,
    after: Option<String>,
) -> Option<gen::CategoryCountableConnection> {
    use sea_orm::{ConnectionTrait, Statement};
    let cid = rustygod_db::catalog::parse_gid(cid_gid)?;
    let rows = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, name, slug FROM product_category WHERE parent_id = $1 ORDER BY name, id",
        [cid.into()],
    )).await.ok()?;
    let total = rows.len() as i32;
    let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
    let lim = first.unwrap_or(20).clamp(1, 100) as usize;
    let edges = rows.into_iter().skip(off).take(lim).filter_map(|r| {
        let (id, name, slug) = (r.try_get::<i32>("", "id").ok()?,
            r.try_get::<String>("", "name").ok()?, r.try_get::<String>("", "slug").ok()?);
        let mut c = metadata::lit_category(crate::common::gid("Category", id), vec![], vec![]);
        c.name = Some(name);
        c.slug = Some(slug);
        Some(gen::CategoryCountableEdge { node: Some(Box::new(c)) })
    }).collect();
    Some(gen::CategoryCountableConnection {
        page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
        edges,
        total_count: Some(total),
    })
}

/// Subtree product count (Saleor counts descendants, not just direct).
pub(crate) async fn category_products(
    db: &sea_orm::DatabaseConnection,
    cid_gid: &str,
) -> Option<GqlProductConnection> {
    use sea_orm::{ConnectionTrait, Statement};
    let cid = rustygod_db::catalog::parse_gid(cid_gid)?;
    let n: i32 = db.query_one(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT COUNT(p.id) AS n FROM product_category c JOIN product_category d ON d.tree_id = c.tree_id AND d.lft BETWEEN c.lft AND c.rght LEFT JOIN product_product p ON p.category_id = d.id WHERE c.id = $1",
        [cid.into()],
    )).await.ok()??.try_get::<i64>("", "n").ok()? as i32;
    Some(GqlProductConnection {
        total_count: Some(n),
        edges: vec![],
        page_info: crate::common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None },
    })
}

pub(crate) async fn category_ancestors(
    db: &sea_orm::DatabaseConnection,
    cid_gid: &str,
    first: Option<i32>,
) -> Option<gen::CategoryCountableConnection> {
    use sea_orm::{ConnectionTrait, Statement};
    let cid = rustygod_db::catalog::parse_gid(cid_gid)?;
    let rows = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT a.id, a.name, a.slug FROM product_category c JOIN product_category a ON a.tree_id = c.tree_id AND a.lft < c.lft AND c.lft < a.rght WHERE c.id = $1 ORDER BY a.lft",
        [cid.into()],
    )).await.ok()?;
    let total = rows.len() as i32;
    let lim = first.unwrap_or(100).clamp(1, 100) as usize;
    let edges = rows.into_iter().take(lim).filter_map(|r| {
        let (id, name, slug) = (r.try_get::<i32>("", "id").ok()?,
            r.try_get::<String>("", "name").ok()?, r.try_get::<String>("", "slug").ok()?);
        let mut c = metadata::lit_category(crate::common::gid("Category", id), vec![], vec![]);
        c.name = Some(name);
        c.slug = Some(slug);
        Some(gen::CategoryCountableEdge { node: Some(Box::new(c)) })
    }).collect();
    Some(gen::CategoryCountableConnection {
        page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }),
        edges,
        total_count: Some(total),
    })
}

pub(crate) async fn category_bg_image(
    db: &sea_orm::DatabaseConnection,
    cid_gid: &str,
) -> Option<crate::account::GqlImage> {
    use sea_orm::{ConnectionTrait, Statement};
    let cid = rustygod_db::catalog::parse_gid(cid_gid)?;
    let r = db.query_one(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT background_image, background_image_alt FROM product_category WHERE id = $1",
        [cid.into()],
    )).await.ok()??;
    let path: String = r.try_get::<Option<String>>("", "background_image").ok()??;
    Some(crate::account::GqlImage {
        url: crate::common::media_url(&path),
        alt: r.try_get::<String>("", "background_image_alt").ok(),
    })
}

/// Collection products tab + list counts (real rows via the shared assembly).
pub(crate) async fn collection_products(
    db: &sea_orm::DatabaseConnection,
    coll_gid: &str,
    first: Option<i32>,
    after: Option<String>,
) -> Option<GqlProductConnection> {
    use sea_orm::{ConnectionTrait, Statement};
    let cid = rustygod_db::catalog::parse_gid(coll_gid)?;
    let rows = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT product_id FROM product_collectionproduct WHERE collection_id = $1 ORDER BY product_id",
        [cid.into()],
    )).await.ok()?;
    let ids: Vec<i32> = rows.into_iter().filter_map(|r| r.try_get::<i32>("", "product_id").ok()).collect();
    let total = ids.len() as i32;
    let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
    let lim = first.unwrap_or(20).clamp(1, 100) as usize;
    let page: Vec<i32> = ids.into_iter().skip(off).take(lim).collect();
    if page.is_empty() {
        return Some(GqlProductConnection {
            total_count: Some(total), edges: vec![],
            page_info: crate::common::PageInfo { has_next_page: false, has_previous_page: off > 0, start_cursor: None, end_cursor: None },
        });
    }
    let mut f = rustygod_db::catalog::ProductListFilter::default();
    f.ids = page;
    // Channel-agnostic: list_products_filtered needs a channel for pricing;
    // default-channel prices are correct for the dashboard (single currency view).
    let items = rustygod_db::catalog::list_products_filtered(db, "default-channel", &f, 100)
        .await.ok()?;
    let assembled = assemble_list_products(db, items).await.ok()?;
    let edges = assembled.into_iter().map(|p| GqlProductEdge {
        cursor: crate::common::encode_cursor(0),
        node: p,
    }).collect();
    Some(GqlProductConnection {
        total_count: Some(total), edges,
        page_info: crate::common::PageInfo { has_next_page: false, has_previous_page: off > 0, start_cursor: None, end_cursor: None },
    })
}

pub(crate) async fn collection_bg_image(
    db: &sea_orm::DatabaseConnection,
    coll_gid: &str,
) -> Option<crate::account::GqlImage> {
    use sea_orm::{ConnectionTrait, Statement};
    let cid = rustygod_db::catalog::parse_gid(coll_gid)?;
    let r = db.query_one(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT background_image, background_image_alt FROM product_collection WHERE id = $1",
        [cid.into()],
    )).await.ok()??;
    let path: String = r.try_get::<Option<String>>("", "background_image").ok()??;
    Some(crate::account::GqlImage {
        url: crate::common::media_url(&path),
        alt: r.try_get::<String>("", "background_image_alt").ok(),
    })
}

/// Shared per-variant stocks+listings applier for the bulk mutations.
/// Returns per-item error strings (empty = ok).
async fn apply_variant_sub(
    db: &sea_orm::DatabaseConnection,
    vid: i32,
    stocks: Option<gen::ProductVariantStocksUpdateInput>,
    listings: Option<gen::ProductVariantChannelListingUpdateInput>,
) -> Vec<String> {
    let mut errs = vec![];
    if let Some(s) = stocks {
        for c in s.create.clone().unwrap_or_default() {
            match crate::common::parse_uuid_gid(&c.warehouse.0) {
                Some(wid) => {
                    if let Err(e) = rustygod_db::catalog_writes::set_variant_stock(db, vid, wid, c.quantity).await {
                        errs.push(e.to_string());
                    }
                }
                None => errs.push("bad warehouse id".into()),
            }
        }
        for u in s.update.clone().unwrap_or_default() {
            match rustygod_db::catalog::parse_gid(&u.stock.0) {
                Some(sid) => {
                    if let Err(e) = rustygod_db::catalog_writes::update_stock_qty(db, sid, u.quantity).await {
                        errs.push(e.to_string());
                    }
                }
                None => errs.push("bad stock id".into()),
            }
        }
        for w in s.remove.clone().unwrap_or_default() {
            // Saleor sends warehouse ids here.
            match crate::common::parse_uuid_gid(&w.0) {
                Some(wid) => {
                    if let Err(e) = rustygod_db::catalog_writes::delete_variant_stock(db, vid, wid).await {
                        errs.push(e.to_string());
                    }
                }
                None => errs.push("bad warehouse id".into()),
            }
        }
    }
    if let Some(l) = listings {
        // create-path entries (channelId + price)
        for c in l.create.clone().unwrap_or_default() {
            match rustygod_db::catalog::parse_gid(&c.channel_id.0) {
                Some(ch) => {
                    let price = match c.price.0.parse::<rust_decimal::Decimal>() {
                        Ok(p) => p,
                        Err(_) => { errs.push("bad price".into()); continue; }
                    };
                    let cost = c.cost_price.as_ref().and_then(|x| x.0.parse().ok());
                    let prior = c.prior_price.as_ref().and_then(|x| x.0.parse().ok());
                    if let Err(e) = rustygod_db::catalog_writes::upsert_variant_listing(db, vid, ch, price, cost, prior).await {
                        errs.push(e.to_string());
                    }
                }
                None => errs.push("bad channel id".into()),
            }
        }
        // update-path entries (listing id + optional price)
        for u in l.update.clone().unwrap_or_default() {
            match rustygod_db::catalog::parse_gid(&u.channel_listing.0) {
                Some(lid) => {
                    // resolve listing -> (variant, channel) for the upsert
                    let row: Option<(i32, i32)> = {
                        use sea_orm::{ConnectionTrait, Statement};
                        db.query_one(Statement::from_sql_and_values(
                            sea_orm::DatabaseBackend::Postgres,
                            "SELECT variant_id, channel_id FROM product_productvariantchannellisting WHERE id = $1",
                            [lid.into()],
                        )).await.ok().flatten().and_then(|r| {
                            Some((r.try_get::<i32>("", "variant_id").ok()?, r.try_get::<i32>("", "channel_id").ok()?))
                        })
                    };
                    if let Some((_, ch)) = row {
                        // price required for the upsert path; absent price =
                        // cost/prior-only touch is a no-op we still accept.
                        if let Some(pstr) = u.price.as_ref() {
                            match pstr.0.parse::<rust_decimal::Decimal>() {
                                Ok(price) => {
                                    let cost = u.cost_price.as_ref().and_then(|x| x.0.parse().ok());
                                    let prior = u.prior_price.as_ref().and_then(|x| x.0.parse().ok());
                                    if let Err(e) = rustygod_db::catalog_writes::upsert_variant_listing(db, vid, ch, price, cost, prior).await {
                                        errs.push(e.to_string());
                                    }
                                }
                                Err(_) => errs.push("bad price".into()),
                            }
                        }
                    } else {
                        errs.push("channel listing not found".into());
                    }
                }
                None => errs.push("bad channel listing id".into()),
            }
        }
        for r in l.remove.clone().unwrap_or_default() {
            if let Some(lid) = rustygod_db::catalog::parse_gid(&r.0) {
                if let Err(e) = rustygod_db::catalog_writes::delete_variant_listing(db, lid).await {
                    errs.push(e.to_string());
                }
            } else {
                errs.push("bad channel listing id".into());
            }
        }
    }
    errs
}
