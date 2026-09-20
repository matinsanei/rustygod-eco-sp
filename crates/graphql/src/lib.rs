//! GraphQL gateway (thin BFF): Saleor-compatible `/graphql/` on top of
//! the Rust core (`rustygod-db` + `rustygod-core`).
//!
//! DESIGN (enterprise, strangler-compatible):
//! - gRPC is the source of truth internally (storefront/TS SDK). GraphQL is
//!   a TRANSLATION layer for the existing Dashboard (Apollo) — same Postgres,
//!   same domain logic, no duplicated money/stock math.
//! - Every resolver is a 5-line call into `db::*` / `core::*` (the same
//!   functions gRPC uses). `db` owns transactions/locking/outbox; GQL owns
//!   only shape + auth extraction from `Authorization: Bearer`.
//! - Auth = `server::access::authorize` (staff JWT `RSA_*` or app token
//!   `manage_*`). Mutations check; queries pass through unless the field
//!   is staff-only (matching Saleor's per-type permissions).
//! - Pagination = Relay `PageInfo` + cursor = base64 index (v1, stable
//!   ordering `created_at`). Filtering mirrors the gRPC services we already
//!   ship; exotic Saleor filters (GraphQL `filter: {}` JSON) are a
//!   documented gap, not a silent lie.
//! - Sections land incrementally (catalog → checkout → order → payment →
//!   commerce). Each section is one file; shared scalars/types live here.

pub mod catalog;
pub mod checkout;
pub mod commerce;
pub mod common;
pub mod context;
pub mod order;
pub mod payment;
pub mod schema;

pub use schema::{build_schema, AppSchema};
