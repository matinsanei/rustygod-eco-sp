//! Commerce (channels/warehouses/taxes) + account groups — slim read
//! surface the Dashboard needs for dropdowns/filters.

use async_graphql::*;

use crate::context::GqlContext;

#[derive(SimpleObject, Clone)]
pub struct GqlChannel { pub id: ID, pub slug: String, pub currency: String }

#[derive(Default)]
pub struct CommerceQuery;

#[Object]
impl CommerceQuery {
    async fn channels(&self, ctx: &Context<'_>) -> Result<Vec<GqlChannel>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::EntityTrait;
        let rows = rustygod_db::entities::channel_channel::Entity::find().all(db).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(|c| GqlChannel { id: ID(c.id.to_string()), slug: c.slug, currency: c.currency_code }).collect())
    }
}
