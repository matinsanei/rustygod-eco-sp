//! Warehouse writes on Django's `warehouse_*` tables.
//!
//! Reads (list/reserve/release) live in `commerce.rs`; this module owns the
//! staff mutations, mirroring `saleor/graphql/warehouse/mutations.py`:
//! - create carries its address row (Django requires one) in a transaction;
//! - click-and-collect is a closed set (`disabled/local/all`);
//! - delete refuses stocked warehouses instead of orphaning allocations;
//! - stocks upsert per (warehouse, variant) with the allocated floor intact;
//! - shipping-zone links are explicit assign/unassign rows.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        account_address, shipping_shippingzone, warehouse_stock, warehouse_warehouse,
        warehouse_warehouse_shipping_zones,
    },
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Warehouse(msg.into())
}

/// Closed set — mirrors `WarehouseClickAndCollectOption`.
pub fn valid_cc_option(opt: &str) -> bool {
    matches!(opt, "disabled" | "local" | "all")
}

pub struct NewWarehouse {
    pub name: String,
    pub slug: String,
    pub email: String,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub is_private: bool,
    pub cc_option: String,
}

pub async fn create_warehouse(
    db: &DatabaseConnection,
    new: NewWarehouse,
) -> Result<warehouse_warehouse::Model> {
    if !valid_cc_option(&new.cc_option) {
        return Err(fail("click_and_collect_option must be disabled, local or all"));
    }
    if new.slug.is_empty() || new.name.is_empty() {
        return Err(fail("warehouse needs a name and a slug"));
    }
    let txn = db.begin().await?;
    if warehouse_warehouse::Entity::find()
        .filter(warehouse_warehouse::Column::Slug.eq(&new.slug))
        .one(&txn)
        .await?
        .is_some()
    {
        return Err(fail(format!("warehouse slug '{}' is taken", new.slug)));
    }
    let addr = account_address::ActiveModel {
        first_name: Set(String::new()),
        last_name: Set(String::new()),
        company_name: Set(new.name.clone()),
        street_address_1: Set(new.street),
        street_address_2: Set(String::new()),
        city: Set(new.city),
        postal_code: Set(new.postal_code),
        country: Set(new.country),
        country_area: Set(String::new()),
        phone: Set(String::new()),
        city_area: Set(String::new()),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        validation_skipped: Set(false),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let wh = warehouse_warehouse::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(new.name),
        email: Set(new.email),
        address_id: Set(addr.id),
        slug: Set(new.slug),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        click_and_collect_option: Set(new.cc_option),
        is_private: Set(new.is_private),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    txn.commit().await?;
    Ok(wh)
}

pub struct WarehousePatch {
    pub name: Option<String>,
    pub email: Option<String>,
    pub cc_option: Option<String>,
    pub is_private: Option<bool>,
}

pub async fn update_warehouse(
    db: &DatabaseConnection,
    warehouse_id: Uuid,
    patch: WarehousePatch,
) -> Result<warehouse_warehouse::Model> {
    let txn = db.begin().await?;
    let row = warehouse_warehouse::Entity::find_by_id(warehouse_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("warehouse {warehouse_id} not found")))?;
    if let Some(cc) = &patch.cc_option {
        if !valid_cc_option(cc) {
            return Err(fail("click_and_collect_option must be disabled, local or all"));
        }
    }
    let mut am: warehouse_warehouse::ActiveModel = row.into();
    if let Some(n) = patch.name {
        if n.is_empty() {
            return Err(fail("warehouse name cannot be empty"));
        }
        am.name = Set(n);
    }
    if let Some(e) = patch.email {
        am.email = Set(e);
    }
    if let Some(cc) = patch.cc_option {
        am.click_and_collect_option = Set(cc);
    }
    if let Some(p) = patch.is_private {
        am.is_private = Set(p);
    }
    let updated = am.update(&txn).await?;
    txn.commit().await?;
    Ok(updated)
}

/// Delete a warehouse. Refuses when stocks reference it (Django would
/// orphan allocations); zone links are cleaned up.
pub async fn delete_warehouse(db: &DatabaseConnection, warehouse_id: Uuid) -> Result<()> {
    let txn = db.begin().await?;
    warehouse_warehouse::Entity::find_by_id(warehouse_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("warehouse {warehouse_id} not found")))?;
    if warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::WarehouseId.eq(warehouse_id))
        .one(&txn)
        .await?
        .is_some()
    {
        return Err(fail("cannot delete a warehouse that holds stock"));
    }
    warehouse_warehouse_shipping_zones::Entity::delete_many()
        .filter(warehouse_warehouse_shipping_zones::Column::WarehouseId.eq(warehouse_id))
        .exec(&txn)
        .await?;
    warehouse_warehouse::Entity::delete_by_id(warehouse_id)
        .exec(&txn)
        .await?;
    txn.commit().await?;
    Ok(())
}

/// Create or restock a (warehouse, variant) row. Quantity may never drop
/// below allocated (the reservations/allocation backstop).
pub async fn upsert_stock(
    db: &DatabaseConnection,
    warehouse_id: Uuid,
    variant_id: i32,
    quantity: i32,
) -> Result<warehouse_stock::Model> {
    if quantity < 0 {
        return Err(fail("stock quantity cannot be negative"));
    }
    let txn = db.begin().await?;
    warehouse_warehouse::Entity::find_by_id(warehouse_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("warehouse {warehouse_id} not found")))?;
    let existing = warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::WarehouseId.eq(warehouse_id))
        .filter(warehouse_stock::Column::ProductVariantId.eq(variant_id))
        .one(&txn)
        .await?;
    let out = match existing {
        Some(s) => {
            if quantity < s.quantity_allocated {
                return Err(fail(format!(
                    "quantity {quantity} is below allocated {}",
                    s.quantity_allocated
                )));
            }
            let mut am: warehouse_stock::ActiveModel = s.into();
            am.quantity = Set(quantity);
            am.update(&txn).await?
        }
        None => {
            warehouse_stock::ActiveModel {
                quantity: Set(quantity),
                product_variant_id: Set(variant_id),
                warehouse_id: Set(warehouse_id),
                quantity_allocated: Set(0),
                ..Default::default()
            }
            .insert(&txn)
            .await?
        }
    };
    txn.commit().await?;
    Ok(out)
}

pub struct ZoneView {
    pub id: i32,
    pub name: String,
    pub countries: String,
    pub is_default: bool,
}

pub async fn list_zones(db: &impl ConnectionTrait) -> Result<Vec<ZoneView>> {
    Ok(shipping_shippingzone::Entity::find()
        .order_by_asc(shipping_shippingzone::Column::Name)
        .all(db)
        .await?
        .into_iter()
        .map(|z| ZoneView {
            id: z.id,
            name: z.name,
            countries: z.countries,
            is_default: z.default,
        })
        .collect())
}

/// Link a warehouse into a shipping zone (idempotent).
pub async fn assign_zone(
    db: &DatabaseConnection,
    warehouse_id: Uuid,
    zone_id: i32,
) -> Result<()> {
    warehouse_warehouse::Entity::find_by_id(warehouse_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("warehouse {warehouse_id} not found")))?;
    shipping_shippingzone::Entity::find_by_id(zone_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("shipping zone {zone_id} not found")))?;
    let linked = warehouse_warehouse_shipping_zones::Entity::find()
        .filter(warehouse_warehouse_shipping_zones::Column::WarehouseId.eq(warehouse_id))
        .filter(warehouse_warehouse_shipping_zones::Column::ShippingzoneId.eq(zone_id))
        .one(db)
        .await?;
    if linked.is_none() {
        warehouse_warehouse_shipping_zones::ActiveModel {
            warehouse_id: Set(warehouse_id),
            shippingzone_id: Set(zone_id),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

pub async fn unassign_zone(
    db: &DatabaseConnection,
    warehouse_id: Uuid,
    zone_id: i32,
) -> Result<bool> {
    let res = warehouse_warehouse_shipping_zones::Entity::delete_many()
        .filter(warehouse_warehouse_shipping_zones::Column::WarehouseId.eq(warehouse_id))
        .filter(warehouse_warehouse_shipping_zones::Column::ShippingzoneId.eq(zone_id))
        .exec(db)
        .await?;
    Ok(res.rows_affected > 0)
}

/// Warehouse ids serving a zone (for availability checks).
pub async fn warehouses_for_zone(
    db: &impl ConnectionTrait,
    zone_id: i32,
) -> Result<Vec<Uuid>> {
    use sea_orm::{QuerySelect, SelectorTrait};
    Ok(warehouse_warehouse_shipping_zones::Entity::find()
        .select_only()
        .column(warehouse_warehouse_shipping_zones::Column::WarehouseId)
        .filter(warehouse_warehouse_shipping_zones::Column::ShippingzoneId.eq(zone_id))
        .into_tuple()
        .all(db)
        .await?)
}

/// Best-effort cleanup for tests: delete stocks, zone links, warehouse,
/// and its address row (reverse of create).
pub async fn delete_warehouse_deep(db: &DatabaseConnection, warehouse_id: Uuid) -> Result<()> {
    let txn = db.begin().await?;
    let row = warehouse_warehouse::Entity::find_by_id(warehouse_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("warehouse {warehouse_id} not found")))?;
    warehouse_stock::Entity::delete_many()
        .filter(warehouse_stock::Column::WarehouseId.eq(warehouse_id))
        .exec(&txn)
        .await?;
    warehouse_warehouse_shipping_zones::Entity::delete_many()
        .filter(warehouse_warehouse_shipping_zones::Column::WarehouseId.eq(warehouse_id))
        .exec(&txn)
        .await?;
    warehouse_warehouse::Entity::delete_by_id(warehouse_id)
        .exec(&txn)
        .await?;
    account_address::Entity::delete_by_id(row.address_id)
        .exec(&txn)
        .await?;
    txn.commit().await?;
    Ok(())
}
