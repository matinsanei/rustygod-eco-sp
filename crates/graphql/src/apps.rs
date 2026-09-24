//! Apps GraphQL: `apps { ...InstalledApp }` + `appExtensions { ... }` —
//! boot-time Dashboard queries (InstalledApps, ExtensionList, AppAlerts).
//! Reads the real `app_app` / `app_appextension` rows Django manages;
//! alert sub-selections (`problems`, `webhooks.eventDeliveries`) resolve
//! empty (same "no alerts" state as a healthy Saleor).

use async_graphql::*;
use chrono::{DateTime, Utc};

use crate::common::{decode_cursor, encode_cursor, PageInfo};
use crate::context::GqlContext;
use crate::gen;

#[derive(SimpleObject, Clone)]
#[graphql(name = "EventDeliveryAttemptCountableConnection")]
pub struct GqlEventDeliveryAttemptConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlEventDeliveryAttemptEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(Clone, Default)]
pub struct GqlAppBrandLogo;

#[Object(name = "AppBrandLogo")]
impl GqlAppBrandLogo {
    /// Saleor `AppBrandLogo.default(size, format): String!` — logo URL.
    /// No brand assets in this backend → empty string (Dashboard hides it).
    async fn default(
        &self,
        size: Option<i32>,
        format: Option<IconThumbnailFormatEnum>,
    ) -> String {
        let _ = (size, format);
        String::new()
    }
}

/// Saleor `IconThumbnailFormatEnum` — Dashboard passes `format: WEBP`.
#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum IconThumbnailFormatEnum {
    #[graphql(name = "ORIGINAL")]
    #[default]
    Original,
    #[graphql(name = "AVIF")]
    Avif,
    #[graphql(name = "WEBP")]
    Webp,
}

/// Saleor `AppTypeEnum` (`apps(filter: {type: THIRDPARTY})`).
/// Kept here (not generated) because `gen::AppFilterInput.type` references
/// this exact GraphQL name.
#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppTypeEnum {
    #[graphql(name = "LOCAL")]
    Local,
    #[graphql(name = "THIRDPARTY")]
    Thirdparty,
}

#[derive(SimpleObject, Clone, Default)]
#[graphql(name = "AppBrand")]
pub struct GqlAppBrand {
    pub logo: Option<GqlAppBrandLogo>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlEventDeliveryAttemptEdge {
    pub node: gen::EventDeliveryAttempt,
    pub cursor: String,
}

#[derive(SimpleObject, Clone)]
pub struct GqlEventDeliveryEdge {
    pub node: gen::EventDelivery,
    pub cursor: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "EventDeliveryCountableConnection")]
pub struct GqlEventDeliveryConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlEventDeliveryEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(SimpleObject, Clone)]
pub struct GqlAppEdge {
    pub node: gen::App,
    pub cursor: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "AppCountableConnection")]
pub struct GqlAppConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlAppEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
}

/// Saleor `AppExtension` — Dashboard `ExtensionList` fragment shape.
#[derive(SimpleObject, Clone)]
#[graphql(name = "AppExtension")]
pub struct GqlAppExtension {
    pub id: ID,
    pub label: String,
    pub identifier: Option<String>,
    pub url: String,
    #[graphql(name = "mountName")]
    pub mount_name: String,
    #[graphql(name = "targetName")]
    pub target_name: String,
    pub settings: serde_json::Value,
    #[graphql(name = "accessToken")]
    pub access_token: Option<String>,
    pub permissions: Vec<crate::commerce::GqlPermission>,
    pub app: gen::App,
}

#[derive(SimpleObject, Clone)]
pub struct GqlAppExtensionEdge {
    pub node: GqlAppExtension,
    pub cursor: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "AppExtensionCountableConnection")]
pub struct GqlAppExtensionConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlAppExtensionEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(SimpleObject, Clone)]
pub struct GqlWebhookEvent {
    pub name: String,
    #[graphql(name = "eventType")]
    pub event_type: gen::WebhookEventTypeEnum,
}

#[derive(SimpleObject, Clone)]
pub struct GqlExportFileEdge { pub node: Option<gen::ExportFile> }

#[derive(SimpleObject, Clone)]
pub struct GqlExportFileConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlExportFileEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(Default)]
pub struct AppsQuery;

#[Object]
impl AppsQuery {
    /// One app by id (Django `app`).
    async fn app(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::App>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let aid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        mini_app(db, aid).await
    }

    /// Installation jobs (Django `appsInstallations`: plain list).
    async fn apps_installations(&self, ctx: &Context<'_>) -> Result<Vec<gen::AppInstallation>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{EntityTrait, QueryOrder};
        let rows = saleor_rustify_db::entities::app_appinstallation::Entity::find()
            .order_by_desc(saleor_rustify_db::entities::app_appinstallation::Column::Id)
            .all(db)
            .await
            .map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.iter().map(installation_view).collect())
    }

    /// Dashboard `InstalledApps*` / `AppHasProblems` / `AppFailedPendingWebhooks`.
    /// Filter input taken as tolerant JSON (documented gap: type filtering
    /// not applied server-side; alerts resolve empty regardless).
    async fn apps(
        &self,
        ctx: &Context<'_>,
        before: Option<String>,
        after: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        filter: Option<gen::AppFilterInput>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::AppSortingInput>,
    ) -> Result<GqlAppConnection> {
        let _ = (before, last, filter, sort_by);
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let rows = load_apps(db).await.map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len() as i32;
        let off = after.and_then(|c| decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(100).clamp(1, 100) as usize;
        let edges = rows
            .into_iter()
            .skip(off)
            .take(lim)
            .enumerate()
            .map(|(i, a)| GqlAppEdge { node: a, cursor: encode_cursor(off + i) })
            .collect();
        Ok(GqlAppConnection {
            total_count: Some(total),
            edges,
            page_info: PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None },
        })
    }

    /// Dashboard `ExtensionList` (`appExtensions(filter, first)`).
    async fn app_extensions(
        &self,
        ctx: &Context<'_>,
        filter: Option<gen::AppExtensionFilterInput>,
        first: Option<i32>,
        before: Option<String>,
        after: Option<String>,
        last: Option<i32>,
    ) -> Result<GqlAppExtensionConnection> {
        let _ = (filter, before, last);
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let rows = load_extensions(db).await.map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len() as i32;
        let off = after.and_then(|c| decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(100).clamp(1, 100) as usize;
        let edges = rows
            .into_iter()
            .skip(off)
            .take(lim)
            .enumerate()
            .map(|(i, e)| GqlAppExtensionEdge { node: e, cursor: encode_cursor(off + i) })
            .collect();
        Ok(GqlAppExtensionConnection {
            total_count: Some(total),
            edges,
            page_info: PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None },
        })
    }

    async fn app_extension(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GqlAppExtension>> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let eid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        let rows = load_extensions(db).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(rows.into_iter().find(|e| {
            saleor_rustify_db::catalog::parse_gid(&e.id.0).unwrap_or(-2) == eid
        }))
    }

    /// Events this backend emits that have Saleor enum equivalents (outbox
    /// fan-out). `checkout_completed` / `fulfillment_returned` are internal
    /// outbox names with no Saleor counterpart yet — documented, not faked.
    async fn webhook_events(&self) -> Result<Vec<GqlWebhookEvent>> {
        use gen::WebhookEventTypeEnum as E;
        Ok(vec![
            GqlWebhookEvent { name: "order_created".into(), event_type: E::ORDERCREATED },
            GqlWebhookEvent { name: "order_updated".into(), event_type: E::ORDERUPDATED },
            GqlWebhookEvent { name: "order_cancelled".into(), event_type: E::ORDERCANCELLED },
        ])
    }

    async fn webhook_sample_payload(
        &self, #[graphql(name = "eventType")] event_type: gen::WebhookEventTypeEnum,
    ) -> Result<Option<serde_json::Value>> {
        let name = format!("{event_type:?}");
        let sample = match name.as_str() {
            "ORDERCREATED" => serde_json::json!({"id": "T3JkZXI6MQ==", "number": "1", "status": "unfulfilled", "checkout_token": "00000000-0000-0000-0000-000000000000"}),
            "ORDERUPDATED" => serde_json::json!({"id": "T3JkZXI6MQ==", "number": "1", "status": "partially_fulfilled"}),
            "ORDERCANCELLED" => serde_json::json!({"id": "T3JkZXI6MQ==", "number": "1", "status": "canceled"}),
            "CHECKOUTCOMPLETED" => serde_json::json!({"id": "T3JkZXI6MQ==", "number": "1", "status": "unfulfilled", "checkout_token": "00000000-0000-0000-0000-000000000000"}),
            "FULFILLMENTRETURNED" => serde_json::json!({"order_id": "T3JkZXI6MQ==", "lines": []}),
            _ => serde_json::json!({"event": name}),
        };
        Ok(Some(sample))
    }

    /// Export files (real job rows; Django `exportFiles`).
    async fn export_files(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        filter: Option<gen::ExportFileFilterInput>,
    ) -> Result<GqlExportFileConnection> {
        let _ = (before, last, filter);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{EntityTrait, QueryOrder};
        let rows = saleor_rustify_db::entities::csv_exportfile::Entity::find()
            .order_by_desc(saleor_rustify_db::entities::csv_exportfile::Column::Id)
            .all(db)
            .await
            .map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len() as i32;
        let off = after.and_then(|c| decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        let mut edges = vec![];
        for (i, r) in rows.into_iter().skip(off).take(lim).enumerate() {
            let node = gen::ExportFile {
                id: Some(ID(crate::common::gid("ExportFile", r.id))),
                status: Some(r.status.to_uppercase()),
                url: r.content_file.map(|p| crate::common::media_url(&p)),
            };
            edges.push(GqlExportFileEdge { node: Some(node) });
            let _ = i;
        }
        Ok(GqlExportFileConnection {
            total_count: Some(total),
            edges,
            page_info: PageInfo { has_next_page: off + lim < total as usize, has_previous_page: off > 0, start_cursor: None, end_cursor: None },
        })
    }

    /// One export file (Django `exportFile`).
    async fn export_file(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::ExportFile>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let jid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        export_file_view(db, jid).await
    }
}

struct AppRow {
    id: i32,
    name: String,
    identifier: String,
    is_active: bool,
    app_type: String,
    app_url: Option<String>,
    manifest_url: Option<String>,
    homepage_url: Option<String>,
    support_url: Option<String>,
    version: Option<String>,
    about_app: Option<String>,
    data_privacy_url: Option<String>,
    created_at: DateTime<Utc>,
}

fn to_gql(a: &AppRow) -> gen::App {
    gen::App {
        id: Some(ID(crate::common::gid("App", a.id))),
        private_metadata: vec![],
        metadata: vec![],
        identifier: Some(a.identifier.clone()),
        permissions: vec![],
        created: Some(a.created_at),
        is_active: Some(a.is_active),
        name: Some(a.name.clone()),
        r#type: Some(a.app_type.clone()),
        tokens: vec![],
        webhooks: vec![],
        about_app: a.about_app.clone(),
        data_privacy_url: a.data_privacy_url.clone(),
        homepage_url: a.homepage_url.clone(),
        support_url: a.support_url.clone(),
        app_url: a.app_url.clone(),
        manifest_url: a.manifest_url.clone(),
        version: a.version.clone(),
        access_token: None,
        author: None,
        brand: None,
    }
}

fn to_row(m: &saleor_rustify_db::entities::app_app::Model) -> AppRow {
    AppRow {
        id: m.id,
        name: m.name.clone(),
        identifier: m.identifier.clone(),
        is_active: m.is_active,
        app_type: m.r#type.clone(),
        app_url: m.app_url.clone(),
        manifest_url: m.manifest_url.clone(),
        homepage_url: m.homepage_url.clone(),
        support_url: m.support_url.clone(),
        version: m.version.clone(),
        about_app: m.about_app.clone(),
        data_privacy_url: m.data_privacy_url.clone(),
        created_at: m.created_at.with_timezone(&Utc),
    }
}

/// Full-model select over `app_app` (no tsvector/interval columns here, so
/// `SELECT *` via the entity Model is safe; extension rows stay slim).
async fn load_apps(
    db: &sea_orm::DatabaseConnection,
) -> Result<Vec<gen::App>, sea_orm::DbErr> {
    use sea_orm::{EntityTrait, QueryOrder};
    use saleor_rustify_db::entities::app_app;
    let rows = app_app::Entity::find()
        .order_by_asc(app_app::Column::Id)
        .all(db)
        .await?;
    Ok(rows.iter().map(|m| to_gql(&to_row(m))).collect())
}

/// Slim select over `app_appextension` + parent app + extension permissions.
async fn load_extensions(
    db: &sea_orm::DatabaseConnection,
) -> Result<Vec<GqlAppExtension>, sea_orm::DbErr> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
    use saleor_rustify_db::entities::{app_app, app_appextension, app_appextension_permissions, permission_permission};
    let exts: Vec<(i32, String, String, i32, String, String, serde_json::Value, Option<String>)> =
        app_appextension::Entity::find()
            .select_only()
            .column(app_appextension::Column::Id)
            .column(app_appextension::Column::Label)
            .column(app_appextension::Column::Url)
            .column(app_appextension::Column::AppId)
            .column(app_appextension::Column::Mount)
            .column(app_appextension::Column::Target)
            .column(app_appextension::Column::Settings)
            .column(app_appextension::Column::Identifier)
            .order_by_asc(app_appextension::Column::Id)
            .into_tuple()
            .all(db)
            .await?;
    // Parent apps in one pass.
    let app_ids: Vec<i32> = exts.iter().map(|e| e.3).collect();
    let mut app_map: std::collections::HashMap<i32, gen::App> = std::collections::HashMap::new();
    if !app_ids.is_empty() {
        let apps = app_app::Entity::find()
            .filter(app_app::Column::Id.is_in(app_ids))
            .all(db)
            .await?;
        for m in &apps {
            let row = to_row(m);
            app_map.insert(row.id, to_gql(&row));
        }
    }
    let mut out = Vec::with_capacity(exts.len());
    for (id, label, url, app_id, mount, target, settings, identifier) in exts {
        let perm_ids: Vec<i32> = app_appextension_permissions::Entity::find()
            .select_only()
            .column(app_appextension_permissions::Column::PermissionId)
            .filter(app_appextension_permissions::Column::AppextensionId.eq(id))
            .into_tuple()
            .all(db)
            .await?;
        let mut permissions = Vec::new();
        for pid in perm_ids {
            if let Some(p) = permission_permission::Entity::find_by_id(pid).one(db).await? {
                permissions.push(crate::commerce::GqlPermission { code: p.codename, name: p.name });
            }
        }
        if let Some(app) = app_map.get(&app_id).cloned() {
            out.push(GqlAppExtension {
                id: ID(crate::common::gid("AppExtension", id)),
                label,
                identifier,
                url,
                mount_name: mount,
                target_name: target,
                settings,
                access_token: None,
                permissions,
                app,
            });
        }
    }
    Ok(out)
}

#[derive(Default)]
pub struct AppsMutation;

fn wherr(field: Option<String>, message: String) -> gen::WebhookError {
    gen::WebhookError { field, message: Some(message), code: None }
}

/// Event enums → stored lowercase strings (Django's event registry).
fn hook_events(input_events: Option<Vec<gen::WebhookEventTypeEnum>>, input_async: Option<Vec<gen::WebhookEventTypeAsyncEnum>>, input_sync: Option<Vec<gen::WebhookEventTypeSyncEnum>>) -> Vec<String> {
    let mut out = vec![];
    for e in input_events.unwrap_or_default() {
        out.push(format!("{e:?}").to_lowercase());
    }
    for e in input_async.unwrap_or_default() {
        out.push(format!("{e:?}").to_lowercase());
    }
    for e in input_sync.unwrap_or_default() {
        out.push(format!("{e:?}").to_lowercase());
    }
    out.sort();
    out.dedup();
    out
}

/// Webhook assembly (events split by sync/async registry).
pub(crate) async fn assemble_webhook(db: &sea_orm::DatabaseConnection, hid: i32) -> Result<Option<gen::Webhook>, Error> {
    use sea_orm::EntityTrait;
    let w = saleor_rustify_db::entities::webhook_webhook::Entity::find_by_id(hid)
        .one(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?;
    let Some(w) = w else { return Ok(None) };
    let events = saleor_rustify_db::webhooks::hook_events(db, hid).await.map_err(|e| Error::new(e.to_string()))?;
    // Sync registry (subset Django treats as synchronous).
    const SYNC: &[&str] = &[
        "payment_list_gateways", "payment_authorize", "payment_capture", "payment_refund",
        "payment_void", "payment_confirm", "payment_process", "order_filter_shipping_methods",
        "checkout_filter_shipping_methods", "shipping_list_methods_for_checkout", "order_calc_taxes",
        "checkout_calc_taxes", "stored_payment_method_delete_requested", "transaction_charge_requested",
        "transaction_refund_requested", "transaction_cancelation_requested", "payment_gateway_initialize_session",
        "transaction_initialize_session", "transaction_process_session", "list_stored_payment_methods",
        "stored_payment_method_request_delete",
    ];
    let mut sync_events = vec![];
    let mut async_events = vec![];
    for ev in events {
        let name = ev.clone();
        let item_a = gen::WebhookEventAsync { name: Some(name.clone()), event_type: Some(ev.clone()) };
        let item_s = gen::WebhookEventSync { name: Some(name), event_type: Some(ev) };
        if SYNC.contains(&item_s.event_type.as_deref().unwrap_or("")) {
            sync_events.push(item_s);
        } else {
            async_events.push(item_a);
        }
    }
    let app = mini_app(db, w.app_id).await?;
    Ok(Some(gen::Webhook {
        id: Some(ID(crate::common::gid("Webhook", hid))),
        name: w.name.clone(),
        sync_events,
        async_events,
        app: app.map(Box::new),
        target_url: Some(w.target_url.clone()),
        is_active: Some(w.is_active),
        secret_key: w.secret_key.clone(),
        subscription_query: w.subscription_query.clone(),
        custom_headers: w.custom_headers.clone().and_then(|v| v.as_str().map(|s| gen::GenJSONString(s.to_string()))),
    }))
}

async fn mini_app(db: &sea_orm::DatabaseConnection, aid: i32) -> Result<Option<gen::App>, Error> {
    use sea_orm::EntityTrait;
    let a = saleor_rustify_db::entities::app_app::Entity::find_by_id(aid)
        .one(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?;
    let Some(a) = a else { return Ok(None) };
    Ok(Some(gen::App {
        id: Some(ID(crate::common::gid("App", aid))),
        private_metadata: vec![],
        metadata: vec![],
        identifier: Some(a.identifier.clone()),
        permissions: vec![],
        created: Some(a.created_at.into()),
        is_active: Some(a.is_active),
        name: Some(a.name.clone()),
        r#type: Some(a.r#type.clone()),
        tokens: vec![],
        webhooks: vec![],
        about_app: a.about_app.clone(),
        data_privacy_url: a.data_privacy_url.clone(),
        homepage_url: a.homepage_url.clone(),
        support_url: a.support_url.clone(),
        app_url: a.app_url.clone(),
        manifest_url: a.manifest_url.clone(),
        version: a.version.clone(),
        access_token: None,
        author: a.author.clone(),
        brand: None,
    }))
}

#[Object]
impl AppsMutation {
    /// Create a webhook on an app (Django `webhookCreate`).
    async fn webhook_create(&self, ctx: &Context<'_>, input: gen::WebhookCreateInput) -> Result<gen::WebhookCreate> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::WebhookCreate { errors: vec![wherr(None, m)], webhook: None };
        let app_id = match input.app.as_ref().and_then(|a| saleor_rustify_db::catalog::parse_gid(&a.0)) {
            Some(a) => a,
            None => return Ok(err("app is required".into())),
        };
        let headers = match input.custom_headers.as_ref() {
            Some(h) => serde_json::from_str::<serde_json::Value>(&h.0).unwrap_or(serde_json::Value::Null),
            None => serde_json::Value::Null,
        };
        let hid = match saleor_rustify_db::webhooks::create_hook(db, &saleor_rustify_db::webhooks::NewHook {
            name: input.name.clone(),
            identifier: input.identifier.clone(),
            target_url: input.target_url.clone(),
            events: hook_events(input.events.clone(), input.async_events.clone(), input.sync_events.clone()),
            app_id,
            is_active: input.is_active.unwrap_or(true),
            secret_key: input.secret_key.clone(),
            subscription_query: input.query.clone(),
            custom_headers: headers,
        }).await {
            Ok(id) => id,
            Err(e) => return Ok(err(e.to_string())),
        };
        Ok(gen::WebhookCreate { errors: vec![], webhook: assemble_webhook(db, hid).await? })
    }

    /// Update a webhook (Django `webhookUpdate`; staff use `id`).
    async fn webhook_update(
        &self, ctx: &Context<'_>, id: Option<ID>, identifier: Option<String>, input: gen::WebhookUpdateInput,
    ) -> Result<gen::WebhookUpdate> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::WebhookUpdate { errors: vec![wherr(None, m)], webhook: None };
        let hid = match id.as_ref().and_then(|i| saleor_rustify_db::catalog::parse_gid(&i.0)) {
            Some(h) => h,
            None => return Ok(err("id is required for staff updates".into())),
        };
        let _ = identifier; // app-scoped identifier path (apps surface milestone)
        let headers = input.custom_headers.as_ref().map(|h| serde_json::from_str::<serde_json::Value>(&h.0).unwrap_or(serde_json::Value::Null));
        let has_events = input.events.is_some() || input.async_events.is_some() || input.sync_events.is_some();
        if let Err(e) = saleor_rustify_db::webhooks::update_hook(db, hid, &saleor_rustify_db::webhooks::HookPatch {
            name: input.name.clone().map(Some),
            target_url: input.target_url.clone(),
            events: has_events.then(|| hook_events(input.events.clone(), input.async_events.clone(), input.sync_events.clone())),
            is_active: input.is_active,
            secret_key: input.secret_key.clone().map(Some),
            subscription_query: input.query.clone().map(Some),
            custom_headers: headers,
        }).await {
            return Ok(err(e.to_string()));
        }
        Ok(gen::WebhookUpdate { errors: vec![], webhook: assemble_webhook(db, hid).await? })
    }

    /// Delete a webhook (Django `webhookDelete`; staff use `id`).
    async fn webhook_delete(&self, ctx: &Context<'_>, id: Option<ID>, identifier: Option<String>) -> Result<gen::WebhookDelete> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let _ = identifier;
        let hid = match id.as_ref().and_then(|i| saleor_rustify_db::catalog::parse_gid(&i.0)) {
            Some(h) => h,
            None => return Ok(gen::WebhookDelete { errors: vec![wherr(None, "id is required".into())] }),
        };
        match saleor_rustify_db::webhooks::delete_hook(db, hid).await {
            Ok(()) => Ok(gen::WebhookDelete { errors: vec![] }),
            Err(e) => Ok(gen::WebhookDelete { errors: vec![wherr(None, e.to_string())] }),
        }
    }

    /// Dry-run: build the object's current payload (the subscription query
    /// AST itself isn't executed — that needs a full subscription engine;
    /// the returned payload is what the query selects from).
    async fn webhook_dry_run(
        &self, ctx: &Context<'_>,
        #[graphql(name = "objectId")] object_id: ID, query: String,
    ) -> Result<gen::WebhookDryRun> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::WebhookDryRun {
            payload: None,
            errors: vec![gen::WebhookDryRunError { field: None, message: Some(m) }],
        };
        let _ = query;
        let raw = &object_id.0;
        let payload = dry_run_payload(db, raw).await;
        match payload {
            Ok(p) => Ok(gen::WebhookDryRun {
                payload: Some(gen::GenJSONString(p)),
                errors: vec![],
            }),
            Err(e) => Ok(err(e)),
        }
    }

    /// Export products to CSV (Django `exportProducts`): synchronous file
    /// job, ALL/IDS scope, served from the media mount.
    async fn export_products(&self, ctx: &Context<'_>, input: gen::ExportProductsInput) -> Result<GqlExportProducts> {
        let _ = crate::account::require_perm(ctx, "manage_products").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let err = |m: String| GqlExportProducts {
            export_file: None,
            errors: vec![gen::ExportError { field: None, message: Some(m), code: None }],
        };
        if !matches!(input.file_type, gen::FileTypesEnum::CSV) {
            return Ok(err("only CSV export is supported".into()));
        }
        if matches!(input.scope, gen::ExportScope::FILTER) && input.filter.is_some() {
            return Ok(err("filtered export is not supported; pass ids or export all".into()));
        }
        let ids = match input.scope {
            gen::ExportScope::IDS => Some(
                input.ids.clone().unwrap_or_default().iter()
                    .filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect::<Vec<_>>(),
            ),
            _ => None,
        };
        match saleor_rustify_db::exports::export_products(db, Some(uid), ids).await {
            Ok(job) => Ok(GqlExportProducts { export_file: export_file_view(db, job).await?, errors: vec![] }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    /// Export gift cards to CSV (Django `exportGiftCards`).
    async fn export_gift_cards(&self, ctx: &Context<'_>, input: gen::ExportGiftCardsInput) -> Result<GqlExportGiftCards> {
        let _ = crate::account::require_perm(ctx, "manage_gift_card").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let err = |m: String| GqlExportGiftCards {
            export_file: None,
            errors: vec![gen::ExportError { field: None, message: Some(m), code: None }],
        };
        if !matches!(input.file_type, gen::FileTypesEnum::CSV) {
            return Ok(err("only CSV export is supported".into()));
        }
        if matches!(input.scope, gen::ExportScope::FILTER) && input.filter.is_some() {
            return Ok(err("filtered export is not supported; pass ids or export all".into()));
        }
        let ids = match input.scope {
            gen::ExportScope::IDS => Some(
                input.ids.clone().unwrap_or_default().iter()
                    .filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect::<Vec<_>>(),
            ),
            _ => None,
        };
        match saleor_rustify_db::exports::export_gift_cards(db, Some(uid), ids).await {
            Ok(job) => Ok(GqlExportGiftCards { export_file: export_file_view(db, job).await?, errors: vec![] }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    /// Export voucher codes to CSV (Django `exportVoucherCodes`).
    async fn export_voucher_codes(&self, ctx: &Context<'_>, input: gen::ExportVoucherCodesInput) -> Result<GqlExportVoucherCodes> {        let _ = crate::account::require_perm(ctx, "manage_discounts").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let err = |m: String| GqlExportVoucherCodes {
            export_file: None,
            errors: vec![gen::ExportError { field: None, message: Some(m), code: None }],
        };
        if !matches!(input.file_type, gen::FileTypesEnum::CSV) {
            return Ok(err("only CSV export is supported".into()));
        }
        let vid = input.voucher_id.as_ref().and_then(|v| saleor_rustify_db::catalog::parse_gid(&v.0));
        let cids = input.ids.clone().map(|ids| {
            ids.iter().filter_map(|i| crate::common::parse_uuid_gid(&i.0)).collect::<Vec<_>>()
        });
        match saleor_rustify_db::exports::export_voucher_codes(db, Some(uid), vid, cids).await {
            Ok(job) => Ok(GqlExportVoucherCodes { export_file: export_file_view(db, job).await?, errors: vec![] }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    /// External notification trigger (Django `externalNotificationTrigger`):
    /// fans a NOTIFY_USER outbox event for the ids; delivery itself rides
    /// the webhook worker like all other notifications.
    async fn external_notification_trigger(
        &self, ctx: &Context<'_>,
        channel: String,
        input: gen::ExternalNotificationTriggerInput,
        #[graphql(name = "pluginId")] plugin_id: Option<String>,
    ) -> Result<GqlExternalNotificationTrigger> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let _ = plugin_id;
        let payload = serde_json::json!({
            "ids": input.ids.iter().map(|i| &i.0).collect::<Vec<_>>(),
            "external_event_type": input.external_event_type,
            "extra_payload": input.extra_payload.as_ref().map(|p| p.0.clone()),
        })
        .to_string();
        match saleor_rustify_db::webhooks::trigger_event(db, "notify_user", Some(&channel), &payload).await {
            Ok(_) => Ok(GqlExternalNotificationTrigger { errors: vec![] }),
            Err(e) => Ok(GqlExternalNotificationTrigger {
                errors: vec![GqlExternalNotificationError { field: None, message: Some(e.to_string()), code: None }],
            }),
        }
    }

    /// Register an app manually (Django `appCreate`): row + first token.
    async fn app_create(&self, ctx: &Context<'_>, input: gen::AppInput) -> Result<gen::AppCreate> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::AppCreate {
            auth_token: None,
            errors: vec![apperr(None, m)],
            app: None,
        };
        let name = input.name.clone().unwrap_or_default();
        if name.trim().is_empty() {
            return Ok(err("name is required".into()));
        }
        let perms: Vec<String> = input.permissions.clone().unwrap_or_default().iter().map(|p| format!("{p:?}").to_lowercase()).collect();
        let aid = match saleor_rustify_db::apps::create_app(db, &name, &perms.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await {
            Ok(id) => id,
            Err(e) => return Ok(err(e.to_string())),
        };
        if let Some(ident) = input.identifier.clone().filter(|s| !s.trim().is_empty()) {
            let _ = saleor_rustify_db::apps::update_app(db, aid, None, Some(ident), None).await;
        }
        let raw = match saleor_rustify_db::apps::create_app_token(db, aid, "default").await {
            Ok((_, r)) => r,
            Err(e) => return Ok(err(e.to_string())),
        };
        Ok(gen::AppCreate {
            auth_token: Some(raw),
            errors: vec![],
            app: mini_app(db, aid).await?,
        })
    }

    /// Update an app (Django `appUpdate`).
    async fn app_update(&self, ctx: &Context<'_>, id: ID, input: gen::AppInput) -> Result<gen::AppUpdate> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let aid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        let perms = input.permissions.clone().map(|v| v.iter().map(|p| format!("{p:?}").to_lowercase()).collect::<Vec<_>>());
        match saleor_rustify_db::apps::update_app(db, aid, input.name.clone(), input.identifier.clone(), perms).await {
            Ok(()) => Ok(gen::AppUpdate { errors: vec![], app: mini_app(db, aid).await? }),
            Err(e) => Ok(gen::AppUpdate { errors: vec![apperr(None, e.to_string())], app: None }),
        }
    }

    /// Delete an app (Django `appDelete`).
    async fn app_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::AppDelete> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let aid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        match saleor_rustify_db::apps::delete_app(db, aid).await {
            Ok(()) => Ok(gen::AppDelete { errors: vec![], app: None }),
            Err(e) => Ok(gen::AppDelete { errors: vec![apperr(None, e.to_string())], app: None }),
        }
    }

    /// Activate an app (Django `appActivate`).
    async fn app_activate(&self, ctx: &Context<'_>, id: ID) -> Result<gen::AppActivate> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let aid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        match saleor_rustify_db::apps::set_app_active(db, aid, true).await {
            Ok(()) => Ok(gen::AppActivate { errors: vec![] }),
            Err(e) => Ok(gen::AppActivate { errors: vec![apperr(None, e.to_string())] }),
        }
    }

    /// Deactivate an app (Django `appDeactivate`).
    async fn app_deactivate(&self, ctx: &Context<'_>, id: ID) -> Result<gen::AppDeactivate> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let aid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        match saleor_rustify_db::apps::set_app_active(db, aid, false).await {
            Ok(()) => Ok(gen::AppDeactivate { errors: vec![] }),
            Err(e) => Ok(gen::AppDeactivate { errors: vec![apperr(None, e.to_string())] }),
        }
    }

    /// Mint an app token (Django `appTokenCreate`; raw value shown once).
    async fn app_token_create(&self, ctx: &Context<'_>, input: gen::AppTokenInput) -> Result<gen::AppTokenCreate> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::AppTokenCreate { auth_token: None, errors: vec![apperr(None, m)], app_token: None };
        let aid = saleor_rustify_db::catalog::parse_gid(&input.app.0).unwrap_or(-1);
        let name = input.name.clone().unwrap_or_else(|| "token".to_string());
        match saleor_rustify_db::apps::create_app_token(db, aid, &name).await {
            Ok((tid, raw)) => Ok(gen::AppTokenCreate {
                auth_token: Some(raw),
                errors: vec![],
                app_token: Some(gen::AppToken {
                    id: Some(ID(crate::common::gid("AppToken", tid))),
                    name: Some(name),
                    auth_token: None,
                }),
            }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    /// Revoke an app token (Django `appTokenDelete`).
    async fn app_token_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::AppTokenDelete> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let tid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        match saleor_rustify_db::apps::revoke_app_token(db, tid).await {
            Ok(true) => Ok(gen::AppTokenDelete { errors: vec![], app_token: None }),
            Ok(false) => Ok(gen::AppTokenDelete { errors: vec![apperr(None, "token not found".into())], app_token: None }),
            Err(e) => Ok(gen::AppTokenDelete { errors: vec![apperr(None, e.to_string())], app_token: None }),
        }
    }

    /// Verify an app token (Django `AppTokenVerify`).
    async fn app_token_verify(&self, ctx: &Context<'_>, token: String) -> Result<GqlAppTokenVerify> {
        let _ = ctx;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        match saleor_rustify_db::apps::verify_app_token(db, &token).await {
            Ok(Some(_)) => Ok(GqlAppTokenVerify { valid: true, errors: vec![] }),
            Ok(None) => Ok(GqlAppTokenVerify { valid: false, errors: vec![] }),
            Err(e) => Ok(GqlAppTokenVerify {
                valid: false,
                errors: vec![apperr(None, e.to_string())],
            }),
        }
    }

    /// Fetch an app manifest (Django `appFetchManifest`): real HTTP fetch.
    async fn app_fetch_manifest(&self, ctx: &Context<'_>, #[graphql(name = "manifestUrl")] manifest_url: String) -> Result<gen::AppFetchManifest> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let err = |m: String| gen::AppFetchManifest { manifest: None, errors: vec![apperr(None, m)] };
        let body = match reqwest::get(&manifest_url).await {
            Ok(r) => match r.json::<serde_json::Value>().await {
                Ok(v) => v,
                Err(e) => return Ok(err(format!("manifest is not valid JSON: {e}"))),
            },
            Err(e) => return Ok(err(format!("could not fetch manifest: {e}"))),
        };
        Ok(gen::AppFetchManifest { manifest: Some(manifest_view(&body)), errors: vec![] })
    }

    /// Install an app from a manifest (Django `appInstall`): fetch, create
    /// the app row + permissions, close the installation job.
    async fn app_install(&self, ctx: &Context<'_>, input: gen::AppInstallInput) -> Result<gen::AppInstall> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::AppInstall { errors: vec![apperr(None, m)], app_installation: None };
        let job = match saleor_rustify_db::apps::open_installation(db, &input.app_name, &input.manifest_url).await {
            Ok(j) => j,
            Err(e) => return Ok(err(e.to_string())),
        };
        let body = match reqwest::get(&input.manifest_url).await {
            Ok(r) => match r.json::<serde_json::Value>().await {
                Ok(v) => v,
                Err(e) => {
                    let _ = saleor_rustify_db::apps::finish_installation(db, job, "failed", Some(format!("manifest is not valid JSON: {e}"))).await;
                    return Ok(err(format!("manifest is not valid JSON: {e}")));
                }
            },
            Err(e) => {
                let _ = saleor_rustify_db::apps::finish_installation(db, job, "failed", Some(format!("could not fetch manifest: {e}"))).await;
                return Ok(err(format!("could not fetch manifest: {e}")));
            }
        };
        // Requested permissions must cover the manifest's (Django parity).
        let want: Vec<String> = input.permissions.clone().unwrap_or_default().iter().map(|p| format!("{p:?}").to_lowercase()).collect();
        let need: Vec<String> = body.get("permissions").and_then(|p| p.as_array()).map(|a| {
            a.iter().filter_map(|x| x.as_str()).map(|s| s.to_lowercase()).collect()
        }).unwrap_or_default();
        if need.iter().any(|n| !want.contains(n)) {
            let _ = saleor_rustify_db::apps::finish_installation(db, job, "failed", Some("manifest permissions exceed granted permissions".to_string())).await;
            return Ok(err("manifest permissions exceed granted permissions".into()));
        }
        let name = body.get("name").and_then(|v| v.as_str()).unwrap_or(&input.app_name).to_string();
        let aid = match saleor_rustify_db::apps::create_app(db, &name, &want.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await {
            Ok(id) => id,
            Err(e) => {
                let _ = saleor_rustify_db::apps::finish_installation(db, job, "failed", Some(e.to_string())).await;
                return Ok(err(e.to_string()));
            }
        };
        if let Some(ident) = body.get("id").and_then(|v| v.as_str()).or_else(|| body.get("identifier").and_then(|v| v.as_str())) {
            let _ = saleor_rustify_db::apps::update_app(db, aid, None, Some(ident.to_string()), None).await;
        }
        if input.activate_after_installation == Some(false) {
            let _ = saleor_rustify_db::apps::set_app_active(db, aid, false).await;
        }
        let _ = saleor_rustify_db::apps::finish_installation(db, job, "success", None).await;
        Ok(gen::AppInstall { errors: vec![], app_installation: installation_row(db, job).await? })
    }

    /// Retry a failed installation (Django `appRetryInstall`).
    async fn app_retry_install(
        &self, ctx: &Context<'_>,
        #[graphql(name = "activateAfterInstallation")] activate_after_installation: Option<bool>, id: ID,
    ) -> Result<gen::AppRetryInstall> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| gen::AppRetryInstall { errors: vec![apperr(None, m)], app_installation: None };
        let jid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        let _ = activate_after_installation;
        // Retry re-opens the job as pending (the install worker — here the
        // next appInstall call — picks it up; Django re-queues the same way).
        use sea_orm::EntityTrait;
        let row = saleor_rustify_db::entities::app_appinstallation::Entity::find_by_id(jid)
            .one(db)
            .await
            .map_err(|e| Error::new(e.to_string()))?;
        let Some(row) = row else { return Ok(err("installation not found".into())) };
        if row.status != "failed" {
            return Ok(err("only failed installations can be retried".into()));
        }
        if let Err(e) = saleor_rustify_db::apps::finish_installation(db, jid, "pending", None).await {
            return Ok(err(e.to_string()));
        }
        Ok(gen::AppRetryInstall { errors: vec![], app_installation: installation_row(db, jid).await? })
    }

    /// Delete a failed installation (Django `appDeleteFailedInstallation`).
    async fn app_delete_failed_installation(&self, ctx: &Context<'_>, id: ID) -> Result<gen::AppDeleteFailedInstallation> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let jid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        match saleor_rustify_db::apps::delete_failed_installation(db, jid).await {
            Ok(r) => Ok(gen::AppDeleteFailedInstallation {
                errors: vec![],
                app_installation: Some(gen::AppInstallation {
                    id: Some(ID(crate::common::gid("AppInstallation", r.id))),
                    status: Some(r.status.to_uppercase()),
                    message: r.message.clone(),
                    app_name: Some(r.app_name.clone()),
                    manifest_url: Some(r.manifest_url.clone()),
                    brand: None,
                }),
            }),
            Err(e) => Ok(gen::AppDeleteFailedInstallation {
                errors: vec![apperr(None, e.to_string())],
                app_installation: None,
            }),
        }
    }

    /// Dismiss app problems (Django `appProblemDismiss`).
    async fn app_problem_dismiss(&self, ctx: &Context<'_>, input: gen::AppProblemDismissInput) -> Result<gen::AppProblemDismiss> {        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let mut ids = vec![];
        let mut keys = vec![];
        let mut app_id = None;
        if let Some(b) = input.by_app.as_ref() {
            for i in b.ids.clone().unwrap_or_default() {
                if let Some(id) = saleor_rustify_db::catalog::parse_gid(&i.0) {
                    ids.push(id);
                }
            }
            keys.extend(b.keys.clone().unwrap_or_default());
        }
        if let Some(b) = input.by_staff_with_ids.as_ref() {
            for i in &b.ids {
                if let Some(id) = saleor_rustify_db::catalog::parse_gid(&i.0) {
                    ids.push(id);
                }
            }
        }
        if let Some(b) = input.by_staff_with_keys.as_ref() {
            keys.extend(b.keys.clone());
            if let Some(a) = saleor_rustify_db::catalog::parse_gid(&b.app.0) {
                app_id = Some(a);
            }
        }
        match saleor_rustify_db::apps::dismiss_problems(db, &ids, &keys, app_id).await {
            Ok(_) => Ok(gen::AppProblemDismiss { errors: vec![] }),
            Err(e) => Ok(gen::AppProblemDismiss {
                errors: vec![gen::AppProblemDismissError { message: Some(e.to_string()) }],
            }),
        }
    }

    /// Record an app problem (Django `appProblemCreate`): staff-reported
    /// runtime issue on the caller's app (derived from the bearer token).
    async fn app_problem_create(&self, ctx: &Context<'_>, input: gen::AppProblemCreateInput) -> Result<GqlAppProblemCreate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlAppProblemCreate { app_problem: None, errors: vec![apperr(None, m)] };
        // Caller app from the bearer (staff fall back to explicit lookup is
        // not possible — problems attach to the calling app, like Django).
        let aid = match crate::account::app_caller(ctx, db).await {
            Ok(a) => a,
            Err(_) => return Ok(err("an app token is required".into())),
        };
        if input.message.trim().is_empty() || input.key.trim().is_empty() {
            return Ok(err("message and key are required".into()));
        }
        match saleor_rustify_db::apps::create_problem(
            db, aid, &input.message, &input.key,
            input.critical_threshold, input.aggregation_period,
        ).await {
            Ok(pid) => Ok(GqlAppProblemCreate { app_problem: assemble_problem(db, pid).await?, errors: vec![] }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    /// Re-enable an app's sync webhooks (Django `appReenableSyncWebhooks`).
    async fn app_reenable_sync_webhooks(&self, ctx: &Context<'_>, #[graphql(name = "appId")] app_id: ID) -> Result<GqlAppReenableSyncWebhooks> {
        let _ = crate::account::require_perm(ctx, "manage_apps").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let aid = saleor_rustify_db::catalog::parse_gid(&app_id.0).unwrap_or(-1);
        match saleor_rustify_db::apps::reenable_sync_webhooks(db, aid).await {
            Ok(()) => Ok(GqlAppReenableSyncWebhooks { errors: vec![], app: mini_app(db, aid).await? }),
            Err(e) => Ok(GqlAppReenableSyncWebhooks { errors: vec![apperr(None, e.to_string())], app: None }),
        }
    }
}

/// One installation row → payload.
async fn installation_row(db: &sea_orm::DatabaseConnection, jid: i32) -> Result<Option<gen::AppInstallation>, Error> {
    use sea_orm::EntityTrait;
    let row = saleor_rustify_db::entities::app_appinstallation::Entity::find_by_id(jid)
        .one(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?;
    Ok(row.as_ref().map(installation_view))
}

/// Export payloads (dashboard-selected shapes; codegen missed these roots).
#[derive(SimpleObject, Clone)]
#[graphql(name = "ExportGiftCards")]
pub struct GqlExportGiftCards {
    #[graphql(name = "exportFile")]
    pub export_file: Option<gen::ExportFile>,
    pub errors: Vec<gen::ExportError>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "ExportVoucherCodes")]
pub struct GqlExportVoucherCodes {
    #[graphql(name = "exportFile")]
    pub export_file: Option<gen::ExportFile>,
    pub errors: Vec<gen::ExportError>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "ExportProducts")]
pub struct GqlExportProducts {
    #[graphql(name = "exportFile")]
    pub export_file: Option<gen::ExportFile>,
    pub errors: Vec<gen::ExportError>,
}

/// Export-file row → dashboard payload (url via the media mount).
async fn export_file_view(db: &sea_orm::DatabaseConnection, job: i32) -> Result<Option<gen::ExportFile>, Error> {
    use sea_orm::EntityTrait;
    let row = saleor_rustify_db::entities::csv_exportfile::Entity::find_by_id(job)
        .one(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?;
    Ok(row.map(|r| gen::ExportFile {
        id: Some(ID(crate::common::gid("ExportFile", r.id))),
        status: Some(r.status.to_uppercase()),
        url: r.content_file.map(|p| crate::common::media_url(&p)),
    }))
}

/// Object payload for dry-run (real ids + status, per object kind).
async fn dry_run_payload(db: &sea_orm::DatabaseConnection, gid: &str) -> std::result::Result<String, String> {
    use base64::Engine as _;
    let text = base64::engine::general_purpose::STANDARD
        .decode(gid)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_default();
    let (kind, pk) = text.split_once(':').unwrap_or(("", ""));
    match kind {
        "Order" => {
            let oid = pk.parse::<uuid::Uuid>().map_err(|_| "bad order id".to_string())?;
            match saleor_rustify_db::order_store::get_order_rows(db, oid).await.map_err(|e| e.to_string())? {
                Some((h, _)) => Ok(serde_json::json!({
                    "id": gid, "number": h.number.to_string(), "status": h.status.to_uppercase(),
                    "user_email": h.user_email, "total_gross_amount": h.total_gross_amount.to_string(),
                    "currency": h.currency,
                }).to_string()),
                None => Err("order not found".to_string()),
            }
        }
        "Product" => Ok(serde_json::json!({"id": gid}).to_string()),
        "Checkout" => Ok(serde_json::json!({"id": gid}).to_string()),
        _ => Err(format!("dry run for {kind} is not supported")),
    }
}

fn apperr(field: Option<String>, message: String) -> gen::AppError {
    gen::AppError { field, message: Some(message), code: None, permissions: vec![] }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "AppProblemCreate")]
pub struct GqlAppProblemCreate {
    #[graphql(name = "appProblem")]
    pub app_problem: Option<gen::AppProblem>,
    pub errors: Vec<gen::AppError>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "AppReenableSyncWebhooks")]
pub struct GqlAppReenableSyncWebhooks {
    pub app: Option<gen::App>,
    pub errors: Vec<gen::AppError>,
}

/// App-problem row → node.
async fn assemble_problem(db: &sea_orm::DatabaseConnection, pid: i32) -> Result<Option<gen::AppProblem>, Error> {
    use sea_orm::EntityTrait;
    let r = saleor_rustify_db::entities::app_appproblem::Entity::find_by_id(pid)
        .one(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?;
    Ok(r.map(|p| gen::AppProblem {
        id: Some(ID(crate::common::gid("AppProblem", p.id))),
        created_at: Some(p.created_at.into()),
        updated_at: Some(p.updated_at.into()),
        count: Some(p.count),
        is_critical: Some(p.is_critical),
        dismissed: Some(gen::AppProblemDismissed {
            by: None,
            user_email: p.dismissed_by_user_email.clone(),
        }),
        message: Some(p.message.clone()),
        key: Some(p.key.clone()),
    }))
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "ExternalNotificationTrigger")]
pub struct GqlExternalNotificationTrigger {
    pub errors: Vec<GqlExternalNotificationError>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "ExternalNotificationError")]
pub struct GqlExternalNotificationError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "AppTokenVerify")]
pub struct GqlAppTokenVerify {
    pub valid: bool,
    pub errors: Vec<gen::AppError>,
}

/// App installation row → payload.
fn installation_view(r: &saleor_rustify_db::entities::app_appinstallation::Model) -> gen::AppInstallation {
    gen::AppInstallation {
        id: Some(ID(crate::common::gid("AppInstallation", r.id))),
        status: Some(r.status.to_uppercase()),
        message: r.message.clone(),
        app_name: Some(r.app_name.clone()),
        manifest_url: Some(r.manifest_url.clone()),
        brand: None,
    }
}

/// Manifest JSON → dashboard payload (permissions named by codename).
fn manifest_view(v: &serde_json::Value) -> gen::Manifest {
    let perms: Vec<crate::commerce::GqlPermission> = v
        .get("permissions")
        .and_then(|p| p.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str())
                .map(|c| crate::commerce::GqlPermission { code: c.to_string(), name: c.to_string() })
                .collect()
        })
        .unwrap_or_default();
    let exts: Vec<gen::AppManifestExtension> = v
        .get("extensions")
        .and_then(|p| p.as_array())
        .map(|a| {
            a.iter()
                .map(|x| gen::AppManifestExtension {
                    permissions: vec![],
                    label: x.get("label").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    url: x.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    mount_name: x.get("mount").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    target_name: x.get("target").and_then(|v| v.as_str()).map(|s| s.to_string()),
                })
                .collect()
        })
        .unwrap_or_default();
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string());
    gen::Manifest {
        identifier: s("id").or_else(|| s("identifier")),
        version: s("version"),
        name: s("name"),
        about: s("about"),
        permissions: perms,
        app_url: s("appUrl"),
        token_target_url: s("tokenTargetUrl"),
        data_privacy: s("dataPrivacy"),
        data_privacy_url: s("dataPrivacyUrl"),
        homepage_url: s("homepageUrl"),
        support_url: s("supportUrl"),
        extensions: exts,
        brand: None,
    }
}
