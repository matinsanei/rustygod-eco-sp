//! Domain layer of rustygod-saleor.
//! Mirrors Saleor's Django models (saleor-core/saleor/...) as plain Rust structs.
//! gRPC/transport concerns live in `rustygod-proto` + `rustygod-server`.

pub mod auth;
pub mod checkout;
pub mod discount;
pub mod money;
pub mod order;
pub mod payments;
pub mod product;
pub mod webhooks;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation failed on {field}: {message}")]
    Validation { field: String, message: String },
    #[error("out of stock: {0}")]
    OutOfStock(String),
}

pub type Result<T> = std::result::Result<T, DomainError>;
