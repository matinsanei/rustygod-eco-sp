//! Commerce reads/writes over Django's tables: discount, shipping,
//! giftcard, menu, page, account, channel, tax, warehouse.
//!
//! Same rules as `catalog`/`relations`: project columns explicitly (codegen
//! mistypes `tsvector`/`INTERVAL`), mirror Django defaults and status
//! strings, never expose password hashes.

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::json;

use crate::{
    catalog::channel_info,
    entities::{
        account_address, account_user, channel_channel, discount_promotion,
        discount_promotionrule, discount_promotionrule_channels, discount_voucher,
        discount_vouchercode, discount_voucherchannellisting, giftcard_giftcard, menu_menu,
        menu_menuitem, page_page, shipping_shippingmethod,
        shipping_shippingmethodchannellisting, shipping_shippingzone_channels, tax_taxclass,
        warehouse_channelwarehouse, warehouse_reservation, warehouse_stock, warehouse_warehouse,
    },
    DbError, Result,
};

// ---------- Discount ----------

pub struct VoucherView {
    pub code: String,
    pub voucher_type: String,
    pub discount_value_type: String,
    pub currency: String,
    pub discount_value: Decimal,
    pub min_spent: Option<Decimal>,
    pub usage_limit: Option<i32>,
    pub used: i32,
}

/// Validate a voucher code like `saleor/discount` checkout validation:
/// active code, voucher date window open, usage under limit, channel listing
/// present. Returns the channel listing values.
pub async fn validate_voucher(
    db: &impl sea_orm::ConnectionTrait,
    code: &str,
    channel_slug: &str,
) -> Result<VoucherView> {
    let code_row = discount_vouchercode::Entity::find()
        .filter(discount_vouchercode::Column::Code.eq(code))
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(code.into())))?;
    if !code_row.is_active {
        return Err(DbError::SeaOrm(sea_orm::DbErr::Custom("voucher code inactive".into())));
    }
    let v = discount_voucher::Entity::find_by_id(code_row.voucher_id)
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(code.into())))?;
    let now = Utc::now();
    let now_dt: sea_orm::prelude::DateTimeWithTimeZone = now.into();
    if v.start_date > now_dt {
        return Err(DbError::SeaOrm(sea_orm::DbErr::Custom("voucher not started".into())));
    }
    if let Some(end) = v.end_date {
        if end < now_dt {
            return Err(DbError::SeaOrm(sea_orm::DbErr::Custom("voucher expired".into())));
        }
    }
    if let Some(limit) = v.usage_limit {
        if code_row.used >= limit {
            return Err(DbError::SeaOrm(sea_orm::DbErr::Custom("voucher usage limit reached".into())));
        }
    }
    let (ch_id, _) = channel_info(db, channel_slug).await?;
    let listing = discount_voucherchannellisting::Entity::find()
        .filter(discount_voucherchannellisting::Column::VoucherId.eq(v.id))
        .filter(discount_voucherchannellisting::Column::ChannelId.eq(ch_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            DbError::SeaOrm(sea_orm::DbErr::Custom("voucher not available on channel".into()))
        })?;
    Ok(VoucherView {
        code: code_row.code,
        voucher_type: v.r#type,
        discount_value_type: v.discount_value_type,
        currency: listing.currency,
        discount_value: listing.discount_value,
        min_spent: listing.min_spent_amount,
        usage_limit: v.usage_limit,
        used: code_row.used,
    })
}

pub struct PromotionRuleView {
    pub id: String,
    pub name: String,
    pub reward_type: String,
    pub reward_value_type: String,
    pub reward_value: String,
}

pub struct PromotionView {
    pub id: String,
    pub name: String,
    pub promotion_type: String,
    pub rules: Vec<PromotionRuleView>,
}

/// Currently active promotions on a channel (date window + rule-channel link),
/// mirroring `saleor/discount` promotion fetching.
pub async fn list_promotions(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
) -> Result<Vec<PromotionView>> {
    let (ch_id, _) = channel_info(db, channel_slug).await?;
    let now: sea_orm::prelude::DateTimeWithTimeZone = Utc::now().into();
    let promos = discount_promotion::Entity::find()
        .filter(discount_promotion::Column::StartDate.lte(now))
        .filter(
            discount_promotion::Column::EndDate
                .is_null()
                .or(discount_promotion::Column::EndDate.gt(now)),
        )
        .all(db)
        .await?;
    let mut out = Vec::new();
    for p in promos {
        let rules = discount_promotionrule::Entity::find()
            .filter(discount_promotionrule::Column::PromotionId.eq(p.id))
            .filter(
                discount_promotionrule::Column::Id.in_subquery(
                    sea_orm::sea_query::Query::select()
                        .column(discount_promotionrule_channels::Column::PromotionruleId)
                        .from(discount_promotionrule_channels::Entity)
                        .and_where(
                            discount_promotionrule_channels::Column::ChannelId.eq(ch_id),
                        )
                        .to_owned(),
                ),
            )
            .all(db)
            .await?;
        if rules.is_empty() {
            continue;
        }
        out.push(PromotionView {
            id: p.id.to_string(),
            name: p.name,
            promotion_type: p.r#type,
            rules: rules
                .into_iter()
                .map(|r| PromotionRuleView {
                    id: r.id.to_string(),
                    name: r.name.unwrap_or_default(),
                    reward_type: r.reward_type.unwrap_or_default(),
                    reward_value_type: r.reward_value_type.unwrap_or_default(),
                    reward_value: r.reward_value.map(|v| v.to_string()).unwrap_or_default(),
                })
                .collect(),
        });
    }
    Ok(out)
}

// ---------- Shipping ----------

pub struct ShippingMethodView {
    pub id: i32,
    pub name: String,
    pub method_type: String,
    pub currency: String,
    pub price_amount: Decimal,
    pub min_order_price: Option<Decimal>,
}

/// Methods whose zone is attached to the channel, with channel prices —
/// mirrors checkout delivery-method collection.
pub async fn list_shipping_methods(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
) -> Result<Vec<ShippingMethodView>> {
    let (ch_id, _) = channel_info(db, channel_slug).await?;
    let zone_ids: Vec<i32> = shipping_shippingzone_channels::Entity::find()
        .select_only()
        .column(shipping_shippingzone_channels::Column::ShippingzoneId)
        .filter(shipping_shippingzone_channels::Column::ChannelId.eq(ch_id))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    if zone_ids.is_empty() {
        return Ok(vec![]);
    }
    let methods = shipping_shippingmethod::Entity::find()
        .filter(shipping_shippingmethod::Column::ShippingZoneId.is_in(zone_ids))
        .all(db)
        .await?;
    let mut out = Vec::new();
    for m in methods {
        if let Some(listing) = shipping_shippingmethodchannellisting::Entity::find()
            .filter(shipping_shippingmethodchannellisting::Column::ShippingMethodId.eq(m.id))
            .filter(shipping_shippingmethodchannellisting::Column::ChannelId.eq(ch_id))
            .one(db)
            .await?
        {
            out.push(ShippingMethodView {
                id: m.id,
                name: m.name,
                method_type: m.r#type,
                currency: listing.currency,
                price_amount: listing.price_amount,
                min_order_price: listing.minimum_order_price_amount,
            });
        }
    }
    Ok(out)
}

// ---------- GiftCard ----------

pub struct GiftCardView {
    pub code: String,
    pub currency: String,
    pub current_balance: Decimal,
    pub initial_balance: Decimal,
    pub is_active: bool,
}

pub async fn get_gift_card(db: &impl sea_orm::ConnectionTrait, code: &str) -> Result<Option<GiftCardView>> {
    Ok(giftcard_giftcard::Entity::find()
        .filter(giftcard_giftcard::Column::Code.eq(code))
        .one(db)
        .await?
        .map(|g| GiftCardView {
            code: g.code,
            currency: g.currency,
            current_balance: g.current_balance_amount,
            initial_balance: g.initial_balance_amount,
            is_active: g.is_active,
        }))
}

// ---------- Menu ----------

pub struct MenuItemView {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub level: i32,
    pub children: Vec<MenuItemView>,
}

pub struct MenuView {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub items: Vec<MenuItemView>,
}

pub async fn get_menu(db: &impl sea_orm::ConnectionTrait, slug: &str) -> Result<Option<MenuView>> {
    let Some(menu) = menu_menu::Entity::find()
        .filter(menu_menu::Column::Slug.eq(slug))
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let items = menu_menuitem::Entity::find()
        .filter(menu_menuitem::Column::MenuId.eq(menu.id))
        .order_by_asc(menu_menuitem::Column::TreeId)
        .order_by_asc(menu_menuitem::Column::Lft)
        .all(db)
        .await?;
    Ok(Some(MenuView {
        id: menu.id,
        name: menu.name,
        slug: menu.slug,
        items: build_menu_tree(&items, None),
    }))
}

fn build_menu_tree(
    items: &[menu_menuitem::Model],
    parent: Option<i32>,
) -> Vec<MenuItemView> {
    items
        .iter()
        .filter(|i| i.parent_id == parent)
        .map(|i| MenuItemView {
            id: i.id,
            name: i.name.clone(),
            url: i.url.clone().unwrap_or_default(),
            level: i.level,
            children: build_menu_tree(items, Some(i.id)),
        })
        .collect()
}

// ---------- Page ----------

pub struct PageView {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub is_published: bool,
}

pub async fn list_pages(db: &DatabaseConnection) -> Result<Vec<PageView>> {
    Ok(page_page::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|p| PageView {
            id: p.id,
            slug: p.slug,
            title: p.title,
            is_published: p.is_published,
        })
        .collect())
}

pub async fn get_page(db: &impl sea_orm::ConnectionTrait, slug: &str) -> Result<Option<PageView>> {
    Ok(page_page::Entity::find()
        .filter(page_page::Column::Slug.eq(slug))
        .one(db)
        .await?
        .map(|p| PageView {
            id: p.id,
            slug: p.slug,
            title: p.title,
            is_published: p.is_published,
        }))
}

// ---------- Account ----------

pub struct CustomerView {
    pub id: i32,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub is_active: bool,
}

/// Safe projection: password hash and tokens are never selected.
pub async fn get_customer(
    db: &impl sea_orm::ConnectionTrait,
    email: &str,
) -> Result<Option<CustomerView>> {
    let row: Option<(i32, String, String, String, bool)> = account_user::Entity::find()
        .select_only()
        .column(account_user::Column::Id)
        .column(account_user::Column::Email)
        .column(account_user::Column::FirstName)
        .column(account_user::Column::LastName)
        .column(account_user::Column::IsActive)
        .filter(account_user::Column::Email.eq(email))
        .into_tuple()
        .one(db)
        .await?;
    Ok(row.map(|(id, email, first_name, last_name, is_active)| CustomerView {
        id,
        email,
        first_name,
        last_name,
        is_active,
    }))
}

pub struct NewAddress {
    pub first_name: String,
    pub last_name: String,
    pub street_address_1: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub phone: String,
}

pub async fn create_address(db: &impl sea_orm::ConnectionTrait, a: &NewAddress) -> Result<i32> {
    let row = account_address::ActiveModel {
        first_name: Set(a.first_name.clone()),
        last_name: Set(a.last_name.clone()),
        company_name: Set(String::new()),
        street_address_1: Set(a.street_address_1.clone()),
        street_address_2: Set(String::new()),
        city: Set(a.city.clone()),
        postal_code: Set(a.postal_code.clone()),
        country: Set(a.country.clone()),
        country_area: Set(String::new()),
        phone: Set(a.phone.clone()),
        city_area: Set(String::new()),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        validation_skipped: Set(false),
        ..Default::default()
    };
    Ok(row.insert(db).await?.id)
}

// ---------- Channel ----------

pub struct ChannelView {
    pub id: i32,
    pub slug: String,
    pub currency_code: String,
    pub is_active: bool,
}

pub async fn list_channels(db: &DatabaseConnection) -> Result<Vec<ChannelView>> {
    let rows: Vec<(i32, String, String, bool)> = channel_channel::Entity::find()
        .select_only()
        .column(channel_channel::Column::Id)
        .column(channel_channel::Column::Slug)
        .column(channel_channel::Column::CurrencyCode)
        .column(channel_channel::Column::IsActive)
        .into_tuple()
        .all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(id, slug, currency_code, is_active)| ChannelView {
            id,
            slug,
            currency_code,
            is_active,
        })
        .collect())
}

// ---------- Tax ----------

pub async fn list_tax_classes(db: &DatabaseConnection) -> Result<Vec<(i32, String)>> {
    Ok(tax_taxclass::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|t| (t.id, t.name))
        .collect())
}

// ---------- Warehouse ----------

pub struct WarehouseView {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub email: String,
}

pub async fn list_warehouses(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
) -> Result<Vec<WarehouseView>> {
    let (ch_id, _) = channel_info(db, channel_slug).await?;
    let ids: Vec<uuid::Uuid> = warehouse_channelwarehouse::Entity::find()
        .select_only()
        .column(warehouse_channelwarehouse::Column::WarehouseId)
        .filter(warehouse_channelwarehouse::Column::ChannelId.eq(ch_id))
        .into_tuple::<uuid::Uuid>()
        .all(db)
        .await?;
    Ok(warehouse_warehouse::Entity::find()
        .filter(warehouse_warehouse::Column::Id.is_in(ids))
        .all(db)
        .await?
        .into_iter()
        .map(|w| WarehouseView {
            id: w.id.to_string(),
            name: w.name,
            slug: w.slug,
            email: w.email,
        })
        .collect())
}

pub struct StockView {
    pub warehouse_id: String,
    pub quantity: i32,
    pub quantity_allocated: i32,
}

pub async fn stocks_for_variant(
    db: &impl sea_orm::ConnectionTrait,
    variant_id: i32,
) -> Result<Vec<StockView>> {
    Ok(warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::ProductVariantId.eq(variant_id))
        .all(db)
        .await?
        .into_iter()
        .map(|s| StockView {
            warehouse_id: s.warehouse_id.to_string(),
            quantity: s.quantity,
            quantity_allocated: s.quantity_allocated,
        })
        .collect())
}

/// Reserve stock for a checkout line (mirrors `saleor/warehouse/reservations`).
/// Fails when the warehouse has no stock row or insufficient free quantity.
pub async fn reserve_stock(
    db: &DatabaseConnection,
    variant_id: i32,
    warehouse_id: uuid::Uuid,
    quantity: i32,
    checkout_line_id: uuid::Uuid,
    expires_in_seconds: i64,
) -> Result<()> {
    use sea_orm::TransactionTrait;
    if quantity < 1 {
        return Err(DbError::SeaOrm(sea_orm::DbErr::Custom(
            "quantity must be positive".into(),
        )));
    }
    let txn = db.begin().await?;
    let stock = warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::ProductVariantId.eq(variant_id))
        .filter(warehouse_stock::Column::WarehouseId.eq(warehouse_id))
        .one(&txn)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound("stock".into())))?;
    if stock.quantity - stock.quantity_allocated < quantity {
        return Err(DbError::SeaOrm(sea_orm::DbErr::Custom(
            "insufficient stock".into(),
        )));
    }
    // Idempotency: same line re-reserving replaces, never duplicates.
    warehouse_reservation::Entity::delete_many()
        .filter(warehouse_reservation::Column::StockId.eq(stock.id))
        .filter(warehouse_reservation::Column::CheckoutLineId.eq(checkout_line_id))
        .exec(&txn)
        .await?;
    warehouse_reservation::ActiveModel {
        quantity_reserved: Set(quantity),
        reserved_until: Set((Utc::now() + chrono::Duration::seconds(expires_in_seconds)).into()),
        stock_id: Set(stock.id),
        checkout_line_id: Set(checkout_line_id),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    txn.commit().await?;
    Ok(())
}

pub async fn release_reservations(
    db: &impl sea_orm::ConnectionTrait,
    checkout_line_id: uuid::Uuid,
) -> Result<()> {
    warehouse_reservation::Entity::delete_many()
        .filter(warehouse_reservation::Column::CheckoutLineId.eq(checkout_line_id))
        .exec(db)
        .await?;
    Ok(())
}

/// Sweeper: delete expired reservations. Idempotent and multi-instance
/// safe (deletes commute — two sweepers deleting the same row is a no-op
/// for the second). Keeps the `free = qty − allocated − reservations`
/// math from rotting (reviewer R9).
pub async fn sweep_expired_reservations(db: &DatabaseConnection) -> Result<u64> {
    let now: sea_orm::prelude::DateTimeWithTimeZone = Utc::now().into();
    let res = warehouse_reservation::Entity::delete_many()
        .filter(warehouse_reservation::Column::ReservedUntil.lte(now))
        .exec(db)
        .await?;
    Ok(res.rows_affected)
}
