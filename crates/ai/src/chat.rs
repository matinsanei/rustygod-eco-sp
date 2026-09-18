//! Retrieval-grounded shopping assistant (deterministic v1).
//! Pipeline: trigram search → also-bought expansion → grounded summary.
//! The LLM milestone keeps this exact shape and only swaps the composer.

use sea_orm::DatabaseConnection;

use crate::{recommend, search, Result};

pub struct AssistantReply {
    /// Text chunks followed by a final chunk carrying the products.
    pub text_parts: Vec<String>,
    pub product_ids: Vec<i32>,
}

#[tracing::instrument(skip(db))]
pub async fn answer(
    db: &DatabaseConnection,
    message: &str,
    channel_id: i32,
    channel_slug: &str,
    currency: &str,
    limit: u64,
) -> Result<AssistantReply> {
    let hits = search::search_products(db, message, channel_id, currency, limit).await?;
    if hits.is_empty() {
        return Ok(AssistantReply {
            text_parts: vec![format!(
                "I couldn't find anything matching \"{message}\" in this channel. \
                 Try a shorter term like a product name or category."
            )],
            product_ids: vec![],
        });
    }
    let mut text = format!("I found {} products for \"{message}\":\n", hits.len());
    for (i, h) in hits.iter().enumerate() {
        text.push_str(&format!(
            "{}. {} — {} {}\n",
            i + 1,
            h.name,
            h.min_price,
            h.currency
        ));
    }
    // Agentic-buying hook v1: attach frequently-bought-together picks.
    let first_variant = top_variant_for(db, hits[0].product_id).await?;
    let mut product_ids: Vec<i32> = hits.iter().map(|h| h.product_id).collect();
    if let Some(vid) = first_variant {
        let recos = recommend::recommend_for_variant(db, vid, channel_slug, 3).await?;
        if !recos.is_empty() {
            text.push_str("Frequently bought together:\n");
            for r in &recos {
                text.push_str(&format!("- {} ({} orders)\n", r.name, r.score));
            }
        }
    }
    Ok(AssistantReply {
        text_parts: vec![text],
        product_ids,
    })
}

async fn top_variant_for(
    db: &DatabaseConnection,
    product_id: i32,
) -> Result<Option<i32>> {
    use rustygod_db::entities::product_productvariant;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect, SelectorTrait};
    Ok(product_productvariant::Entity::find()
        .select_only()
        .column(product_productvariant::Column::Id)
        .filter(product_productvariant::Column::ProductId.eq(product_id))
        .into_tuple::<i32>()
        .one(db)
        .await?)
}
