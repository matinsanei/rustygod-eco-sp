//! Lexical-tier product search over `product_product` using pg_trgm
//! similarity — the same gin_trgm_ops indexes Django's own search uses.
//! Only channel-published products are returned (Saleor visibility rule).

use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

use crate::Result;

#[derive(Clone)]
pub struct SearchHit {
    pub product_id: i32,
    pub name: String,
    pub slug: String,
    pub currency: String,
    pub min_price: Decimal,
    pub score: f64,
}

#[tracing::instrument(skip(db))]
pub async fn search_products(
    db: &DatabaseConnection,
    query: &str,
    channel_id: i32,
    currency: &str,
    limit: u64,
) -> Result<Vec<SearchHit>> {
    // Saleor ranks by trigram similarity; the `%` operator uses the gin index.
    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"SELECT p.id, p.name, p.slug, similarity(p.name, $1)::float8 AS sml,
               (SELECT MIN(l.price_amount)
                  FROM product_productvariant v
                  JOIN product_productvariantchannellisting l
                    ON l.variant_id = v.id AND l.channel_id = $3
                 WHERE v.product_id = p.id) AS min_price
           FROM product_product p
          WHERE p.name % $2
            AND p.id IN (SELECT product_id FROM product_productchannellisting
                         WHERE channel_id = $3 AND is_published)
          ORDER BY sml DESC LIMIT $4"#,
        vec![
            query.into(),
            query.into(),
            channel_id.into(),
            (limit as i64).into(),
        ],
    );
    let rows = db.query_all(stmt).await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(SearchHit {
            product_id: r.try_get("", "id")?,
            name: r.try_get("", "name")?,
            slug: r.try_get("", "slug")?,
            currency: currency.to_string(),
            min_price: r
                .try_get::<Option<Decimal>>("", "min_price")?
                .unwrap_or(Decimal::ZERO),
            score: r.try_get("", "sml")?,
        });
    }
    Ok(out)
}

use crate::{traits::Embedder, vectors};

/// Blended search: trigram lexical (exact-ish, Saleor-ranked) ∪ embedding
/// semantic (synonyms, cross-language-ish). Each side is min-max normalized
/// to 0..1 over its own result set, then combined 0.6 lexical / 0.4
/// semantic — lexical wins ties, semantics rescues zero-hit queries.
#[tracing::instrument(skip(db, embedder))]
pub async fn search_products_blended(
    db: &DatabaseConnection,
    embedder: &impl Embedder,
    query: &str,
    channel_id: i32,
    currency: &str,
    limit: u64,
) -> Result<Vec<SearchHit>> {
    let wide = limit * 2 + 5;
    let lex = search_products(db, query, channel_id, currency, wide).await?;
    let published = vectors::published_in_channel(db, channel_id).await?;
    let sem = vectors::semantic_search(db, embedder, query, &published, wide as usize).await?;
    if lex.is_empty() && sem.is_empty() {
        return Ok(vec![]);
    }
    fn norm(scores: &[f64]) -> Vec<f64> {
        let (mn, mx) = scores.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &s| {
            (a.min(s), b.max(s))
        });
        if !mx.is_finite() || (mx - mn) < 1e-9 {
            return scores.iter().map(|_| 1.0).collect();
        }
        scores.iter().map(|s| (s - mn) / (mx - mn)).collect()
    }
    let lex_n = norm(&lex.iter().map(|h| h.score).collect::<Vec<_>>());
    let sem_n = norm(&sem.iter().map(|(_, s)| *s).collect::<Vec<_>>());
    // Details for semantic-only ids (lexical hits already carry theirs).
    let lex_ids: std::collections::HashSet<i32> = lex.iter().map(|h| h.product_id).collect();
    let missing: Vec<i32> = sem.iter().map(|(p, _)| *p).filter(|p| !lex_ids.contains(p)).collect();
    let mut detail: std::collections::HashMap<i32, (String, String, Decimal)> = std::collections::HashMap::new();
    if !missing.is_empty() {
        let ids = missing.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        let rows = db
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    "SELECT p.id, p.name, p.slug, \
                     (SELECT MIN(l.price_amount) FROM product_productvariant v \
                      JOIN product_productvariantchannellisting l ON l.variant_id = v.id AND l.channel_id = {channel_id} \
                      WHERE v.product_id = p.id) AS min_price \
                     FROM product_product p WHERE p.id IN ({ids})"
                ),
            ))
            .await?;
        for r in rows {
            detail.insert(
                r.try_get::<i32>("", "id")?,
                (
                    r.try_get::<String>("", "name")?,
                    r.try_get::<String>("", "slug")?,
                    r.try_get::<Option<Decimal>>("", "min_price")?.unwrap_or(Decimal::ZERO),
                ),
            );
        }
    }
    let mut combined: std::collections::HashMap<i32, (f64, Option<&SearchHit>)> =
        std::collections::HashMap::new();
    for (h, n) in lex.iter().zip(lex_n) {
        combined.insert(h.product_id, (0.6 * n, Some(h)));
    }
    for ((pid, _), n) in sem.iter().zip(sem_n) {
        combined
            .entry(*pid)
            .and_modify(|e| e.0 += 0.4 * n)
            .or_insert((0.4 * n, None));
    }
    let mut out: Vec<SearchHit> = Vec::with_capacity(combined.len());
    for (pid, (score, hit)) in combined {
        if let Some(h) = hit {
            out.push(SearchHit { score, ..h.clone() });
        } else if let Some((name, slug, min_price)) = detail.remove(&pid) {
            out.push(SearchHit {
                product_id: pid,
                name,
                slug,
                currency: currency.to_string(),
                min_price,
                score,
            });
        }
    }
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(limit as usize);
    Ok(out)
}
