//! Promotion/voucher money math + catalogue predicate evaluation.
//!
//! Exact mirrors of Saleor (`saleor/discount/utils/promotion.py`,
//! `saleor/discount/models.py`, `prices/discount.py`, `prices/money.py`):
//! - fixed: `max(price - value, 0)`
//! - percentage: `discount = round_half_up(price * pct/100, currency_precision)`;
//!   result floored at zero — same as `fractional_discount` + `fixed_discount`.
//! - best rule wins per variant (max saving); catalogue predicates gate
//!   applicability with AND/OR nesting and base64 global ids.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// ISO currency precision, mirroring `prices`/`babel` data
/// (`get_currency_precision`): most currencies 2, a known zero-decimal set 0,
/// a known 3-decimal set 3.
pub fn currency_precision(code: &str) -> u32 {
    match code {
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
        "BIF" | "BYR" | "CLF" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW"
        | "PYG" | "RWF" | "UGX" | "UYI" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        _ => 2,
    }
}

fn quantize_half_up(amount: Decimal, precision: u32) -> Decimal {
    use rust_decimal::RoundingStrategy::MidpointAwayFromZero;
    amount.round_dp_with_strategy(precision, MidpointAwayFromZero)
}

/// `fixed_discount`: `max(price - value, 0)`.
pub fn apply_fixed(price: Decimal, value: Decimal) -> Decimal {
    (price - value).max(Decimal::ZERO)
}

/// `percentage_discount` with `ROUND_HALF_UP` at currency precision.
pub fn apply_percentage(price: Decimal, percentage: Decimal, currency: &str) -> Decimal {
    let fraction = percentage / Decimal::from(100);
    let discount = quantize_half_up(price * fraction, currency_precision(currency));
    apply_fixed(price, discount)
}

/// Discount saved by one rule on one unit.
pub fn saving_for_rule(
    price: Decimal,
    value_type: &str,
    value: Decimal,
    currency: &str,
) -> Decimal {
    let discounted = match value_type {
        "fixed" => apply_fixed(price, value),
        "percentage" => apply_percentage(price, value, currency),
        _ => price,
    };
    (price - discounted).max(Decimal::ZERO)
}

/// `get_best_promotion_discount`: the rule with maximum saving.
/// Returns `(rule_index, discounted_price)`.
pub fn best_rule(
    price: Decimal,
    rules: &[(String, String, Decimal)],
    currency: &str,
) -> Option<(usize, Decimal)> {
    rules
        .iter()
        .enumerate()
        .map(|(i, (_, vt, v))| {
            let saving = saving_for_rule(price, vt, *v, currency);
            (i, price - saving)
        })
        .max_by(|a, b| {
            // Compare savings = price - discounted.
            (price - a.1).cmp(&(price - b.1))
        })
}

// ---------- Catalogue predicates ----------

/// Decoded global id: ("Product", 135).
fn decode_global_id(gid: &str) -> Option<(String, i32)> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(gid).ok()?;
    let s = String::from_utf8(bytes).ok()?;
    let (kind, id) = s.split_once(':')?;
    Some((kind.to_string(), id.parse::<i32>().ok()?))
}

/// Leaf predicate after JSON parse: which object ids match.
#[derive(Debug, Default, Clone)]
pub struct LeafMatch {
    pub products: Vec<i32>,
    pub variants: Vec<i32>,
    pub categories: Vec<i32>,
    pub collections: Vec<i32>,
}

/// Evaluate AND/OR predicate tree for one variant. Semantics mirror
/// `filter_qs_by_predicate`: OR branches union, AND branches intersect,
/// leaves match by membership. Empty predicate matches nothing.
pub fn predicate_matches(
    predicate: &serde_json::Value,
    variant_id: i32,
    product_id: i32,
    category_id: Option<i32>,
    collection_ids: &[i32],
) -> bool {
    eval_node(predicate, variant_id, product_id, category_id, collection_ids).unwrap_or(false)
}

fn eval_node(
    node: &serde_json::Value,
    variant_id: i32,
    product_id: i32,
    category_id: Option<i32>,
    collection_ids: &[i32],
) -> Option<bool> {
    let obj = node.as_object()?;
    let mut acc: Option<bool> = None;

    if let Some(or) = obj.get("OR").and_then(|v| v.as_array()) {
        let mut any = false;
        let mut seen = false;
        for child in or {
            if let Some(b) = eval_node(child, variant_id, product_id, category_id, collection_ids) {
                seen = true;
                any = any || b;
            }
        }
        if seen {
            acc = Some(acc.unwrap_or(false) || any);
        }
    }
    if let Some(and) = obj.get("AND").and_then(|v| v.as_array()) {
        let mut all = true;
        let mut seen = false;
        for child in and {
            if let Some(b) = eval_node(child, variant_id, product_id, category_id, collection_ids) {
                seen = true;
                all = all && b;
            }
        }
        if seen {
            acc = Some(acc.unwrap_or(true) && all);
        }
    }

    let mut leaf = LeafMatch::default();
    collect_leaf_only(node, &mut leaf);
    let leaf_hit = !leaf.products.is_empty()
        && leaf.products.contains(&product_id)
        || !leaf.variants.is_empty() && leaf.variants.contains(&variant_id)
        || !leaf.categories.is_empty()
            && category_id.map(|c| leaf.categories.contains(&c)).unwrap_or(false)
        || !leaf.collections.is_empty()
            && leaf.collections.iter().any(|c| collection_ids.contains(c));
    let leaf_present = !leaf.products.is_empty()
        || !leaf.variants.is_empty()
        || !leaf.categories.is_empty()
        || !leaf.collections.is_empty();
    if leaf_present {
        acc = Some(acc.unwrap_or(false) || leaf_hit);
    }
    acc
}

fn collect_leaf_only(node: &serde_json::Value, out: &mut LeafMatch) {
    let Some(obj) = node.as_object() else { return };
    for (key, target) in [
        ("productPredicate", &mut out.products),
        ("variantPredicate", &mut out.variants),
        ("categoryPredicate", &mut out.categories),
        ("collectionPredicate", &mut out.collections),
    ] {
        if let Some(pred) = obj.get(key) {
            if let Some(ids) = pred.get("ids").and_then(|v| v.as_array()) {
                for gid in ids.iter().filter_map(|v| v.as_str()) {
                    if let Some((_, id)) = decode_global_id(gid) {
                        target.push(id);
                    }
                }
            }
        }
    }
}

/// Variant ids covered by a rule via the explicit M2M
/// (`discount_promotionrule_variants`) plus predicate match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCandidate {
    pub rule_id: String,
    pub reward_value_type: String,
    pub reward_value: Decimal,
    pub promotion_name: String,
    pub rule_name: String,
    pub promotion_end: Option<String>,
}
