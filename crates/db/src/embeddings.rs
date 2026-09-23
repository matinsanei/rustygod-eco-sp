//! Product embedding store for the vector tier (`rustygod_product_embedding`).
//!
//! pgvector is NOT installed on the Saleor Postgres (stock
//! `postgres:15-alpine` ships no `vector` extension), so this module
//! deliberately avoids any extension dependency: embeddings live in a plain
//! cosine scoring happens in Rust over the full product set
//! (`rustygod_ai::traits::cosine` — the same function the in-memory store
//! uses). The catalog is hundreds of rows; a full scan is microseconds.
//! When pgvector (or Qdrant) lands, only `load_all`/`cosine_sql` change —
//! the table shape (`product_id`, `embedding`, `dim`, `source`, `updated_at`)
//! maps 1:1 onto a `vector(dim)` column migration.

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

use crate::{DbError, Result};

/// `CREATE TABLE IF NOT EXISTS` parity with `plugin_store::ensure_table`:
/// our own tables are created at runtime, never via Django migrations.
pub async fn ensure_table(db: &DatabaseConnection) -> Result<()> {
    // Postgres races concurrent `CREATE TABLE IF NOT EXISTS` on the same
    // name (duplicate `pg_type` key, 23505): parallel test workers and the
    // beat both hit this on first boot. "Already exists" in any form means
    // success — the shape is fixed, there is nothing to migrate.
    match db.execute(Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        r#"CREATE TABLE IF NOT EXISTS rustygod_product_embedding (
            product_id INTEGER PRIMARY KEY REFERENCES product_product(id) DEFERRABLE INITIALLY DEFERRED,
            embedding REAL[] NOT NULL,
            dim SMALLINT NOT NULL,
            source TEXT NOT NULL DEFAULT '',
            updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )"#
        .to_string(),
    ))
    .await
    {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("already exists") => Ok(()),
        Err(e) => Err(DbError::SeaOrm(e)),
    }
}

pub struct StoredEmbedding {
    pub product_id: i32,
    pub embedding: Vec<f32>,
}

/// Load every stored embedding (catalog-scale full scan by design).
pub async fn load_all(db: &impl ConnectionTrait) -> Result<Vec<StoredEmbedding>> {
    let rows = db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT product_id, embedding FROM rustygod_product_embedding".to_string(),
        ))
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let embedding: Vec<f32> = r
            .try_get("", "embedding")
            .map_err(DbError::SeaOrm)?;
        out.push(StoredEmbedding {
            product_id: r.try_get("", "product_id").map_err(DbError::SeaOrm)?,
            embedding,
        });
    }
    Ok(out)
}

/// Upsert one product embedding with its source-text fingerprint (staleness
/// guard: unchanged products are skipped by the refresh job).
pub async fn upsert(
    db: &impl ConnectionTrait,
    product_id: i32,
    embedding: &[f32],
    source: &str,
) -> Result<()> {
    // REAL[] literal: f32 Debug prints shortest round-trip, so the cast is
    // exact. Parameterizing arrays through sea-query is version-fragile;
    // a literal keeps this driver-proof.
    let lit = embedding
        .iter()
        .map(|x| format!("{x:?}"))
        .collect::<Vec<_>>()
        .join(",");
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        format!(
            "INSERT INTO rustygod_product_embedding (product_id, embedding, dim, source, updated_at) \
             VALUES ($1, ARRAY[{lit}]::real[], $2, $3, now()) \
             ON CONFLICT (product_id) DO UPDATE SET embedding = EXCLUDED.embedding, \
               dim = EXCLUDED.dim, source = EXCLUDED.source, updated_at = now()"
        ),
        [product_id.into(), (embedding.len() as i16).into(), source.into()],
    ))
    .await?;
    Ok(())
}

/// Source fingerprints of everything stored (for the skip-unchanged fast path).
pub async fn stored_sources(db: &impl ConnectionTrait) -> Result<std::collections::HashMap<i32, String>> {
    let rows = db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT product_id, source FROM rustygod_product_embedding".to_string(),
        ))
        .await?;
    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        map.insert(
            r.try_get::<i32>("", "product_id").map_err(DbError::SeaOrm)?,
            r.try_get::<String>("", "source").map_err(DbError::SeaOrm)?,
        );
    }
    Ok(map)
}
