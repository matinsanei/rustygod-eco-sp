//! Flat-rate tax math, mirroring
//! `saleor/tax/calculations/__init__.py::calculate_flat_rate_tax`.
//!
//! Rates are **percentages** (23 = 23%). Quantization matches Django's
//! `quantize_price`: banker's rounding (HALF_EVEN) to currency precision —
//! note this differs from the promotions HALF_UP, and the difference is
//! intentional parity, not a bug.

use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy::MidpointNearestEven;

/// Split a unit amount into (net, gross) under a flat rate.
///
/// - `prices_entered_with_tax = true`: the amount IS gross, net derives.
/// - `false`: the amount IS net, gross derives.
/// Both sides quantized HALF_EVEN to 2dp (Django's `quantize_price` on a
/// 2-decimal currency; minor-unit-exact inputs stay exact).
pub fn flat_rate_tax(
    amount: Decimal,
    rate_percent: Decimal,
    prices_entered_with_tax: bool,
) -> (Decimal, Decimal) {
    let multiplier = Decimal::ONE + rate_percent / Decimal::from(100);
    let (net, gross) = if prices_entered_with_tax {
        (amount / multiplier, amount)
    } else {
        (amount, amount * multiplier)
    };
    (quantize(net), quantize(gross))
}

fn quantize(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(2, MidpointNearestEven)
}

/// Tax amount embedded in a gross/net pair.
pub fn tax_amount(net: Decimal, gross: Decimal) -> Decimal {
    quantize(gross - net)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn net_entered_gross_derives() {
        // 100.00 net @ 23% -> 123.00 gross.
        let (net, gross) = flat_rate_tax(dec!(100), dec!(23), false);
        assert_eq!((net, gross), (dec!(100), dec!(123)));
    }

    #[test]
    fn gross_entered_net_derives() {
        // 123.00 gross @ 23% -> 100.00 net.
        let (net, gross) = flat_rate_tax(dec!(123), dec!(23), true);
        assert_eq!((net, gross), (dec!(100), dec!(123)));
    }

    #[test]
    fn zero_rate_is_identity() {
        let (net, gross) = flat_rate_tax(dec!(19.99), dec!(0), false);
        assert_eq!((net, gross), (dec!(19.99), dec!(19.99)));
    }

    #[test]
    fn bankers_rounding_matches_django_quantize() {
        // 10.005 -> HALF_EVEN gives 10.00 (HALF_UP would give 10.01).
        // Django's quantize_price uses the default (HALF_EVEN) context.
        assert_eq!(quantize(dec!(10.005)), dec!(10.00));
        assert_eq!(quantize(dec!(10.015)), dec!(10.02));
    }

    #[test]
    fn standard_vat_arithmetic() {
        // 40.00 @ 8.25% (US-style): gross 43.30.
        let (net, gross) = flat_rate_tax(dec!(40), dec!(8.25), false);
        assert_eq!(net, dec!(40));
        assert_eq!(gross, dec!(43.30));
        assert_eq!(tax_amount(net, gross), dec!(3.30));
    }
}
