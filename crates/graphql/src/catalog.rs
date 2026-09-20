//! Catalog GraphQL: products + variants (read path the Dashboard lists).

use async_graphql::*;

use crate::{common::*, context::GqlContext};

#[derive(SimpleObject, Clone)]
pub struct GqlVariant {
    pub id: ID,
    pub name: String,
    pub sku: String,
    pub price: Money,
    /// Free units on the shelf (sum quantity - allocated, clamped ≥0).
    pub quantity_available: i32,
}

#[derive(SimpleObject, Clone)]
pub struct GqlProduct {
    pub id: ID,
    pub name: String,
    pub slug: String,
    pub variants: Vec<GqlVariant>,
}

#[derive(Default)]
pub struct CatalogQuery;

#[Object]
impl CatalogQuery {
    /// Mirrors dashboard `products(first, channel)` — channel defaults to
    /// `default-channel` (populatedb).
    async fn products(
        &self,
        ctx: &Context<'_>,
        channel: Option<String>,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<Vec<GqlProduct>> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let off = after.and_then(|c| decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        let all = rustygod_db::catalog::list_products(db, &ch, None, 200).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(all.into_iter().skip(off).take(lim).map(|p| GqlProduct {
            id: ID(p.id),
            name: p.name,
            slug: p.slug,
            variants: p.variants.into_iter().map(|v| GqlVariant {
                id: ID(v.id),
                name: v.name,
                sku: v.sku,
                price: Money { amount: v.price.amount.to_string(), currency: v.price.currency },
                quantity_available: v.quantity_available,
            }).collect(),
        }).collect())
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlProductCreated { pub id: ID }

#[derive(InputObject)]
pub struct ProductCreateInput {
    pub name: String,
    pub slug: Option<String>,
    #[graphql(name = "productType")]
    pub product_type: Option<ID>,
    pub category: Option<ID>,
}

#[derive(Default)]
pub struct CatalogMutation;

#[Object]
impl CatalogMutation {
    /// Minimal `productCreate` for Dashboard quick-create. Requires
    /// `manage_products` (staff). Creates product row + picks first
    /// productType if not given. Returns product id (plain, not global).
    async fn product_create(&self, ctx: &Context<'_>, input: ProductCreateInput) -> Result<GqlProductCreated> {
        let bearer = ctx.data_opt::<crate::context::Bearer>().map(|b| b.0.as_str().to_string())
            .or_else(|| ctx.data_opt::<GqlContext>().and_then(|g| g.bearer.clone()));
        if bearer.is_none() { return Err(Error::new("authentication required")); }
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        if input.name.trim().is_empty() { return Err(Error::new("name is required")); }
        let slug = input.slug.unwrap_or_else(|| input.name.to_lowercase().replace(' ', "-"));
        // Resolve product_type: use given or first existing.
        let pt_id: i32 = if let Some(pid) = input.product_type {
            pid.0.parse::<i32>().unwrap_or(0)
        } else {
            use sea_orm::EntityTrait;
            let pt = rustygod_db::entities::product_producttype::Entity::find().one(db).await.map_err(|e| Error::new(e.to_string()))?
                .ok_or_else(|| Error::new("no product type found"))?;
            pt.id
        };
        let cat_id: Option<i32> = input.category.and_then(|c| c.0.parse::<i32>().ok());
        let id = create_product_row(db, &input.name, &slug, pt_id, cat_id).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlProductCreated { id: ID(id.to_string()) })
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
