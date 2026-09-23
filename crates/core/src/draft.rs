//! Draft-order domain logic, mirroring
//! `saleor/graphql/order/mutations/draft_order_{create,complete}.py`.
//!
//! Pure functions only — persistence lives in `saleor_rustify_db::drafts`.

use rust_decimal::Decimal;

use crate::DomainError;

/// Status a draft transitions to on completion.
/// Mirrors `DraftOrderComplete`: `UNFULFILLED` when the channel auto-confirms,
/// `UNCONFIRMED` otherwise.
pub fn complete_status(automatically_confirm_all_new_orders: bool) -> &'static str {
    if automatically_confirm_all_new_orders {
        "unfulfilled"
    } else {
        "unconfirmed"
    }
}

/// Guard: only `draft` orders are editable/completable/deletable.
/// Mirrors `DraftOrderComplete.validate_order` ("The order is not draft.").
pub fn require_draft(status: &str) -> crate::Result<()> {
    if status != "draft" {
        return Err(DomainError::Validation {
            field: "id".into(),
            message: "The order is not draft.".into(),
        });
    }
    Ok(())
}

/// Pre-tax line total (net == gross until tax plugins run).
pub fn line_total(unit_price: Decimal, quantity: i32) -> Decimal {
    unit_price * Decimal::from(quantity.max(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn complete_status_follows_channel_flag() {
        assert_eq!(complete_status(true), "unfulfilled");
        assert_eq!(complete_status(false), "unconfirmed");
    }

    #[test]
    fn require_draft_rejects_everything_else() {
        assert!(require_draft("draft").is_ok());
        for s in ["unfulfilled", "unconfirmed", "fulfilled", "canceled", "expired"] {
            let err = require_draft(s).unwrap_err().to_string();
            assert!(err.contains("not draft"), "{s}: {err}");
        }
    }

    #[test]
    fn line_total_clamps_negative_qty() {
        assert_eq!(line_total(dec!(10.5), 3), dec!(31.5));
        assert_eq!(line_total(dec!(10.5), -2), dec!(0));
    }
}
