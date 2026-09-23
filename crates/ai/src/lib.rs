//! `rustygod-ai`: native AI capabilities over Saleor data.
//!
//! Backend tiers:
//! - **Available now, no downloads**: trigram lexical search (pg_trgm, using
//!   Saleor's own gin indexes), co-occurrence recommendations from Django's
//!   order lines, deterministic local `Embedder`/`VectorStore`.
//! - **Milestone**: Candle/ONNX embedders and Qdrant/pgvector stores implement
//!   the same `Embedder`/`VectorStore` traits — drop-in, no API change.

pub mod agent;
pub mod chat;
pub mod embed;
pub mod recommend;
pub mod search;
pub mod traits;
pub mod vectors;

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("store error: {0}")]
    Store(#[from] rustygod_db::DbError),
    #[error("ai error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AiError>;
