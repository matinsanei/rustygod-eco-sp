//! Gift-card domain logic, mirroring `saleor/giftcard/`.
//!
//! Pure functions only — persistence lives in `rustygod_db::giftcards`.
//! Money stays `Decimal`-exact; the wire format (string) is a server concern.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Code shape produced by [`generate_code`]: `XXXX-XXXX-XXXX`, uppercase hex.
pub const CODE_LEN: usize = 14;

/// Generate a fresh gift-card code (`XXXX-XXXX-XXXX`, uppercase hex).
///
/// Mirrors `generate_random_code` in `saleor/core/utils/promo_code.py`.
/// Uniqueness against the DB is the caller's job (retry on conflict).
pub fn generate_code() -> String {
    let bytes: [u8; 6] = rand::random();
    let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
    format!("{}-{}-{}", &hex[0..4], &hex[4..8], &hex[8..12])
}

/// Check the `XXXX-XXXX-XXXX` uppercase-hex shape (Django: min length 8,
/// unique; the dashed shape is what `generate_random_code` produces).
pub fn is_valid_code_shape(code: &str) -> bool {
    if code.len() != CODE_LEN {
        return false;
    }
    let mut chars = code.chars();
    for i in 0..CODE_LEN {
        let c = chars.next().unwrap_or(' ');
        if i == 4 || i == 9 {
            if c != '-' {
                return false;
            }
        } else if !c.is_ascii_hexdigit() || c.is_ascii_lowercase() {
            return false;
        }
    }
    true
}

/// Whether the card passes the `active(date)` manager filter:
/// `is_active` and (`expiry_date` null or `>= today`).
///
/// Mirrors `GiftCardQueryset.active` in `saleor/giftcard/models.py`.
pub fn is_active_card(is_active: bool, expiry_date: Option<NaiveDate>, today: NaiveDate) -> bool {
    if !is_active {
        return false;
    }
    match expiry_date {
        None => true,
        Some(d) => d >= today,
    }
}

/// Why a customer-restricted card cannot be used by `user_id`, if at all.
///
/// Mirrors `GiftCard.usage_restriction_reason`.
pub fn restriction_reason(
    assigned_to_id: Option<i32>,
    assigned_to_email: Option<&str>,
    user_id: Option<i32>,
) -> Option<&'static str> {
    if assigned_to_email.is_some() && assigned_to_id.is_none() {
        return Some("restricted to a deleted customer");
    }
    if let Some(owner) = assigned_to_id {
        if Some(owner) != user_id {
            return Some("restricted to another customer");
        }
    }
    None
}

/// Amount a redemption actually takes: `min(balance, requested)`, never negative.
pub fn redeem_amount(current_balance: Decimal, requested: Decimal) -> Decimal {
    current_balance.max(dec!(0)).min(requested.max(dec!(0)))
}

/// Apply gift-card balances to a checkout/order gross total.
///
/// Greedy in card order (matches the DB ordering `code`), total floored at
/// zero — mirrors `calculate_checkout_total_with_gift_cards` which subtracts
/// the summed active balances and clamps with `max(total, zero)`.
///
/// Returns `(covered, remaining_total)`.
pub fn cover_total(total_gross: Decimal, balances: &[Decimal]) -> (Decimal, Decimal) {
    let mut remaining = total_gross.max(dec!(0));
    let mut covered = dec!(0);
    for b in balances {
        if remaining <= dec!(0) {
            break;
        }
        let take = (*b).max(dec!(0)).min(remaining);
        covered += take;
        remaining -= take;
    }
    (covered, remaining)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn generated_code_has_dashed_shape() {
        for _ in 0..100 {
            let code = generate_code();
            assert!(is_valid_code_shape(&code), "{code}");
        }
    }

    #[test]
    fn generated_codes_are_unique_in_practice() {
        let codes: std::collections::HashSet<_> =
            (0..1000).map(|_| generate_code()).collect();
        assert_eq!(codes.len(), 1000);
    }

    #[test]
    fn code_shape_rejects_garbage() {
        assert!(!is_valid_code_shape("short"));
        assert!(!is_valid_code_shape("abcd-efgh-ijkl")); // lowercase
        assert!(!is_valid_code_shape("ABCD_EFGH_IJKL")); // wrong sep
        assert!(is_valid_code_shape("A1B2-C3D4-E5F6"));
    }

    #[test]
    fn active_filter_matches_django_manager() {
        let today = day(2026, 9, 18);
        assert!(is_active_card(true, None, today));
        assert!(is_active_card(true, Some(day(2026, 9, 18)), today));
        assert!(is_active_card(true, Some(day(2026, 12, 1)), today));
        assert!(!is_active_card(true, Some(day(2026, 9, 17)), today));
        assert!(!is_active_card(false, None, today));
    }

    #[test]
    fn restriction_reason_matches_django() {
        assert_eq!(restriction_reason(None, None, None), None);
        assert_eq!(restriction_reason(Some(7), None, Some(7)), None);
        assert_eq!(
            restriction_reason(Some(7), None, Some(8)),
            Some("restricted to another customer")
        );
        assert_eq!(
            restriction_reason(Some(7), None, None),
            Some("restricted to another customer")
        );
        assert_eq!(
            restriction_reason(None, Some("gone@x.io"), None),
            Some("restricted to a deleted customer")
        );
    }

    #[test]
    fn redeem_clamps_to_balance_and_zero() {
        assert_eq!(redeem_amount(dec!(50), dec!(30)), dec!(30));
        assert_eq!(redeem_amount(dec!(20), dec!(30)), dec!(20));
        assert_eq!(redeem_amount(dec!(0), dec!(30)), dec!(0));
        assert_eq!(redeem_amount(dec!(-5), dec!(30)), dec!(0));
        assert_eq!(redeem_amount(dec!(50), dec!(-1)), dec!(0));
    }

    #[test]
    fn cover_total_greedy_and_floored() {
        let (c, r) = cover_total(dec!(100), &[dec!(60), dec!(50)]);
        assert_eq!((c, r), (dec!(100), dec!(0)));
        let (c, r) = cover_total(dec!(100), &[dec!(30)]);
        assert_eq!((c, r), (dec!(30), dec!(70)));
        let (c, r) = cover_total(dec!(0), &[dec!(30)]);
        assert_eq!((c, r), (dec!(0), dec!(0)));
        let (c, r) = cover_total(dec!(100), &[]);
        assert_eq!((c, r), (dec!(0), dec!(100)));
    }
}
