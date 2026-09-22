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

#[derive(Default)]
pub struct AppsQuery;

#[Object]
impl AppsQuery {
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

fn to_row(m: &rustygod_db::entities::app_app::Model) -> AppRow {
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
    use rustygod_db::entities::app_app;
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
    use rustygod_db::entities::{app_app, app_appextension, app_appextension_permissions, permission_permission};
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
