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
                let list: Vec<i32> = ids.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect();
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
        let Some(gid) = rustygod_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        Ok(assemble_gift_card(db, gid).await.map_err(Error::new)?)
    }

    /// Saleor `shippingZone(id)`.
    async fn shipping_zone(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::ShippingZone>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(zid) = rustygod_db::catalog::parse_gid(&id.0) else { return Ok(None) };
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
                let list: Vec<i32> = prods.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect();
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
        type Ch = rustygod_db::entities::channel_channel::Entity;
        use rustygod_db::entities::channel_channel::Column as ChCol;
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
            rustygod_db::catalog::parse_gid(&i.0)
        } else if let Some(s) = slug {
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
            rustygod_db::entities::channel_channel::Entity::find()
                .select_only().column(rustygod_db::entities::channel_channel::Column::Id)
                .filter(rustygod_db::entities::channel_channel::Column::Slug.eq(s))
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
        let rows = rustygod_db::commerce::list_warehouses(db, "default-channel").await.map_err(|e| Error::new(e.to_string()))?;
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
        let rows = rustygod_db::entities::menu_menu::Entity::find().all(db).await.map_err(|e| Error::new(e.to_string()))?;
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
        let rows = rustygod_db::commerce::list_tax_classes(db).await.map_err(|e| Error::new(e.to_string()))?;
        let edges = rows.into_iter().skip(off).take(lim).map(|(id, name)| gen::TaxClassCountableEdge { node: Some(gen::TaxClass {
            id: Some(ID(crate::common::gid("TaxClass", id))),
            private_metadata: vec![],
            metadata: vec![],
            name: Some(name),
            countries: vec![],
        })}).collect();
        Ok(gen::TaxClassCountableConnection { edges, page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })
    }

    async fn shipping_methods(&self, ctx: &Context<'_>, channel: Option<String>) -> Result<Vec<GqlShippingMethod>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let rows = rustygod_db::commerce::list_shipping_methods(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
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
        let rows = rustygod_db::commerce::list_pages(db).await.map_err(|e| Error::new(e.to_string()))?;
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
        let rows = rustygod_db::commerce::list_promotions(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
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
            rustygod_db::entities::menu_menu::Entity::find()
                .filter(rustygod_db::entities::menu_menu::Column::Name.eq(n))
                .one(db).await.map_err(|e| Error::new(e.to_string()))?.map(|m| m.slug)
        } else if let Some(i) = id {
            use sea_orm::EntityTrait;
            let pk: i32 = rustygod_db::catalog::parse_gid(&i.0).unwrap_or(-1);
            rustygod_db::entities::menu_menu::Entity::find_by_id(pk).one(db).await.map_err(|e| Error::new(e.to_string()))?.map(|m| m.slug)
        } else { None };
        let m = match resolved_slug {
            Some(s) => rustygod_db::commerce::get_menu(db, &s).await.map_err(|e| Error::new(e.to_string()))?,
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
    let site: Option<String> = rustygod_db::entities::django_site::Entity::find()
        .select_only().column(rustygod_db::entities::django_site::Column::Name)
        .filter(rustygod_db::entities::django_site::Column::Id.eq(1))
        .into_tuple().one(db).await.ok().flatten();
    let desc: Option<String> = rustygod_db::entities::site_sitesettings::Entity::find()
        .select_only().column(rustygod_db::entities::site_sitesettings::Column::Description)
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
    let v: Option<serde_json::Value> = rustygod_db::entities::site_sitesettings::Entity::find()
        .select_only()
        .column(rustygod_db::entities::site_sitesettings::Column::Metadata)
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
    let rows = rustygod_db::entities::permission_permission::Entity::find()
        .select_only()
        .column(rustygod_db::entities::permission_permission::Column::Codename)
        .column(rustygod_db::entities::permission_permission::Column::Name)
        .order_by_asc(rustygod_db::entities::permission_permission::Column::Codename)
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
        let bearer = ctx.data_opt::<crate::context::Bearer>().map(|b| b.0.as_str().to_string())
            .or_else(|| ctx.data_opt::<GqlContext>().and_then(|g| g.bearer.clone()));
        if bearer.is_none() { return Err(Error::new("authentication required")); }
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let serr = |m: String| gen::ShippingError { field: None, message: Some(m), code: None, channels: vec![] };
        let Some(zid) = rustygod_db::catalog::parse_gid(&id.0) else {
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
        let ch_ids = |ids: &Option<Vec<ID>>| ids.as_ref().map(|v| v.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect::<Vec<_>>()).unwrap_or_default();
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
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
        if let Some(meta) = input.metadata.clone().or(input.private_metadata.clone()) {
            if let Some(row) = rustygod_db::entities::site_sitesettings::Entity::find()
                .filter(rustygod_db::entities::site_sitesettings::Column::Id.eq(1))
                .one(db)
                .await
                .map_err(|e| Error::new(e.to_string()))?
            {
                let merged = crate::common::merge_metadata(
                    &serde_json::to_value(&row.metadata).unwrap_or(serde_json::Value::Null),
                    &meta,
                );
                let mut am: rustygod_db::entities::site_sitesettings::ActiveModel = row.into();
                if input.private_metadata.is_some() {
                    am.private_metadata = Set(merged.clone());
                } else {
                    am.metadata = Set(merged);
                }
                am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            }
        }
        if let Some(desc) = input.description.clone() {
            if let Some(row) = rustygod_db::entities::site_sitesettings::Entity::find()
                .filter(rustygod_db::entities::site_sitesettings::Column::Id.eq(1))
                .one(db).await.map_err(|e| Error::new(e.to_string()))?
            {
                let mut am: rustygod_db::entities::site_sitesettings::ActiveModel = row.into();
                am.description = Set(desc);
                am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            }
        }
        if let Some(site_name) = input.name.clone() {
            if let Some(row) = rustygod_db::entities::django_site::Entity::find_by_id(1)
                .one(db).await.map_err(|e| Error::new(e.to_string()))?
            {
                let mut am: rustygod_db::entities::django_site::ActiveModel = row.into();
                am.name = Set(site_name);
                am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            }
        }
        let shop = to_gen_shop(ctx).await?;
        Ok(GqlShopSettingsUpdate { shop: Some(shop), errors: vec![] })
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
async fn assemble_gift_card(
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
