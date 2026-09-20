//! Commerce (channels/warehouses/taxes/shipping/promotions/pages/menus) —
//! slim read surface the Dashboard needs for dropdowns/filters/lists.

use async_graphql::*;

use crate::context::GqlContext;

#[derive(SimpleObject, Clone)]
pub struct GqlChannel { pub id: ID, pub slug: String, pub currency: String }

#[derive(SimpleObject, Clone)]
pub struct GqlWarehouse { pub id: ID, pub name: String, pub slug: String }

#[derive(SimpleObject, Clone)]
pub struct GqlTaxClass { pub id: ID, pub name: String }

#[derive(SimpleObject, Clone)]
pub struct GqlShippingMethod { pub id: ID, pub name: String, pub price: String }

#[derive(SimpleObject, Clone)]
pub struct GqlPage { pub id: ID, pub slug: String, pub title: String }

#[derive(SimpleObject, Clone)]
pub struct GqlMenu { pub id: ID, pub slug: String, pub name: String }

#[derive(SimpleObject, Clone)]
pub struct GqlPromotion { pub id: ID, pub name: String, pub r#type: String }

#[derive(Default)]
pub struct CommerceQuery;

#[Object]
impl CommerceQuery {
    async fn channels(&self, ctx: &Context<'_>) -> Result<Vec<GqlChannel>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let rows = rustygod_db::commerce::list_channels(db).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(|c| GqlChannel { id: ID(c.id.to_string()), slug: c.slug, currency: c.currency_code }).collect())
    }

    async fn warehouses(&self, ctx: &Context<'_>, channel: Option<String>) -> Result<Vec<GqlWarehouse>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let rows = rustygod_db::commerce::list_warehouses(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(|w| GqlWarehouse { id: ID(w.id), name: w.name, slug: w.slug }).collect())
    }

    async fn tax_classes(&self, ctx: &Context<'_>) -> Result<Vec<GqlTaxClass>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let rows = rustygod_db::commerce::list_tax_classes(db).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(|(id, name)| GqlTaxClass { id: ID(id.to_string()), name }).collect())
    }

    async fn shipping_methods(&self, ctx: &Context<'_>, channel: Option<String>) -> Result<Vec<GqlShippingMethod>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let rows = rustygod_db::commerce::list_shipping_methods(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(|m| GqlShippingMethod { id: ID(m.id.to_string()), name: m.name, price: m.price_amount.to_string() }).collect())
    }

    async fn pages(&self, ctx: &Context<'_>) -> Result<Vec<GqlPage>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let rows = rustygod_db::commerce::list_pages(db).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(|p| GqlPage { id: ID(p.id.to_string()), slug: p.slug, title: p.title }).collect())
    }

    async fn promotions(&self, ctx: &Context<'_>, channel: Option<String>) -> Result<Vec<GqlPromotion>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let rows = rustygod_db::commerce::list_promotions(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(|p| GqlPromotion { id: ID(p.id), name: p.name, r#type: p.promotion_type }).collect())
    }

    async fn menu(&self, ctx: &Context<'_>, slug: String) -> Result<Option<GqlMenu>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let m = rustygod_db::commerce::get_menu(db, &slug).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(m.map(|x| GqlMenu { id: ID(x.id.to_string()), slug: x.slug, name: x.name }))
    }
}
