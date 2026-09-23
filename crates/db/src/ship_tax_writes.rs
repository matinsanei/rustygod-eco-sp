//! Shipping zones/methods + tax classes + country rates (Django parity).
//!
//! Django references (`saleor/graphql/shipping`, `saleor/graphql/tax`,
//! `saleor/shipping/models.py`):
//! - zone `countries` is a stored list; warehouses/channels ride explicit
//!   m2m tables (`warehouse_shipping_zones`, `shipping_zone_channels`);
//! - method prices live on `shippingmethodchannellisting` (create makes the
//!   header; money arrives via channel-listing);
//! - postal rules carry inclusion (`include`/`exclude`);
//! - tax country rates are `taxclasscountryrate` rows (class-scoped or the
//!   class-less default); tax classes in use by products/methods refuse
//!   deletion (Django's PROTECT, explicit here).

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set, TransactionTrait,
};
use serde_json::json;

use crate::{
    entities::{
        product_product, shipping_shippingmethod, shipping_shippingmethod_excluded_products,
        shipping_shippingmethodchannellisting, shipping_shippingmethodpostalcoderule,
        shipping_shippingzone, shipping_shippingzone_channels, tax_taxclass,
        tax_taxclasscountryrate, warehouse_channelwarehouse, warehouse_warehouse_shipping_zones,
        warehouse_warehouse,
    },
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::App(format!("shipping/tax error: {}", msg.into()))
}

// ------------------------------------------------------------------ zones --

pub struct NewZone {
    pub name: String,
    pub description: String,
    pub countries: Vec<String>,
    pub default: bool,
    pub warehouse_ids: Vec<uuid::Uuid>,
    pub channel_ids: Vec<i32>,
}

/// Create a shipping zone with links (Django `shippingZoneCreate`).
pub async fn create_zone(db: &sea_orm::DatabaseConnection, z: &NewZone) -> Result<i32> {
    if z.name.trim().is_empty() {
        return Err(fail("name is required"));
    }
    let txn = db.begin().await?;
    let row = shipping_shippingzone::ActiveModel {
        name: Set(z.name.trim().to_string()),
        countries: Set(z.countries.join(",")),
        default: Set(z.default),
        description: Set(z.description.clone()),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    link_zone(&txn, row.id, &z.warehouse_ids, &z.channel_ids).await?;
    txn.commit().await?;
    Ok(row.id)
}

async fn link_zone(
    txn: &impl ConnectionTrait,
    zone_id: i32,
    warehouses: &[uuid::Uuid],
    channels: &[i32],
) -> Result<()> {
    for wid in warehouses {
        if warehouse_warehouse::Entity::find_by_id(*wid).one(txn).await?.is_none() {
            return Err(fail(format!("warehouse {wid} not found")));
        }
        let exists = warehouse_warehouse_shipping_zones::Entity::find()
            .filter(warehouse_warehouse_shipping_zones::Column::ShippingzoneId.eq(zone_id))
            .filter(warehouse_warehouse_shipping_zones::Column::WarehouseId.eq(*wid))
            .one(txn)
            .await?
            .is_some();
        if !exists {
            warehouse_warehouse_shipping_zones::ActiveModel {
                shippingzone_id: Set(zone_id),
                warehouse_id: Set(*wid),
                ..Default::default()
            }
            .insert(txn)
            .await?;
        }
    }
    for ch in channels {
        let exists = shipping_shippingzone_channels::Entity::find()
            .filter(shipping_shippingzone_channels::Column::ShippingzoneId.eq(zone_id))
            .filter(shipping_shippingzone_channels::Column::ChannelId.eq(*ch))
            .one(txn)
            .await?
            .is_some();
        if !exists {
            shipping_shippingzone_channels::ActiveModel {
                shippingzone_id: Set(zone_id),
                channel_id: Set(*ch),
                ..Default::default()
            }
            .insert(txn)
            .await?;
        }
    }
    Ok(())
}

async fn unlink_zone(
    txn: &impl ConnectionTrait,
    zone_id: i32,
    warehouses: &[uuid::Uuid],
    channels: &[i32],
) -> Result<()> {
    if !warehouses.is_empty() {
        warehouse_warehouse_shipping_zones::Entity::delete_many()
            .filter(warehouse_warehouse_shipping_zones::Column::ShippingzoneId.eq(zone_id))
            .filter(warehouse_warehouse_shipping_zones::Column::WarehouseId.is_in(warehouses.to_vec()))
            .exec(txn)
            .await?;
    }
    if !channels.is_empty() {
        shipping_shippingzone_channels::Entity::delete_many()
            .filter(shipping_shippingzone_channels::Column::ShippingzoneId.eq(zone_id))
            .filter(shipping_shippingzone_channels::Column::ChannelId.is_in(channels.to_vec()))
            .exec(txn)
            .await?;
    }
    Ok(())
}

#[derive(Default)]
pub struct ZonePatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub countries: Option<Vec<String>>,
    pub default: Option<bool>,
    pub add_warehouses: Vec<uuid::Uuid>,
    pub add_channels: Vec<i32>,
    pub remove_warehouses: Vec<uuid::Uuid>,
    pub remove_channels: Vec<i32>,
}

/// Update a zone (Django `shippingZoneUpdate`).
pub async fn update_zone(db: &sea_orm::DatabaseConnection, id: i32, p: &ZonePatch) -> Result<()> {
    let txn = db.begin().await?;
    let z = shipping_shippingzone::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("shipping zone {id} not found")))?;
    let mut am: shipping_shippingzone::ActiveModel = z.into();
    if let Some(n) = p.name.clone() {
        if n.trim().is_empty() {
            return Err(fail("name cannot be empty"));
        }
        am.name = Set(n.trim().to_string());
    }
    if let Some(d) = p.description.clone() {
        am.description = Set(d);
    }
    if let Some(c) = p.countries.clone() {
        am.countries = Set(c.join(","));
    }
    if let Some(d) = p.default {
        am.default = Set(d);
    }
    am.update(&txn).await?;
    link_zone(&txn, id, &p.add_warehouses, &p.add_channels).await?;
    unlink_zone(&txn, id, &p.remove_warehouses, &p.remove_channels).await?;
    txn.commit().await?;
    Ok(())
}

/// Delete a zone with its methods + listings (Django's collector cascade).
pub async fn delete_zone(db: &sea_orm::DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await?;
    if shipping_shippingzone::Entity::find_by_id(id).one(&txn).await?.is_none() {
        return Err(fail(format!("shipping zone {id} not found")));
    }
    let methods: Vec<i32> = shipping_shippingmethod::Entity::find()
        .select_only()
        .column(shipping_shippingmethod::Column::Id)
        .filter(shipping_shippingmethod::Column::ShippingZoneId.eq(id))
        .into_tuple()
        .all(&txn)
        .await?;
    for m in methods {
        delete_method_in(&txn, m).await?;
    }
    warehouse_warehouse_shipping_zones::Entity::delete_many()
        .filter(warehouse_warehouse_shipping_zones::Column::ShippingzoneId.eq(id))
        .exec(&txn)
        .await?;
    shipping_shippingzone_channels::Entity::delete_many()
        .filter(shipping_shippingzone_channels::Column::ShippingzoneId.eq(id))
        .exec(&txn)
        .await?;
    if let Some(z) = shipping_shippingzone::Entity::find_by_id(id).one(&txn).await? {
        let am: shipping_shippingzone::ActiveModel = z.into();
        am.delete(&txn).await?;
    }
    txn.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------- methods --

pub struct NewMethod {
    pub zone_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub method_type: String,
    pub min_weight: Option<f64>,
    pub max_weight: Option<f64>,
    pub min_days: Option<i32>,
    pub max_days: Option<i32>,
    pub tax_class_id: Option<i32>,
    pub postal_rules: Vec<(String, Option<String>)>,
    pub inclusion: Option<String>,
}

/// Create a shipping method (Django `shippingPriceCreate`; money arrives
/// via channel listings).
pub async fn create_method(db: &sea_orm::DatabaseConnection, m: &NewMethod) -> Result<i32> {
    if m.name.trim().is_empty() {
        return Err(fail("name is required"));
    }
    if m.method_type != "price" && m.method_type != "weight" {
        return Err(fail("type must be price or weight"));
    }
    let txn = db.begin().await?;
    if shipping_shippingzone::Entity::find_by_id(m.zone_id).one(&txn).await?.is_none() {
        return Err(fail(format!("shipping zone {} not found", m.zone_id)));
    }
    if let Some(t) = m.tax_class_id {
        if tax_taxclass::Entity::find_by_id(t).one(&txn).await?.is_none() {
            return Err(fail(format!("tax class {t} not found")));
        }
    }
    let row = shipping_shippingmethod::ActiveModel {
        name: Set(m.name.trim().to_string()),
        maximum_order_weight: Set(m.max_weight),
        minimum_order_weight: Set(m.min_weight),
        r#type: Set(m.method_type.clone()),
        shipping_zone_id: Set(m.zone_id),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        maximum_delivery_days: Set(m.max_days),
        minimum_delivery_days: Set(m.min_days),
        description: Set(m.description.clone().map(|d| json!(d))),
        tax_class_id: Set(m.tax_class_id),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    for (start, end) in &m.postal_rules {
        shipping_shippingmethodpostalcoderule::ActiveModel {
            start: Set(start.clone()),
            end: Set(end.clone()),
            shipping_method_id: Set(row.id),
            inclusion_type: Set(m.inclusion.clone().unwrap_or_else(|| "include".to_string())),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    txn.commit().await?;
    Ok(row.id)
}

#[derive(Default)]
pub struct MethodPatch {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub method_type: Option<String>,
    pub min_weight: Option<Option<f64>>,
    pub max_weight: Option<Option<f64>>,
    pub min_days: Option<Option<i32>>,
    pub max_days: Option<Option<i32>>,
    pub tax_class_id: Option<Option<i32>>,
    pub add_postal_rules: Vec<(String, Option<String>)>,
    pub delete_postal_rules: Vec<i32>,
    pub inclusion: Option<String>,
}

/// Update a method (Django `shippingPriceUpdate`).
pub async fn update_method(db: &sea_orm::DatabaseConnection, id: i32, p: &MethodPatch) -> Result<()> {
    let txn = db.begin().await?;
    let m = shipping_shippingmethod::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("shipping method {id} not found")))?;
    let mut am: shipping_shippingmethod::ActiveModel = m.into();
    if let Some(n) = p.name.clone() {
        if n.trim().is_empty() {
            return Err(fail("name cannot be empty"));
        }
        am.name = Set(n.trim().to_string());
    }
    if let Some(d) = p.description.clone() {
        am.description = Set(d.map(|s| json!(s)));
    }
    if let Some(t) = p.method_type.clone() {
        if t != "price" && t != "weight" {
            return Err(fail("type must be price or weight"));
        }
        am.r#type = Set(t);
    }
    if let Some(x) = p.min_weight {
        am.minimum_order_weight = Set(x);
    }
    if let Some(x) = p.max_weight {
        am.maximum_order_weight = Set(x);
    }
    if let Some(x) = p.min_days {
        am.minimum_delivery_days = Set(x);
    }
    if let Some(x) = p.max_days {
        am.maximum_delivery_days = Set(x);
    }
    if let Some(t) = p.tax_class_id {
        if let Some(tid) = t {
            if tax_taxclass::Entity::find_by_id(tid).one(&txn).await?.is_none() {
                return Err(fail(format!("tax class {tid} not found")));
            }
        }
        am.tax_class_id = Set(t);
    }
    am.update(&txn).await?;
    for (start, end) in &p.add_postal_rules {
        shipping_shippingmethodpostalcoderule::ActiveModel {
            start: Set(start.clone()),
            end: Set(end.clone()),
            shipping_method_id: Set(id),
            inclusion_type: Set(p.inclusion.clone().unwrap_or_else(|| "include".to_string())),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    if !p.delete_postal_rules.is_empty() {
        shipping_shippingmethodpostalcoderule::Entity::delete_many()
            .filter(shipping_shippingmethodpostalcoderule::Column::Id.is_in(p.delete_postal_rules.clone()))
            .filter(shipping_shippingmethodpostalcoderule::Column::ShippingMethodId.eq(id))
            .exec(&txn)
            .await?;
    }
    txn.commit().await?;
    Ok(())
}

async fn delete_method_in(txn: &impl ConnectionTrait, id: i32) -> Result<()> {
    shipping_shippingmethodchannellisting::Entity::delete_many()
        .filter(shipping_shippingmethodchannellisting::Column::ShippingMethodId.eq(id))
        .exec(txn)
        .await?;
    shipping_shippingmethodpostalcoderule::Entity::delete_many()
        .filter(shipping_shippingmethodpostalcoderule::Column::ShippingMethodId.eq(id))
        .exec(txn)
        .await?;
    shipping_shippingmethod_excluded_products::Entity::delete_many()
        .filter(shipping_shippingmethod_excluded_products::Column::ShippingmethodId.eq(id))
        .exec(txn)
        .await?;
    if let Some(m) = shipping_shippingmethod::Entity::find_by_id(id).one(txn).await? {
        let am: shipping_shippingmethod::ActiveModel = m.into();
        am.delete(txn).await?;
    }
    Ok(())
}

/// Delete a method with listings/rules/exclusions (Django `shippingPriceDelete`).
pub async fn delete_method(db: &sea_orm::DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await?;
    if shipping_shippingmethod::Entity::find_by_id(id).one(&txn).await?.is_none() {
        return Err(fail(format!("shipping method {id} not found")));
    }
    delete_method_in(&txn, id).await?;
    txn.commit().await?;
    Ok(())
}

/// Exclude products (Django `shippingPriceExcludeProducts`).
pub async fn exclude_products(db: &sea_orm::DatabaseConnection, id: i32, products: &[i32]) -> Result<()> {
    let txn = db.begin().await?;
    if shipping_shippingmethod::Entity::find_by_id(id).one(&txn).await?.is_none() {
        return Err(fail(format!("shipping method {id} not found")));
    }
    for p in products {
        let exists = shipping_shippingmethod_excluded_products::Entity::find()
            .filter(shipping_shippingmethod_excluded_products::Column::ShippingmethodId.eq(id))
            .filter(shipping_shippingmethod_excluded_products::Column::ProductId.eq(*p))
            .one(&txn)
            .await?
            .is_some();
        if !exists {
            shipping_shippingmethod_excluded_products::ActiveModel {
                shippingmethod_id: Set(id),
                product_id: Set(*p),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }
    txn.commit().await?;
    Ok(())
}

/// Remove products from exclusion (Django `shippingPriceRemoveProductFromExclude`).
pub async fn include_products(db: &sea_orm::DatabaseConnection, id: i32, products: &[i32]) -> Result<()> {
    shipping_shippingmethod_excluded_products::Entity::delete_many()
        .filter(shipping_shippingmethod_excluded_products::Column::ShippingmethodId.eq(id))
        .filter(shipping_shippingmethod_excluded_products::Column::ProductId.is_in(products.to_vec()))
        .exec(db)
        .await?;
    Ok(())
}

pub struct ListingPatch {
    pub channel_id: i32,
    pub price: Option<Decimal>,
    pub min_price: Option<Option<Decimal>>,
    pub max_price: Option<Option<Decimal>>,
}

/// Method channel listings add/remove (Django `shippingMethodChannelListingUpdate`).
pub async fn update_method_listings(
    db: &sea_orm::DatabaseConnection,
    method_id: i32,
    add: &[ListingPatch],
    remove_channel_ids: &[i32],
    currency: &str,
) -> Result<()> {
    let txn = db.begin().await?;
    if shipping_shippingmethod::Entity::find_by_id(method_id).one(&txn).await?.is_none() {
        return Err(fail(format!("shipping method {method_id} not found")));
    }
    for l in add {
        let existing = shipping_shippingmethodchannellisting::Entity::find()
            .filter(shipping_shippingmethodchannellisting::Column::ShippingMethodId.eq(method_id))
            .filter(shipping_shippingmethodchannellisting::Column::ChannelId.eq(l.channel_id))
            .one(&txn)
            .await?;
        match existing {
            Some(row) => {
                let mut am: shipping_shippingmethodchannellisting::ActiveModel = row.into();
                if let Some(p) = l.price {
                    am.price_amount = Set(p);
                }
                if let Some(m) = l.min_price {
                    am.minimum_order_price_amount = Set(m);
                }
                if let Some(m) = l.max_price {
                    am.maximum_order_price_amount = Set(m);
                }
                am.update(&txn).await?;
            }
            None => {
                shipping_shippingmethodchannellisting::ActiveModel {
                    price_amount: Set(l.price.unwrap_or(Decimal::ZERO)),
                    currency: Set(currency.to_string()),
                    minimum_order_price_amount: Set(l.min_price.flatten()),
                    maximum_order_price_amount: Set(l.max_price.flatten()),
                    channel_id: Set(l.channel_id),
                    shipping_method_id: Set(method_id),
                    ..Default::default()
                }
                .insert(&txn)
                .await?;
            }
        }
    }
    if !remove_channel_ids.is_empty() {
        shipping_shippingmethodchannellisting::Entity::delete_many()
            .filter(shipping_shippingmethodchannellisting::Column::ShippingMethodId.eq(method_id))
            .filter(shipping_shippingmethodchannellisting::Column::ChannelId.is_in(remove_channel_ids.to_vec()))
            .exec(&txn)
            .await?;
    }
    txn.commit().await?;
    Ok(())
}

// --------------------------------------------------------------- tax class --

/// Create a tax class with country rates (Django `taxClassCreate`).
pub async fn create_tax_class(
    db: &sea_orm::DatabaseConnection,
    name: &str,
    rates: &[(String, Decimal)],
) -> Result<i32> {
    if name.trim().is_empty() {
        return Err(fail("name is required"));
    }
    let txn = db.begin().await?;
    let row = tax_taxclass::ActiveModel {
        name: Set(name.trim().to_string()),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    for (country, rate) in rates {
        if *rate < Decimal::ZERO || *rate > Decimal::from(100) {
            return Err(fail("rate must be 0-100"));
        }
        tax_taxclasscountryrate::ActiveModel {
            country: Set(country.clone()),
            rate: Set(*rate),
            tax_class_id: Set(Some(row.id)),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    txn.commit().await?;
    Ok(row.id)
}

#[derive(Default)]
pub struct TaxClassPatch {
    pub name: Option<String>,
    pub update_rates: Vec<(String, Decimal)>,
    pub remove_countries: Vec<String>,
}

/// Update a tax class (Django `taxClassUpdate`).
pub async fn update_tax_class(db: &sea_orm::DatabaseConnection, id: i32, p: &TaxClassPatch) -> Result<()> {
    let txn = db.begin().await?;
    let t = tax_taxclass::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("tax class {id} not found")))?;
    let mut am: tax_taxclass::ActiveModel = t.into();
    if let Some(n) = p.name.clone() {
        if n.trim().is_empty() {
            return Err(fail("name cannot be empty"));
        }
        am.name = Set(n.trim().to_string());
    }
    am.update(&txn).await?;
    for (country, rate) in &p.update_rates {
        if *rate < Decimal::ZERO || *rate > Decimal::from(100) {
            return Err(fail("rate must be 0-100"));
        }
        let existing = tax_taxclasscountryrate::Entity::find()
            .filter(tax_taxclasscountryrate::Column::TaxClassId.eq(Some(id)))
            .filter(tax_taxclasscountryrate::Column::Country.eq(country.clone()))
            .one(&txn)
            .await?;
        match existing {
            Some(row) => {
                let mut ram: tax_taxclasscountryrate::ActiveModel = row.into();
                ram.rate = Set(*rate);
                ram.update(&txn).await?;
            }
            None => {
                tax_taxclasscountryrate::ActiveModel {
                    country: Set(country.clone()),
                    rate: Set(*rate),
                    tax_class_id: Set(Some(id)),
                    ..Default::default()
                }
                .insert(&txn)
                .await?;
            }
        }
    }
    for country in &p.remove_countries {
        tax_taxclasscountryrate::Entity::delete_many()
            .filter(tax_taxclasscountryrate::Column::TaxClassId.eq(Some(id)))
            .filter(tax_taxclasscountryrate::Column::Country.eq(country.clone()))
            .exec(&txn)
            .await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Delete a tax class (Django `taxClassDelete`): in-use classes refuse
/// (products, shipping methods, shipping tax classes).
pub async fn delete_tax_class(db: &sea_orm::DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await?;
    if tax_taxclass::Entity::find_by_id(id).one(&txn).await?.is_none() {
        return Err(fail(format!("tax class {id} not found")));
    }
    let used_products = product_product::Entity::find()
        .filter(product_product::Column::TaxClassId.eq(Some(id)))
        .count(&txn)
        .await?;
    let used_methods = shipping_shippingmethod::Entity::find()
        .filter(shipping_shippingmethod::Column::TaxClassId.eq(Some(id)))
        .count(&txn)
        .await?;
    if used_products + used_methods > 0 {
        return Err(fail("tax class in use cannot be deleted"));
    }
    tax_taxclasscountryrate::Entity::delete_many()
        .filter(tax_taxclasscountryrate::Column::TaxClassId.eq(Some(id)))
        .exec(&txn)
        .await?;
    if let Some(t) = tax_taxclass::Entity::find_by_id(id).one(&txn).await? {
        let am: tax_taxclass::ActiveModel = t.into();
        am.delete(&txn).await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Country rates update (Django `taxCountryConfigurationUpdate`):
/// `{class, rate:null}` deletes that class rate; `{rate}` without class
/// writes the class-less default for the country.
pub async fn update_country_rates(
    db: &sea_orm::DatabaseConnection,
    country: &str,
    rates: &[(Option<i32>, Option<Decimal>)],
) -> Result<()> {
    let txn = db.begin().await?;
    for (class_id, rate) in rates {
        match (class_id, rate) {
            (Some(cid), None) => {
                tax_taxclasscountryrate::Entity::delete_many()
                    .filter(tax_taxclasscountryrate::Column::TaxClassId.eq(Some(*cid)))
                    .filter(tax_taxclasscountryrate::Column::Country.eq(country))
                    .exec(&txn)
                    .await?;
            }
            (Some(cid), Some(r)) => {
                if *r < Decimal::ZERO || *r > Decimal::from(100) {
                    return Err(fail("rate must be 0-100"));
                }
                if tax_taxclass::Entity::find_by_id(*cid).one(&txn).await?.is_none() {
                    return Err(fail(format!("tax class {cid} not found")));
                }
                let existing = tax_taxclasscountryrate::Entity::find()
                    .filter(tax_taxclasscountryrate::Column::TaxClassId.eq(Some(*cid)))
                    .filter(tax_taxclasscountryrate::Column::Country.eq(country))
                    .one(&txn)
                    .await?;
                match existing {
                    Some(row) => {
                        let mut am: tax_taxclasscountryrate::ActiveModel = row.into();
                        am.rate = Set(*r);
                        am.update(&txn).await?;
                    }
                    None => {
                        tax_taxclasscountryrate::ActiveModel {
                            country: Set(country.to_string()),
                            rate: Set(*r),
                            tax_class_id: Set(Some(*cid)),
                            ..Default::default()
                        }
                        .insert(&txn)
                        .await?;
                    }
                }
            }
            (None, Some(r)) => {
                if *r < Decimal::ZERO || *r > Decimal::from(100) {
                    return Err(fail("rate must be 0-100"));
                }
                let existing = tax_taxclasscountryrate::Entity::find()
                    .filter(tax_taxclasscountryrate::Column::TaxClassId.is_null())
                    .filter(tax_taxclasscountryrate::Column::Country.eq(country))
                    .one(&txn)
                    .await?;
                match existing {
                    Some(row) => {
                        let mut am: tax_taxclasscountryrate::ActiveModel = row.into();
                        am.rate = Set(*r);
                        am.update(&txn).await?;
                    }
                    None => {
                        tax_taxclasscountryrate::ActiveModel {
                            country: Set(country.to_string()),
                            rate: Set(*r),
                            tax_class_id: Set(None),
                            ..Default::default()
                        }
                        .insert(&txn)
                        .await?;
                    }
                }
            }
            (None, None) => {}
        }
    }
    txn.commit().await?;
    Ok(())
}

/// Delete a country's whole rate configuration (Django
/// `taxCountryConfigurationDelete`).
pub async fn delete_country_rates(db: &sea_orm::DatabaseConnection, country: &str) -> Result<()> {
    tax_taxclasscountryrate::Entity::delete_many()
        .filter(tax_taxclasscountryrate::Column::Country.eq(country))
        .exec(db)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------- channels --

/// Reorder a channel's warehouses (Django `channelReorderWarehouses`).
pub async fn reorder_channel_warehouses(
    db: &sea_orm::DatabaseConnection,
    channel_id: i32,
    moves: &[(uuid::Uuid, i32)],
) -> Result<()> {
    let txn = db.begin().await?;
    for (wid, sort) in moves {
        if let Some(row) = warehouse_channelwarehouse::Entity::find()
            .filter(warehouse_channelwarehouse::Column::ChannelId.eq(channel_id))
            .filter(warehouse_channelwarehouse::Column::WarehouseId.eq(*wid))
            .one(&txn)
            .await?
        {
            let mut am: warehouse_channelwarehouse::ActiveModel = row.into();
            am.sort_order = Set(Some(*sort));
            am.update(&txn).await?;
        }
    }
    txn.commit().await?;
    Ok(())
}

/// Attach warehouses to a channel (Django `addWarehouses` on create/update).
pub async fn add_channel_warehouses(
    db: &sea_orm::DatabaseConnection,
    channel_id: i32,
    warehouse_ids: &[uuid::Uuid],
) -> Result<()> {
    let txn = db.begin().await?;
    for wid in warehouse_ids {
        if warehouse_warehouse::Entity::find_by_id(*wid).one(&txn).await?.is_none() {
            return Err(fail(format!("warehouse {wid} not found")));
        }
        let exists = warehouse_channelwarehouse::Entity::find()
            .filter(warehouse_channelwarehouse::Column::ChannelId.eq(channel_id))
            .filter(warehouse_channelwarehouse::Column::WarehouseId.eq(*wid))
            .one(&txn)
            .await?
            .is_some();
        if !exists {
            warehouse_channelwarehouse::ActiveModel {
                warehouse_id: Set(*wid),
                channel_id: Set(channel_id),
                sort_order: Set(None),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }
    txn.commit().await?;
    Ok(())
}
