//! Vector tier over `rustygod_product_embedding` (no pgvector needed).
//!
//! Embeddings are produced by any `Embedder` (default: the deterministic
//! `HashingEmbedder`; a Candle/ONNX model slots in unchanged) and scored
//! with the shared `cosine` — the same function the in-memory store uses,
//! so the SQL tier and the baseline agree by construction.

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::collections::{HashMap, HashSet};

use crate::{
    traits::{cosine, Embedder},
    Result,
};

pub struct RefreshReport {
    pub scanned: usize,
    pub embedded: usize,
    pub skipped: usize,
}

/// Embed every product missing from (or changed since) the store.
/// Source text = product name + variant names; the stored fingerprint makes
/// re-runs embed only what changed. Safe to run on every boot.
#[tracing::instrument(skip(db, embedder))]
pub async fn refresh_product_embeddings(
    db: &DatabaseConnection,
    embedder: &impl Embedder,
) -> Result<RefreshReport> {
    rustygod_db::embeddings::ensure_table(db).await?;
    let rows = db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT p.id, p.name, COALESCE(string_agg(v.name, ' | '), '') AS variants \
             FROM product_product p \
             LEFT JOIN product_productvariant v ON v.product_id = p.id \
             GROUP BY p.id ORDER BY p.id"
                .to_string(),
        ))
        .await?;
    let stored = rustygod_db::embeddings::stored_sources(db).await?;
    let mut pending: Vec<(i32, String)> = Vec::new();
    let mut skipped = 0usize;
    for r in rows {
        let pid: i32 = r.try_get("", "id")?;
        let name: String = r.try_get("", "name")?;
        let variants: String = r.try_get("", "variants")?;
        let source = if variants.is_empty() { name } else { format!("{name}\n{variants}") };
        if stored.get(&pid).is_some_and(|s| s == &source) {
            skipped += 1;
        } else {
            pending.push((pid, source));
        }
    }
    let scanned = pending.len() + skipped;
    let mut embedded = 0usize;
    // Batch the embedder (local hashing is cheap; a model backend is not).
    for chunk in pending.chunks(64) {
        let texts: Vec<String> = chunk.iter().map(|(_, s)| s.clone()).collect();
        let vecs = embedder.embed(&texts).await?;
        for ((pid, source), vec) in chunk.iter().zip(vecs) {
            rustygod_db::embeddings::upsert(db, *pid, &vec, source).await?;
            embedded += 1;
        }
    }
    Ok(RefreshReport { scanned, embedded, skipped })
}

/// Cosine-ranked product ids for a raw query embedding, restricted to the
/// published set. `limit` caps the scan output, not the scan itself
/// (catalog-scale full scan by design — see `rustygod_db::embeddings`).
pub async fn similar_to_vector(
    db: &DatabaseConnection,
    query: &[f32],
    published: &HashSet<i32>,
    limit: usize,
) -> Result<Vec<(i32, f64)>> {
    let all = rustygod_db::embeddings::load_all(db).await?;
    let mut scored: Vec<(i32, f64)> = all
        .into_iter()
        .filter(|e| published.contains(&e.product_id) && e.embedding.len() == query.len())
        .map(|e| {
            let s = cosine(&e.embedding, query) as f64;
            (e.product_id, s)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    Ok(scored)
}

/// Embed `query` and rank published products (semantic search).
pub async fn semantic_search(
    db: &DatabaseConnection,
    embedder: &impl Embedder,
    query: &str,
    published: &HashSet<i32>,
    limit: usize,
) -> Result<Vec<(i32, f64)>> {
    let vecs = embedder.embed(&[query.to_string()]).await?;
    similar_to_vector(db, &vecs[0], published, limit).await
}

/// Products most similar to `product_id` (self excluded).
pub async fn similar_products(
    db: &DatabaseConnection,
    product_id: i32,
    published: &HashSet<i32>,
    limit: usize,
) -> Result<Vec<(i32, f64)>> {
    let all = rustygod_db::embeddings::load_all(db).await?;
    let Some(own) = all.iter().find(|e| e.product_id == product_id).map(|e| (e.embedding.clone(),)) else {
        return Ok(vec![]);
    };
    let own = own.0;
    let mut scored: Vec<(i32, f64)> = all
        .into_iter()
        .filter(|e| {
            e.product_id != product_id
                && published.contains(&e.product_id)
                && e.embedding.len() == own.len()
        })
        .map(|e| {
            let s = cosine(&e.embedding, &own) as f64;
            (e.product_id, s)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    Ok(scored)
}

/// Product ids published in a channel (visibility rule shared by search).
pub async fn published_in_channel(
    db: &DatabaseConnection,
    channel_id: i32,
) -> Result<HashSet<i32>> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT product_id FROM product_productchannellisting WHERE channel_id = $1 AND is_published".to_string(),
            [channel_id.into()],
        ))
        .await?;
    let mut set = HashSet::with_capacity(rows.len());
    for r in rows {
        set.insert(r.try_get::<i32>("", "product_id")?);
    }
    Ok(set)
}

/// First (lowest-id) channel-listed variant per product — the representative
/// variant recommender surfaces, priced in one batch by the caller.
pub async fn representative_variants(
    db: &DatabaseConnection,
    channel_id: i32,
    product_ids: &[i32],
) -> Result<HashMap<i32, i32>> {
    if product_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let ids = product_ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    let rows = db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT DISTINCT ON (v.product_id) v.product_id, v.id AS vid \
                 FROM product_productvariant v \
                 JOIN product_productvariantchannellisting l ON l.variant_id = v.id AND l.channel_id = {channel_id} \
                 WHERE v.product_id IN ({ids}) ORDER BY v.product_id, v.id"
            ),
        ))
        .await?;
    let mut map = HashMap::with_capacity(rows.len());
    for r in rows {
        map.insert(r.try_get::<i32>("", "product_id")?, r.try_get::<i32>("", "vid")?);
    }
    Ok(map)
}
