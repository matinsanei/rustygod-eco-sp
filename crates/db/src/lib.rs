//! `rustygod-db`: persistence layer over the **real Saleor PostgreSQL schema**.
//!
//! Entities are generated 1:1 from the live Django-managed database
//! (`sea-orm-cli generate entity`), so Django and Rust can share one database
//! under the Expand-Contract pattern: same tables, same constraints, zero data migration.

pub mod apps;
pub mod auth;
pub mod catalog;
pub mod checkout_store;
pub mod commerce;
pub mod drafts;
pub mod entities;
pub mod fulfillment;
pub mod giftcards;
pub mod invoices;
pub mod order_store;
pub mod payments;
pub mod promotions;
pub mod relations;
pub mod webhooks;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    SeaOrm(#[from] sea_orm::DbErr),
    #[error("checkout {0} not found")]
    CheckoutNotFound(String),
    #[error("gift card {0} not found")]
    GiftCardNotFound(String),
    #[error("gift card not applicable: {0}")]
    GiftCardNotApplicable(String),
    #[error("gift card conflict: {0}")]
    GiftCardConflict(String),
    #[error("draft order error: {0}")]
    Draft(String),
    #[error("invoice error: {0}")]
    Invoice(String),
    #[error("app error: {0}")]
    App(String),
    #[error("lock poisoned: {0}")]
    Lock(String),
}

impl From<sea_orm::TransactionError<sea_orm::DbErr>> for DbError {
    fn from(e: sea_orm::TransactionError<sea_orm::DbErr>) -> Self {
        match e {
            sea_orm::TransactionError::Connection(e)
            | sea_orm::TransactionError::Transaction(e) => Self::SeaOrm(e),
        }
    }
}

pub type Result<T> = std::result::Result<T, DbError>;

/// Connect to the Saleor PostgreSQL. Defaults to the local dev database
/// (Saleor `populatedb` data). The Rust server and Django read the same rows.
pub async fn connect(database_url: &str) -> Result<DatabaseConnection> {
    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(64).min_connections(4);
    Ok(Database::connect(opt).await?)
}

pub fn database_url() -> String {
    std::env::var("RUSTYGOD_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://saleor:saleor@localhost:5434/saleor".to_string()
    })
}
