//! Collaborative recommendations from Django's `order_orderline`
//! co-occurrence: customers who bought X also bought Y, scored by shared
//! order count. Prices/names resolve through the channel listings, so the
//! figures match what the catalog serves.

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

use crate::Result;

pub struct RecoHit {
    pub variant_id: i32,
    pub score: i64,
    pub price: rust_decimal::Decimal,
    pub currency: String,
    pub name: String,
}

#[tracing::instrument(skip(db))]
pub async fn recommend_for_variant(
    db: &DatabaseConnection,
    variant_id: i32,
    channel_slug: &str,
    limit: u64,
) -> Result<Vec<RecoHit>> {
    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"SELECT l2.variant_id AS vid, COUNT(*) AS score
             FROM order_orderline l1
             JOIN order_orderline l2
               ON l1.order_id = l2.order_id AND l2.variant_id <> l1.variant_id
            WHERE l1.variant_id = $1 AND l2.variant_id IS NOT NULL
            GROUP BY l2.variant_id
            ORDER BY score DESC LIMIT $2"#,
        vec![variant_id.into(), (limit as i64).into()],
    );
    let rows = db.query_all(stmt).await?;
    if rows.is_empty() {
        return Ok(vec![]);
    }
    let vids: Vec<i32> = rows
        .iter()
        .map(|r| r.try_get("", "vid"))
        .collect::<std::result::Result<_, _>>()?;
    let pricing =
        rustygod_db::catalog::checkout_pricing(db, channel_slug, &vids).await?;
    let mut out = Vec::new();
    for r in rows {
        let vid: i32 = r.try_get("", "vid")?;
        let score: i64 = r.try_get("", "score")?;
        if let Some((price, name)) = pricing.get(&vid) {
            out.push(RecoHit {
                variant_id: vid,
                score,
                price: price.amount,
                currency: price.currency.clone(),
                name: name.clone(),
            });
        }
    }
    Ok(out)
}
