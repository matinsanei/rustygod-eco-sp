//! Commerce (channels/warehouses/taxes/shipping/promotions/pages/menus) —
//! slim read surface the Dashboard needs for dropdowns/filters/lists.

use async_graphql::*;

use crate::common::{GqlCountryDisplay, GqlStockSettings};
use crate::context::GqlContext;
use crate::gen;

#[derive(SimpleObject, Clone)]
pub struct GqlWarehouse { pub id: ID, pub name: String, pub slug: String }

#[derive(SimpleObject, Clone)]
pub struct GqlWarehouseEdge { pub node: GqlWarehouse, pub cursor: String }

#[derive(SimpleObject, Clone)]
pub struct GqlTaxType { pub description: Option<String>, #[graphql(name = "taxCode")] pub tax_code: Option<String> }

#[derive(SimpleObject, Clone)]
pub struct GqlStockEdge { pub node: Option<gen::Stock> }

#[derive(SimpleObject, Clone)]
pub struct GqlStockConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlStockEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: crate::common::PageInfo,
}

async fn warehouse_gen(db: &sea_orm::DatabaseConnection, wid: uuid::Uuid) -> Result<Option<gen::Warehouse>, String> {
    use sea_orm::EntityTrait;
    let row = saleor_rustify_db::entities::warehouse_warehouse::Entity::find_by_id(wid)
        .one(db).await.map_err(|e| e.to_string())?;
    Ok(row.map(|w| {
        let mut g = crate::metadata::lit_warehouse(crate::common::gid("Warehouse", wid), vec![], vec![]);
        g.name = Some(w.name);
        g.slug = Some(w.slug);
        g.email = Some(w.email);
        g.is_private = Some(w.is_private);
        g.click_and_collect_option = Some(w.click_and_collect_option);
        g
    }))
}

async fn stock_node(db: &sea_orm::DatabaseConnection, sid: i32) -> Result<Option<gen::Stock>, String> {
    use sea_orm::EntityTrait;
    let row = saleor_rustify_db::entities::warehouse_stock::Entity::find_by_id(sid)
        .one(db).await.map_err(|e| e.to_string())?;
    let Some(s) = row else { return Ok(None) };
    Ok(Some(gen::Stock {
        id: Some(ID(crate::common::gid("Stock", sid))),
        warehouse: warehouse_gen(db, s.warehouse_id).await?,
        quantity: Some(s.quantity),
        quantity_allocated: Some(s.quantity_allocated),
    }))
}

async fn tax_config_node(db: &sea_orm::DatabaseConnection, tid: i32) -> Result<Option<gen::TaxConfiguration>, String> {
    use sea_orm::{ConnectionTrait, Statement};
    let row = db.query_one(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT t.id, t.channel_id, c.name AS channel_name, t.charge_taxes, t.tax_calculation_strategy, t.display_gross_prices, t.prices_entered_with_tax, t.tax_app_id, t.metadata, t.private_metadata FROM tax_taxconfiguration t JOIN channel_channel c ON c.id = t.channel_id WHERE t.id = $1",
        [tid.into()],
    )).await.map_err(|e| e.to_string())?;
    let Some(r) = row else { return Ok(None) };
    let countries = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT country, charge_taxes, tax_calculation_strategy, display_gross_prices, tax_app_id FROM tax_taxconfigurationpercountry WHERE tax_configuration_id = $1 ORDER BY country",
        [tid.into()],
    )).await.map_err(|e| e.to_string())?
    .into_iter().filter_map(|c| {
        let code = c.try_get::<String>("", "country").ok()?;
        Some(gen::TaxConfigurationPerCountry {
            country: Some(crate::common::GqlCountryDisplay { code: code.clone(), country: code }),
            charge_taxes: c.try_get::<bool>("", "charge_taxes").ok(),
            tax_calculation_strategy: c.try_get::<Option<String>>("", "tax_calculation_strategy").ok().flatten(),
            display_gross_prices: c.try_get::<bool>("", "display_gross_prices").ok(),
            tax_app_id: c.try_get::<Option<String>>("", "tax_app_id").ok().flatten(),
        })
    }).collect();
    let meta = |c: &str| {
        r.try_get::<serde_json::Value>("", c).ok()
            .map(|v| crate::common::json_to_metadata_items(&v)).unwrap_or_default()
    };
    let ch: i32 = r.try_get::<i32>("", "channel_id").map_err(|e| e.to_string())?;
    let mut channel = crate::metadata::lit_channel(crate::common::gid("Channel", ch), vec![], vec![]);
    channel.name = r.try_get::<String>("", "channel_name").ok();
    Ok(Some(gen::TaxConfiguration {
        id: Some(ID(crate::common::gid("TaxConfiguration", tid))),
        private_metadata: meta("private_metadata"),
        metadata: meta("metadata"),
        channel: Some(Box::new(channel)),
        charge_taxes: r.try_get::<bool>("", "charge_taxes").ok(),
        tax_calculation_strategy: r.try_get::<Option<String>>("", "tax_calculation_strategy").ok().flatten(),
        display_gross_prices: r.try_get::<bool>("", "display_gross_prices").ok(),
        prices_entered_with_tax: r.try_get::<bool>("", "prices_entered_with_tax").ok(),
        tax_app_id: r.try_get::<Option<String>>("", "tax_app_id").ok().flatten(),
        countries,
    }))
}

async fn menu_item_node(db: &sea_orm::DatabaseConnection, mid: i32) -> Result<Option<gen::MenuItem>, String> {
    use saleor_rustify_db::entities::{menu_menu, menu_menuitem};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
    let Some(m) = menu_menuitem::Entity::find_by_id(mid).one(db).await.map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    // Whole-menu fetch, tree built in memory (sync recursion — menus are tiny).
    let rows: Vec<(i32, Option<i32>, String, Option<String>, i32)> = menu_menuitem::Entity::find()
        .select_only()
        .column(menu_menuitem::Column::Id)
        .column(menu_menuitem::Column::ParentId)
        .column(menu_menuitem::Column::Name)
        .column(menu_menuitem::Column::Url)
        .column(menu_menuitem::Column::Level)
        .filter(menu_menuitem::Column::MenuId.eq(m.menu_id))
        .order_by_asc(menu_menuitem::Column::SortOrder)
        .into_tuple()
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let menu_name: Option<String> = menu_menu::Entity::find_by_id(m.menu_id)
        .one(db).await.map_err(|e| e.to_string())?.map(|x| x.name);
    let mut lit = crate::metadata::lit_menu(crate::common::gid("Menu", m.menu_id), vec![], vec![]);
    lit.name = menu_name;
    let menu = Some(Box::new(lit));
    fn build(
        id: i32,
        rows: &[(i32, Option<i32>, String, Option<String>, i32)],
        menu: &Option<Box<gen::Menu>>,
        depth: u8,
    ) -> Option<gen::MenuItem> {
        let (_, _, name, url, level) = rows.iter().find(|(i, _, _, _, _)| *i == id)?.clone();
        let mut children = vec![];
        if depth < 4 {
            for (kid, parent, _, _, _) in rows.iter() {
                if parent == &Some(id) {
                    if let Some(n) = build(*kid, rows, &None, depth + 1) {
                        children.push(Box::new(n));
                    }
                }
            }
        }
        let mut item = crate::metadata::lit_menu_item(crate::common::gid("MenuItem", id), vec![], vec![]);
        item.name = Some(name);
        item.url = url;
        item.level = Some(level);
        item.menu = menu.clone();
        item.children = children;
        Some(item)
    }
    Ok(build(mid, &rows, &menu, 0))
}

#[derive(SimpleObject, Clone)]
pub struct GqlMenuItemEdge { pub node: Option<gen::MenuItem> }

#[derive(SimpleObject, Clone)]
pub struct GqlMenuItemConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlMenuItemEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: crate::common::PageInfo,
}

#[derive(SimpleObject, Clone)]
pub struct GqlStockBulkResult {
    pub stock: Option<gen::Stock>,
    pub errors: Vec<gen::WarehouseError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlStockBulkUpdate {
    pub count: i32,
    pub results: Vec<GqlStockBulkResult>,
    pub errors: Vec<gen::WarehouseError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlWarehouseZoneAssign {
    pub warehouse: Option<gen::Warehouse>,
    pub errors: Vec<gen::WarehouseError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlWarehouseZoneUnassign {
    pub warehouse: Option<gen::Warehouse>,
    pub errors: Vec<gen::WarehouseError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlGiftCardBalanceAdjust {
    #[graphql(name = "giftCard")]
    pub gift_card: Option<gen::GiftCard>,
    pub errors: Vec<gen::GiftCardError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlMenuItemDelete {
    #[graphql(name = "menuItem")]
    pub menu_item: Option<gen::MenuItem>,
    pub errors: Vec<gen::MenuError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlAssignNavigation {
    pub menu: Option<gen::Menu>,
    pub errors: Vec<gen::MenuError>,
}

#[derive(Union, Clone)]
pub enum GqlTaxSourceObject {
    Checkout(crate::checkout::GqlCheckout),
    Order(Box<gen::Order>),
}

#[derive(SimpleObject, Clone)]
pub struct GqlTaxExemptionManage {
    #[graphql(name = "taxableObject")]
    pub taxable_object: Option<GqlTaxSourceObject>,
    pub errors: Vec<GqlTaxExemptionManageError>,
}

fn werr(field: Option<String>, message: String) -> gen::WarehouseError {
    gen::WarehouseError { field, message: Some(message), code: None }
}

fn gcerr(field: Option<String>, message: String) -> gen::GiftCardError {
    gen::GiftCardError { field, message: Some(message), code: None }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionBulkDelete")]
pub struct GqlPromotionBulkDelete {
    pub count: Option<i32>,
    pub errors: Vec<gen::DiscountError>,
}

fn discount_err(field: Option<String>, message: String) -> gen::DiscountError {
    gen::DiscountError { field, message: Some(message), code: None, channels: vec![], voucher_codes: vec![] }
}

fn reward_value_type_str(v: &gen::RewardValueTypeEnum) -> String {
    match v {
        gen::RewardValueTypeEnum::FIXED => "fixed".to_string(),
        gen::RewardValueTypeEnum::PERCENTAGE => "percentage".to_string(),
    }
}

fn reward_type_str(v: &gen::RewardTypeEnum) -> String {
    match v {
        gen::RewardTypeEnum::SUBTOTALDISCOUNT => "subtotal_discount".to_string(),
        gen::RewardTypeEnum::GIFT => "gift".to_string(),
    }
}

fn discount_value_type_str(v: &gen::DiscountValueTypeEnum) -> String {
    match v {
        gen::DiscountValueTypeEnum::FIXED => "fixed".to_string(),
        gen::DiscountValueTypeEnum::PERCENTAGE => "percentage".to_string(),
    }
}

/// Catalogue predicate → engine condition-dict JSON. Id-list leaves convert
/// exactly; other where-clauses narrow to their id lists (documented —
/// the engine only reads ids, so nothing it understands is lost).
fn catalogue_predicate_json(p: &gen::CataloguePredicateInput) -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut o = Map::new();
    if let Some(a) = p.and.as_ref() {
        o.insert("AND".to_string(), Value::Array(a.iter().map(catalogue_predicate_json).collect()));
    }
    if let Some(r) = p.or.as_ref() {
        o.insert("OR".to_string(), Value::Array(r.iter().map(catalogue_predicate_json).collect()));
    }
    if let Some(ids) = p.variant_predicate.as_ref().and_then(|w| w.ids.clone()) {
        if !ids.is_empty() {
            o.insert("variantPredicate".to_string(), serde_json::json!({"ids": ids.iter().map(|i| i.0.clone()).collect::<Vec<_>>()}));
        }
    }
    if let Some(ids) = p.product_predicate.as_ref().and_then(|w| w.ids.clone()) {
        if !ids.is_empty() {
            o.insert("productPredicate".to_string(), serde_json::json!({"ids": ids.iter().map(|i| i.0.clone()).collect::<Vec<_>>()}));
        }
    }
    if let Some(ids) = p.category_predicate.as_ref().and_then(|w| w.ids.clone()) {
        if !ids.is_empty() {
            o.insert("categoryPredicate".to_string(), serde_json::json!({"ids": ids.iter().map(|i| i.0.clone()).collect::<Vec<_>>()}));
        }
    }
    if let Some(ids) = p.collection_predicate.as_ref().and_then(|w| w.ids.clone()) {
        if !ids.is_empty() {
            o.insert("collectionPredicate".to_string(), serde_json::json!({"ids": ids.iter().map(|i| i.0.clone()).collect::<Vec<_>>()}));
        }
    }
    Value::Object(o)
}

fn decimal_filter_json(f: &gen::DecimalFilterInput) -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut o = Map::new();
    if let Some(e) = f.eq.as_ref() {
        o.insert("eq".to_string(), Value::String(e.0.clone()));
    }
    if let Some(one) = f.one_of.as_ref() {
        o.insert("oneOf".to_string(), Value::Array(one.iter().map(|v| Value::String(v.0.clone())).collect()));
    }
    if let Some(r) = f.range.as_ref() {
        let mut range = Map::new();
        if let Some(g) = r.gte.as_ref() {
            range.insert("gte".to_string(), Value::String(g.0.clone()));
        }
        if let Some(l) = r.lte.as_ref() {
            range.insert("lte".to_string(), Value::String(l.0.clone()));
        }
        o.insert("range".to_string(), Value::Object(range));
    }
    Value::Object(o)
}

fn discounted_object_json(w: &gen::DiscountedObjectWhereInput) -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut o = Map::new();
    if let Some(a) = w.and.as_ref() {
        o.insert("AND".to_string(), Value::Array(a.iter().map(discounted_object_json).collect()));
    }
    if let Some(r) = w.or.as_ref() {
        o.insert("OR".to_string(), Value::Array(r.iter().map(discounted_object_json).collect()));
    }
    if let Some(b) = w.base_subtotal_price.as_ref() {
        o.insert("baseSubtotalPrice".to_string(), decimal_filter_json(b));
    }
    if let Some(b) = w.base_total_price.as_ref() {
        o.insert("baseTotalPrice".to_string(), decimal_filter_json(b));
    }
    Value::Object(o)
}

/// Order predicate → engine money-gate JSON (same keys the evaluator reads).
fn order_predicate_json(p: &gen::OrderPredicateInput) -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut o = Map::new();
    if let Some(a) = p.and.as_ref() {
        o.insert("AND".to_string(), Value::Array(a.iter().map(order_predicate_json).collect()));
    }
    if let Some(r) = p.or.as_ref() {
        o.insert("OR".to_string(), Value::Array(r.iter().map(order_predicate_json).collect()));
    }
    if let Some(d) = p.discounted_object_predicate.as_ref() {
        o.insert("discountedObjectPredicate".to_string(), discounted_object_json(d));
    }
    Value::Object(o)
}

/// PromotionRuleInput (nested or top-level create shape) → db NewRule.
fn promo_rule_from(r: gen::PromotionRuleInput) -> Result<saleor_rustify_db::promo_writes::NewRule, Error> {
    Ok(saleor_rustify_db::promo_writes::NewRule {
        name: r.name.clone(),
        description: r.description.clone().unwrap_or(serde_json::Value::Null),
        catalogue_predicate: r.catalogue_predicate.as_ref().map(catalogue_predicate_json).unwrap_or(serde_json::Value::Object(Default::default())),
        order_predicate: r.order_predicate.as_ref().map(order_predicate_json).unwrap_or(serde_json::Value::Object(Default::default())),
        reward_value_type: r.reward_value_type.clone().map(|v| reward_value_type_str(&v)),
        reward_value: r.reward_value.clone().map(|v| v.0.parse::<rust_decimal::Decimal>()).transpose().map_err(|_| Error::new("bad reward value"))?,
        reward_type: r.reward_type.clone().map(|v| reward_type_str(&v)),
        channel_ids: r.channels.clone().unwrap_or_default().iter().filter_map(|c| saleor_rustify_db::catalog::parse_gid(&c.0)).collect(),
        gift_variant_ids: r.gifts.clone().unwrap_or_default().iter().filter_map(|c| saleor_rustify_db::catalog::parse_gid(&c.0)).collect(),
    })
}

/// Rule's promotion id (for update/delete payloads).
async fn rule_promotion(db: &sea_orm::DatabaseConnection, rid: uuid::Uuid) -> Result<uuid::Uuid, Error> {
    use sea_orm::{EntityTrait, QuerySelect};
    saleor_rustify_db::entities::discount_promotionrule::Entity::find_by_id(rid)
        .select_only()
        .column(saleor_rustify_db::entities::discount_promotionrule::Column::PromotionId)
        .into_tuple::<uuid::Uuid>()
        .one(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .ok_or_else(|| Error::new("promotion rule not found"))
}

/// One rule view out of a freshly assembled promotion.
async fn promotion_rule_view(
    db: &sea_orm::DatabaseConnection,
    pid: uuid::Uuid,
    rid: uuid::Uuid,
) -> Result<Option<gen::PromotionRule>, Error> {
    let promo = assemble_promotion(db, pid).await.map_err(Error::new)?;
    let want = crate::common::gid("PromotionRule", rid);
    Ok(promo.and_then(|p| p.rules.into_iter().find(|r| r.id.as_ref().map(|i| i.0 == want).unwrap_or(false))))
}

/// CatalogueInput → four id lists.
fn catalogue_ids(input: &gen::CatalogueInput) -> (Vec<i32>, Vec<i32>, Vec<i32>, Vec<i32>) {
    let ids = |v: &Option<Vec<ID>>| v.clone().unwrap_or_default().iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect::<Vec<_>>();
    (
        ids(&input.products),
        ids(&input.variants),
        ids(&input.categories),
        ids(&input.collections),
    )
}

/// VoucherInput → db NewVoucher (type inferred from catalogue assignment).
async fn voucher_from(input: gen::VoucherInput) -> std::result::Result<saleor_rustify_db::promo_writes::NewVoucher, String> {
    let products: Vec<i32> = input.products.clone().unwrap_or_default().iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect();
    let variants: Vec<i32> = input.variants.clone().unwrap_or_default().iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect();
    let categories: Vec<i32> = input.categories.clone().unwrap_or_default().iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect();
    let collections: Vec<i32> = input.collections.clone().unwrap_or_default().iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect();
    let mut codes: Vec<String> = input.add_codes.clone().unwrap_or_default();
    if let Some(c) = input.code.clone() {
        codes.push(c);
    }
    Ok(saleor_rustify_db::promo_writes::NewVoucher {
        name: input.name.clone(),
        voucher_type: saleor_rustify_db::promo_writes::infer_voucher_type(products.len(), variants.len(), categories.len(), collections.len()),
        discount_value_type: input.discount_value_type.clone().map(|v| discount_value_type_str(&v)).unwrap_or_else(|| "fixed".to_string()),
        products,
        variants,
        categories,
        collections,
        min_items: input.min_checkout_items_quantity,
        countries: input.countries.clone().unwrap_or_default(),
        apply_once_per_order: input.apply_once_per_order.unwrap_or(false),
        apply_once_per_customer: input.apply_once_per_customer.unwrap_or(false),
        only_for_staff: input.only_for_staff.unwrap_or(false),
        single_use: input.single_use.unwrap_or(false),
        usage_limit: input.usage_limit,
        start: input.start_date.map(|d| d.with_timezone(&chrono::Utc)).unwrap_or_else(chrono::Utc::now),
        end: input.end_date.map(|d| d.with_timezone(&chrono::Utc)),
        codes,
        listings: vec![],
    })
}

/// Currency for voucher listings: first channel's, else USD.
async fn listing_currency(db: &sea_orm::DatabaseConnection, listings: &[saleor_rustify_db::promo_writes::ChannelListingInput]) -> String {
    if let Some(l) = listings.first() {
        use sea_orm::{EntityTrait, QuerySelect};
        if let Ok(Some(cur)) = saleor_rustify_db::entities::channel_channel::Entity::find_by_id(l.channel_id)
            .select_only()
            .column(saleor_rustify_db::entities::channel_channel::Column::CurrencyCode)
            .into_tuple::<String>()
            .one(db)
            .await
        {
            return cur;
        }
    }
    "USD".to_string()
}

/// Voucher assembly (scalars + channel listings; catalogue connections stay
/// stubs — the mutations' payloads select header data, tabs refetch).
async fn assemble_voucher(db: &sea_orm::DatabaseConnection, vid: i32) -> Result<gen::Voucher, Error> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let v = saleor_rustify_db::entities::discount_voucher::Entity::find_by_id(vid)
        .one(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .ok_or_else(|| Error::new("voucher not found"))?;
    let used: i64 = saleor_rustify_db::entities::discount_vouchercode::Entity::find()
        .select_only()
        .column(saleor_rustify_db::entities::discount_vouchercode::Column::Used)
        .filter(saleor_rustify_db::entities::discount_vouchercode::Column::VoucherId.eq(vid))
        .into_tuple::<i32>()
        .all(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .into_iter()
        .map(i64::from)
        .sum();
    let listings = saleor_rustify_db::entities::discount_voucherchannellisting::Entity::find()
        .filter(saleor_rustify_db::entities::discount_voucherchannellisting::Column::VoucherId.eq(vid))
        .all(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?;
    let mut channel_listings = vec![];
    for l in listings {
        channel_listings.push(gen::VoucherChannelListing {
            id: Some(ID(crate::common::gid("VoucherChannelListing", l.id))),
            channel: None,
            discount_value: Some(l.discount_value.to_string().parse::<f64>().unwrap_or(0.0)),
            currency: Some(l.currency.clone()),
            min_spent: l.min_spent_amount.map(|a| crate::common::Money { amount: a.to_string(), currency: l.currency.clone(), fraction_digits: None }),
        });
    }
    let countries: Vec<crate::common::GqlCountryDisplay> = v
        .countries
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| crate::common::GqlCountryDisplay { code: s.to_string(), country: s.to_string() })
        .collect();
    Ok(gen::Voucher {
        id: Some(ID(crate::common::gid("Voucher", vid))),
        private_metadata: vec![],
        metadata: vec![],
        name: v.name.clone(),
        code: None,
        usage_limit: v.usage_limit,
        used: Some(used as i32),
        start_date: Some(v.start_date.into()),
        end_date: v.end_date.map(|d| d.into()),
        apply_once_per_order: Some(v.apply_once_per_order),
        apply_once_per_customer: Some(v.apply_once_per_customer),
        single_use: Some(v.single_use),
        only_for_staff: Some(v.only_for_staff),
        min_checkout_items_quantity: v.min_checkout_items_quantity,
        countries,
        discount_value_type: Some(v.discount_value_type.to_uppercase()),
        r#type: Some(v.r#type.to_uppercase()),
        channel_listings,
    })
}

/// Gift-card id gid → code lookup (mutations address cards by id; the
/// ledger addresses them by code).
async fn gift_card_code(db: &sea_orm::DatabaseConnection, gid: &str) -> std::result::Result<String, String> {
    let cid = saleor_rustify_db::catalog::parse_gid(gid).ok_or_else(|| "bad gift card id".to_string())?;
    use sea_orm::{EntityTrait, QuerySelect};
    saleor_rustify_db::entities::giftcard_giftcard::Entity::find_by_id(cid)
        .select_only()
        .column(saleor_rustify_db::entities::giftcard_giftcard::Column::Code)
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "gift card not found".to_string())
}

fn merr(field: Option<String>, message: String) -> gen::MenuError {
    gen::MenuError { field, message: Some(message), code: None }
}

#[derive(SimpleObject, Clone)]
pub struct GqlTaxExemptionManageError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

fn terr_tax(field: Option<String>, message: String) -> GqlTaxExemptionManageError {
    GqlTaxExemptionManageError { field, message: Some(message), code: None }
}

async fn resolve_stock_variant(
    db: &sea_orm::DatabaseConnection,
    id: Option<&ID>,
    ext: Option<&String>,
) -> Result<i32, String> {
    if let Some(i) = id {
        return saleor_rustify_db::catalog::parse_gid(&i.0).ok_or_else(|| "bad variant id".to_string());
    }
    if let Some(x) = ext {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        return saleor_rustify_db::entities::product_productvariant::Entity::find()
            .select_only().column(saleor_rustify_db::entities::product_productvariant::Column::Id)
            .filter(saleor_rustify_db::entities::product_productvariant::Column::Sku.eq(x))
            .into_tuple::<i32>().one(db).await.map_err(|e| e.to_string())?
            .ok_or_else(|| "variant not found".to_string());
    }
    Err("variant id or external reference required".to_string())
}

async fn resolve_stock_warehouse(
    db: &sea_orm::DatabaseConnection,
    id: Option<&ID>,
    ext: Option<&String>,
) -> Result<uuid::Uuid, String> {
    if let Some(i) = id {
        return crate::common::parse_uuid_gid(&i.0).ok_or_else(|| "bad warehouse id".to_string());
    }
    if let Some(x) = ext {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        return saleor_rustify_db::entities::warehouse_warehouse::Entity::find()
            .select_only().column(saleor_rustify_db::entities::warehouse_warehouse::Column::Id)
            .filter(saleor_rustify_db::entities::warehouse_warehouse::Column::ExternalReference.eq(x))
            .into_tuple::<uuid::Uuid>().one(db).await.map_err(|e| e.to_string())?
            .ok_or_else(|| "warehouse not found".to_string());
    }
    Err("warehouse id or external reference required".to_string())
}

async fn stock_id_for(db: &sea_orm::DatabaseConnection, vid: i32, wid: uuid::Uuid) -> Option<i32> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    saleor_rustify_db::entities::warehouse_stock::Entity::find()
        .select_only().column(saleor_rustify_db::entities::warehouse_stock::Column::Id)
        .filter(saleor_rustify_db::entities::warehouse_stock::Column::ProductVariantId.eq(vid))
        .filter(saleor_rustify_db::entities::warehouse_stock::Column::WarehouseId.eq(wid))
        .into_tuple::<i32>().one(db).await.ok()?
}

async fn zone_links(db: &sea_orm::DatabaseConnection, wid: uuid::Uuid, zids: &[i32], add: bool) -> Result<(), String> {
    use sea_orm::{ConnectionTrait, Statement};
    for zid in zids {
        let sql = if add {
            "INSERT INTO warehouse_warehouse_shipping_zones (warehouse_id, shippingzone_id) VALUES ($1::uuid, $2) ON CONFLICT DO NOTHING"
        } else {
            "DELETE FROM warehouse_warehouse_shipping_zones WHERE warehouse_id = $1::uuid AND shippingzone_id = $2"
        };
        db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            [wid.to_string().into(), (*zid).into()],
        )).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(SimpleObject, Clone)]
pub struct GqlWarehouseConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlWarehouseEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: crate::common::PageInfo,
}

#[derive(SimpleObject, Clone)]
pub struct GqlTaxClass { pub id: ID, pub name: String }

#[derive(SimpleObject, Clone)]
pub struct GqlShippingMethod { pub id: ID, pub name: String, pub price: String }

#[derive(SimpleObject, Clone)]
pub struct GqlPage { pub id: ID, pub slug: String, pub title: String }

#[derive(SimpleObject, Clone)]
pub struct GqlMenu { pub id: ID, pub slug: String, pub name: String }

#[derive(SimpleObject, Clone)]
pub struct GqlMenuEdge { pub node: GqlMenu, pub cursor: String }

#[derive(SimpleObject, Clone)]
pub struct GqlMenuConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlMenuEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: crate::common::PageInfo,
}

#[derive(SimpleObject, Clone)]
pub struct GqlPromotion { pub id: ID, pub name: String, pub r#type: String }

#[derive(SimpleObject, Clone)]
pub struct GqlDomain {
    pub host: String,
    #[graphql(name = "sslEnabled")]
    pub ssl_enabled: bool,
    pub url: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "LanguageDisplay")]
pub struct GqlLanguageDisplay { pub code: String, pub language: String }

#[derive(SimpleObject, Clone)]
#[graphql(name = "Permission")]
pub struct GqlPermission { pub code: String, pub name: String }

#[derive(SimpleObject, Clone)]
#[graphql(name = "Limits")]
pub struct GqlLimits {
    pub channels: Option<i32>,
    pub orders: Option<i32>,
    #[graphql(name = "productVariants")]
    pub product_variants: Option<i32>,
    #[graphql(name = "staffUsers")]
    pub staff_users: Option<i32>,
    pub warehouses: Option<i32>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "LimitInfo")]
pub struct GqlLimitInfo {
    #[graphql(name = "currentUsage")]
    pub current_usage: GqlLimits,
    #[graphql(name = "allowedUsage")]
    pub allowed_usage: GqlLimits,
}

/// Saleor `Announcement` (Dashboard banner). This backend serves no remote
/// announcements → always `[]` (same as Saleor with no feed entries).
#[derive(SimpleObject, Clone)]
#[graphql(name = "Announcement")]
pub struct GqlAnnouncement {
    pub title: String,
    #[graphql(name = "messageHtml")]
    pub message_html: String,
    pub importance: String,
    pub r#type: String,
    #[graphql(name = "createdAt")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[graphql(name = "updatedAt")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub extra: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlExternalAuth { pub id: ID, pub name: String }

/// Saleor `Shop` — Dashboard boot queries (`ShopInfo`, site settings,
/// `RefreshLimits`) all hit this type, so it must expose every field
/// those fragments select (was: only 2 fields → "Unknown field" storm).
#[derive(SimpleObject, Clone)]
pub struct GqlShop {
    pub id: ID,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    #[graphql(name = "schemaVersion")]
    pub schema_version: String,
    pub domain: GqlDomain,
    pub countries: Vec<GqlCountryDisplay>,
    #[graphql(name = "defaultCountry")]
    pub default_country: Option<GqlCountryDisplay>,
    pub languages: Vec<GqlLanguageDisplay>,
    pub permissions: Vec<GqlPermission>,
    #[graphql(name = "defaultWeightUnit")]
    pub default_weight_unit: Option<String>,
    #[graphql(name = "headerText")]
    pub header_text: Option<String>,
    #[graphql(name = "trackInventoryByDefault")]
    pub track_inventory_by_default: Option<bool>,
    #[graphql(name = "fulfillmentAutoApprove")]
    pub fulfillment_auto_approve: bool,
    #[graphql(name = "fulfillmentAllowUnpaid")]
    pub fulfillment_allow_unpaid: bool,
    #[graphql(name = "availableExternalAuthentications")]
    pub available_external_authentications: Vec<GqlExternalAuth>,
    #[graphql(name = "passwordLoginMode")]
    pub password_login_mode: String,
    #[graphql(name = "allowStorefrontTraffic")]
    pub allow_storefront_traffic: bool,
    #[graphql(name = "useLegacyUpdateWebhookEmission")]
    pub use_legacy_update_webhook_emission: Option<bool>,
    #[graphql(name = "useLegacyShippingZoneStockAvailability")]
    pub use_legacy_shipping_zone_stock_availability: bool,
    #[graphql(name = "preserveAllAddressFields")]
    pub preserve_all_address_fields: bool,
    #[graphql(name = "limitQuantityPerCheckout")]
    pub limit_quantity_per_checkout: Option<i32>,
    #[graphql(name = "reserveStockDurationAnonymousUser")]
    pub reserve_stock_duration_anonymous_user: Option<i32>,
    #[graphql(name = "reserveStockDurationAuthenticatedUser")]
    pub reserve_stock_duration_authenticated_user: Option<i32>,
    #[graphql(name = "enableAccountConfirmationByEmail")]
    pub enable_account_confirmation_by_email: Option<bool>,
    #[graphql(name = "defaultMailSenderName")]
    pub default_mail_sender_name: Option<String>,
    #[graphql(name = "defaultMailSenderAddress")]
    pub default_mail_sender_address: Option<String>,
    #[graphql(name = "customerSetPasswordUrl")]
    pub customer_set_password_url: Option<String>,
    #[graphql(name = "companyAddress")]
    pub company_address: Option<crate::order::GqlAddress>,
    pub limits: GqlLimitInfo,
    /// Shop metadata (`site_sitesettings.metadata`) — Dashboard navigation
    /// pins live here (`ShopNavigationPins` boot query).
    pub metadata: Vec<crate::common::MetadataItem>,
    /// Saleor announcement banners — always empty (no remote feed).
    pub announcements: Vec<GqlAnnouncement>,
}

#[derive(Default)]
pub struct CommerceQuery;

/// Shared shop assembly: real site rows + permissions where we have them,
/// stubs elsewhere (matches every Dashboard shop fragment by construction
/// since `gen::Shop` is generated from them).
async fn to_gen_shop(ctx: &Context<'_>) -> Result<gen::Shop, async_graphql::Error> {
    let (site_name, site_desc) = site_info(ctx).await;
    let shop_metadata = site_metadata(ctx).await;
    Ok(gen::Shop {
        private_metadata: vec![],
        metadata: shop_metadata,
        id: Some(ID("Shop:1".into())),
        available_payment_gateways: vec![],
        available_external_authentications: vec![],
        channel_currencies: vec!["USD".into()],
        default_country: Some(GqlCountryDisplay { code: "US".into(), country: "United States".into() }),
        default_mail_sender_name: None,
        default_mail_sender_address: None,
        description: site_desc,
        domain: Some(gen::Domain { host: Some("localhost:8000".into()), url: Some("http://localhost:8000/".into()) }),
        languages: vec![
            GqlLanguageDisplay { code: "EN".into(), language: "English".into() },
            GqlLanguageDisplay { code: "FA".into(), language: "Persian".into() },
        ],
        name: Some(site_name),
        permissions: all_permissions(ctx).await,
        fulfillment_auto_approve: Some(true),
        fulfillment_allow_unpaid: Some(true),
        allow_storefront_traffic: Some(true),
        default_weight_unit: Some("KG".into()),
        reserve_stock_duration_anonymous_user: None,
        reserve_stock_duration_authenticated_user: None,
        limit_quantity_per_checkout: None,
        company_address: None,
        customer_set_password_url: None,
        staff_notification_recipients: vec![],
        enable_account_confirmation_by_email: None,
        limits: Some(GqlLimitInfo {
            current_usage: GqlLimits { channels: Some(1), orders: Some(0), product_variants: Some(0), staff_users: Some(1), warehouses: Some(1) },
            allowed_usage: GqlLimits { channels: Some(100), orders: Some(10000), product_variants: Some(10000), staff_users: Some(100), warehouses: Some(100) },
        }),
        announcements: vec![],
        // API-compat version (matches the dashboard schema we implement) +
        // engine tag. Displayed as `core v3.23.33-rustyfi` in Configuration.
        version: Some("3.23.33-rustyfi".into()),
        available_tax_apps: vec![],
        preserve_all_address_fields: Some(false),
        password_login_mode: Some("ENABLED".into()),
        use_legacy_shipping_zone_stock_availability: Some(false),
        use_legacy_update_webhook_emission: Some(false),
    })
}

#[Object]
impl CommerceQuery {
    async fn shop(&self, ctx: &Context<'_>) -> Result<gen::Shop> {
        to_gen_shop(ctx).await
    }

    /// Taxes → channels page: per-channel tax configurations with
    /// per-country overrides.
    async fn tax_configurations(
        &self, ctx: &Context<'_>,
        filter: Option<gen::TaxConfigurationFilterInput>,
        before: Option<String>, after: Option<String>, first: Option<i32>, last: Option<i32>,
    ) -> Result<Option<gen::TaxConfigurationCountableConnection>> {
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{ConnectionTrait, Statement};
        use std::collections::HashMap;
        let mut conds: Vec<String> = vec![];
        let mut params: Vec<sea_orm::Value> = vec![];
        if let Some(f) = filter.as_ref() {
            if let Some(ids) = f.ids.as_ref() {
                let list: Vec<i32> = ids.iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect();
                if !list.is_empty() {
                    let ph = (1..=list.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
                    conds.push(format!("t.id IN ({ph})"));
                    params.extend(list.into_iter().map(|i| i.into()));
                }
            }
        }
        let where_sql = if conds.is_empty() { String::new() } else { format!("WHERE {}", conds.join(" AND ")) };
        let rows = db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT t.id, t.channel_id, c.slug AS channel_slug, c.name AS channel_name, t.charge_taxes, t.tax_calculation_strategy, t.display_gross_prices, t.prices_entered_with_tax, t.tax_app_id, t.metadata, t.private_metadata FROM tax_taxconfiguration t JOIN channel_channel c ON c.id = t.channel_id {where_sql} ORDER BY t.id"),
            params,
        )).await.map_err(|e| Error::new(e.to_string()))?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        let tids: Vec<i32> = rows.iter().filter_map(|r| r.try_get::<i32>("", "id").ok()).collect();
        let mut per_country: HashMap<i32, Vec<gen::TaxConfigurationPerCountry>> = HashMap::new();
        if !tids.is_empty() {
            let list = (1..=tids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
            for r in db.query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT tax_configuration_id, country, charge_taxes, tax_calculation_strategy, display_gross_prices, tax_app_id FROM tax_taxconfigurationpercountry WHERE tax_configuration_id IN ({list}) ORDER BY country"),
                tids.iter().map(|i| (*i).into()).collect::<Vec<sea_orm::Value>>(),
            )).await.map_err(|e| Error::new(e.to_string()))? {
                if let (Ok(tid), Ok(code)) = (r.try_get::<i32>("", "tax_configuration_id"), r.try_get::<String>("", "country")) {
                    per_country.entry(tid).or_default().push(gen::TaxConfigurationPerCountry {
                        country: Some(crate::common::GqlCountryDisplay { code: code.clone(), country: code }),
                        charge_taxes: r.try_get::<bool>("", "charge_taxes").ok(),
                        tax_calculation_strategy: r.try_get::<Option<String>>("", "tax_calculation_strategy").ok().flatten(),
                        display_gross_prices: r.try_get::<bool>("", "display_gross_prices").ok(),
                        tax_app_id: r.try_get::<Option<String>>("", "tax_app_id").ok().flatten(),
                    });
                }
            }
        }
        let meta = |r: &sea_orm::QueryResult, c: &str| {
            r.try_get::<serde_json::Value>("", c).ok()
                .map(|v| crate::common::json_to_metadata_items(&v)).unwrap_or_default()
        };
        let edges = rows.into_iter().skip(off).take(lim).filter_map(|r| {
            let (tid, ch) = (r.try_get::<i32>("", "id").ok()?, r.try_get::<i32>("", "channel_id").ok()?);
            let mut channel = crate::metadata::lit_channel(crate::common::gid("Channel", ch), vec![], vec![]);
            channel.name = r.try_get::<String>("", "channel_name").ok();
            Some(gen::TaxConfigurationCountableEdge { node: Some(gen::TaxConfiguration {
                id: Some(ID(crate::common::gid("TaxConfiguration", tid))),
                private_metadata: meta(&r, "private_metadata"),
                metadata: meta(&r, "metadata"),
                channel: Some(Box::new(channel)),
                charge_taxes: r.try_get::<bool>("", "charge_taxes").ok(),
                tax_calculation_strategy: r.try_get::<Option<String>>("", "tax_calculation_strategy").ok().flatten(),
                display_gross_prices: r.try_get::<bool>("", "display_gross_prices").ok(),
                prices_entered_with_tax: r.try_get::<bool>("", "prices_entered_with_tax").ok(),
                countries: per_country.get(&tid).cloned().unwrap_or_default(),
                tax_app_id: r.try_get::<Option<String>>("", "tax_app_id").ok().flatten(),
            })})
        }).collect();
        Ok(Some(gen::TaxConfigurationCountableConnection { edges }))
    }

    /// Shipping zones with channels + warehouses (channel setup banner
    /// coverage + zone pages). `channel` slug narrows to that channel.
    async fn shipping_zones(
        &self, ctx: &Context<'_>,
        filter: Option<gen::ShippingZoneFilterInput>, channel: Option<String>,
        before: Option<String>, after: Option<String>, first: Option<i32>, last: Option<i32>,
    ) -> Result<Option<gen::ShippingZoneCountableConnection>> {
        let _ = (filter, before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{ConnectionTrait, Statement};
        let (sql, params): (String, Vec<sea_orm::Value>) = match channel {
            Some(slug) => (
                "SELECT z.id FROM shipping_shippingzone z JOIN shipping_shippingzone_channels zc ON zc.shippingzone_id = z.id \
                 JOIN channel_channel c ON c.id = zc.channel_id WHERE c.slug = $1 ORDER BY z.id".into(),
                vec![slug.into()]),
            None => ("SELECT id FROM shipping_shippingzone ORDER BY id".into(), vec![]),
        };
        let rows = db.query_all(Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params))
            .await.map_err(|e| Error::new(e.to_string()))?;
        let ids: Vec<i32> = rows.into_iter().filter_map(|r| r.try_get::<i32>("", "id").ok()).collect();
        let total = ids.len() as i32;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(100).clamp(1, 100) as usize;
        let mut edges = vec![];
        for zid in ids.into_iter().skip(off).take(lim) {
            if let Some(z) = assemble_zone(db, zid).await.map_err(Error::new)? {
                edges.push(gen::ShippingZoneCountableEdge { node: Some(z) });
            }
        }
        Ok(Some(gen::ShippingZoneCountableConnection {
            page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
            edges,
            total_count: Some(total),
        }))
    }

    /// Saleor `giftCard(id)` — details page (was a None stub).
    async fn gift_card(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::GiftCard>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(gid) = saleor_rustify_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        Ok(assemble_gift_card(db, gid).await.map_err(Error::new)?)
    }

    /// Saleor `shippingZone(id)`.
    async fn shipping_zone(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::ShippingZone>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(zid) = saleor_rustify_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        Ok(assemble_zone(db, zid).await.map_err(Error::new)?)
    }

    /// Dashboard gift-card list: search/filter/sort server-side; product,
    /// tags and balances per row.
    async fn gift_cards(
        &self, ctx: &Context<'_>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::GiftCardSortingInput>,
        filter: Option<gen::GiftCardFilterInput>,
        search: Option<String>,
        before: Option<String>, after: Option<String>, first: Option<i32>, last: Option<i32>,
    ) -> Result<Option<gen::GiftCardCountableConnection>> {
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{ConnectionTrait, Statement};
        use std::collections::HashMap;
        let mut conds: Vec<String> = vec![];
        let mut params: Vec<sea_orm::Value> = vec![];
        if let Some(s) = search.as_ref().filter(|s| !s.trim().is_empty()) {
            params.push(format!("%{s}%").into());
            let p = params.len();
            conds.push(format!("(g.code ILIKE ${p} OR g.assigned_to_email ILIKE ${p} OR g.created_by_email ILIKE ${p})"));
        }
        if let Some(f) = filter.as_ref() {
            if let Some(a) = f.is_active {
                params.push(a.into());
                conds.push(format!("g.is_active = ${}", params.len()));
            }
            if let Some(c) = f.code.as_ref().filter(|s| !s.trim().is_empty()) {
                params.push(format!("%{c}%").into());
                conds.push(format!("g.code ILIKE ${}", params.len()));
            }
            if let Some(c) = f.currency.as_ref() {
                params.push(c.clone().into());
                conds.push(format!("g.currency = ${}", params.len()));
            }
            if let Some(e) = f.created_by_email.as_ref() {
                params.push(e.clone().into());
                conds.push(format!("g.created_by_email = ${}", params.len()));
            }
            if let Some(u) = f.used {
                conds.push(if u { "g.used_by_id IS NOT NULL".into() } else { "g.used_by_id IS NULL".into() });
            }
            if let Some(prods) = f.products.as_ref() {
                let list: Vec<i32> = prods.iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect();
                if !list.is_empty() {
                    let base = params.len();
                    let ph = (1..=list.len()).map(|i| format!("${}", base + i)).collect::<Vec<_>>().join(", ");
                    conds.push(format!("g.product_id IN ({ph})"));
                    params.extend(list.into_iter().map(|i| i.into()));
                }
            }
            if let Some(tags) = f.tags.as_ref().filter(|t| !t.is_empty()) {
                let base = params.len();
                let ph = (1..=tags.len()).map(|i| format!("${}", base + i)).collect::<Vec<_>>().join(", ");
                conds.push(format!("EXISTS(SELECT 1 FROM giftcard_giftcard_tags gt JOIN giftcard_giftcardtag t ON t.id = gt.giftcardtag_id WHERE gt.giftcard_id = g.id AND t.name IN ({ph}))"));
                params.extend(tags.iter().map(|s| s.clone().into()));
            }
        }
        let desc = matches!(sort_by.as_ref().map(|s| &s.direction), Some(gen::OrderDirection::DESC));
        let order = match sort_by.as_ref().map(|s| &s.field) {
            Some(gen::GiftCardSortField::CURRENTBALANCE) if desc => "g.current_balance_amount DESC, g.id",
            Some(gen::GiftCardSortField::CURRENTBALANCE) => "g.current_balance_amount ASC, g.id",
            Some(gen::GiftCardSortField::CREATEDAT) if desc => "g.created_at DESC, g.id",
            Some(gen::GiftCardSortField::CREATEDAT) => "g.created_at ASC, g.id",
            _ if desc => "g.id DESC",
            _ => "g.id ASC",
        };
        let where_sql = if conds.is_empty() { String::new() } else { format!("WHERE {}", conds.join(" AND ")) };
        let rows = db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT g.id, g.code, g.is_active, g.expiry_date, g.current_balance_amount, g.currency, g.product_id, g.assigned_to_email FROM giftcard_giftcard g {where_sql} ORDER BY {order}"),
            params,
        )).await.map_err(|e| Error::new(e.to_string()))?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        let page: Vec<(i32, String, bool, Option<chrono::DateTime<chrono::Utc>>, rust_decimal::Decimal, String, Option<i32>, Option<String>)> =
            rows.into_iter().filter_map(|r| {
                let exp: Option<chrono::DateTime<chrono::Utc>> = r.try_get::<Option<chrono::NaiveDate>>("", "expiry_date").ok().flatten()
                    .and_then(|d| d.and_hms_opt(0, 0, 0)).map(|n| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(n, chrono::Utc));
                Some((r.try_get::<i32>("", "id").ok()?,
                      r.try_get::<String>("", "code").ok()?,
                      r.try_get::<bool>("", "is_active").ok()?,
                      exp,
                      r.try_get::<rust_decimal::Decimal>("", "current_balance_amount").ok()?,
                      r.try_get::<String>("", "currency").ok()?,
                      r.try_get::<Option<i32>>("", "product_id").ok().flatten(),
                      r.try_get::<Option<String>>("", "assigned_to_email").ok().flatten()))
            }).collect();
        // product names + tags, batched
        let pids: Vec<i32> = page.iter().filter_map(|t| t.6).collect();
        let mut pnames: HashMap<i32, String> = HashMap::new();
        if !pids.is_empty() {
            let list = (1..=pids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
            for r in db.query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT id, name FROM product_product WHERE id IN ({list})"),
                pids.iter().map(|i| (*i).into()).collect::<Vec<sea_orm::Value>>(),
            )).await.map_err(|e| Error::new(e.to_string()))? {
                if let (Ok(id), Ok(name)) = (r.try_get::<i32>("", "id"), r.try_get::<String>("", "name")) {
                    pnames.insert(id, name);
                }
            }
        }
        let gids: Vec<i32> = page.iter().map(|t| t.0).collect();
        let mut tags: HashMap<i32, Vec<gen::GiftCardTag>> = HashMap::new();
        if !gids.is_empty() {
            let list = (1..=gids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
            for r in db.query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT gt.giftcard_id, t.id, t.name FROM giftcard_giftcard_tags gt JOIN giftcard_giftcardtag t ON t.id = gt.giftcardtag_id WHERE gt.giftcard_id IN ({list})"),
                gids.iter().map(|i| (*i).into()).collect::<Vec<sea_orm::Value>>(),
            )).await.map_err(|e| Error::new(e.to_string()))? {
                if let (Ok(gid), Ok(tid), Ok(name)) = (r.try_get::<i32>("", "giftcard_id"), r.try_get::<i32>("", "id"), r.try_get::<String>("", "name")) {
                    tags.entry(gid).or_default().push(gen::GiftCardTag {
                        id: Some(ID(crate::common::gid("GiftCardTag", tid))),
                        name: Some(name),
                    });
                }
            }
        }
        let all_count = page.len() as i32;
        let edges = page.into_iter().skip(off).take(lim).map(|(id, code, active, exp, bal, cur, pid, email)| {
            let last4: String = code.chars().rev().take(4).collect::<String>().chars().rev().collect();
            let product = pid.map(|p| {
                let mut pr = crate::metadata::lit_product(crate::common::gid("Product", p), vec![], vec![]);
                pr.name = pnames.get(&p).cloned();
                pr
            });
            let mut gc = crate::metadata::lit_gift_card(crate::common::gid("GiftCard", id), vec![], vec![]);
            gc.last4_code_chars = Some(last4);
            gc.assigned_to_email = email;
            gc.is_active = Some(active);
            gc.expiry_date = exp;
            gc.product = product;
            gc.tags = tags.get(&id).cloned().unwrap_or_default();
            gc.current_balance = Some(crate::common::Money { amount: bal.to_string(), currency: cur, fraction_digits: None });
            gen::GiftCardCountableEdge { node: Some(gc) }
        }).collect();
        Ok(Some(gen::GiftCardCountableConnection {
            page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
            edges,
            total_count: Some(all_count),
        }))
    }

    /// Dashboard channels list fetches FULL `...ChannelDetails` per channel
    /// (warehouses drive the setup banner), so the list is fully assembled.
    async fn channels(&self, ctx: &Context<'_>) -> Result<Vec<gen::Channel>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{EntityTrait, QuerySelect};
        type Ch = saleor_rustify_db::entities::channel_channel::Entity;
        use saleor_rustify_db::entities::channel_channel::Column as ChCol;
        let ids: Vec<i32> = Ch::find()
            .select_only().column(ChCol::Id)
            .into_tuple::<i32>().all(db).await.map_err(|e| Error::new(e.to_string()))?;
        let mut out = vec![];
        for id in ids {
            if let Some(c) = assemble_channel(db, id).await.map_err(|e| Error::new(e.to_string()))? {
                out.push(c);
            }
        }
        Ok(out)
    }

    /// Saleor `channel(id, slug)` — details page (was a None stub → the
    /// dashboard rendered its NotFound page, including `?action=setup`).
    async fn channel(&self, ctx: &Context<'_>, id: Option<ID>, slug: Option<String>) -> Result<Option<gen::Channel>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let cid: Option<i32> = if let Some(i) = id {
            saleor_rustify_db::catalog::parse_gid(&i.0)
        } else if let Some(s) = slug {
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
            saleor_rustify_db::entities::channel_channel::Entity::find()
                .select_only().column(saleor_rustify_db::entities::channel_channel::Column::Id)
                .filter(saleor_rustify_db::entities::channel_channel::Column::Slug.eq(s))
                .into_tuple::<i32>().one(db).await.map_err(|e| Error::new(e.to_string()))?
        } else { None };
        match cid {
            Some(id) => Ok(assemble_channel(db, id).await.map_err(|e| Error::new(e.to_string()))?),
            None => Ok(None),
        }
    }

    async fn warehouses(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        filter: Option<gen::WarehouseFilterInput>, #[graphql(name = "sortBy")] sort_by: Option<gen::WarehouseSortingInput>,
    ) -> Result<gen::WarehouseCountableConnection> {
        let _ = (before, last, filter, sort_by);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(100).clamp(1, 100) as usize;
        let rows = saleor_rustify_db::commerce::list_warehouses(db, "default-channel").await.map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len() as i32;
        let edges = rows.into_iter().skip(off).take(lim).map(|w| gen::WarehouseCountableEdge { node: Some(gen::Warehouse {
            id: Some(ID(crate::common::gid("Warehouse", &w.id))),
            private_metadata: vec![],
            metadata: vec![],
            name: Some(w.name),
            slug: None,
            email: None,
            is_private: None,
            address: None,
            click_and_collect_option: None,
        })}).collect();
        Ok(gen::WarehouseCountableConnection { total_count: Some(total), edges, page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })
    }

    async fn menus(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        channel: Option<String>, filter: Option<gen::MenuFilterInput>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::MenuSortingInput>,
    ) -> Result<gen::MenuCountableConnection> {
        let _ = (before, last, channel, filter, sort_by);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::EntityTrait;
        let rows = saleor_rustify_db::entities::menu_menu::Entity::find().all(db).await.map_err(|e| Error::new(e.to_string()))?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(100).clamp(1, 100) as usize;
        let edges = rows.into_iter().skip(off).take(lim).map(|m| gen::MenuCountableEdge { node: Some(gen::Menu {
            id: Some(ID(crate::common::gid("Menu", m.id))),
            private_metadata: vec![],
            metadata: vec![],
            name: Some(m.name),
            items: vec![],
        })}).collect();
        Ok(gen::MenuCountableConnection { edges, page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })
    }

    async fn tax_classes(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        filter: Option<gen::TaxClassFilterInput>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::TaxClassSortingInput>,
    ) -> Result<gen::TaxClassCountableConnection> {
        let _ = (before, last, filter, sort_by);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(100).clamp(1, 100) as usize;
        let rows = saleor_rustify_db::commerce::list_tax_classes(db).await.map_err(|e| Error::new(e.to_string()))?;
        let edges = rows.into_iter().skip(off).take(lim).map(|(id, name)| gen::TaxClassCountableEdge { node: Some(gen::TaxClass {
            id: Some(ID(crate::common::gid("TaxClass", id))),
            private_metadata: vec![],
            metadata: vec![],
            name: Some(name),
            countries: vec![],
        })}).collect();
        Ok(gen::TaxClassCountableConnection { edges, page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })
    }

    /// Saleor `stock(id)` / `stocks` — warehouse stock rows with channel
    /// visibility inherited from the variant (dashboard inventory views).
    async fn stock(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::Stock>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(sid) = saleor_rustify_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        stock_node(db, sid).await.map_err(Error::new)
    }

    async fn stocks(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        filter: Option<gen::StockFilterInput>,
    ) -> Result<Option<GqlStockConnection>> {
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use sea_orm::{ConnectionTrait, Statement};
        let mut conds: Vec<String> = vec![];
        let mut params: Vec<sea_orm::Value> = vec![];
        if let Some(f) = filter.as_ref() {
            if let Some(q) = f.quantity {
                params.push((q as i64).into());
                conds.push(format!("s.quantity = ${}", params.len()));
            }
            if let Some(s) = f.search.as_ref().filter(|s| !s.trim().is_empty()) {
                params.push(format!("%{s}%").into());
                let p = params.len();
                conds.push(format!("(v.sku ILIKE ${p} OR p.name ILIKE ${p})"));
            }
        }
        let where_sql = if conds.is_empty() { String::new() } else { format!("WHERE {}", conds.join(" AND ")) };
        let rows = db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT s.id FROM warehouse_stock s JOIN product_productvariant v ON v.id = s.product_variant_id JOIN product_product p ON p.id = v.product_id {where_sql} ORDER BY s.id"),
            params,
        )).await.map_err(|e| Error::new(e.to_string()))?;
        let ids: Vec<i32> = rows.into_iter().filter_map(|r| r.try_get::<i32>("", "id").ok()).collect();
        let total = ids.len();
        let mut edges = vec![];
        for sid in ids.into_iter().skip(off).take(lim) {
            if let Some(node) = stock_node(db, sid).await.map_err(Error::new)? {
                edges.push(GqlStockEdge { node: Some(node) });
            }
        }
        Ok(Some(GqlStockConnection {
            page_info: crate::common::PageInfo { has_next_page: off + lim < total, has_previous_page: off > 0, start_cursor: None, end_cursor: None },
            edges: edges.into_iter().map(|e| GqlStockEdge { node: e.node }).collect(),
            total_count: Some(total as i32),
        }))
    }

    async fn tax_class(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::TaxClass>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(tid) = saleor_rustify_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        let rows = saleor_rustify_db::commerce::list_tax_classes(db).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().find(|(i, _)| *i == tid).map(|(id, name)| {
            let mut t = crate::metadata::lit_tax_class(crate::common::gid("TaxClass", id), vec![], vec![]);
            t.name = Some(name);
            t
        }))
    }

    async fn tax_configuration(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::TaxConfiguration>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(tid) = saleor_rustify_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        tax_config_node(db, tid).await.map_err(Error::new)
    }

    async fn tax_country_configuration(&self, ctx: &Context<'_>, #[graphql(name = "countryCode")] country_code: gen::CountryCode) -> Result<Option<gen::TaxCountryConfiguration>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let code = format!("{country_code:?}");
        use sea_orm::{ConnectionTrait, Statement};
        let rows = db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT r.country, r.tax_class_id, c.name AS class_name, r.rate FROM tax_taxclasscountryrate r LEFT JOIN tax_taxclass c ON c.id = r.tax_class_id WHERE r.country = $1 ORDER BY r.id",
            [code.clone().into()],
        )).await.map_err(|e| Error::new(e.to_string()))?;
        let mut rates = vec![];
        for r in rows {
            rates.push(gen::TaxClassCountryRate {
                country: Some(crate::common::GqlCountryDisplay { code: code.clone(), country: code.clone() }),
                tax_class: r.try_get::<Option<i32>>("", "tax_class_id").ok().flatten().map(|tc| {
                    let mut t = crate::metadata::lit_tax_class(crate::common::gid("TaxClass", tc), vec![], vec![]);
                    t.name = r.try_get::<Option<String>>("", "class_name").ok().flatten();
                    Box::new(t)
                }),
                rate: r.try_get::<rust_decimal::Decimal>("", "rate").ok().and_then(|d| d.to_string().parse::<f64>().ok()),
            });
        }
        Ok(Some(gen::TaxCountryConfiguration {
            country: Some(crate::common::GqlCountryDisplay { code: code.clone(), country: code }),
            tax_class_country_rates: rates,
        }))
    }

    /// Deprecated in Saleor (use `taxClasses`): tax classes as gateway types.
    async fn tax_types(&self, ctx: &Context<'_>) -> Result<Vec<GqlTaxType>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let rows = saleor_rustify_db::commerce::list_tax_classes(db).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(|(_, name)| GqlTaxType { description: Some(name.clone()), tax_code: Some(name) }).collect())
    }

    async fn gift_card_currencies(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{ConnectionTrait, Statement};
        let rows = db.query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT DISTINCT currency FROM giftcard_giftcard ORDER BY 1".to_string(),
        )).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().filter_map(|r| r.try_get::<String>("", "currency").ok()).collect())
    }

    async fn gift_card_tags(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        filter: Option<gen::GiftCardTagFilterInput>,
    ) -> Result<gen::GiftCardTagCountableConnection> {
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use sea_orm::{ConnectionTrait, Statement};
        let (sql, params) = match filter.as_ref().and_then(|f| f.search.clone()).filter(|s| !s.trim().is_empty()) {
            Some(s) => ("SELECT id, name FROM giftcard_giftcardtag WHERE name ILIKE $1 ORDER BY name".to_string(), vec![format!("%{s}%").into()]),
            None => ("SELECT id, name FROM giftcard_giftcardtag ORDER BY name".to_string(), vec![]),
        };
        let rows = db.query_all(Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params))
            .await.map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len();
        let edges = rows.into_iter().skip(off).take(lim).filter_map(|r| {
            Some(gen::GiftCardTagCountableEdge { node: Some(gen::GiftCardTag {
                id: Some(ID(crate::common::gid("GiftCardTag", r.try_get::<i32>("", "id").ok()?))),
                name: Some(r.try_get::<String>("", "name").ok()?),
            }) })
        }).collect();
        Ok(gen::GiftCardTagCountableConnection {
            page_info: Some(crate::common::PageInfo { has_next_page: off + lim < total, has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
            edges,
            total_count: Some(total as i32),
        })
    }

    async fn gift_card_settings(&self, ctx: &Context<'_>) -> Result<gen::GiftCardSettings> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{ConnectionTrait, Statement};
        let row = db.query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT gift_card_expiry_type, gift_card_expiry_period, gift_card_expiry_period_type FROM site_sitesettings WHERE id = 1".to_string(),
        )).await.map_err(|e| Error::new(e.to_string()))?;
        let et = row.as_ref().and_then(|r| r.try_get::<String>("", "gift_card_expiry_type").ok()).unwrap_or_else(|| "never_expire".into());
        let expiry_type = match et.as_str() {
            "expiry_period" => "EXPIRY_PERIOD",
            _ => "NEVER_EXPIRE",
        }.to_string();
        let period = row.as_ref().and_then(|r| r.try_get::<Option<i32>>("", "gift_card_expiry_period").ok().flatten())
            .map(|n| gen::TimePeriod {
                amount: Some(n),
                r#type: row.as_ref().and_then(|r| r.try_get::<Option<String>>("", "gift_card_expiry_period_type").ok().flatten()),
            });
        Ok(gen::GiftCardSettings { expiry_type: Some(expiry_type), expiry_period: period })
    }

    async fn menu_item(&self, ctx: &Context<'_>, id: ID, channel: Option<String>) -> Result<Option<gen::MenuItem>> {
        let _ = channel;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(mid) = saleor_rustify_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        menu_item_node(db, mid).await.map_err(Error::new)
    }

    async fn menu_items(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        channel: Option<String>, #[graphql(name = "sortBy")] sort_by: Option<gen::MenuItemSortingInput>,
        filter: Option<gen::MenuItemFilterInput>,
    ) -> Result<Option<GqlMenuItemConnection>> {
        let _ = (before, last, channel, sort_by);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use sea_orm::{ConnectionTrait, Statement};
        let (sql, params) = match filter.as_ref().and_then(|f| f.search.clone()).filter(|s| !s.trim().is_empty()) {
            Some(s) => ("SELECT id FROM menu_menuitem WHERE name ILIKE $1 ORDER BY id".to_string(), vec![format!("%{s}%").into()]),
            None => ("SELECT id FROM menu_menuitem ORDER BY id".to_string(), vec![]),
        };
        let rows = db.query_all(Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params))
            .await.map_err(|e| Error::new(e.to_string()))?;
        let ids: Vec<i32> = rows.into_iter().filter_map(|r| r.try_get::<i32>("", "id").ok()).collect();
        let total = ids.len();
        let mut edges = vec![];
        for mid in ids.into_iter().skip(off).take(lim) {
            if let Some(node) = menu_item_node(db, mid).await.map_err(Error::new)? {
                edges.push(GqlMenuItemEdge { node: Some(node) });
            }
        }
        Ok(Some(GqlMenuItemConnection {
            page_info: crate::common::PageInfo { has_next_page: off + lim < total, has_previous_page: off > 0, start_cursor: None, end_cursor: None },
            edges: edges.into_iter().map(|e| GqlMenuItemEdge { node: e.node }).collect(),
            total_count: Some(total as i32),
        }))
    }

    async fn shipping_methods(&self, ctx: &Context<'_>, channel: Option<String>) -> Result<Vec<GqlShippingMethod>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let rows = saleor_rustify_db::commerce::list_shipping_methods(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(|m| GqlShippingMethod { id: ID(crate::common::gid("ShippingMethod", m.id)), name: m.name, price: m.price_amount.to_string() }).collect())
    }

    async fn pages(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        filter: Option<gen::PageFilterInput>, #[graphql(name = "sortBy")] sort_by: Option<gen::PageSortingInput>, #[graphql(name = "where")] where_input: Option<gen::PageWhereInput>, search: Option<String>,
    ) -> Result<gen::PageCountableConnection> {
        let _ = (before, last, filter, sort_by, where_input, search);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(100).clamp(1, 100) as usize;
        let rows = saleor_rustify_db::commerce::list_pages(db).await.map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len() as i32;
        let edges = rows.into_iter().skip(off).take(lim).map(|p| gen::PageCountableEdge { node: Some(gen::Page {
            id: Some(ID(crate::common::gid("Page", p.id))),
            private_metadata: vec![],
            metadata: vec![],
            seo_title: None,
            seo_description: None,
            title: Some(p.title),
            content: None,
            published_at: None,
            is_published: Some(p.is_published),
            slug: Some(p.slug),
            page_type: None,
            attributes: vec![],
        })}).collect();
        Ok(gen::PageCountableConnection { total_count: Some(total), edges, page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })
    }

    async fn promotions(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        channel: Option<String>, #[graphql(name = "where")] where_input: Option<gen::PromotionWhereInput>, #[graphql(name = "sortBy")] sort_by: Option<gen::PromotionSortingInput>,
    ) -> Result<gen::PromotionCountableConnection> {
        let _ = (before, last, where_input, sort_by);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(100).clamp(1, 100) as usize;
        let rows = saleor_rustify_db::commerce::list_promotions(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        let edges = rows.into_iter().skip(off).take(lim).map(|p| gen::PromotionCountableEdge { node: Some(gen::Promotion {
            id: Some(ID(crate::common::gid("Promotion", &p.id))),
            private_metadata: vec![],
            metadata: vec![],
            name: Some(p.name),
            // Dashboard switches on PromotionTypeEnum (CATALOGUE/ORDER) and
            // throws on anything else — DB stores lowercase.
            r#type: Some(p.promotion_type.to_uppercase()),
            description: None,
            start_date: None,
            end_date: None,
            rules: vec![],
        })}).collect();
        Ok(gen::PromotionCountableConnection { edges, page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })
    }

    /// Saleor `promotion(id)` — full details for DiscountDetails
    /// (`/discounts/sales/:id` routes here; the list root stays slim).
    async fn promotion(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::Promotion>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(pid) = crate::common::parse_uuid_gid(&id.0) else { return Ok(None) };
        Ok(assemble_promotion(db, pid).await.map_err(|e| Error::new(e.to_string()))?)
    }

    /// Saleor `menu(channel, id, name, slug)` — resolve by id, slug, or name.
    async fn menu(&self, ctx: &Context<'_>, channel: Option<String>, id: Option<ID>, name: Option<String>, slug: Option<String>) -> Result<Option<gen::Menu>> {
        let _ = channel;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let resolved_slug: Option<String> = if let Some(s) = slug { Some(s) } else if let Some(n) = name {
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
            saleor_rustify_db::entities::menu_menu::Entity::find()
                .filter(saleor_rustify_db::entities::menu_menu::Column::Name.eq(n))
                .one(db).await.map_err(|e| Error::new(e.to_string()))?.map(|m| m.slug)
        } else if let Some(i) = id {
            use sea_orm::EntityTrait;
            let pk: i32 = saleor_rustify_db::catalog::parse_gid(&i.0).unwrap_or(-1);
            saleor_rustify_db::entities::menu_menu::Entity::find_by_id(pk).one(db).await.map_err(|e| Error::new(e.to_string()))?.map(|m| m.slug)
        } else { None };
        let m = match resolved_slug {
            Some(s) => saleor_rustify_db::commerce::get_menu(db, &s).await.map_err(|e| Error::new(e.to_string()))?,
            None => None,
        };
        Ok(m.map(|x| gen::Menu {
            id: Some(ID(crate::common::gid("Menu", &x.id))),
            private_metadata: vec![],
            metadata: vec![],
            name: Some(x.name),
            items: vec![],
        }))
    }
}

/// Site name/description from the shared Postgres rows Django reads
/// (`django_site` + `site_sitesettings`). Falls back to demo values offline.
async fn site_info(ctx: &Context<'_>) -> (String, Option<String>) {
    let fallback = ("Test Saleor - a sample shop!".to_string(), None);
    let Ok(g) = ctx.data::<GqlContext>() else { return fallback };
    let Ok(db) = g.db() else { return fallback };
    // Slim selects only (never SELECT * — tsvector/interval columns break SeaORM).
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let site: Option<String> = saleor_rustify_db::entities::django_site::Entity::find()
        .select_only().column(saleor_rustify_db::entities::django_site::Column::Name)
        .filter(saleor_rustify_db::entities::django_site::Column::Id.eq(1))
        .into_tuple().one(db).await.ok().flatten();
    let desc: Option<String> = saleor_rustify_db::entities::site_sitesettings::Entity::find()
        .select_only().column(saleor_rustify_db::entities::site_sitesettings::Column::Description)
        .into_tuple().one(db).await.ok().flatten();
    (
        site.unwrap_or(fallback.0),
        desc.or(fallback.1),
    )
}

/// Shop metadata from `site_sitesettings.metadata` (JSON dict) — the same
/// row Django reads. Slim select only.
async fn site_metadata(ctx: &Context<'_>) -> Vec<crate::common::MetadataItem> {
    let Ok(g) = ctx.data::<GqlContext>() else { return vec![] };
    let Ok(db) = g.db() else { return vec![] };
    use sea_orm::{EntityTrait, QuerySelect};
    let v: Option<serde_json::Value> = saleor_rustify_db::entities::site_sitesettings::Entity::find()
        .select_only()
        .column(saleor_rustify_db::entities::site_sitesettings::Column::Metadata)
        .into_tuple()
        .one(db)
        .await
        .ok()
        .flatten();
    v.map(|x| crate::common::json_to_metadata_items(&x)).unwrap_or_default()
}
/// Human-readable permission list for `shop { permissions }` — same
/// `permission_permission` rows Django's `format_permissions_for_display` uses.
async fn all_permissions(ctx: &Context<'_>) -> Vec<GqlPermission> {
    let Ok(g) = ctx.data::<GqlContext>() else { return vec![] };
    let Ok(db) = g.db() else { return vec![] };
    use sea_orm::{EntityTrait, QueryOrder, QuerySelect};
    let rows = saleor_rustify_db::entities::permission_permission::Entity::find()
        .select_only()
        .column(saleor_rustify_db::entities::permission_permission::Column::Codename)
        .column(saleor_rustify_db::entities::permission_permission::Column::Name)
        .order_by_asc(saleor_rustify_db::entities::permission_permission::Column::Codename)
        .into_tuple::<(String, String)>()
        .all(db).await.unwrap_or_default();
    rows.into_iter().map(|(code, name)| GqlPermission { code: crate::common::permission_enum_code(&code), name }).collect()
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "ShopSettingsUpdate")]
pub struct GqlShopSettingsUpdate {
    pub shop: Option<gen::Shop>,
    pub errors: Vec<gen::ShopError>,
}

#[derive(Default)]
pub struct CommerceMutation;

/// Raw-SQL execute helper for zone membership edits (table-driven, no entities).
async fn zexec(
    db: &impl sea_orm::ConnectionTrait,
    sql: String,
    params: Vec<sea_orm::Value>,
) -> Result<sea_orm::ExecResult, sea_orm::DbErr> {
    db.execute(sea_orm::Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params)).await
}

#[Object]
impl CommerceMutation {
    /// Dashboard shipping-zone editor (channel setup completion path):
    /// rename/describe/retarget countries + add/remove channels + warehouses.
    /// Methods/rates have their own mutations; only membership edits land here.
    async fn shipping_zone_update(
        &self, ctx: &Context<'_>, id: ID, input: gen::ShippingZoneUpdateInput,
    ) -> Result<gen::ShippingZoneUpdate> {
        let _ = crate::account::require_perm(ctx, "manage_shipping").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let serr = |m: String| gen::ShippingError { field: None, message: Some(m), code: None, channels: vec![] };
        let Some(zid) = saleor_rustify_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::ShippingZoneUpdate { errors: vec![serr("bad zone id".into())], shipping_zone: None });
        };
        use sea_orm::TransactionTrait;
        let txn = db.begin().await.map_err(|e| Error::new(e.to_string()))?;
        // scalar edits
        let mut params: Vec<sea_orm::Value> = vec![zid.into()];
        let mut sets: Vec<String> = vec![];
        let mut push = |col: &str, v: sea_orm::Value| {
            params.push(v);
            sets.push(format!("{col} = ${}", params.len()));
        };
        if let Some(v) = input.name.as_deref() { push("name", v.to_string().into()); }
        if let Some(v) = input.description.as_deref() { push("description", v.to_string().into()); }
        if let Some(v) = input.countries.as_ref() { push("countries", v.join(",").into()); }
        if let Some(v) = input.default { push("\"default\"", v.into()); }
        if !sets.is_empty() {
            if let Err(e) = zexec(&txn, format!("UPDATE shipping_shippingzone SET {} WHERE id = $1", sets.join(", ")), params).await {
                return Ok(gen::ShippingZoneUpdate { errors: vec![serr(e.to_string())], shipping_zone: None });
            }
        }
        // membership edits (dashboard sends globals; channels are int pks)
        let ch_ids = |ids: &Option<Vec<ID>>| ids.as_ref().map(|v| v.iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect::<Vec<_>>()).unwrap_or_default();
        for cid in ch_ids(&input.add_channels) {
            if let Err(e) = zexec(&txn, "INSERT INTO shipping_shippingzone_channels (shippingzone_id, channel_id) VALUES ($1, $2) ON CONFLICT DO NOTHING".into(),
                vec![zid.into(), cid.into()]).await {
                return Ok(gen::ShippingZoneUpdate { errors: vec![serr(e.to_string())], shipping_zone: None });
            }
        }
        for cid in ch_ids(&input.remove_channels) {
            if let Err(e) = zexec(&txn, "DELETE FROM shipping_shippingzone_channels WHERE shippingzone_id = $1 AND channel_id = $2".into(),
                vec![zid.into(), cid.into()]).await {
                return Ok(gen::ShippingZoneUpdate { errors: vec![serr(e.to_string())], shipping_zone: None });
            }
        }
        // warehouses are uuid pks
        let wh_ids = |ids: &Option<Vec<ID>>| ids.as_ref().map(|v| v.iter().filter_map(|i| crate::common::parse_uuid_gid(&i.0).map(|u| u.to_string())).collect::<Vec<_>>()).unwrap_or_default();
        for wid in wh_ids(&input.add_warehouses) {
            if let Err(e) = zexec(&txn, "INSERT INTO warehouse_warehouse_shipping_zones (shippingzone_id, warehouse_id) VALUES ($1, $2::uuid) ON CONFLICT DO NOTHING".into(),
                vec![zid.into(), wid.into()]).await {
                return Ok(gen::ShippingZoneUpdate { errors: vec![serr(e.to_string())], shipping_zone: None });
            }
        }
        for wid in wh_ids(&input.remove_warehouses) {
            if let Err(e) = zexec(&txn, "DELETE FROM warehouse_warehouse_shipping_zones WHERE shippingzone_id = $1 AND warehouse_id = $2::uuid".into(),
                vec![zid.into(), wid.into()]).await {
                return Ok(gen::ShippingZoneUpdate { errors: vec![serr(e.to_string())], shipping_zone: None });
            }
        }
        txn.commit().await.map_err(|e| Error::new(e.to_string()))?;
        match assemble_zone(db, zid).await {
            Ok(z) => Ok(gen::ShippingZoneUpdate { errors: vec![], shipping_zone: z }),
            Err(e) => Ok(gen::ShippingZoneUpdate { errors: vec![serr(e)], shipping_zone: None }),
        }
    }

    /// Dashboard taxes → channels Save button (Django `TaxConfigurationUpdate`):
    /// scalar patch plus per-country upserts/removals on the channel's row.
    async fn tax_configuration_update(        &self, ctx: &Context<'_>, id: ID, input: gen::TaxConfigurationUpdateInput,
    ) -> Result<gen::TaxConfigurationUpdate> {
        let _ = crate::account::require_perm(ctx, "manage_taxes").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let terr = |m: String| gen::TaxConfigurationUpdateError { field: None, message: Some(m), code: None };
        let Some(tid) = saleor_rustify_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::TaxConfigurationUpdate { errors: vec![terr("bad tax configuration id".into())] });
        };
        let strategy = input.tax_calculation_strategy.as_ref().map(|s| match format!("{s:?}").as_str() {
            "TAXAPP" => "TAX_APP".to_string(),
            _ => "FLAT_RATES".to_string(),
        });
        let mut patch = saleor_rustify_db::taxes::TaxConfigPatch {
            charge_taxes: input.charge_taxes,
            strategy,
            display_gross: input.display_gross_prices,
            prices_entered_with_tax: input.prices_entered_with_tax,
            use_weighted_tax_for_shipping: input.use_weighted_tax_for_shipping,
            tax_app_id: input.tax_app_id.clone(),
            upsert_countries: vec![],
            remove_countries: input.remove_countries_configuration.as_ref().map(|v| v.as_slice()).unwrap_or(&[])
                .iter().map(|c| format!("{c:?}")).collect(),
        };
        for c in input.update_countries_configuration.as_ref().map(|v| v.as_slice()).unwrap_or(&[]) {
            patch.upsert_countries.push(saleor_rustify_db::taxes::CountryOverride {
                country_code: format!("{:?}", c.country_code),
                charge_taxes: c.charge_taxes,
                strategy: c.tax_calculation_strategy.as_ref().map(|s| match format!("{s:?}").as_str() {
                    "TAXAPP" => "TAX_APP".to_string(),
                    _ => "FLAT_RATES".to_string(),
                }),
                display_gross: c.display_gross_prices,
                tax_app_id: c.tax_app_id.clone(),
                use_weighted_tax_for_shipping: c.use_weighted_tax_for_shipping.unwrap_or(false),
            });
        }
        match saleor_rustify_db::taxes::update_tax_configuration(db, tid, &patch).await {
            Ok(()) => Ok(gen::TaxConfigurationUpdate { errors: vec![] }),
            Err(e) => Ok(gen::TaxConfigurationUpdate { errors: vec![terr(e.to_string())] }),
        }
    }

    /// Dashboard shop settings + navigation pins (`ShopSettingsUpdate`,
    /// `UpdateShopNavigationPins`, `OrderSettingsUpdate`,
    /// `UpdateDefaultWeightUnit`): Saleor's `ShopSettingsInput` in,
    /// `{ shop errors }` out. Metadata merges into
    /// `site_sitesettings.metadata` (same JSON dict Django uses); name and
    /// description persist to the real site rows; remaining scalars are
    /// accepted-ignored until their domain ports land.
    async fn shop_settings_update(
        &self,
        ctx: &Context<'_>,
        input: gen::ShopSettingsInput,
    ) -> Result<GqlShopSettingsUpdate> {
        let _ = crate::account::require_perm(ctx, "manage_settings").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
        if let Some(meta) = input.metadata.clone().or(input.private_metadata.clone()) {
            if let Some(row) = saleor_rustify_db::entities::site_sitesettings::Entity::find()
                .filter(saleor_rustify_db::entities::site_sitesettings::Column::Id.eq(1))
                .one(db)
                .await
                .map_err(|e| Error::new(e.to_string()))?
            {
                let merged = crate::common::merge_metadata(
                    &serde_json::to_value(&row.metadata).unwrap_or(serde_json::Value::Null),
                    &meta,
                );
                let mut am: saleor_rustify_db::entities::site_sitesettings::ActiveModel = row.into();
                if input.private_metadata.is_some() {
                    am.private_metadata = Set(merged.clone());
                } else {
                    am.metadata = Set(merged);
                }
                am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            }
        }
        if let Some(desc) = input.description.clone() {
            if let Some(row) = saleor_rustify_db::entities::site_sitesettings::Entity::find()
                .filter(saleor_rustify_db::entities::site_sitesettings::Column::Id.eq(1))
                .one(db).await.map_err(|e| Error::new(e.to_string()))?
            {
                let mut am: saleor_rustify_db::entities::site_sitesettings::ActiveModel = row.into();
                am.description = Set(desc);
                am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            }
        }
        if let Some(site_name) = input.name.clone() {
            if let Some(row) = saleor_rustify_db::entities::django_site::Entity::find_by_id(1)
                .one(db).await.map_err(|e| Error::new(e.to_string()))?
            {
                let mut am: saleor_rustify_db::entities::django_site::ActiveModel = row.into();
                am.name = Set(site_name);
                am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            }
        }
        let shop = to_gen_shop(ctx).await?;
        Ok(GqlShopSettingsUpdate { shop: Some(shop), errors: vec![] })
    }

    /// Bulk stock set (Django `stockBulkUpdate`): variant × warehouse →
    /// quantity, per-row errors, `count` of applied rows.
    async fn stock_bulk_update(
        &self, ctx: &Context<'_>,
        #[graphql(name = "errorPolicy")] error_policy: Option<gen::ErrorPolicyEnum>,
        stocks: Vec<gen::StockBulkUpdateInput>,
    ) -> Result<GqlStockBulkUpdate> {
        let _ = error_policy;
        let _ = crate::account::require_perm(ctx, "manage_products").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let mut results = vec![];
        let mut n = 0;
        for s in &stocks {
            let vid = match resolve_stock_variant(db, s.variant_id.as_ref(), s.variant_external_reference.as_ref()).await {
                Ok(v) => v,
                Err(e) => { results.push(GqlStockBulkResult { stock: None, errors: vec![werr(None, e)] }); continue; }
            };
            let wid = match resolve_stock_warehouse(db, s.warehouse_id.as_ref(), s.warehouse_external_reference.as_ref()).await {
                Ok(v) => v,
                Err(e) => { results.push(GqlStockBulkResult { stock: None, errors: vec![werr(None, e)] }); continue; }
            };
            match saleor_rustify_db::catalog_writes::set_variant_stock(db, vid, wid, s.quantity).await {
                Ok(()) => {
                    n += 1;
                    let node = stock_id_for(db, vid, wid).await.and_then(|sid| Some(sid));
                    let mut r = GqlStockBulkResult { stock: None, errors: vec![] };
                    if let Some(sid) = node {
                        r.stock = stock_node(db, sid).await.unwrap_or(None);
                    }
                    results.push(r);
                }
                Err(e) => results.push(GqlStockBulkResult { stock: None, errors: vec![werr(None, e.to_string())] }),
            }
        }
        Ok(GqlStockBulkUpdate { count: n, results, errors: vec![] })
    }

    async fn assign_warehouse_shipping_zone(&self, ctx: &Context<'_>, id: ID, #[graphql(name = "shippingZoneIds")] shipping_zone_ids: Vec<ID>) -> Result<GqlWarehouseZoneAssign> {
        let _ = crate::account::require_perm(ctx, "manage_shipping").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(wid) = crate::common::parse_uuid_gid(&id.0) else {
            return Ok(GqlWarehouseZoneAssign { warehouse: None, errors: vec![werr(Some("id".into()), "bad warehouse id".into())] });
        };
        let zids: Vec<i32> = shipping_zone_ids.iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect();
        if let Err(e) = zone_links(db, wid, &zids, true).await {
            return Ok(GqlWarehouseZoneAssign { warehouse: None, errors: vec![werr(None, e)] });
        }
        Ok(GqlWarehouseZoneAssign { warehouse: warehouse_gen(db, wid).await.unwrap_or(None), errors: vec![] })
    }

    async fn unassign_warehouse_shipping_zone(&self, ctx: &Context<'_>, id: ID, #[graphql(name = "shippingZoneIds")] shipping_zone_ids: Vec<ID>) -> Result<GqlWarehouseZoneUnassign> {
        let _ = crate::account::require_perm(ctx, "manage_shipping").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(wid) = crate::common::parse_uuid_gid(&id.0) else {
            return Ok(GqlWarehouseZoneUnassign { warehouse: None, errors: vec![werr(Some("id".into()), "bad warehouse id".into())] });
        };
        let zids: Vec<i32> = shipping_zone_ids.iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect();
        if let Err(e) = zone_links(db, wid, &zids, false).await {
            return Ok(GqlWarehouseZoneUnassign { warehouse: None, errors: vec![werr(None, e)] });
        }
        Ok(GqlWarehouseZoneUnassign { warehouse: warehouse_gen(db, wid).await.unwrap_or(None), errors: vec![] })
    }

    /// Adjust a gift card balance (Django `giftCardBalanceAdjust` → new
    /// balance + event row via `adjust_balance`).
    async fn gift_card_balance_adjust(&self, ctx: &Context<'_>, amount: rust_decimal::Decimal, id: ID) -> Result<GqlGiftCardBalanceAdjust> {
        let req = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlGiftCardBalanceAdjust { gift_card: None, errors: vec![gcerr(None, m)] };
        let Some(cid) = saleor_rustify_db::catalog::parse_gid(&id.0) else {
            return Ok(err("bad gift card id".into()));
        };
        let code: Option<String> = {
            use sea_orm::EntityTrait;
            saleor_rustify_db::entities::giftcard_giftcard::Entity::find_by_id(cid)
                .one(db).await.map_err(|e| Error::new(e.to_string()))?.map(|c| c.code)
        };
        let Some(code) = code else { return Ok(err("gift card not found".into())) };
        match saleor_rustify_db::giftcards::adjust_balance(db, &code, amount, Some(req)).await {
            Ok(_) => Ok(GqlGiftCardBalanceAdjust {
                gift_card: assemble_gift_card(db, cid).await.map_err(Error::new)?,
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    /// Full gift-card create (Django `giftCardCreate`): balance, custom
    /// code (validated unique), tags, note, assignment, metadata.
    async fn gift_card_create(&self, ctx: &Context<'_>, input: gen::GiftCardCreateInput) -> Result<gen::GiftCardCreate> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::GiftCardCreate { errors: vec![gcerr(None, m)], gift_card: None };
        let amount = input.balance.amount.0.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO);
        if amount <= rust_decimal::Decimal::ZERO {
            return Ok(err("balance must be positive".into()));
        }
        if let Some(em) = input.user_email.as_ref() {
            if em.trim().is_empty() || !em.contains('@') {
                return Ok(err("provided email is invalid".into()));
            }
            if input.channel.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                return Ok(err("channel slug must be specified when user_email is provided".into()));
            }
        }
        let expiry = input.expiry_date.as_ref().map(|d| d.date_naive());
        let card = match saleor_rustify_db::giftcards::issue(db, saleor_rustify_db::giftcards::IssueInput {
            initial_balance: amount,
            currency: input.balance.currency.clone(),
            created_by_email: input.user_email.clone(),
            expiry_date: expiry,
            is_active: input.is_active,
            custom_code: input.code.clone(),
        }, Some(staff)).await {
            Ok(c) => c,
            Err(e) => return Ok(err(e.to_string())),
        };
        if !input.add_tags.clone().unwrap_or_default().is_empty() {
            if let Err(e) = saleor_rustify_db::giftcards::add_tags(db, card.id, &input.add_tags.clone().unwrap_or_default()).await {
                return Ok(err(e.to_string()));
            }
        }
        if let Some(n) = input.note.as_ref().filter(|s| !s.trim().is_empty()) {
            if let Err(e) = saleor_rustify_db::giftcards::add_note(db, card.id, n, Some(staff)).await {
                return Ok(err(e.to_string()));
            }
        }
        if let Some(a) = input.assigned_to.as_ref() {
            let uid = saleor_rustify_db::catalog::parse_gid(&a.0).unwrap_or(-1);
            let em: Option<String> = {
                use sea_orm::{EntityTrait, QuerySelect};
                saleor_rustify_db::entities::account_user::Entity::find_by_id(uid)
                    .select_only()
                    .column(saleor_rustify_db::entities::account_user::Column::Email)
                    .into_tuple()
                    .one(db).await.map_err(|e| Error::new(e.to_string()))?
                    .unwrap_or(None)
            };
            if let Some(em) = em {
                if let Err(e) = saleor_rustify_db::giftcards::assign(db, &card.code, uid, &em, Some(staff)).await {
                    return Ok(err(e.to_string()));
                }
            }
        } else if let Some(em) = input.user_email.as_ref() {
            // Django links loose emails to a matching user when possible.
            let uid: Option<i32> = {
                use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
                saleor_rustify_db::entities::account_user::Entity::find()
                    .select_only()
                    .column(saleor_rustify_db::entities::account_user::Column::Id)
                    .filter(saleor_rustify_db::entities::account_user::Column::Email.eq(em.trim()))
                    .into_tuple()
                    .one(db).await.map_err(|e| Error::new(e.to_string()))?
                    .unwrap_or(None)
            };
            if let Some(uid) = uid {
                let _ = saleor_rustify_db::giftcards::assign(db, &card.code, uid, em, Some(staff)).await;
            }
        }
        // Metadata (dashboard sends it on create).
        if input.metadata.is_some() || input.private_metadata.is_some() {
            use sea_orm::{ActiveModelTrait, EntityTrait, Set};
            if let Some(row) = saleor_rustify_db::entities::giftcard_giftcard::Entity::find_by_id(card.id).one(db).await.map_err(|e| Error::new(e.to_string()))? {
                let mut am: saleor_rustify_db::entities::giftcard_giftcard::ActiveModel = row.into();
                if let Some(m) = input.metadata.as_ref() {
                    let cur = serde_json::to_value(&am.metadata.clone().unwrap()).unwrap_or(serde_json::Value::Null);
                    am.metadata = Set(crate::common::merge_metadata(&cur, m));
                }
                if let Some(m) = input.private_metadata.as_ref() {
                    let cur = serde_json::to_value(&am.private_metadata.clone().unwrap()).unwrap_or(serde_json::Value::Null);
                    am.private_metadata = Set(crate::common::merge_metadata(&cur, m));
                }
                am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            }
        }
        Ok(gen::GiftCardCreate {
            errors: vec![],
            gift_card: assemble_gift_card(db, card.id).await.map_err(Error::new)?,
        })
    }

    /// Gift-card update (Django `giftCardUpdate`): tags, expiry, balance,
    /// metadata. Activation rides activate/deactivate.
    async fn gift_card_update(&self, ctx: &Context<'_>, id: ID, input: gen::GiftCardUpdateInput) -> Result<gen::GiftCardUpdate> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::GiftCardUpdate { errors: vec![gcerr(None, m)], gift_card: None };
        let Some(cid) = saleor_rustify_db::catalog::parse_gid(&id.0) else {
            return Ok(err("bad gift card id".into()));
        };
        let balance = match input.balance_amount.as_ref() {
            Some(b) => match b.0.parse::<rust_decimal::Decimal>() {
                Ok(v) if v > rust_decimal::Decimal::ZERO => Some(v),
                _ => return Ok(err("balance must be positive".into())),
            },
            None => None,
        };
        let expiry = input.expiry_date.as_ref().map(|d| Some(d.date_naive()));
        match saleor_rustify_db::giftcards::update_card(db, cid, &saleor_rustify_db::giftcards::UpdateCard {
            add_tags: input.add_tags.clone().unwrap_or_default(),
            remove_tags: input.remove_tags.clone().unwrap_or_default(),
            expiry_date: expiry,
            balance_amount: balance,
            is_active: None,
        }, Some(staff)).await {
            Ok(_) => {},
            Err(e) => return Ok(err(e.to_string())),
        }
        if input.metadata.is_some() || input.private_metadata.is_some() {
            use sea_orm::{ActiveModelTrait, EntityTrait, Set};
            if let Some(row) = saleor_rustify_db::entities::giftcard_giftcard::Entity::find_by_id(cid).one(db).await.map_err(|e| Error::new(e.to_string()))? {
                let mut am: saleor_rustify_db::entities::giftcard_giftcard::ActiveModel = row.into();
                if let Some(m) = input.metadata.as_ref() {
                    let cur = serde_json::to_value(&am.metadata.clone().unwrap()).unwrap_or(serde_json::Value::Null);
                    am.metadata = Set(crate::common::merge_metadata(&cur, m));
                }
                if let Some(m) = input.private_metadata.as_ref() {
                    let cur = serde_json::to_value(&am.private_metadata.clone().unwrap()).unwrap_or(serde_json::Value::Null);
                    am.private_metadata = Set(crate::common::merge_metadata(&cur, m));
                }
                am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            }
        }
        Ok(gen::GiftCardUpdate {
            errors: vec![],
            gift_card: assemble_gift_card(db, cid).await.map_err(Error::new)?,
        })
    }

    /// Delete an unused card (spent cards are history and refuse).
    async fn gift_card_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::GiftCardDelete> {
        let _ = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(cid) = saleor_rustify_db::catalog::parse_gid(&id.0) else {
            return Ok(gen::GiftCardDelete { errors: vec![gcerr(Some("id".into()), "bad gift card id".into())] });
        };
        match saleor_rustify_db::giftcards::delete_card(db, cid).await {
            Ok(()) => Ok(gen::GiftCardDelete { errors: vec![] }),
            Err(e) => Ok(gen::GiftCardDelete { errors: vec![gcerr(None, e.to_string())] }),
        }
    }

    /// Activate a card.
    async fn gift_card_activate(&self, ctx: &Context<'_>, id: ID) -> Result<gen::GiftCardActivate> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::GiftCardActivate { gift_card: None, errors: vec![gcerr(None, m)] };
        match gift_card_code(db, &id.0).await {
            Ok(code) => match saleor_rustify_db::giftcards::set_active(db, &code, true, Some(staff)).await {
                Ok(c) => Ok(gen::GiftCardActivate { gift_card: assemble_gift_card(db, c.id).await.map_err(Error::new)?, errors: vec![] }),
                Err(e) => Ok(err(e.to_string())),
            },
            Err(e) => Ok(err(e)),
        }
    }

    /// Deactivate a card.
    async fn gift_card_deactivate(&self, ctx: &Context<'_>, id: ID) -> Result<gen::GiftCardDeactivate> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::GiftCardDeactivate { gift_card: None, errors: vec![gcerr(None, m)] };
        match gift_card_code(db, &id.0).await {
            Ok(code) => match saleor_rustify_db::giftcards::set_active(db, &code, false, Some(staff)).await {
                Ok(c) => Ok(gen::GiftCardDeactivate { gift_card: assemble_gift_card(db, c.id).await.map_err(Error::new)?, errors: vec![] }),
                Err(e) => Ok(err(e.to_string())),
            },
            Err(e) => Ok(err(e)),
        }
    }

    /// Staff note on a card (returns the card + the note event).
    async fn gift_card_add_note(&self, ctx: &Context<'_>, id: ID, input: gen::GiftCardAddNoteInput) -> Result<gen::GiftCardAddNote> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::GiftCardAddNote { gift_card: None, event: None, errors: vec![gcerr(None, m)] };
        let Some(cid) = saleor_rustify_db::catalog::parse_gid(&id.0) else {
            return Ok(err("bad gift card id".into()));
        };
        match saleor_rustify_db::giftcards::add_note(db, cid, &input.message, Some(staff)).await {
            Ok((_, eid)) => Ok(gen::GiftCardAddNote {
                gift_card: assemble_gift_card(db, cid).await.map_err(Error::new)?,
                event: Some(gen::GiftCardEvent {
                    id: Some(ID(crate::common::gid("GiftCardEvent", eid))),
                    date: Some(chrono::Utc::now()),
                    r#type: Some("NOTE_ADDED".into()),
                    user: None,
                    app: None,
                    message: Some(input.message.clone()),
                    email: None,
                    order_id: None,
                    order_number: None,
                    tags: vec![],
                    old_tags: vec![],
                    balance: None,
                    assigned_to: None,
                    expiry_date: None,
                    old_expiry_date: None,
                }),
                errors: vec![],
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    /// Resend the card code (event recorded; send rides the SMTP milestone).
    async fn gift_card_resend(&self, ctx: &Context<'_>, input: gen::GiftCardResendInput) -> Result<gen::GiftCardResend> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::GiftCardResend { gift_card: None, errors: vec![gcerr(None, m)] };
        let Some(cid) = saleor_rustify_db::catalog::parse_gid(&input.id.0) else {
            return Ok(err("bad gift card id".into()));
        };
        match saleor_rustify_db::giftcards::resend(db, cid, input.email.clone(), Some(staff)).await {
            Ok(c) => Ok(gen::GiftCardResend { gift_card: assemble_gift_card(db, c.id).await.map_err(Error::new)?, errors: vec![] }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    /// Expiry settings on the site row.
    async fn gift_card_settings_update(&self, ctx: &Context<'_>, input: gen::GiftCardSettingsUpdateInput) -> Result<gen::GiftCardSettingsUpdate> {
        let _ = crate::account::require_perm(ctx, "manage_settings").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::GiftCardSettingsUpdate {
            gift_card_settings: None,
            errors: vec![gen::GiftCardSettingsError { field: None, message: Some(m), code: None }],
        };
        let et = match input.expiry_type.as_ref() {
            Some(gen::GiftCardSettingsExpiryTypeEnum::NEVEREXPIRE) => Some("never_expire".to_string()),
            Some(gen::GiftCardSettingsExpiryTypeEnum::EXPIRYPERIOD) => Some("expiry_period".to_string()),
            None => None,
        };
        let days = match input.expiry_period.as_ref() {
            Some(p) => {
                let mult = match p.r#type {
                    gen::TimePeriodTypeEnum::DAY => 1,
                    gen::TimePeriodTypeEnum::WEEK => 7,
                    gen::TimePeriodTypeEnum::MONTH => 30,
                    gen::TimePeriodTypeEnum::YEAR => 365,
                };
                Some(p.amount * mult)
            }
            None => None,
        };
        if let Err(e) = saleor_rustify_db::giftcards::update_settings(db, et, days).await {
            return Ok(err(e.to_string()));
        }
        Ok(gen::GiftCardSettingsUpdate {
            gift_card_settings: Some(gen::GiftCardSettings {
                expiry_type: Some("EXPIRY_PERIOD".into()),
                expiry_period: None,
            }),
            errors: vec![],
        })
    }

    /// Bulk issue cards (Django `giftCardBulkCreate`, max 100).
    async fn gift_card_bulk_create(&self, ctx: &Context<'_>, input: gen::GiftCardBulkCreateInput) -> Result<gen::GiftCardBulkCreate> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let amount = input.balance.amount.0.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO);
        if amount <= rust_decimal::Decimal::ZERO {
            return Ok(gen::GiftCardBulkCreate { gift_cards: vec![], errors: vec![gcerr(None, "balance must be positive".into())] });
        }
        let expiry = input.expiry_date.as_ref().map(|d| d.date_naive());
        match saleor_rustify_db::giftcards::bulk_issue(
            db, input.count, amount, &input.balance.currency,
            &input.tags.clone().unwrap_or_default(), expiry, input.is_active, Some(staff),
        ).await {
            Ok(cards) => {
                let mut out = Vec::with_capacity(cards.len());
                for c in cards {
                    match assemble_gift_card(db, c.id).await {
                        Ok(Some(gc)) => out.push(gc),
                        Ok(None) => {},
                        Err(e) => return Ok(gen::GiftCardBulkCreate { gift_cards: vec![], errors: vec![gcerr(None, e)] }),
                    }
                }
                Ok(gen::GiftCardBulkCreate { gift_cards: out, errors: vec![] })
            }
            Err(e) => Ok(gen::GiftCardBulkCreate { gift_cards: vec![], errors: vec![gcerr(None, e.to_string())] }),
        }
    }

    /// Bulk delete (attempt all; failures collected as messages).
    async fn gift_card_bulk_delete(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<gen::GiftCardBulkDelete> {
        let _ = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let mut errors = vec![];
        for i in &ids {
            match saleor_rustify_db::catalog::parse_gid(&i.0) {
                Some(cid) => {
                    if let Err(e) = saleor_rustify_db::giftcards::delete_card(db, cid).await {
                        errors.push(gcerr(Some(i.0.clone()), e.to_string()));
                    }
                }
                None => errors.push(gcerr(Some(i.0.clone()), "bad gift card id".into())),
            }
        }
        Ok(gen::GiftCardBulkDelete { errors })
    }

    /// Bulk activate.
    async fn gift_card_bulk_activate(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<gen::GiftCardBulkActivate> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let mut n = 0;
        let mut errors = vec![];
        for i in &ids {
            match gift_card_code(db, &i.0).await {
                Ok(code) => match saleor_rustify_db::giftcards::set_active(db, &code, true, Some(staff)).await {
                    Ok(_) => n += 1,
                    Err(e) => errors.push(gcerr(Some(i.0.clone()), e.to_string())),
                },
                Err(e) => errors.push(gcerr(Some(i.0.clone()), e)),
            }
        }
        Ok(gen::GiftCardBulkActivate { count: Some(n), errors })
    }

    /// Bulk deactivate.
    async fn gift_card_bulk_deactivate(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<gen::GiftCardBulkDeactivate> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let mut n = 0;
        let mut errors = vec![];
        for i in &ids {
            match gift_card_code(db, &i.0).await {
                Ok(code) => match saleor_rustify_db::giftcards::set_active(db, &code, false, Some(staff)).await {
                    Ok(_) => n += 1,
                    Err(e) => errors.push(gcerr(Some(i.0.clone()), e.to_string())),
                },
                Err(e) => errors.push(gcerr(Some(i.0.clone()), e)),
            }
        }
        Ok(gen::GiftCardBulkDeactivate { count: Some(n), errors })
    }

    /// Assign a card to a user.
    async fn gift_card_assign_user(&self, ctx: &Context<'_>, id: ID, #[graphql(name = "userId")] user_id: ID) -> Result<gen::GiftCardAssignUser> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::GiftCardAssignUser { gift_card: None, errors: vec![gcerr(None, m)] };
        let uid = saleor_rustify_db::catalog::parse_gid(&user_id.0).unwrap_or(-1);
        let em: Option<String> = {
            use sea_orm::{EntityTrait, QuerySelect};
            saleor_rustify_db::entities::account_user::Entity::find_by_id(uid)
                .select_only()
                .column(saleor_rustify_db::entities::account_user::Column::Email)
                .into_tuple()
                .one(db).await.map_err(|e| Error::new(e.to_string()))?
                .unwrap_or(None)
        };
        let Some(em) = em else { return Ok(err("user not found".into())) };
        match gift_card_code(db, &id.0).await {
            Ok(code) => match saleor_rustify_db::giftcards::assign(db, &code, uid, &em, Some(staff)).await {
                Ok(c) => Ok(gen::GiftCardAssignUser { gift_card: assemble_gift_card(db, c.id).await.map_err(Error::new)?, errors: vec![] }),
                Err(e) => Ok(err(e.to_string())),
            },
            Err(e) => Ok(err(e)),
        }
    }

    /// Unassign a card.
    async fn gift_card_unassign_user(&self, ctx: &Context<'_>, id: ID) -> Result<gen::GiftCardUnassignUser> {
        let staff = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::GiftCardUnassignUser { gift_card: None, errors: vec![gcerr(None, m)] };
        match gift_card_code(db, &id.0).await {
            Ok(code) => match saleor_rustify_db::giftcards::unassign(db, &code, Some(staff)).await {
                Ok(c) => Ok(gen::GiftCardUnassignUser { gift_card: assemble_gift_card(db, c.id).await.map_err(Error::new)?, errors: vec![] }),
                Err(e) => Ok(err(e.to_string())),
            },
            Err(e) => Ok(err(e)),
        }
    }

    /// Create a promotion with rules (Django `promotionCreate`). Predicates
    /// persist in the engine's condition-dict shape (id-list leaves; exotic
    /// where-clauses narrow to their id lists, documented).
    async fn promotion_create(&self, ctx: &Context<'_>, input: gen::PromotionCreateInput) -> Result<gen::PromotionCreate> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::PromotionCreate {
            errors: vec![gen::PromotionCreateError { field: None, message: Some(m), code: None, index: None }],
            promotion: None,
        };
        let mut rules = vec![];
        for r in input.rules.clone().unwrap_or_default() {
            rules.push(promo_rule_from(r)?);
        }
        let ptype = match input.r#type {
            gen::PromotionTypeEnum::CATALOGUE => "catalogue",
            gen::PromotionTypeEnum::ORDER => "order",
        };
        let pid = match saleor_rustify_db::promo_writes::create_promotion(
            db, &input.name, ptype,
            input.description.clone().unwrap_or(serde_json::Value::Null),
            input.start_date.map(|d| d.with_timezone(&chrono::Utc)),
            input.end_date.map(|d| d.with_timezone(&chrono::Utc)),
            rules,
        ).await {
            Ok(id) => id,
            Err(e) => return Ok(err(e.to_string())),
        };
        Ok(gen::PromotionCreate {
            errors: vec![],
            promotion: assemble_promotion(db, pid).await.map_err(Error::new)?,
        })
    }

    /// Update a promotion header (Django `promotionUpdate`).
    async fn promotion_update(&self, ctx: &Context<'_>, id: ID, input: gen::PromotionUpdateInput) -> Result<gen::PromotionUpdate> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::PromotionUpdate {
            errors: vec![gen::PromotionUpdateError { field: None, message: Some(m), code: None }],
            promotion: None,
        };
        let pid = crate::common::parse_uuid_gid(&id.0).ok_or_else(|| Error::new("bad promotion id"))?;
        if let Err(e) = saleor_rustify_db::promo_writes::update_promotion(
            db, pid,
            input.name.clone(),
            input.description.clone(),
            input.start_date.map(|d| d.with_timezone(&chrono::Utc)),
            input.end_date.map(|d| Some(d.with_timezone(&chrono::Utc))),
        ).await {
            // end_date None = unchanged (clearing needs explicit null; the
            // dashboard sends new ranges, never clears — documented).
            return Ok(err(e.to_string()));
        }
        Ok(gen::PromotionUpdate {
            errors: vec![],
            promotion: assemble_promotion(db, pid).await.map_err(Error::new)?,
        })
    }

    /// Delete a promotion with rules (Django `promotionDelete`).
    async fn promotion_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::PromotionDelete> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let pid = crate::common::parse_uuid_gid(&id.0).ok_or_else(|| Error::new("bad promotion id"))?;
        match saleor_rustify_db::promo_writes::delete_promotion(db, pid).await {
            Ok(()) => Ok(gen::PromotionDelete { errors: vec![] }),
            Err(e) => Ok(gen::PromotionDelete {
                errors: vec![gen::PromotionDeleteError { field: None, message: Some(e.to_string()), code: None }],
            }),
        }
    }

    /// Bulk promotion delete (survivors commit).
    async fn promotion_bulk_delete(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<GqlPromotionBulkDelete> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let uuids: Vec<uuid::Uuid> = ids.iter().filter_map(|i| crate::common::parse_uuid_gid(&i.0)).collect();
        match saleor_rustify_db::promo_writes::bulk_delete_promotions(db, &uuids).await {
            Ok(n) => Ok(GqlPromotionBulkDelete { count: Some(n), errors: vec![] }),
            Err(e) => Ok(GqlPromotionBulkDelete {
                count: Some(0),
                errors: vec![discount_err(None, e.to_string())],
            }),
        }
    }

    /// Create a rule on a promotion (Django `promotionRuleCreate`).
    async fn promotion_rule_create(&self, ctx: &Context<'_>, input: gen::PromotionRuleCreateInput) -> Result<gen::PromotionRuleCreate> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::PromotionRuleCreate {
            errors: vec![gen::PromotionRuleCreateError { field: None, message: Some(m), code: None }],
            promotion_rule: None,
        };
        let pid = crate::common::parse_uuid_gid(&input.promotion.0).ok_or_else(|| Error::new("bad promotion id"))?;
        let rule = promo_rule_from(gen::PromotionRuleInput {
            name: input.name.clone(),
            description: input.description.clone(),
            catalogue_predicate: input.catalogue_predicate.clone(),
            order_predicate: input.order_predicate.clone(),
            reward_value_type: input.reward_value_type.clone(),
            reward_value: input.reward_value.clone(),
            reward_type: input.reward_type.clone(),
            channels: input.channels.clone(),
            gifts: input.gifts.clone(),
        })?;
        let rid = match saleor_rustify_db::promo_writes::create_rule(db, pid, &rule).await {
            Ok(id) => id,
            Err(e) => return Ok(err(e.to_string())),
        };
        Ok(gen::PromotionRuleCreate {
            errors: vec![],
            promotion_rule: promotion_rule_view(db, pid, rid).await?,
        })
    }

    /// Update a rule (Django `promotionRuleUpdate`).
    async fn promotion_rule_update(&self, ctx: &Context<'_>, id: ID, input: gen::PromotionRuleUpdateInput) -> Result<gen::PromotionRuleUpdate> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::PromotionRuleUpdate {
            errors: vec![gen::PromotionRuleUpdateError { field: None, message: Some(m), code: None, channels: vec![] }],
            promotion_rule: None,
        };
        let rid = crate::common::parse_uuid_gid(&id.0).ok_or_else(|| Error::new("bad rule id"))?;
        let upd = saleor_rustify_db::promo_writes::UpdateRule {
            name: input.name.clone(),
            description: input.description.clone(),
            catalogue_predicate: input.catalogue_predicate.as_ref().map(catalogue_predicate_json),
            order_predicate: input.order_predicate.as_ref().map(order_predicate_json),
            reward_value_type: input.reward_value_type.clone().map(|v| Some(reward_value_type_str(&v))),
            reward_value: input.reward_value.clone().map(|v| Some(v.0.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO))),
            reward_type: input.reward_type.clone().map(|v| Some(reward_type_str(&v))),
            add_channels: input.add_channels.clone().unwrap_or_default().iter().filter_map(|c| saleor_rustify_db::catalog::parse_gid(&c.0)).collect(),
            remove_channels: input.remove_channels.clone().unwrap_or_default().iter().filter_map(|c| saleor_rustify_db::catalog::parse_gid(&c.0)).collect(),
            add_gifts: input.add_gifts.clone().unwrap_or_default().iter().filter_map(|c| saleor_rustify_db::catalog::parse_gid(&c.0)).collect(),
            remove_gifts: input.remove_gifts.clone().unwrap_or_default().iter().filter_map(|c| saleor_rustify_db::catalog::parse_gid(&c.0)).collect(),
        };
        if let Err(e) = saleor_rustify_db::promo_writes::update_rule(db, rid, &upd).await {
            return Ok(err(e.to_string()));
        }
        let pid = rule_promotion(db, rid).await?;
        Ok(gen::PromotionRuleUpdate {
            errors: vec![],
            promotion_rule: promotion_rule_view(db, pid, rid).await?,
        })
    }

    /// Delete a rule (Django `promotionRuleDelete`).
    async fn promotion_rule_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::PromotionRuleDelete> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let rid = crate::common::parse_uuid_gid(&id.0).ok_or_else(|| Error::new("bad rule id"))?;
        let pid = rule_promotion(db, rid).await.unwrap_or(rid);
        match saleor_rustify_db::promo_writes::delete_rule(db, rid).await {
            Ok(()) => Ok(gen::PromotionRuleDelete {
                errors: vec![],
                promotion_rule: None,
            }),
            Err(e) => Ok(gen::PromotionRuleDelete {
                errors: vec![gen::PromotionRuleDeleteError { field: None, message: Some(e.to_string()), code: None }],
                promotion_rule: None,
            }),
        }
    }

    /// Create a voucher (Django `voucherCreate`): header, codes, catalogue,
    /// channel listings in one transaction.
    async fn voucher_create(&self, ctx: &Context<'_>, input: gen::VoucherInput) -> Result<gen::VoucherCreate> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::VoucherCreate {
            errors: vec![discount_err(None, m)],
            voucher: None,
        };
        let nv = match voucher_from(input).await {
            Ok(v) => v,
            Err(e) => return Ok(err(e)),
        };
        // Listings need a currency: first channel's, else USD.
        let currency = listing_currency(db, &nv.listings).await;
        let vid = match saleor_rustify_db::promo_writes::create_voucher(db, &nv, &currency).await {
            Ok(id) => id,
            Err(e) => return Ok(err(e.to_string())),
        };
        Ok(gen::VoucherCreate {
            errors: vec![],
            voucher: Some(assemble_voucher(db, vid).await?),
        })
    }

    /// Update a voucher (Django `voucherUpdate`).
    async fn voucher_update(&self, ctx: &Context<'_>, id: ID, input: gen::VoucherInput) -> Result<gen::VoucherUpdate> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::VoucherUpdate {
            errors: vec![discount_err(None, m)],
            voucher: None,
        };
        let vid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        let upd = saleor_rustify_db::promo_writes::UpdateVoucher {
            name: Some(input.name.clone()),
            discount_value_type: input.discount_value_type.clone().map(|v| discount_value_type_str(&v)),
            usage_limit: Some(input.usage_limit),
            start: input.start_date.map(|d| d.with_timezone(&chrono::Utc)),
            end: input.end_date.map(|d| Some(d.with_timezone(&chrono::Utc))),
            min_items: Some(input.min_checkout_items_quantity),
            countries: input.countries.clone(),
            apply_once_per_order: input.apply_once_per_order,
            apply_once_per_customer: input.apply_once_per_customer,
            only_for_staff: input.only_for_staff,
            single_use: input.single_use,
            add_codes: {
                let mut c = input.add_codes.clone().unwrap_or_default();
                if let Some(s) = input.code.clone() {
                    c.push(s);
                }
                c
            },
        };
        // update end=None means "unchanged" here (dashboard always sends full
        // ranges; explicit clearing is a documented gap).
        let mut upd = upd;
        if input.end_date.is_none() {
            upd.end = None;
        }
        if let Err(e) = saleor_rustify_db::promo_writes::update_voucher(db, vid, &upd, "USD").await {
            return Ok(err(e.to_string()));
        }
        Ok(gen::VoucherUpdate {
            errors: vec![],
            voucher: Some(assemble_voucher(db, vid).await?),
        })
    }

    /// Delete a voucher with all links (Django `voucherDelete`).
    async fn voucher_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::VoucherDelete> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let vid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        match saleor_rustify_db::promo_writes::delete_voucher(db, vid).await {
            Ok(()) => Ok(gen::VoucherDelete { errors: vec![] }),
            Err(e) => Ok(gen::VoucherDelete { errors: vec![discount_err(None, e.to_string())] }),
        }
    }

    /// Bulk voucher delete (survivors commit).
    async fn voucher_bulk_delete(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<gen::VoucherBulkDelete> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let vids: Vec<i32> = ids.iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect();
        let mut errors = vec![];
        let mut n = 0;
        for vid in vids {
            match saleor_rustify_db::promo_writes::delete_voucher(db, vid).await {
                Ok(()) => n += 1,
                Err(e) => errors.push(discount_err(None, e.to_string())),
            }
        }
        let _ = n;
        Ok(gen::VoucherBulkDelete { errors })
    }

    /// Add catalogue rows (Django `voucherCataloguesAdd`).
    async fn voucher_catalogues_add(&self, ctx: &Context<'_>, id: ID, input: gen::CatalogueInput) -> Result<gen::VoucherAddCatalogues> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let vid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        let (p, v, c, co) = catalogue_ids(&input);
        match saleor_rustify_db::promo_writes::voucher_catalogues(db, vid, true, &p, &v, &c, &co).await {
            Ok(()) => Ok(gen::VoucherAddCatalogues { voucher: Some(assemble_voucher(db, vid).await?), errors: vec![] }),
            Err(e) => Ok(gen::VoucherAddCatalogues { voucher: None, errors: vec![discount_err(None, e.to_string())] }),
        }
    }

    /// Remove catalogue rows (Django `voucherCataloguesRemove`).
    async fn voucher_catalogues_remove(&self, ctx: &Context<'_>, id: ID, input: gen::CatalogueInput) -> Result<gen::VoucherRemoveCatalogues> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let vid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        let (p, v, c, co) = catalogue_ids(&input);
        match saleor_rustify_db::promo_writes::voucher_catalogues(db, vid, false, &p, &v, &c, &co).await {
            Ok(()) => Ok(gen::VoucherRemoveCatalogues { voucher: Some(assemble_voucher(db, vid).await?), errors: vec![] }),
            Err(e) => Ok(gen::VoucherRemoveCatalogues { voucher: None, errors: vec![discount_err(None, e.to_string())] }),
        }
    }

    /// Channel listings add/remove (Django `voucherChannelListingUpdate`).
    async fn voucher_channel_listing_update(&self, ctx: &Context<'_>, id: ID, input: gen::VoucherChannelListingInput) -> Result<gen::VoucherChannelListingUpdate> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let vid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        let mut add = vec![];
        for l in input.add_channels.clone().unwrap_or_default() {
            let ch = saleor_rustify_db::catalog::parse_gid(&l.channel_id.0).unwrap_or(-1);
            if ch < 0 {
                continue;
            }
            add.push(saleor_rustify_db::promo_writes::ChannelListingInput {
                channel_id: ch,
                discount_value: l.discount_value.as_ref().and_then(|v| v.0.parse::<rust_decimal::Decimal>().ok()).unwrap_or(rust_decimal::Decimal::ZERO),
                min_spent: l.min_amount_spent.as_ref().and_then(|v| v.0.parse::<rust_decimal::Decimal>().ok()),
            });
        }
        let remove: Vec<i32> = input.remove_channels.clone().unwrap_or_default().iter().filter_map(|c| saleor_rustify_db::catalog::parse_gid(&c.0)).collect();
        let currency = listing_currency(db, &add).await;
        match saleor_rustify_db::promo_writes::voucher_channel_listings(db, vid, &add, &remove, &currency).await {
            Ok(()) => Ok(gen::VoucherChannelListingUpdate { voucher: Some(assemble_voucher(db, vid).await?), errors: vec![] }),
            Err(e) => Ok(gen::VoucherChannelListingUpdate { voucher: None, errors: vec![discount_err(None, e.to_string())] }),
        }
    }

    /// Bulk code delete by code-row ids.
    async fn voucher_code_bulk_delete(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<gen::VoucherCodeBulkDelete> {
        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let uuids: Vec<uuid::Uuid> = ids.iter().filter_map(|i| crate::common::parse_uuid_gid(&i.0)).collect();
        match saleor_rustify_db::promo_writes::delete_voucher_codes(db, &uuids).await {
            Ok(n) => Ok(gen::VoucherCodeBulkDelete { count: Some(n), errors: vec![] }),
            Err(e) => Ok(gen::VoucherCodeBulkDelete {
                count: Some(0),
                errors: vec![gen::VoucherCodeBulkDeleteError { path: None, message: Some(e.to_string()), code: None }],
            }),
        }
    }

    /// Delete a menu item with its subtree (MPTT range delete, like Django's
    /// collector on the tree).
    async fn menu_item_delete(&self, ctx: &Context<'_>, id: ID) -> Result<GqlMenuItemDelete> {
        let _ = crate::account::require_perm(ctx, "manage_menus").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(mid) = saleor_rustify_db::catalog::parse_gid(&id.0) else {
            return Ok(GqlMenuItemDelete { menu_item: None, errors: vec![merr(Some("id".into()), "bad menu item id".into())] });
        };
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        use saleor_rustify_db::entities::menu_menuitem::{Column as MCol, Entity as MEnt};
        let Some(m) = MEnt::find_by_id(mid).one(db).await.map_err(|e| Error::new(e.to_string()))? else {
            return Ok(GqlMenuItemDelete { menu_item: None, errors: vec![merr(Some("id".into()), "menu item not found".into())] });
        };
        MEnt::delete_many()
            .filter(MCol::TreeId.eq(m.tree_id))
            .filter(MCol::Lft.gte(m.lft))
            .filter(MCol::Rght.lte(m.rght))
            .exec(db).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlMenuItemDelete { menu_item: None, errors: vec![] })
    }

    /// Assign storefront navigation menus (Django writes
    /// `site_sitesettings.top/bottom_menu`).
    async fn assign_navigation(
        &self, ctx: &Context<'_>, menu: Option<ID>, #[graphql(name = "navigationType")] navigation_type: gen::NavigationType,
    ) -> Result<GqlAssignNavigation> {
        let _ = crate::account::require_perm(ctx, "manage_menus").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlAssignNavigation { menu: None, errors: vec![merr(None, m)] };
        let mid = match menu.as_ref() {
            Some(i) => match saleor_rustify_db::catalog::parse_gid(&i.0) {
                Some(v) => Some(v),
                None => return Ok(err("bad menu id".into())),
            },
            None => None,
        };
        if let Some(m) = mid {
            use sea_orm::EntityTrait;
            if saleor_rustify_db::entities::menu_menu::Entity::find_by_id(m).one(db).await.map_err(|e| Error::new(e.to_string()))?.is_none() {
                return Ok(err("menu not found".into()));
            }
        }
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};
        let row = saleor_rustify_db::entities::site_sitesettings::Entity::find_by_id(1)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("site settings missing"))?;
        {
            let mut am: saleor_rustify_db::entities::site_sitesettings::ActiveModel = row.into();
            match format!("{navigation_type:?}").as_str() {
                "SECONDARY" => am.bottom_menu_id = Set(mid),
                _ => am.top_menu_id = Set(mid),
            }
            am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
        }
        let menu_obj = match mid {
            Some(m) => {
                let row = saleor_rustify_db::entities::menu_menu::Entity::find_by_id(m)
                    .one(db).await.map_err(|e| Error::new(e.to_string()))?;
                row.map(|x| {
                    let mut g = crate::metadata::lit_menu(crate::common::gid("Menu", m), vec![], vec![]);
                    g.name = Some(x.name);
                    g
                })
            }
            None => None,
        };
        Ok(GqlAssignNavigation { menu: menu_obj, errors: vec![] })
    }

    /// Toggle tax exemption on a checkout or order (Django resolves the id
    /// against both tables).
    async fn tax_exemption_manage(&self, ctx: &Context<'_>, id: ID, #[graphql(name = "taxExemption")] tax_exemption: bool) -> Result<GqlTaxExemptionManage> {
        let _ = crate::account::require_perm(ctx, "handle_taxes").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlTaxExemptionManage { taxable_object: None, errors: vec![terr_tax(None, m)] };
        let Some(uuid) = crate::common::parse_uuid_gid(&id.0) else {
            return Ok(err("bad id".into()));
        };
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};
        if let Some(co) = saleor_rustify_db::entities::checkout_checkout::Entity::find_by_id(uuid).one(db).await.map_err(|e| Error::new(e.to_string()))? {
            let ch = match co.channel_id { 2 => "channel-pln".to_string(), _ => "default-channel".to_string() };
            let mut am: saleor_rustify_db::entities::checkout_checkout::ActiveModel = co.into();
            am.tax_exemption = Set(tax_exemption);
            am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            let (co2, lines) = saleor_rustify_db::checkout_store::load_checkout(db, uuid).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout vanished"))?;
            return Ok(GqlTaxExemptionManage {
                taxable_object: Some(GqlTaxSourceObject::Checkout(crate::checkout::to_gql_checkout(&co2, &lines, &ch))),
                errors: vec![],
            });
        }
        if let Some(row) = saleor_rustify_db::entities::order_order::Entity::find_by_id(uuid).one(db).await.map_err(|e| Error::new(e.to_string()))? {
            let mut am: saleor_rustify_db::entities::order_order::ActiveModel = row.into();
            am.tax_exemption = Set(tax_exemption);
            am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            let (h, ls) = saleor_rustify_db::order_store::get_order_rows(db, uuid).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("order vanished"))?;
            return Ok(GqlTaxExemptionManage {
                taxable_object: Some(GqlTaxSourceObject::Order(Box::new(crate::order::to_gen_order(db, &h, ls).await))),
                errors: vec![],
            });
        }
        Ok(err("no checkout or order with this id".into()))
    }
}

/// Full promotion assembly (details page): dates, description, metadata +
/// rules with channels, gifts, predicates and rewards.
async fn assemble_promotion(
    db: &sea_orm::DatabaseConnection,
    pid: uuid::Uuid,
) -> Result<Option<gen::Promotion>, String> {
    use sea_orm::Statement;
    use sea_orm::ConnectionTrait;
    use std::collections::HashMap;
    let q = |sql: String, params: Vec<sea_orm::Value>| async move {
        db.query_all(Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params)).await
    };
    let prows = q("SELECT id::text AS id, name, description, type, start_date, end_date, metadata, private_metadata FROM discount_promotion WHERE id = $1::uuid".into(),
        vec![pid.to_string().into()]).await.map_err(|e| e.to_string())?;
    let Some(prow) = prows.into_iter().next() else { return Ok(None) };
    let rrows = q("SELECT id::text AS id, name, description, catalogue_predicate, order_predicate, reward_type, reward_value_type, reward_value FROM discount_promotionrule WHERE promotion_id = $1::uuid ORDER BY id".into(),
        vec![pid.to_string().into()]).await.map_err(|e| e.to_string())?;
    let rids: Vec<String> = rrows.iter().filter_map(|r| r.try_get::<String>("", "id").ok()).collect();
    // rule channels + gifts, batched
    let mut rchannels: HashMap<String, Vec<gen::Channel>> = HashMap::new();
    let mut rgifts: HashMap<String, Vec<ID>> = HashMap::new();
    if !rids.is_empty() {
        let list = rids.iter().enumerate().map(|(i, _)| format!("${}::uuid", i + 1)).collect::<Vec<_>>().join(", ");
        let params: Vec<sea_orm::Value> = rids.iter().map(|s| s.clone().into()).collect();
        for r in q(format!("SELECT rc.promotionrule_id::text AS rid, c.id, c.slug, c.name, c.currency_code, c.is_active, c.default_country FROM discount_promotionrule_channels rc JOIN channel_channel c ON c.id = rc.channel_id WHERE rc.promotionrule_id IN ({list})"), params.clone())
            .await.map_err(|e| e.to_string())? {
            if let (Ok(rid), Ok(cid)) = (r.try_get::<String>("", "rid"), r.try_get::<i32>("", "id")) {
                let dc = r.try_get::<String>("", "default_country").unwrap_or_else(|_| "US".into());
                let mut c = crate::metadata::lit_channel(crate::common::gid("Channel", cid), vec![], vec![]);
                c.slug = r.try_get::<String>("", "slug").ok();
                c.name = r.try_get::<String>("", "name").ok();
                c.is_active = r.try_get::<bool>("", "is_active").ok();
                c.currency_code = r.try_get::<String>("", "currency_code").ok();
                c.default_country = Some(GqlCountryDisplay { code: dc.clone(), country: dc });
                rchannels.entry(rid).or_default().push(c);
            }
        }
        for r in q(format!("SELECT promotionrule_id::text AS rid, productvariant_id FROM discount_promotionrule_gifts WHERE promotionrule_id IN ({list})"), params)
            .await.map_err(|e| e.to_string())? {
            if let (Ok(rid), Ok(vid)) = (r.try_get::<String>("", "rid"), r.try_get::<i32>("", "productvariant_id")) {
                rgifts.entry(rid).or_default().push(ID(crate::common::gid("ProductVariant", vid)));
            }
        }
    }
    let meta = |r: &sea_orm::QueryResult, c: &str| {
        r.try_get::<serde_json::Value>("", c).ok()
            .map(|v| crate::common::json_to_metadata_items(&v)).unwrap_or_default()
    };
    let rules = rrows.into_iter().filter_map(|r| {
        let rid = r.try_get::<String>("", "id").ok()?;
        Some(gen::PromotionRule {
            id: Some(ID(crate::common::gid("PromotionRule", &rid))),
            name: r.try_get::<Option<String>>("", "name").ok().flatten(),
            description: r.try_get::<Option<serde_json::Value>>("", "description").ok().flatten(),
            channels: rchannels.get(&rid).cloned().unwrap_or_default(),
            reward_value: r.try_get::<Option<rust_decimal::Decimal>>("", "reward_value").ok().flatten().map(|d| gen::GenPositiveDecimal(d.to_string())),
            reward_value_type: r.try_get::<Option<String>>("", "reward_value_type").ok().flatten(),
            catalogue_predicate: r.try_get::<Option<serde_json::Value>>("", "catalogue_predicate").ok().flatten(),
            order_predicate: r.try_get::<Option<serde_json::Value>>("", "order_predicate").ok().flatten(),
            reward_type: r.try_get::<Option<String>>("", "reward_type").ok().flatten(),
            gift_ids: rgifts.get(&rid).cloned().unwrap_or_default(),
        })
    }).collect();
    Ok(Some(gen::Promotion {
        id: Some(ID(crate::common::gid("Promotion", pid))),
        private_metadata: meta(&prow, "private_metadata"),
        metadata: meta(&prow, "metadata"),
        name: prow.try_get::<String>("", "name").ok(),
        r#type: prow.try_get::<String>("", "type").ok().map(|t| t.to_uppercase()),
        description: prow.try_get::<Option<serde_json::Value>>("", "description").ok().flatten(),
        start_date: prow.try_get::<Option<chrono::DateTime<chrono::Utc>>>("", "start_date").ok().flatten(),
        end_date: prow.try_get::<Option<chrono::DateTime<chrono::Utc>>>("", "end_date").ok().flatten(),
        rules,
    }))
}

/// Full channel assembly (list + details). INTERVAL columns are read via
/// raw SQL with Saleor's own unit conversions (Minute scalar as-is,
/// Day = `.days`, Hour = `.seconds // 3600` — see channel/types.py).
async fn assemble_channel(
    db: &sea_orm::DatabaseConnection,
    cid: i32,
) -> Result<Option<gen::Channel>, String> {
    use sea_orm::{ConnectionTrait, Statement};
    let rows = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, slug, name, is_active, currency_code, default_country, \
                allocation_strategy, metadata, private_metadata, \
                automatically_confirm_all_new_orders, automatically_fulfill_non_shippable_gift_card, \
                expire_orders_after, order_mark_as_paid_strategy, \
                (EXTRACT(EPOCH FROM delete_expired_orders_after)/86400)::int AS del_exp_days, \
                allow_unpaid_orders, default_transaction_flow_strategy, release_funds_for_expired_checkouts, \
                (EXTRACT(EPOCH FROM checkout_ttl_before_releasing_funds)/3600)::int AS ttl_hours, \
                automatically_complete_fully_paid_checkouts, automatic_completion_delay, \
                automatic_completion_cut_off_date, allow_legacy_gift_card_use, \
                EXISTS(SELECT 1 FROM order_order o WHERE o.channel_id = channel_channel.id) AS has_orders \
         FROM channel_channel WHERE id = $1",
        [cid.into()],
    )).await.map_err(|e| e.to_string())?;
    let Some(r) = rows.into_iter().next() else { return Ok(None) };
    let get = |c: &str| r.try_get::<String>("", c).ok();
    let getb = |c: &str| r.try_get::<bool>("", c).ok();
    let geti = |c: &str| r.try_get::<Option<i32>>("", c).ok().flatten();
    let getdt = |c: &str| r.try_get::<Option<chrono::DateTime<chrono::Utc>>>("", c).ok().flatten();
    let meta = |c: &str| {
        r.try_get::<serde_json::Value>("", c).ok()
            .map(|v| crate::common::json_to_metadata_items(&v)).unwrap_or_default()
    };
    let dc = get("default_country").unwrap_or_else(|| "US".into());
    // warehouses via the ChannelWarehouse through-table (uuid ids)
    let mut warehouses = vec![];
    let wrows = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT w.id::text AS id, w.name FROM warehouse_channelwarehouse cw \
         JOIN warehouse_warehouse w ON w.id::text = cw.warehouse_id::text \
         WHERE cw.channel_id = $1 ORDER BY cw.sort_order, w.name",
        [cid.into()],
    )).await.map_err(|e| e.to_string())?;
    for w in wrows {
        if let (Ok(wid), Ok(wname)) = (w.try_get::<String>("", "id"), w.try_get::<String>("", "name")) {
            let mut wh = crate::metadata::lit_warehouse(crate::common::gid("Warehouse", &wid), vec![], vec![]);
            wh.name = Some(wname);
            warehouses.push(wh);
        }
    }
    // tax configuration row for this channel (one-to-one in practice)
    let trows = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, charge_taxes, tax_calculation_strategy, display_gross_prices, prices_entered_with_tax, metadata, private_metadata \
         FROM tax_taxconfiguration WHERE channel_id = $1 LIMIT 1",
        [cid.into()],
    )).await.map_err(|e| e.to_string())?;
    let tax_configuration = trows.into_iter().next().map(|t| {
        let tid: i32 = t.try_get::<i32>("", "id").unwrap_or(0);
        gen::TaxConfiguration {
            id: Some(ID(crate::common::gid("TaxConfiguration", tid))),
            private_metadata: t.try_get::<serde_json::Value>("", "private_metadata").ok()
                .map(|v| crate::common::json_to_metadata_items(&v)).unwrap_or_default(),
            metadata: t.try_get::<serde_json::Value>("", "metadata").ok()
                .map(|v| crate::common::json_to_metadata_items(&v)).unwrap_or_default(),
            channel: None,
            charge_taxes: t.try_get::<bool>("", "charge_taxes").ok(),
            tax_calculation_strategy: t.try_get::<Option<String>>("", "tax_calculation_strategy").ok().flatten(),
            display_gross_prices: t.try_get::<bool>("", "display_gross_prices").ok(),
            prices_entered_with_tax: t.try_get::<bool>("", "prices_entered_with_tax").ok(),
            countries: vec![],
            tax_app_id: None,
        }
    });
    Ok(Some(gen::Channel {
        id: Some(ID(crate::common::gid("Channel", cid))),
        private_metadata: meta("private_metadata"),
        metadata: meta("metadata"),
        slug: get("slug"),
        name: get("name"),
        is_active: getb("is_active"),
        currency_code: get("currency_code"),
        has_orders: r.try_get::<bool>("", "has_orders").ok(),
        default_country: Some(GqlCountryDisplay { code: dc.clone(), country: dc }),
        warehouses,
        // Saleor outputs the enum NAME (PRIORITIZE_HIGH_STOCK); the DB stores
        // the lowercase value — same round-trip class as Promotion.type.
        stock_settings: Some(GqlStockSettings { allocation_strategy: get("allocation_strategy").map(|s| s.to_uppercase()).unwrap_or_else(|| "PRIORITIZE_SORTING_ORDER".into()) }),
        order_settings: Some(gen::OrderSettings {
            automatically_confirm_all_new_orders: getb("automatically_confirm_all_new_orders"),
            automatically_fulfill_non_shippable_gift_card: getb("automatically_fulfill_non_shippable_gift_card"),
            expire_orders_after: geti("expire_orders_after"),
            mark_as_paid_strategy: get("order_mark_as_paid_strategy"),
            delete_expired_orders_after: geti("del_exp_days"),
            allow_unpaid_orders: getb("allow_unpaid_orders"),
        }),
        checkout_settings: Some(gen::CheckoutSettings {
            automatically_complete_fully_paid_checkouts: getb("automatically_complete_fully_paid_checkouts"),
            automatic_completion_delay: geti("automatic_completion_delay"),
            automatic_completion_cut_off_date: getdt("automatic_completion_cut_off_date"),
            allow_legacy_gift_card_use: getb("allow_legacy_gift_card_use"),
        }),
        payment_settings: Some(gen::PaymentSettings {
            default_transaction_flow_strategy: get("default_transaction_flow_strategy"),
            release_funds_for_expired_checkouts: getb("release_funds_for_expired_checkouts"),
            checkout_ttl_before_releasing_funds: geti("ttl_hours"),
        }),
        tax_configuration,
    }))
}

/// Full shipping-zone assembly (setup banner + zone pages). Countries are a
/// comma-separated column upstream (`US`, `AD,AL,...`) — split, not JSON.
async fn assemble_zone(
    db: &sea_orm::DatabaseConnection,
    zid: i32,
) -> Result<Option<gen::ShippingZone>, String> {
    use sea_orm::{ConnectionTrait, Statement};
    use std::collections::HashMap;
    let q = |sql: String, params: Vec<sea_orm::Value>| async move {
        db.query_all(Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params)).await
    };
    let zrows = q("SELECT id, name, description, countries, \"default\", metadata, private_metadata FROM shipping_shippingzone WHERE id = $1".into(),
        vec![zid.into()]).await.map_err(|e| e.to_string())?;
    let Some(z) = zrows.into_iter().next() else { return Ok(None) };
    let meta = |c: &str| {
        z.try_get::<serde_json::Value>("", c).ok()
            .map(|v| crate::common::json_to_metadata_items(&v)).unwrap_or_default()
    };
    let countries: Vec<crate::common::GqlCountryDisplay> = z
        .try_get::<String>("", "countries").unwrap_or_default()
        .split(',').map(|s| s.trim()).filter(|s| !s.is_empty())
        .map(|code| crate::common::GqlCountryDisplay { code: code.to_string(), country: code.to_string() })
        .collect();
    // channels of this zone
    let mut channels: Vec<Box<gen::Channel>> = vec![];
    for r in q("SELECT c.id, c.slug, c.name, c.currency_code FROM shipping_shippingzone_channels zc JOIN channel_channel c ON c.id = zc.channel_id WHERE zc.shippingzone_id = $1".into(),
        vec![zid.into()]).await.map_err(|e| e.to_string())? {
        if let Ok(cid) = r.try_get::<i32>("", "id") {
            let mut c = crate::metadata::lit_channel(crate::common::gid("Channel", cid), vec![], vec![]);
            c.slug = r.try_get::<String>("", "slug").ok();
            c.name = r.try_get::<String>("", "name").ok();
            c.currency_code = r.try_get::<String>("", "currency_code").ok();
            channels.push(Box::new(c));
        }
    }
    // warehouses of this zone
    let mut warehouses: Vec<Box<gen::Warehouse>> = vec![];
    for r in q("SELECT w.id::text AS id, w.name FROM warehouse_warehouse_shipping_zones wz JOIN warehouse_warehouse w ON w.id = wz.warehouse_id WHERE wz.shippingzone_id = $1".into(),
        vec![zid.into()]).await.map_err(|e| e.to_string())? {
        if let (Ok(wid), Ok(wname)) = (r.try_get::<String>("", "id"), r.try_get::<String>("", "name")) {
            let mut w = crate::metadata::lit_warehouse(crate::common::gid("Warehouse", &wid), vec![], vec![]);
            w.name = Some(wname);
            warehouses.push(Box::new(w));
        }
    }
    // methods with channel listings (prices)
    let mrows = q("SELECT m.id, m.name, m.description::text AS description, m.type, m.tax_class_id, t.name AS tax_name FROM shipping_shippingmethod m LEFT JOIN tax_taxclass t ON t.id = m.tax_class_id WHERE m.shipping_zone_id = $1 ORDER BY m.id".into(),
        vec![zid.into()]).await.map_err(|e| e.to_string())?;
    let mids: Vec<i32> = mrows.iter().filter_map(|r| r.try_get::<i32>("", "id").ok()).collect();
    let mut listings: HashMap<i32, Vec<gen::ShippingMethodChannelListing>> = HashMap::new();
    if !mids.is_empty() {
        let list = (1..=mids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
        for r in q(format!("SELECT l.id, l.shipping_method_id, l.channel_id, l.price_amount, l.minimum_order_price_amount, l.maximum_order_price_amount, l.currency, c.slug, c.name AS cname FROM shipping_shippingmethodchannellisting l JOIN channel_channel c ON c.id = l.channel_id WHERE l.shipping_method_id IN ({list})"),
            mids.iter().map(|i| (*i).into()).collect()).await.map_err(|e| e.to_string())? {
            if let (Ok(_), Ok(mid), Ok(ch)) = (r.try_get::<i32>("", "id"), r.try_get::<i32>("", "shipping_method_id"), r.try_get::<i32>("", "channel_id")) {
                let cur = r.try_get::<String>("", "currency").unwrap_or_else(|_| "USD".into());
                let m = |a: Option<rust_decimal::Decimal>| a.map(|v| crate::common::Money { amount: v.to_string(), currency: cur.clone(), fraction_digits: None });
                let mut c = crate::metadata::lit_channel(crate::common::gid("Channel", ch), vec![], vec![]);
                c.slug = r.try_get::<String>("", "slug").ok();
                c.name = r.try_get::<String>("", "cname").ok();
                c.currency_code = Some(cur.clone());
                listings.entry(mid).or_default().push(gen::ShippingMethodChannelListing {
                    id: r.try_get::<i32>("", "id").ok().map(|lid| ID(crate::common::gid("ShippingMethodChannelListing", lid))),
                    channel: Some(Box::new(c)),
                    maximum_order_price: m(r.try_get::<Option<rust_decimal::Decimal>>("", "maximum_order_price_amount").ok().flatten()),
                    minimum_order_price: m(r.try_get::<Option<rust_decimal::Decimal>>("", "minimum_order_price_amount").ok().flatten()),
                    price: m(r.try_get::<Option<rust_decimal::Decimal>>("", "price_amount").ok().flatten()),
                });
            }
        }
    }
    let mut methods = vec![];
    let mut mutplo: Vec<(rust_decimal::Decimal, String)> = vec![];
    for m in mrows {
        let Ok(mid) = m.try_get::<i32>("", "id") else { continue };
        let mut sm = crate::metadata::lit_shipping_method_type(crate::common::gid("ShippingMethod", mid), vec![], vec![]);
        sm.name = m.try_get::<String>("", "name").ok();
        sm.description = m.try_get::<Option<String>>("", "description").ok().flatten().map(gen::GenJSONString);
        sm.r#type = m.try_get::<String>("", "type").ok();
        if let Some(tn) = m.try_get::<Option<String>>("", "tax_name").ok().flatten() {
            let mut tc = crate::metadata::lit_tax_class(crate::common::gid("TaxClass", 0), vec![], vec![]);
            tc.name = Some(tn);
            sm.tax_class = Some(tc);
        }
        sm.channel_listings = listings.get(&mid).cloned().unwrap_or_default();
        for l in &sm.channel_listings {
            if let Some(p) = l.price.as_ref().and_then(|x| x.amount.parse::<rust_decimal::Decimal>().ok()) {
                mutplo.push((p, l.price.as_ref().map(|x| x.currency.clone()).unwrap_or_else(|| "USD".into())));
            }
        }
        methods.push(sm);
    }
    let price_range = if mutplo.is_empty() { None } else {
        mutplo.sort_by(|a, b| a.0.cmp(&b.0));
        let (lo, lc) = mutplo.first().cloned().unwrap();
        let (hi, _) = mutplo.last().cloned().unwrap();
        Some(gen::MoneyRange {
            start: Some(crate::common::Money { amount: lo.to_string(), currency: lc.clone(), fraction_digits: None }),
            stop: Some(crate::common::Money { amount: hi.to_string(), currency: lc, fraction_digits: None }),
        })
    };
    Ok(Some(gen::ShippingZone {
        id: Some(ID(crate::common::gid("ShippingZone", zid))),
        private_metadata: meta("private_metadata"),
        metadata: meta("metadata"),
        name: z.try_get::<String>("", "name").ok(),
        default: z.try_get::<bool>("", "default").ok(),
        price_range,
        countries,
        shipping_methods: methods,
        warehouses,
        channels,
        description: z.try_get::<String>("", "description").ok(),
    }))
}

/// Minimal user object for gift-card relations (id/email/names only).
async fn gc_users(
    db: &sea_orm::DatabaseConnection,
    ids: &[i32],
) -> std::collections::HashMap<i32, gen::User> {
    use sea_orm::{ConnectionTrait, Statement};
    let mut out = std::collections::HashMap::new();
    if ids.is_empty() {
        return out;
    }
    let list = (1..=ids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
    let rows = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        format!("SELECT id, email, first_name, last_name FROM account_user WHERE id IN ({list})"),
        ids.iter().map(|i| (*i).into()).collect::<Vec<sea_orm::Value>>(),
    )).await.unwrap_or_default();
    for r in rows {
        if let (Ok(uid), Ok(email)) = (r.try_get::<i32>("", "id"), r.try_get::<String>("", "email")) {
            let mut u = crate::metadata::lit_user(crate::common::gid("User", uid), vec![], vec![]);
            u.email = Some(email);
            u.first_name = Some(r.try_get::<String>("", "first_name").unwrap_or_default());
            u.last_name = Some(r.try_get::<String>("", "last_name").unwrap_or_default());
            out.insert(uid, u);
        }
    }
    out
}

/// Full gift-card details (dashboard GiftCardData + events).
pub(crate) async fn assemble_gift_card(
    db: &sea_orm::DatabaseConnection,
    gid_int: i32,
) -> Result<Option<gen::GiftCard>, String> {
    use sea_orm::{ConnectionTrait, Statement};
    use std::collections::HashMap;
    let rows = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, code, created_at, last_used_on, is_active, initial_balance_amount, \
                current_balance_amount, currency, app_id, created_by_id, created_by_email, \
                expiry_date, metadata, private_metadata, product_id, assigned_to_id, \
                assigned_to_email FROM giftcard_giftcard WHERE id = $1",
        [gid_int.into()],
    )).await.map_err(|e| e.to_string())?;
    let Some(r) = rows.into_iter().next() else { return Ok(None) };
    let meta = |c: &str| {
        r.try_get::<serde_json::Value>("", c).ok()
            .map(|v| crate::common::json_to_metadata_items(&v)).unwrap_or_default()
    };
    let money = |a: rust_decimal::Decimal, cur: String| crate::common::Money { amount: a.to_string(), currency: cur, fraction_digits: None };
    let cur = r.try_get::<String>("", "currency").unwrap_or_else(|_| "USD".into());
    let code: String = r.try_get::<String>("", "code").unwrap_or_default();
    let last4: String = code.chars().rev().take(4).collect::<String>().chars().rev().collect();
    // users + product + tags, batched
    let mut uids: Vec<i32> = vec![];
    for c in ["created_by_id", "assigned_to_id"] {
        if let Some(u) = r.try_get::<Option<i32>>("", c).ok().flatten() {
            uids.push(u);
        }
    }
    let pid = r.try_get::<Option<i32>>("", "product_id").ok().flatten();
    let mut product = None;
    if let Some(p) = pid {
        let prows = db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT id, name FROM product_product WHERE id = $1",
            [p.into()],
        )).await.map_err(|e| e.to_string())?;
        if let Some(pr) = prows.into_iter().next() {
            let mut pp = crate::metadata::lit_product(crate::common::gid("Product", p), vec![], vec![]);
            pp.name = pr.try_get::<String>("", "name").ok();
            product = Some(pp);
        }
    }
    let mut tags = vec![];
    for t in db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT t.id, t.name FROM giftcard_giftcard_tags gt JOIN giftcard_giftcardtag t ON t.id = gt.giftcardtag_id WHERE gt.giftcard_id = $1",
        [gid_int.into()],
    )).await.map_err(|e| e.to_string())? {
        if let (Ok(tid), Ok(tname)) = (t.try_get::<i32>("", "id"), t.try_get::<String>("", "name")) {
            tags.push(gen::GiftCardTag { id: Some(ID(crate::common::gid("GiftCardTag", tid))), name: Some(tname) });
        }
    }
    // events with JSON parameters (Saleor resolvers read `parameters.*`)
    let erows = db.query_all(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, date, type, parameters, app_id, user_id, order_id FROM giftcard_giftcardevent WHERE gift_card_id = $1 ORDER BY date, id",
        [gid_int.into()],
    )).await.map_err(|e| e.to_string())?;
    // users/apps/orders referenced by events
    let mut euids: Vec<i32> = uids.clone();
    let mut eappids: Vec<i32> = vec![];
    let mut eoids: Vec<String> = vec![];
    let mut evs: Vec<(i32, String, serde_json::Value, Option<i32>, Option<i32>, Option<String>)> = vec![];
    for e in &erows {
        let eid: i32 = match e.try_get::<i32>("", "id") { Ok(v) => v, Err(_) => continue };
        let params: serde_json::Value = e.try_get::<serde_json::Value>("", "parameters").unwrap_or(serde_json::Value::Null);
        let get_int = |k: &str| params.get(k).and_then(|v| v.as_i64()).map(|v| v as i32);
        if let Some(u) = e.try_get::<Option<i32>>("", "user_id").ok().flatten() { euids.push(u); }
        if let Some(a) = e.try_get::<Option<i32>>("", "app_id").ok().flatten() { eappids.push(a); }
        for k in ["assigned_to_id", "previous_assigned_to_id"] {
            if let Some(u) = get_int(k) { euids.push(u); }
        }
        if let Some(o) = e.try_get::<Option<uuid::Uuid>>("", "order_id").ok().flatten() {
            eoids.push(o.to_string());
        }
        evs.push((eid,
            e.try_get::<String>("", "type").unwrap_or_default(),
            params,
            e.try_get::<Option<i32>>("", "user_id").ok().flatten(),
            e.try_get::<Option<i32>>("", "app_id").ok().flatten(),
            e.try_get::<Option<uuid::Uuid>>("", "order_id").ok().flatten().map(|u| u.to_string())));
    }
    euids.sort_unstable();
    euids.dedup();
    let users = gc_users(db, &euids).await;
    let mut apps: HashMap<i32, gen::App> = HashMap::new();
    if !eappids.is_empty() {
        eappids.sort_unstable();
        eappids.dedup();
        let list = (1..=eappids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
        for a in db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT id, name FROM app_app WHERE id IN ({list})"),
            eappids.iter().map(|i| (*i).into()).collect::<Vec<sea_orm::Value>>(),
        )).await.map_err(|e| e.to_string())? {
            if let (Ok(aid), Ok(aname)) = (a.try_get::<i32>("", "id"), a.try_get::<String>("", "name")) {
                let mut app = crate::metadata::lit_app(crate::common::gid("App", aid), vec![], vec![]);
                app.name = Some(aname);
                apps.insert(aid, app);
            }
        }
    }
    let mut onums: HashMap<String, i32> = HashMap::new();
    if !eoids.is_empty() {
        let list = eoids.iter().enumerate().map(|(i, _)| format!("${}::uuid", i + 1)).collect::<Vec<_>>().join(", ");
        for o in db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT id::text AS id, number FROM order_order WHERE id IN ({list})"),
            eoids.iter().map(|s| s.clone().into()).collect::<Vec<sea_orm::Value>>(),
        )).await.map_err(|e| e.to_string())? {
            if let (Ok(oid), Ok(num)) = (o.try_get::<String>("", "id"), o.try_get::<i32>("", "number")) {
                onums.insert(oid, num);
            }
        }
    }
    let mnode = |o: &serde_json::Value, cur: &str| {
        o.as_str().and_then(|s| s.parse::<rust_decimal::Decimal>().ok())
            .or_else(|| o.as_f64().and_then(|f| rust_decimal::Decimal::from_f64_retain(f)))
            .map(|d| money(d, cur.to_string()))
    };
    let mut events = vec![];
    for (eid, etype, params, euid, eaid, eoid) in &evs {
        let bal = params.get("balance");
        let bal_cur = bal.and_then(|b| b.get("currency")).and_then(|c| c.as_str()).unwrap_or(&cur).to_string();
        let get_bal = |k: &str| bal.and_then(|b| b.get(k)).and_then(|v| mnode(v, &bal_cur));
        let asg = if etype == "ASSIGNED_TO_USER" || etype == "UNASSIGNED_FROM_USER" {
            Some(gen::GiftCardEventAssignment {
                old_assigned_to: params.get("previous_assigned_to_id").and_then(|v| v.as_i64()).map(|v| v as i32)
                    .and_then(|u| users.get(&u)).map(|u| Box::new(u.clone())),
                current_assigned_to: params.get("assigned_to_id").and_then(|v| v.as_i64()).map(|v| v as i32)
                    .and_then(|u| users.get(&u)).map(|u| Box::new(u.clone())),
                old_assigned_to_email: params.get("previous_assigned_to_email").and_then(|v| v.as_str()).map(|s| s.to_string()),
                current_assigned_to_email: params.get("assigned_to_email").and_then(|v| v.as_str()).map(|s| s.to_string()),
            })
        } else { None };
        let parse_day = |k: &str| {
            params.get(k).and_then(|v| v.as_str())
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|n| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(n, chrono::Utc))
        };
        events.push(gen::GiftCardEvent {
            id: Some(ID(crate::common::gid("GiftCardEvent", eid))),
            date: None,
            r#type: Some(etype.clone()),
            user: euid.and_then(|u| users.get(&u)).map(|u| Box::new(u.clone())),
            app: eaid.and_then(|a| apps.get(&a)).cloned(),
            message: params.get("message").and_then(|v| v.as_str()).map(|s| s.to_string()),
            email: params.get("email").and_then(|v| v.as_str()).map(|s| s.to_string()),
            order_id: eoid.clone().map(|o| ID(crate::common::gid("Order", o))),
            order_number: eoid.clone().and_then(|o| onums.get(&o)).map(|n| n.to_string()),
            tags: params.get("tags").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default(),
            old_tags: params.get("old_tags").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default(),
            balance: bal.map(|_| gen::GiftCardEventBalance {
                initial_balance: get_bal("initial_balance"),
                current_balance: get_bal("current_balance"),
                old_initial_balance: get_bal("old_initial_balance"),
                old_current_balance: get_bal("old_current_balance"),
            }),
            assigned_to: asg,
            expiry_date: parse_day("expiry_date"),
            old_expiry_date: parse_day("old_expiry_date"),
        });
    }
    let to_dt = |c: &str| r.try_get::<Option<chrono::DateTime<chrono::Utc>>>("", c).ok().flatten();
    let exp: Option<chrono::DateTime<chrono::Utc>> = r.try_get::<Option<chrono::NaiveDate>>("", "expiry_date").ok().flatten()
        .and_then(|d| d.and_hms_opt(0, 0, 0)).map(|n| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(n, chrono::Utc));
    // bought-in channel: BOUGHT event -> order -> channel slug (Saleor parity)
    let bought_oid: Option<String> = evs.iter()
        .find(|(_, t, _, _, _, _)| t == "BOUGHT")
        .and_then(|(_, _, _, _, _, o)| o.clone());
    let mut bought_in_channel = None;
    if let Some(oid) = bought_oid {
        let corows = db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT c.slug FROM order_order o JOIN channel_channel c ON c.id = o.channel_id WHERE o.id = $1::uuid",
            [oid.into()],
        )).await.map_err(|e| e.to_string())?;
        bought_in_channel = corows.into_iter().next().and_then(|r| r.try_get::<String>("", "slug").ok());
    }
    Ok(Some(gen::GiftCard {
        id: Some(ID(crate::common::gid("GiftCard", gid_int))),
        private_metadata: meta("private_metadata"),
        metadata: meta("metadata"),
        display_code: Some(last4.clone()),
        last4_code_chars: Some(last4),
        code: Some(code),
        created: to_dt("created_at"),
        created_by: r.try_get::<Option<i32>>("", "created_by_id").ok().flatten().and_then(|u| users.get(&u)).cloned().map(Box::new),
        created_by_email: r.try_get::<Option<String>>("", "created_by_email").ok().flatten(),
        assigned_to: r.try_get::<Option<i32>>("", "assigned_to_id").ok().flatten().and_then(|u| users.get(&u)).cloned().map(Box::new),
        assigned_to_email: r.try_get::<Option<String>>("", "assigned_to_email").ok().flatten(),
        last_used_on: to_dt("last_used_on"),
        expiry_date: exp,
        app: r.try_get::<Option<i32>>("", "app_id").ok().flatten().and_then(|a| apps.get(&a)).cloned(),
        product,
        events,
        tags,
        bought_in_channel,
        is_active: r.try_get::<bool>("", "is_active").ok(),
        initial_balance: r.try_get::<rust_decimal::Decimal>("", "initial_balance_amount").ok().map(|d| money(d, cur.clone())),
        current_balance: r.try_get::<rust_decimal::Decimal>("", "current_balance_amount").ok().map(|d| money(d, cur.clone())),
    }))
}
