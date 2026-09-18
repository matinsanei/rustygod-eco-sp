//! Promotion money math + predicate evaluation, mirroring
//! `saleor/discount/utils/promotion.py`, `PromotionRule.get_discount` and
//! `prices/discount.py` + `prices/money.py` quantization.

use rust_decimal::Decimal;
use rustygod_core::discount::*;
use serde_json::json;

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

#[test]
fn fixed_discount_floors_at_zero() {
    assert_eq!(apply_fixed(dec("40.00"), dec("12.00")), dec("28.00"));
    assert_eq!(apply_fixed(dec("10.00"), dec("25.00")), Decimal::ZERO);
}

#[test]
fn percentage_uses_half_up_at_currency_precision() {
    // 30% of 40.00 = 12.00 exactly.
    assert_eq!(apply_percentage(dec("40.00"), dec("30"), "USD"), dec("28.00"));
    // 12.5% of 10.00 = 1.25 -> 8.75.
    assert_eq!(apply_percentage(dec("10.00"), dec("12.5"), "USD"), dec("8.75"));
    // HALF_UP (not banker's): 10% of 10.05 = 1.005 -> 1.01 -> 9.04.
    assert_eq!(apply_percentage(dec("10.05"), dec("10"), "USD"), dec("9.04"));
    // Zero-decimal currency: 10% of 1000 JPY = 100 -> 900.
    assert_eq!(apply_percentage(dec("1000"), dec("10"), "JPY"), dec("900"));
    // 3-decimal currency keeps 3 places.
    assert_eq!(apply_percentage(dec("10.000"), dec("10"), "BHD"), dec("9.000"));
}

#[test]
fn best_rule_picks_max_saving() {
    let rules = vec![
        ("r1".to_string(), "percentage".to_string(), dec("30")),
        ("r2".to_string(), "fixed".to_string(), dec("5")),
    ];
    // 30% of 40 = 12 > 5 fixed -> r1 wins, price 28.
    let (idx, price) = best_rule(dec("40.00"), &rules, "USD").unwrap();
    assert_eq!(idx, 0);
    assert_eq!(price, dec("28.00"));
    // On a 10.00 price: 30% = 3 < 5 fixed -> r2 wins.
    let (idx, price) = best_rule(dec("10.00"), &rules, "USD").unwrap();
    assert_eq!(idx, 1);
    assert_eq!(price, dec("5.00"));
}

#[test]
fn predicate_matches_products_and_variants() {
    // GIDs are base64 "Product:135" / "ProductVariant:349" like Django writes.
    use base64::Engine;
    let gid = |s: &str| base64::engine::general_purpose::STANDARD.encode(s);
    let pred = json!({
        "productPredicate": { "ids": [gid("Product:135")] },
        "variantPredicate": { "ids": [gid("ProductVariant:349")] },
    });
    assert!(predicate_matches(&pred, 1, 135, None, &[]));
    assert!(predicate_matches(&pred, 349, 999, None, &[]));
    assert!(!predicate_matches(&pred, 1, 999, None, &[]));
}

#[test]
fn predicate_and_or_nesting() {
    use base64::Engine;
    let gid = |s: &str| base64::engine::general_purpose::STANDARD.encode(s);
    let pred = json!({
        "AND": [
            { "productPredicate": { "ids": [gid("Product:1")] } },
            { "OR": [
                { "variantPredicate": { "ids": [gid("ProductVariant:9")] } },
                { "categoryPredicate": { "ids": [gid("Category:3")] } },
            ]},
        ],
    });
    assert!(predicate_matches(&pred, 9, 1, Some(7), &[]));
    assert!(predicate_matches(&pred, 2, 1, Some(3), &[]));
    assert!(!predicate_matches(&pred, 2, 1, Some(7), &[]));
    assert!(!predicate_matches(&pred, 9, 2, Some(7), &[]));
}

#[test]
fn empty_predicate_matches_nothing() {
    assert!(!predicate_matches(&json!({}), 1, 1, None, &[]));
    assert!(!predicate_matches(&json!(null), 1, 1, None, &[]));
}

#[test]
fn currency_precision_table() {
    assert_eq!(currency_precision("USD"), 2);
    assert_eq!(currency_precision("JPY"), 0);
    assert_eq!(currency_precision("BHD"), 3);
    assert_eq!(currency_precision("XXX"), 2);
}
