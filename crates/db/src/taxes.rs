//! Flat-rate taxes over Django's `tax_*` tables.
//!
//! Mirrors `saleor/tax/calculations/{checkout,order}.py` (FLAT_RATES path):
//! - default rate = `TaxClassCountryRate` for the country with
//!   `tax_class = NULL`, else 0;
//! - class rate = the class's row for the country, else the default
//!   (`get_tax_rate_for_country`);
//! - per-channel `TaxConfiguration` (`charge_taxes`, `prices_entered_with_tax`)
//!   decides whether math happens at all and in which direction.

use rust_decimal::Decimal;
use rustygod_core::tax as domain;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect,
};

use crate::{
    catalog::channel_info,
    entities::{
        product_product, product_productvariant, tax_taxclasscountryrate, tax_taxconfiguration,
    },
    DbError, Result,
};

pub struct ChannelTaxConfig {
    pub charge_taxes: bool,
    pub prices_entered_with_tax: bool,
    pub strategy: Option<String>,
}

/// Tax configuration for a channel slug. Missing row = Django defaults
/// (`charge_taxes=True` on the model; prices entered net).
pub async fn channel_config(
    db: &impl ConnectionTrait,
    channel_slug: &str,
) -> Result<ChannelTaxConfig> {
    let (ch_id, _) = channel_info(db, channel_slug).await?;
    let row = tax_taxconfiguration::Entity::find()
        .filter(tax_taxconfiguration::Column::ChannelId.eq(ch_id))
        .one(db)
        .await?;
    Ok(match row {
        Some(r) => ChannelTaxConfig {
            charge_taxes: r.charge_taxes,
            prices_entered_with_tax: r.prices_entered_with_tax,
            strategy: r.tax_calculation_strategy,
        },
        None => ChannelTaxConfig {
            charge_taxes: true,
            prices_entered_with_tax: false,
            strategy: None,
        },
    })
}

/// Rate (percent) for a tax class + country, with the null-class default
/// fallback (`get_tax_rate_for_country`).
pub async fn rate_for_class(
    db: &impl ConnectionTrait,
    tax_class_id: Option<i32>,
    country: &str,
) -> Result<Decimal> {
    let default: Decimal = tax_taxclasscountryrate::Entity::find()
        .select_only()
        .column(tax_taxclasscountryrate::Column::Rate)
        .filter(tax_taxclasscountryrate::Column::Country.eq(country))
        .filter(tax_taxclasscountryrate::Column::TaxClassId.is_null())
        .into_tuple()
        .one(db)
        .await?
        .unwrap_or(Decimal::ZERO);
    let Some(tcid) = tax_class_id else {
        return Ok(default);
    };
    Ok(tax_taxclasscountryrate::Entity::find()
        .select_only()
        .column(tax_taxclasscountryrate::Column::Rate)
        .filter(tax_taxclasscountryrate::Column::Country.eq(country))
        .filter(tax_taxclasscountryrate::Column::TaxClassId.eq(tcid))
        .into_tuple()
        .one(db)
        .await?
        .unwrap_or(default))
}

/// Tax class of a variant (via its product).
pub async fn class_for_variant(
    db: &impl ConnectionTrait,
    variant_id: i32,
) -> Result<Option<i32>> {
    let product_id: Option<i32> = product_productvariant::Entity::find_by_id(variant_id)
        .select_only()
        .column(product_productvariant::Column::ProductId)
        .into_tuple()
        .one(db)
        .await?;
    let Some(pid) = product_id else {
        return Err(DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(format!(
            "variant {variant_id}"
        ))));
    };
    Ok(product_product::Entity::find_by_id(pid)
        .select_only()
        .column(product_product::Column::TaxClassId)
        .into_tuple::<Option<i32>>()
        .one(db)
        .await?
        .flatten())
}

pub struct TaxedLine {
    pub variant_id: i32,
    pub quantity: i32,
    pub unit_net: Decimal,
    pub unit_gross: Decimal,
    pub total_net: Decimal,
    pub total_gross: Decimal,
    pub tax_rate: Decimal,
}

/// Apply channel taxes to lines. When the channel doesn't charge taxes,
/// net == gross (today's storage behavior — and correct for such channels).
pub async fn calculate_lines(
    db: &impl ConnectionTrait,
    channel_slug: &str,
    country: &str,
    lines: &[(i32, Decimal, i32)],
) -> Result<Vec<TaxedLine>> {
    let cfg = channel_config(db, channel_slug).await?;
    let mut out = Vec::with_capacity(lines.len());
    for (vid, unit, qty) in lines {
        let (rate, unit_net, unit_gross) = if cfg.charge_taxes {
            let class = class_for_variant(db, *vid).await?;
            let rate = rate_for_class(db, class, country).await?;
            let (n, g) = domain::flat_rate_tax(*unit, rate, cfg.prices_entered_with_tax);
            (rate, n, g)
        } else {
            (Decimal::ZERO, *unit, *unit)
        };
        let q = Decimal::from(*qty);
        // Line totals quantize like Django (unit-taxed × qty, then quantize).
        let zero = Decimal::ZERO;
        out.push(TaxedLine {
            variant_id: *vid,
            quantity: *qty,
            unit_net,
            unit_gross,
            total_net: domain::flat_rate_tax(unit_net * q, zero, false).0,
            total_gross: domain::flat_rate_tax(unit_gross * q, zero, false).0,
            tax_rate: rate,
        });
    }
    Ok(out)
}
