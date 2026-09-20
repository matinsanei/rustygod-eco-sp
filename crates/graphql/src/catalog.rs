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
