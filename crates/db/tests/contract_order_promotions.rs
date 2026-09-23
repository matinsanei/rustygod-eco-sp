//! Order-promotion contract (T3): subtotal discounts (fixed/percentage,
//! capped, predicate-gated), voucher precedence, gift lines competing by
//! money value, stale clearing, and complete-carry (discount row + gift
//! order line with allocation). Mirrors
//! `saleor/discount/utils/promotion.py` selection semantics.

use chrono::Utc;
use rust_decimal::Decimal;
use saleor_rustify_db::{catalog, checkout_store, complete, database_url, order_promotions, order_store};
use sea_orm::DatabaseConnection;
use serde_json::json;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn stock_guard() -> fs2::FileLockGuard {
    fs2::FileLockGuard::new()
}

mod fs2 {
    pub struct FileLockGuard {
        _f: std::fs::File,
    }
    impl FileLockGuard {
        pub fn new() -> Self {
            use fs2::FileExt;
            let f = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open("/tmp/rustify-stock.lock")
                .expect("stock lock");
            f.lock_exclusive().expect("stock lock");
            Self { _f: f }
        }
    }
}

struct Promo {
    promo_id: Uuid,
    rule_id: Uuid,
}

/// Create an order promotion + rule bound to default-channel.
/// `reward`: ("subtotal_discount", Some(("percentage", "10")), predicate)
/// or ("gift", None, predicate); gifts linked separately.
async fn make_promo(
    db: &DatabaseConnection,
    name: &str,
    reward_type: &str,
    vt: Option<&str>,
    value: Option<Decimal>,
    predicate: serde_json::Value,
) -> Promo {
    use saleor_rustify_db::entities::{
        discount_promotion, discount_promotionrule, discount_promotionrule_channels,
    };
    use sea_orm::{ActiveModelTrait, Set};
    let (ch_id, _) = catalog::channel_info(db, "default-channel").await.unwrap();
    let t = Utc::now();
    let promo_id = Uuid::new_v4();
    discount_promotion::ActiveModel {
        id: Set(promo_id),
        private_metadata: Set(json!({})),
        metadata: Set(json!({})),
        name: Set(name.to_string()),
        description: Set(None),
        old_sale_id: Set(None),
        start_date: Set((t - chrono::Duration::days(1)).into()),
        end_date: Set(None),
        created_at: Set(t.into()),
        updated_at: Set(t.into()),
        last_notification_scheduled_at: Set(None),
        r#type: Set("order".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let rule_id = Uuid::new_v4();
    discount_promotionrule::ActiveModel {
        id: Set(rule_id),
        name: Set(Some(format!("{name} rule"))),
        description: Set(None),
        catalogue_predicate: Set(json!({})),
        reward_value_type: Set(vt.map(|s| s.to_string())),
        reward_value: Set(value),
        promotion_id: Set(promo_id),
        old_channel_listing_id: Set(None),
        order_predicate: Set(predicate),
        reward_type: Set(Some(reward_type.to_string())),
        variants_dirty: Set(Some(false)),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    discount_promotionrule_channels::ActiveModel {
        promotionrule_id: Set(rule_id),
        channel_id: Set(ch_id),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    Promo { promo_id, rule_id }
}

async fn link_gift(db: &DatabaseConnection, rule_id: Uuid, variant_id: i32) {
    use saleor_rustify_db::entities::discount_promotionrule_gifts;
    use sea_orm::{ActiveModelTrait, Set};
    discount_promotionrule_gifts::ActiveModel {
        promotionrule_id: Set(rule_id),
        productvariant_id: Set(variant_id),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn drop_promo(db: &DatabaseConnection, p: &Promo) {
    use saleor_rustify_db::entities::{
        discount_checkoutdiscount, discount_orderdiscount, discount_promotion,
        discount_promotionrule, discount_promotionrule_channels, discount_promotionrule_gifts,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    // Discount rows reference the rule (FK): clear them first.
    discount_checkoutdiscount::Entity::delete_many()
        .filter(discount_checkoutdiscount::Column::PromotionRuleId.eq(p.rule_id))
        .exec(db)
        .await
        .unwrap();
    discount_orderdiscount::Entity::delete_many()
        .filter(discount_orderdiscount::Column::PromotionRuleId.eq(p.rule_id))
        .exec(db)
        .await
        .unwrap();
    discount_promotionrule_gifts::Entity::delete_many()
        .filter(discount_promotionrule_gifts::Column::PromotionruleId.eq(p.rule_id))
        .exec(db)
        .await
        .unwrap();
    discount_promotionrule_channels::Entity::delete_many()
        .filter(discount_promotionrule_channels::Column::PromotionruleId.eq(p.rule_id))
        .exec(db)
        .await
        .unwrap();
    discount_promotionrule::Entity::delete_by_id(p.rule_id).exec(db).await.unwrap();
    discount_promotion::Entity::delete_by_id(p.promo_id).exec(db).await.unwrap();
}

fn gte_predicate(amount: &str) -> serde_json::Value {
    json!({"discountedObjectPredicate": {"baseSubtotalPrice": {"range": {"gte": amount}}}})
}

/// Checkout with 2 units of a stocked variant; returns (token, vid, unit).
async fn stocked_checkout(db: &DatabaseConnection, qty: i32) -> (Uuid, i32, Decimal) {
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    let token =
        checkout_store::create_checkout_row(db, ch_id, &currency, "promo@example.com")
            .await
            .unwrap();
    let products = catalog::list_products(db, "default-channel", None, 100).await.unwrap();
    let vid: i32 = products
        .iter()
        .flat_map(|p| &p.variants)
        .find(|v| v.quantity_available >= qty)
        .expect("need a stocked variant")
        .id
        .parse()
        .unwrap();
    let pricing = catalog::checkout_pricing(db, "default-channel", &[vid]).await.unwrap();
    let unit = pricing[&vid].0.amount;
    checkout_store::add_line_row(db, token, vid, qty, unit, &currency, None)
        .await
        .unwrap();
    checkout_store::refresh_totals(db, token).await.unwrap();
    (token, vid, unit)
}

async fn checkout_total(db: &DatabaseConnection, token: Uuid) -> Decimal {
    let (co, _) = checkout_store::load_checkout(db, token).await.unwrap().unwrap();
    co.total_gross_amount
}

async fn promo_rows(db: &DatabaseConnection, token: Uuid) -> usize {
    use saleor_rustify_db::entities::discount_checkoutdiscount;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    discount_checkoutdiscount::Entity::find()
        .filter(discount_checkoutdiscount::Column::CheckoutId.eq(token))
        .filter(discount_checkoutdiscount::Column::Type.eq("order_promotion"))
        .all(db)
        .await
        .unwrap()
        .len()
}

async fn gift_lines(db: &DatabaseConnection, token: Uuid) -> Vec<Uuid> {
    use saleor_rustify_db::entities::checkout_checkoutline;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    checkout_checkoutline::Entity::find()
        .filter(checkout_checkoutline::Column::CheckoutId.eq(token))
        .filter(checkout_checkoutline::Column::IsGift.eq(true))
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|l| l.id)
        .collect()
}

#[tokio::test]
async fn percentage_discount_applies_over_threshold() {
    let _guard = stock_guard();
    let db = db().await;
    let (token, _vid, unit) = stocked_checkout(&db, 2).await;
    let subtotal = unit * dec("2");
    // Threshold at half the actual subtotal (fixture-agnostic).
    let gate = (subtotal / dec("2")).round_dp(2).to_string();
    let p = make_promo(&db, "ten-off", "subtotal_discount", Some("percentage"), Some(dec("10")), gte_predicate(&gate)).await;
    checkout_store::refresh_totals(&db, token).await.unwrap();
    // 10% HALF_UP at USD precision.
    let want_disc = (subtotal * dec("10") / dec("100")).round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
    let out = order_promotions::refresh_order_promotion(&db, token).await.unwrap();
    checkout_store::refresh_totals(&db, token).await.unwrap();
    match out {
        order_promotions::RefreshOutcome::Discount { amount, .. } => assert_eq!(amount, want_disc),
        other => panic!("expected discount, got {other:?}"),
    }
    assert_eq!(promo_rows(&db, token).await, 1);
    assert_eq!(checkout_total(&db, token).await, (subtotal - want_disc).max(dec("0")));
    drop_promo(&db, &p).await;
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn fixed_caps_at_subtotal_and_gates_on_predicate() {
    let _guard = stock_guard();
    let db = db().await;
    let p = make_promo(&db, "huge-fixed", "subtotal_discount", Some("fixed"), Some(dec("1000")), gte_predicate("10")).await;
    let (token, _vid, unit) = stocked_checkout(&db, 1).await;
    let out = order_promotions::refresh_order_promotion(&db, token).await.unwrap();
    checkout_store::refresh_totals(&db, token).await.unwrap();
    match out {
        order_promotions::RefreshOutcome::Discount { amount, .. } => assert_eq!(amount, unit),
        other => panic!("expected capped discount, got {other:?}"),
    }
    assert_eq!(checkout_total(&db, token).await, dec("0"), "floored, never negative");

    // Shrink below the gate: nothing applies, rows cleared.
    let p2 = make_promo(&db, "high-gate", "subtotal_discount", Some("percentage"), Some(dec("50")), gte_predicate("100000")).await;
    drop_promo(&db, &p).await;
    let out = order_promotions::refresh_order_promotion(&db, token).await.unwrap();
    checkout_store::refresh_totals(&db, token).await.unwrap();
    assert!(matches!(out, order_promotions::RefreshOutcome::Cleared));
    assert_eq!(promo_rows(&db, token).await, 0);
    assert_eq!(checkout_total(&db, token).await, unit);
    drop_promo(&db, &p2).await;
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn voucher_suppresses_order_promotion() {
    let _guard = stock_guard();
    let db = db().await;
    let p = make_promo(&db, "suppressed", "subtotal_discount", Some("percentage"), Some(dec("20")), gte_predicate("1")).await;
    let (token, _vid, unit) = stocked_checkout(&db, 1).await;
    let out = order_promotions::refresh_order_promotion(&db, token).await.unwrap();
    checkout_store::refresh_totals(&db, token).await.unwrap();
    assert!(matches!(out, order_promotions::RefreshOutcome::Discount { .. }));
    // Staff applies a voucher code (row edit mirrors apply_voucher's stamp).
    {
        use saleor_rustify_db::entities::checkout_checkout;
        use sea_orm::{ActiveModelTrait, EntityTrait};
        let co = checkout_checkout::Entity::find_by_id(token).one(&db).await.unwrap().unwrap();
        let mut am: checkout_checkout::ActiveModel = co.into();
        am.voucher_code = sea_orm::Set(Some("STAFF10".into()));
        am.update(&db).await.unwrap();
    }
    let out = order_promotions::refresh_order_promotion(&db, token).await.unwrap();
    checkout_store::refresh_totals(&db, token).await.unwrap();
    assert!(matches!(out, order_promotions::RefreshOutcome::Cleared), "voucher wins");
    assert_eq!(promo_rows(&db, token).await, 0);
    drop_promo(&db, &p).await;
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
    let _ = unit;
}

#[tokio::test]
async fn gift_line_created_free_and_competes_by_value() {
    let _guard = stock_guard();
    let db = db().await;
    // Gift worth testing: separate stocked variant as the gift.
    let (token, _vid, unit) = stocked_checkout(&db, 2).await;
    let subtotal = unit * dec("2");
    let products = catalog::list_products(&db, "default-channel", None, 100).await.unwrap();
    let gift_vid: i32 = products
        .iter()
        .flat_map(|p| &p.variants)
        .map(|v| v.id.parse::<i32>().unwrap())
        .find(|id| *id != _vid)
        .expect("need a second variant for the gift");
    let pricing = catalog::checkout_pricing(&db, "default-channel", &[gift_vid]).await.unwrap();
    let gift_price = pricing[&gift_vid].0.amount;
    assert!(gift_price > dec("0"));

    // Case A: lone gift rule → gift wins, checkout total untouched.
    let g = make_promo(&db, "free-gift", "gift", None, None, gte_predicate("1")).await;
    link_gift(&db, g.rule_id, gift_vid).await;
    let out = order_promotions::refresh_order_promotion(&db, token).await.unwrap();
    checkout_store::refresh_totals(&db, token).await.unwrap();
    let gift_line = match out {
        order_promotions::RefreshOutcome::Gift { line_id, variant_id, .. } => {
            assert_eq!(variant_id, gift_vid);
            line_id
        }
        other => panic!("expected gift, got {other:?}"),
    };
    assert_eq!(gift_lines(&db, token).await, vec![gift_line]);
    assert_eq!(promo_rows(&db, token).await, 0, "gift XOR discount");
    assert_eq!(checkout_total(&db, token).await, subtotal, "gift is free");
    // Gift row audit: qty 1, totals 0, listing price kept.
    {
        use saleor_rustify_db::entities::checkout_checkoutline;
        use sea_orm::{EntityTrait};
        let row = checkout_checkoutline::Entity::find_by_id(gift_line).one(&db).await.unwrap().unwrap();
        assert_eq!(row.quantity, 1);
        assert_eq!(row.total_price_gross_amount, dec("0"));
        assert_eq!(row.undiscounted_unit_price_amount, gift_price);
    }

    // Case B: 50% discount vs gift — max money value wins.
    let disc_saving = (subtotal / dec("2")).round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
    let d = make_promo(&db, "half-off", "subtotal_discount", Some("percentage"), Some(dec("50")), gte_predicate("1")).await;
    let out = order_promotions::refresh_order_promotion(&db, token).await.unwrap();
    checkout_store::refresh_totals(&db, token).await.unwrap();
    if disc_saving > gift_price {
        assert!(matches!(out, order_promotions::RefreshOutcome::Discount { .. }), "discount wins: {out:?}");
        assert!(gift_lines(&db, token).await.is_empty(), "loser cleared");
    } else {
        assert!(matches!(out, order_promotions::RefreshOutcome::Gift { .. }), "gift wins: {out:?}");
        assert_eq!(promo_rows(&db, token).await, 0);
    }
    drop_promo(&db, &g).await;
    drop_promo(&db, &d).await;
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
}

#[tokio::test]
async fn complete_carries_discount_row_and_gift_line() {
    let _guard = stock_guard();
    let db = db().await;
    let p = make_promo(&db, "carry-ten", "subtotal_discount", Some("percentage"), Some(dec("10")), gte_predicate("1")).await;
    let (token, _vid, unit) = stocked_checkout(&db, 2).await;
    let subtotal = unit * dec("2");
    let want_disc = (subtotal * dec("10") / dec("100")).round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
    let done = complete::complete_checkout(&db, token).await.unwrap();
    // Order total carries the discount; OrderDiscount row links the rule.
    let (header, lines) = order_store::get_order_rows(&db, done.order_id).await.unwrap().unwrap();
    assert_eq!(header.total_gross_amount, (subtotal - want_disc).max(dec("0")));
    {
        use saleor_rustify_db::entities::discount_orderdiscount;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        let d = discount_orderdiscount::Entity::find()
            .filter(discount_orderdiscount::Column::OrderId.eq(done.order_id))
            .filter(discount_orderdiscount::Column::Type.eq("order_promotion"))
            .one(&db)
            .await
            .unwrap()
            .expect("order discount row must be carried");
        assert_eq!(d.amount_value, want_disc);
        assert_eq!(d.promotion_rule_id, Some(p.rule_id));
    }
    assert!(lines.iter().all(|l| !l.is_gift));
    drop_promo(&db, &p).await;
    // NOTE: completed order left in place (suite convention: complete tests
    // leave orders; allocations pin stock like the other suites).
}

#[tokio::test]
async fn complete_carries_gift_line_with_allocation() {
    let _guard = stock_guard();
    let db = db().await;
    let (token, _vid, _unit) = stocked_checkout(&db, 2).await;
    // Tracked gift variant (real stock movement expected).
    let products = catalog::list_products(&db, "default-channel", None, 100).await.unwrap();
    use saleor_rustify_db::entities::product_productvariant;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let mut gift_vid = 0;
    for p in products.iter().flat_map(|p| &p.variants) {
        let id: i32 = p.id.parse().unwrap();
        if p.quantity_available < 1 {
            continue;
        }
        let v = product_productvariant::Entity::find_by_id(id).one(&db).await.unwrap().unwrap();
        if v.track_inventory {
            gift_vid = id;
            break;
        }
    }
    assert_ne!(gift_vid, 0);
    let g = make_promo(&db, "carry-gift", "gift", None, None, gte_predicate("1")).await;
    link_gift(&db, g.rule_id, gift_vid).await;
    checkout_store::refresh_totals(&db, token).await.unwrap();
    assert_eq!(gift_lines(&db, token).await.len(), 1);

    let done = complete::complete_checkout(&db, token).await.unwrap();
    let (_, lines) = order_store::get_order_rows(&db, done.order_id).await.unwrap().unwrap();
    let gift = lines.iter().find(|l| l.is_gift).expect("gift order line must exist");
    assert_eq!(gift.quantity, 1);
    assert_eq!(gift.total_price_gross_amount, dec("0"));
    assert_eq!(gift.unit_price_gross_amount, dec("0"));
    assert!(gift.undiscounted_unit_price_gross_amount > dec("0"));
    assert!(gift.unit_discount_amount > dec("0"), "discount audit trail");
    // Real units, real allocation (tracked gift variant).
    use saleor_rustify_db::entities::warehouse_allocation;
    let alloced: i32 = warehouse_allocation::Entity::find()
        .filter(warehouse_allocation::Column::OrderLineId.eq(gift.id))
        .all(&db)
        .await
        .unwrap()
        .iter()
        .map(|a| a.quantity_allocated)
        .sum();
    assert_eq!(alloced, 1);
    drop_promo(&db, &g).await;
}
