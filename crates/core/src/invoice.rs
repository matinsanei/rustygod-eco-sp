//! Invoice domain logic, mirroring `saleor/invoice/` and
//! `saleor/graphql/invoice/mutations/`.
//!
//! Pure functions only — persistence lives in `saleor_rustify_db::invoices`.
//! v1 note: Django renders invoices through async plugins; we fulfill
//! synchronously with the same `pending → success/failed/deleted` job
//! state machine and the same event vocabulary.

use crate::DomainError;

/// Job statuses — mirrors `JobStatus` in `saleor/core/__init__.py`.
pub mod status {
    pub const PENDING: &str = "pending";
    pub const SUCCESS: &str = "success";
    pub const FAILED: &str = "failed";
    pub const DELETED: &str = "deleted";
}

/// Event types — mirrors `InvoiceEvents` in `saleor/invoice/__init__.py`.
pub mod events {
    pub const REQUESTED: &str = "requested";
    pub const REQUESTED_DELETION: &str = "requested_deletion";
    pub const CREATED: &str = "created";
    pub const DELETED: &str = "deleted";
    pub const SENT: &str = "sent";
}

/// Ready invoices are the successful ones (`InvoiceQueryset.ready`).
pub fn is_ready(job_status: &str) -> bool {
    job_status == status::SUCCESS
}

/// Guard for `InvoiceRequest.clean_order`: no invoices for draft,
/// unconfirmed or expired orders.
pub fn require_requestable(order_status: &str) -> crate::Result<()> {
    match order_status {
        "draft" | "unconfirmed" | "expired" => Err(DomainError::Validation {
            field: "orderId".into(),
            message: "Cannot request an invoice for draft, unconfirmed or expired order.".into(),
        }),
        _ => Ok(()),
    }
}

/// Guard: only successful (ready) invoices can be sent to the customer.
pub fn require_sendable(job_status: &str) -> crate::Result<()> {
    if is_ready(job_status) {
        Ok(())
    } else {
        Err(DomainError::Validation {
            field: "id".into(),
            message: "Only ready invoices can be sent.".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_means_success_only() {
        assert!(is_ready(status::SUCCESS));
        for s in [status::PENDING, status::FAILED, status::DELETED, "bogus"] {
            assert!(!is_ready(s), "{s}");
        }
    }

    #[test]
    fn request_guard_matches_django_clean_order() {
        for s in ["draft", "unconfirmed", "expired"] {
            let err = require_requestable(s).unwrap_err().to_string();
            assert!(err.contains("draft, unconfirmed or expired"), "{s}: {err}");
        }
        for s in ["unfulfilled", "fulfilled", "partially fulfilled", "canceled", "returned"] {
            assert!(require_requestable(s).is_ok(), "{s}");
        }
    }

    #[test]
    fn send_guard_requires_success() {
        assert!(require_sendable(status::SUCCESS).is_ok());
        assert!(require_sendable(status::PENDING).is_err());
        assert!(require_sendable(status::FAILED).is_err());
    }
}
