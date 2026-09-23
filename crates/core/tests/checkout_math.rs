//! Behavioral contract ported from
//! `saleor-core/saleor/checkout/tests/test_base_calculations.py`.
//!
//! Saleor rule under contract:
//! - a checkout line's unit price is the variant's channel price,
//!   unless the line carries an explicit `price_override`;
//! - the checkout total is the exact decimal sum of line totals;
//! - money math is decimal, never float.

use rust_decimal::Decimal;
use saleor_rustify_core::{checkout::Checkout, money::Money};

fn usd(amount: &str) -> Money {
    Money::new(amount.parse::<Decimal>().unwrap(), "USD")
}

#[test]
fn line_unit_price_is_variant_price() {
    // Mirrors test_calculate_base_line_unit_price.
    let mut co = Checkout::new("default-channel", "a@test.example", "USD");
    co.add_line("var_1".into(), 1, usd("29.99")).unwrap();

    assert_eq!(co.lines[0].unit_price, usd("29.99"));
    assert_eq!(co.total(), usd("29.99"));
}

#[test]
fn price_override_wins_over_variant_price() {
    // Mirrors test_calculate_base_line_unit_price_with_custom_price.
    let mut co = Checkout::new("default-channel", "a@test.example", "USD");
    co.add_line("var_1".into(), 1, usd("12.22")).unwrap();

    assert_eq!(co.lines[0].unit_price, usd("12.22"));
    assert_eq!(co.total(), usd("12.22"));
}

#[test]
fn total_is_exact_decimal_sum() {
    // 2 x 29.99 + 1 x 14.50 must be exactly 74.48 — float would give 74.47999...
    let mut co = Checkout::new("default-channel", "a@test.example", "USD");
    co.add_line("var_1".into(), 2, usd("29.99")).unwrap();
    co.add_line("var_2".into(), 1, usd("14.50")).unwrap();

    assert_eq!(co.total().amount.to_string(), "74.48");
}

#[test]
fn same_variant_accumulates_quantity() {
    let mut co = Checkout::new("default-channel", "a@test.example", "USD");
    co.add_line("var_1".into(), 1, usd("10.00")).unwrap();
    co.add_line("var_1".into(), 2, usd("10.00")).unwrap();

    assert_eq!(co.lines.len(), 1);
    assert_eq!(co.lines[0].quantity, 3);
    assert_eq!(co.total(), usd("30.00"));
}

#[test]
fn currency_mismatch_is_rejected() {
    let mut co = Checkout::new("default-channel", "a@test.example", "USD");
    let err = co.add_line("var_1".into(), 1, usd("10.00")).is_ok();
    assert!(err);
    let pln = Money::new(Decimal::new(1000, 2), "PLN");
    assert!(co.add_line("var_1".into(), 1, pln).is_err());
}

#[test]
fn non_positive_quantity_is_rejected() {
    let mut co = Checkout::new("default-channel", "a@test.example", "USD");
    assert!(co.add_line("var_1".into(), 0, usd("10.00")).is_err());
    assert!(co.add_line("var_1".into(), -2, usd("10.00")).is_err());
    assert!(co.lines.is_empty());
}

#[test]
fn empty_checkout_total_is_zero() {
    let co = Checkout::new("default-channel", "a@test.example", "USD");
    assert_eq!(co.total(), Money::zero("USD"));
}
