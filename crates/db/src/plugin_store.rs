//! Plugin registry persistence in our own `rustygod_plugin` table.
//!
//! Expand-Contract note: the table is `rustygod_*`-prefixed and unknown to
//! Django, which ignores tables it doesn't manage — sharing the database
//! stays zero-migration. Raw SQL (no generated entity) keeps it that way.

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde_json::json;

use crate::{DbError, Result};

pub struct StoredPlugin {
    pub name: String,
    pub version: String,
    pub extension_points: Vec<String>,
    pub capabilities: Vec<String>,
    pub config: serde_json::Value,
    pub wasm: Vec<u8>,
}

/// Idempotent schema setup, run at service boot.
pub async fn ensure_table(db: &DatabaseConnection) -> Result<()> {
    db.execute(Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        r#"CREATE TABLE IF NOT EXISTS rustygod_plugin (
            name TEXT PRIMARY KEY,
            version TEXT NOT NULL DEFAULT '',
            extension_points JSONB NOT NULL DEFAULT '[]',
            capabilities JSONB NOT NULL DEFAULT '[]',
            config JSONB NOT NULL DEFAULT '{}',
            wasm BYTEA NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )"#
        .to_string(),
    ))
    .await?;
    Ok(())
}

pub async fn save_plugin(db: &DatabaseConnection, p: &StoredPlugin) -> Result<()> {
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"INSERT INTO rustygod_plugin
            (name, version, extension_points, capabilities, config, wasm, updated_at)
            VALUES ($1, $2, $3::jsonb, $4::jsonb, $5::jsonb, $6, now())
            ON CONFLICT (name) DO UPDATE SET
              version = EXCLUDED.version,
              extension_points = EXCLUDED.extension_points,
              capabilities = EXCLUDED.capabilities,
              config = EXCLUDED.config,
              wasm = EXCLUDED.wasm,
              updated_at = now()"#,
        [
            p.name.clone().into(),
            p.version.clone().into(),
            json!(p.extension_points).to_string().into(),
            json!(p.capabilities).to_string().into(),
            p.config.to_string().into(),
            p.wasm.clone().into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn load_all(db: &DatabaseConnection) -> Result<Vec<StoredPlugin>> {
    let rows = db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT name, version, extension_points::text, capabilities::text, \
             config::text, wasm FROM rustygod_plugin ORDER BY name"
                .to_string(),
        ))
        .await
        .map_err(DbError::SeaOrm)?;
    let mut out = Vec::new();
    for r in rows {
        let get = |k: &str| -> Result<serde_json::Value> {
            let s: String = r.try_get("", k).map_err(DbError::SeaOrm)?;
            serde_json::from_str(&s).map_err(|e| DbError::PluginStore(e.to_string()))
        };
        let str_list = |v: serde_json::Value| {
            v.as_array()
                .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default()
        };
        out.push(StoredPlugin {
            name: r.try_get("", "name").map_err(DbError::SeaOrm)?,
            version: r.try_get("", "version").map_err(DbError::SeaOrm)?,
            extension_points: str_list(get("extension_points")?),
            capabilities: str_list(get("capabilities")?),
            config: get("config")?,
            wasm: r.try_get("", "wasm").map_err(DbError::SeaOrm)?,
        });
    }
    Ok(out)
}

pub async fn delete_plugin(db: &DatabaseConnection, name: &str) -> Result<bool> {
    let res = db
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM rustygod_plugin WHERE name = $1".to_string(),
            [name.to_string().into()],
        ))
        .await?;
    Ok(res.rows_affected() > 0)
}
