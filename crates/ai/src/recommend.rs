//! Collaborative recommendations from Django's `order_orderline`
//! co-occurrence: customers who bought X also bought Y, scored by shared
//! order count. Prices/names resolve through the channel listings, so the
//! figures match what the catalog serves.

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::collections::HashSet;

use crate::{vectors, Result};

pub struct RecoHitV2 {
    pub variant_id: i32,
    pub score: f64,
    pub price: rust_decimal::Decimal,
    pub currency: String,
    pub name: String,
}

/// Recommender v2: co-occurrence first (proven purchase signal), embedding
/// similarity as backfill (cold-start / thin-order coverage). Scores are
/// f64 throughout; the gRPC contract already carries `score` as double.
#[tracing::instrument(skip(db))]
pub async fn recommend_v2(
    db: &DatabaseConnection,
    variant_id: i32,
    channel_slug: &str,
    limit: u64,
) -> Result<Vec<RecoHitV2>> {
    let (ch_id, _) = saleor_rustify_db::catalog::channel_info(db, channel_slug).await?;
    let co = recommend_for_variant(db, variant_id, channel_slug, limit).await?;
    let mut out: Vec<RecoHitV2> = co
        .into_iter()
        .map(|r| RecoHitV2 {
            variant_id: r.variant_id,
            score: r.score as f64,
            price: r.price,
            currency: r.currency,
            name: r.name,
        })
        .collect();
    if out.len() >= limit as usize {
        out.truncate(limit as usize);
        return Ok(out);
    }
    // Backfill: variants of embedding-similar products.
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT product_id FROM product_productvariant WHERE id = $1".to_string(),
            [variant_id.into()],
        ))
        .await?;
    let pid: Option<i32> = rows.first().and_then(|r| r.try_get("", "product_id").ok());
    let Some(pid) = pid else { return Ok(out) };
    let published = vectors::published_in_channel(db, ch_id).await?;
    let sims = vectors::similar_products(db, pid, &published, (limit * 2) as usize).await?;
    let pids: Vec<i32> = sims.iter().map(|(p, _)| *p).collect();
    let rep = vectors::representative_variants(db, ch_id, &pids).await?;
    let vids: Vec<i32> = rep.values().copied().collect();
    let pricing = saleor_rustify_db::catalog::checkout_pricing(db, channel_slug, &vids).await?;
    let seen: HashSet<i32> = out.iter().map(|r| r.variant_id).collect();
    for (p, s) in sims {
        if out.len() >= limit as usize {
            break;
        }
        let Some(vid) = rep.get(&p) else { continue };
        if seen.contains(vid) {
            continue;
        }
        if let Some((price, name)) = pricing.get(vid) {
            out.push(RecoHitV2 {
                variant_id: *vid,
                score: s,
                price: price.amount,
                currency: price.currency.clone(),
                name: name.clone(),
            });
        }
    }
    Ok(out)
}

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
        saleor_rustify_db::catalog::checkout_pricing(db, channel_slug, &vids).await?;
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
