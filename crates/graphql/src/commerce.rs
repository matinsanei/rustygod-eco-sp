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
        version: Some("3.24.0-a.0".into()),
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

    async fn channels(&self, ctx: &Context<'_>) -> Result<Vec<gen::Channel>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        // Slim select: channel_channel has INTERVAL columns
        // (delete_expired_orders_after, checkout_ttl_before_releasing_funds)
        // that SeaORM cannot decode into the entity's String fields.
        use sea_orm::{EntityTrait, QuerySelect};
        type Ch = rustygod_db::entities::channel_channel::Entity;
        use rustygod_db::entities::channel_channel::Column as ChCol;
        let rows: Vec<(i32, String, String, bool, String, String, String)> = Ch::find()
            .select_only()
            .column(ChCol::Id).column(ChCol::Slug).column(ChCol::Name)
            .column(ChCol::IsActive).column(ChCol::CurrencyCode)
            .column(ChCol::DefaultCountry).column(ChCol::AllocationStrategy)
            .into_tuple().all(db).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(|(id, slug, name, is_active, currency_code, default_country, allocation_strategy)| gen::Channel {
            id: Some(ID(id.to_string())),
            private_metadata: vec![],
            metadata: vec![],
            slug: Some(slug),
            name: Some(name),
            is_active: Some(is_active),
            currency_code: Some(currency_code),
            has_orders: None,
            default_country: Some(GqlCountryDisplay { code: default_country.clone(), country: default_country }),
            warehouses: vec![],
            stock_settings: Some(GqlStockSettings { allocation_strategy }),
            order_settings: None,
            checkout_settings: None,
            payment_settings: None,
            tax_configuration: None,
        }).collect())
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
            id: Some(ID(w.id)),
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
            id: Some(ID(m.id.to_string())),
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
            id: Some(ID(id.to_string())),
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
        Ok(rows.into_iter().map(|m| GqlShippingMethod { id: ID(m.id.to_string()), name: m.name, price: m.price_amount.to_string() }).collect())
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
            id: Some(ID(p.id.to_string())),
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
            id: Some(ID(p.id)),
            private_metadata: vec![],
            metadata: vec![],
            name: Some(p.name),
            r#type: Some(p.promotion_type),
            description: None,
            start_date: None,
            end_date: None,
            rules: vec![],
        })}).collect();
        Ok(gen::PromotionCountableConnection { edges, page_info: Some(crate::common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })
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
            let pk: i32 = i.0.parse().unwrap_or(-1);
            rustygod_db::entities::menu_menu::Entity::find_by_id(pk).one(db).await.map_err(|e| Error::new(e.to_string()))?.map(|m| m.slug)
        } else { None };
        let m = match resolved_slug {
            Some(s) => rustygod_db::commerce::get_menu(db, &s).await.map_err(|e| Error::new(e.to_string()))?,
            None => None,
        };
        Ok(m.map(|x| gen::Menu {
            id: Some(ID(x.id.to_string())),
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

#[Object]
impl CommerceMutation {
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
