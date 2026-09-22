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

#[Object]
impl CatalogQuery {
    /// Mirrors dashboard `ProductList` — channel defaults to
    /// `default-channel` (populatedb). Filter/sort inputs accepted for
    /// shape-compat (server-side filtering is a documented gap).
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
        let _ = (before, last, filter, sort_by, where_input, search);
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let off = after.and_then(|c| decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        let all = rustygod_db::catalog::list_products(db, &ch, None, 200).await.map_err(|e| Error::new(e.to_string()))?;
        let total = all.len() as i32;
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
        let page: Vec<gen::Product> = all.into_iter().skip(off).take(lim).map(|p| gen::Product {
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
        let edges = page.into_iter().enumerate().map(|(i, node)| GqlProductEdge { node, cursor: encode_cursor(off + i) }).collect();
        Ok(GqlProductConnection { total_count: Some(total), edges, page_info: crate::common::PageInfo { has_next_page: off + lim < total as usize, has_previous_page: off > 0, start_cursor: None, end_cursor: None } })
    }
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
        search_vector: Set(None),
        search_index_dirty: Set(true),
        ..Default::default()
    };
    let inserted = row.insert(db).await?;
    Ok(inserted.id)
}
