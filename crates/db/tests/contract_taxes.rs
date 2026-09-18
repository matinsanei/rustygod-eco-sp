//! Tax contract vs Django's `tax_*` tables and flat-rate calculations.
//! Mirrors `saleor/tax/tests/` + `tax/calculations/`: default-rate fallback,
//! class rates, channel config, and net/gross line math.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rustygod_db::{catalog, database_url, taxes};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn channel_config_matches_django_defaults() {
    let db = db().await;
    let cfg = taxes::channel_config(&db, "default-channel").await.unwrap();
    // Populatedb runs FLAT_RATES, charging taxes on net-entered prices.
    assert!(cfg.charge_taxes);
    assert!(!cfg.prices_entered_with_tax);
    assert_eq!(cfg.strategy.as_deref(), Some("FLAT_RATES"));
}

#[tokio::test]
async fn default_rate_falls_back_by_country() {
    let db = db().await;
    // Null-class rows are the defaults (populatedb EU set).
    assert_eq!(taxes::rate_for_class(&db, None, "DE").await.unwrap(), dec!(19));
    assert_eq!(taxes::rate_for_class(&db, None, "PL").await.unwrap(), dec!(23));
    // Unknown class id falls back to the country default...
    assert_eq!(taxes::rate_for_class(&db, Some(i32::MAX), "DE").await.unwrap(), dec!(19));
    // ...and unknown countries fall back to zero, like Django.
    assert_eq!(taxes::rate_for_class(&db, None, "XX").await.unwrap(), dec!(0));
}

#[tokio::test]
async fn class_rate_beats_default_when_present() {
    let db = db().await;
    use rustygod_db::entities::tax_taxclasscountryrate;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let specific = tax_taxclasscountryrate::Entity::find()
        .filter(tax_taxclasscountryrate::Column::TaxClassId.is_not_null())
        .one(&db)
        .await
        .unwrap();
    if let Some(row) = specific {
        let got = taxes::rate_for_class(&db, row.tax_class_id, &row.country)
            .await
            .unwrap();
        assert_eq!(got, row.rate);
    }
}

#[tokio::test]
async fn lines_tax_like_django_flat_rates() {
    let db = db().await;
    // A listed variant on default-channel (net-entered, charging).
    let products = catalog::list_products(&db, "default-channel", None, 5)
        .await
        .unwrap();
    let vid: i32 = products
        .iter()
        .flat_map(|p| &p.variants)
        .next()
        .unwrap()
        .id
        .parse()
        .unwrap();
    let pricing = catalog::checkout_pricing(&db, "default-channel", &[vid])
        .await
        .unwrap();
    let unit = pricing[&vid].0.amount;

    let lines = taxes::calculate_lines(&db, "default-channel", "DE", &[(vid, unit, 2)])
        .await
        .unwrap();
    assert_eq!(lines.len(), 1);
    let l = &lines[0];
    // No class on populatedb products -> default DE 19%.
    assert_eq!(l.tax_rate, dec!(19));
    assert_eq!(l.unit_net, unit);
    assert_eq!(l.unit_gross, rustygod_core::tax::flat_rate_tax(unit, dec!(19), false).1);
    assert_eq!(l.total_gross - l.total_net, l.unit_gross * Decimal::from(2) - l.unit_net * Decimal::from(2));
}
