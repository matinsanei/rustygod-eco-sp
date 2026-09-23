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

/// Per-country override for `update_tax_configuration`.
#[derive(Debug, Clone)]
pub struct CountryOverride {
    pub country_code: String,
    pub charge_taxes: bool,
    pub strategy: Option<String>,
    pub display_gross: bool,
    pub tax_app_id: Option<String>,
    pub use_weighted_tax_for_shipping: bool,
}

#[derive(Debug, Default)]
pub struct TaxConfigPatch {
    pub charge_taxes: Option<bool>,
    pub strategy: Option<String>,
    pub display_gross: Option<bool>,
    pub prices_entered_with_tax: Option<bool>,
    pub use_weighted_tax_for_shipping: Option<bool>,
    pub tax_app_id: Option<String>,
    pub upsert_countries: Vec<CountryOverride>,
    pub remove_countries: Vec<String>,
}

/// Update a tax configuration + its per-country rows (Django
/// `TaxConfigurationUpdate`: scalar patch plus country upserts/removals).
pub async fn update_tax_configuration(
    db: &impl ConnectionTrait,
    config_id: i32,
    patch: &TaxConfigPatch,
) -> Result<()> {
    use crate::entities::tax_taxconfigurationpercountry;
    use sea_orm::{ActiveModelTrait, Set};
    let Some(m) = tax_taxconfiguration::Entity::find_by_id(config_id).one(db).await? else {
        return Err(DbError::App("tax configuration not found".into()));
    };
    // Per-country rows first (FK-safe: children before parent touch).
    for c in &patch.upsert_countries {
        let existing: Option<i32> = tax_taxconfigurationpercountry::Entity::find()
            .select_only()
            .column(tax_taxconfigurationpercountry::Column::Id)
            .filter(tax_taxconfigurationpercountry::Column::TaxConfigurationId.eq(config_id))
            .filter(tax_taxconfigurationpercountry::Column::Country.eq(&c.country_code))
            .into_tuple::<i32>()
            .one(db)
            .await?;
        match existing {
            Some(id) => {
                let row = tax_taxconfigurationpercountry::Entity::find_by_id(id)
                    .one(db)
                    .await?
                    .ok_or_else(|| DbError::App("country row vanished".into()))?;
                let mut am: tax_taxconfigurationpercountry::ActiveModel = row.into();
                am.charge_taxes = Set(c.charge_taxes);
                am.tax_calculation_strategy = Set(c.strategy.clone());
                am.display_gross_prices = Set(c.display_gross);
                am.tax_app_id = Set(c.tax_app_id.clone());
                am.use_weighted_tax_for_shipping = Set(c.use_weighted_tax_for_shipping);
                am.update(db).await?;
            }
            None => {
                tax_taxconfigurationpercountry::ActiveModel {
                    country: Set(c.country_code.clone()),
                    charge_taxes: Set(c.charge_taxes),
                    tax_calculation_strategy: Set(c.strategy.clone()),
                    display_gross_prices: Set(c.display_gross),
                    tax_configuration_id: Set(config_id),
                    tax_app_id: Set(c.tax_app_id.clone()),
                    use_weighted_tax_for_shipping: Set(c.use_weighted_tax_for_shipping),
                    ..Default::default()
                }
                .insert(db)
                .await?;
            }
        }
    }
    if !patch.remove_countries.is_empty() {
        tax_taxconfigurationpercountry::Entity::delete_many()
            .filter(tax_taxconfigurationpercountry::Column::TaxConfigurationId.eq(config_id))
            .filter(tax_taxconfigurationpercountry::Column::Country.is_in(patch.remove_countries.clone()))
            .exec(db)
            .await?;
    }
    let mut am: tax_taxconfiguration::ActiveModel = m.into();
    if let Some(v) = patch.charge_taxes {
        am.charge_taxes = Set(v);
    }
    if patch.strategy.is_some() {
        am.tax_calculation_strategy = Set(patch.strategy.clone());
    }
    if let Some(v) = patch.display_gross {
        am.display_gross_prices = Set(v);
    }
    if let Some(v) = patch.prices_entered_with_tax {
        am.prices_entered_with_tax = Set(v);
    }
    if let Some(v) = patch.use_weighted_tax_for_shipping {
        am.use_weighted_tax_for_shipping = Set(v);
    }
    if patch.tax_app_id.is_some() {
        am.tax_app_id = Set(patch.tax_app_id.clone());
    }
    am.update(db).await?;
    Ok(())
}
