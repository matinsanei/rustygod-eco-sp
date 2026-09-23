//! Shared scalars & page type.

use async_graphql::*;

/// Saleor `Permission.code` is the `PermissionEnum` NAME
/// (`format_permissions_for_display` does
/// `PermissionEnum.get(f"{app_label}.{codename}")`), NOT the raw DB codename.
/// Map extracted verbatim from
/// `saleor-core/saleor/permission/enums.py`; unknown codenames fall back to
/// uppercased codename (Saleor's naming convention). The Dashboard gates every
/// sidebar section on these values (`MANAGE_PRODUCTS`, ...), so lowercase
/// codes hide the whole menu.
pub fn permission_enum_code(codename: &str) -> String {
    match codename {
        "handle_checkouts" => "HANDLE_CHECKOUTS",
        "handle_payments" => "HANDLE_PAYMENTS",
        "handle_taxes" => "HANDLE_TAXES",
        "impersonate_user" => "IMPERSONATE_USER",
        "manage_apps" => "MANAGE_APPS",
        "manage_channels" => "MANAGE_CHANNELS",
        "manage_checkouts" => "MANAGE_CHECKOUTS",
        "manage_customer_types_and_attributes" => "MANAGE_CUSTOMER_TYPES_AND_ATTRIBUTES",
        "manage_discounts" => "MANAGE_DISCOUNTS",
        "manage_gift_card" => "MANAGE_GIFT_CARD",
        "manage_menus" => "MANAGE_MENUS",
        "manage_observability" => "MANAGE_OBSERVABILITY",
        "manage_orders" => "MANAGE_ORDERS",
        "manage_orders_import" => "MANAGE_ORDERS_IMPORT",
        "manage_pages" => "MANAGE_PAGES",
        "manage_page_types_and_attributes" => "MANAGE_PAGE_TYPES_AND_ATTRIBUTES",
        "manage_plugins" => "MANAGE_PLUGINS",
        "manage_products" => "MANAGE_PRODUCTS",
        "manage_product_types_and_attributes" => "MANAGE_PRODUCT_TYPES_AND_ATTRIBUTES",
        "manage_settings" => "MANAGE_SETTINGS",
        "manage_shipping" => "MANAGE_SHIPPING",
        "manage_staff" => "MANAGE_STAFF",
        "manage_taxes" => "MANAGE_TAXES",
        "manage_translations" => "MANAGE_TRANSLATIONS",
        "manage_users" => "MANAGE_USERS",
        other => return other.to_uppercase(),
    }
    .to_string()
}

#[derive(SimpleObject, Clone)]
pub struct PageInfo {
    #[graphql(name = "hasNextPage")]
    pub has_next_page: bool,
    #[graphql(name = "hasPreviousPage")]
    pub has_previous_page: bool,
    #[graphql(name = "startCursor")]
    pub start_cursor: Option<String>,
    #[graphql(name = "endCursor")]
    pub end_cursor: Option<String>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "CountryDisplay")]
pub struct GqlCountryDisplay { pub code: String, pub country: String }

#[derive(SimpleObject, Clone)]
#[graphql(name = "StockSettings")]
pub struct GqlStockSettings {
    #[graphql(name = "allocationStrategy")]
    pub allocation_strategy: String,
}

#[derive(SimpleObject, Clone)]
pub struct Money {
    pub amount: String,
    pub currency: String,
    #[graphql(name = "fractionDigits")]
    pub fraction_digits: Option<i32>,
}

impl From<saleor_rustify_core::money::Money> for Money {
    fn from(m: saleor_rustify_core::money::Money) -> Self {
        Self { amount: m.amount.to_string(), currency: m.currency, fraction_digits: None }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "MetadataItem")]
pub struct MetadataItem { pub key: String, pub value: String }

#[derive(InputObject, Clone, Debug)]
pub struct MetadataInput {
    pub key: String,
    pub value: String,
}

/// `metadata` JSON (`JsonBinary` dict) → `[MetadataItem]`. Saleor stores
/// metadata as a JSON object; empty/missing → `[]`.
pub fn json_to_metadata_items(v: &serde_json::Value) -> Vec<MetadataItem> {
    v.as_object().map(|m| m.iter().map(|(k, val)| MetadataItem {
        key: k.clone(),
        value: val.as_str().map(|s| s.to_string()).unwrap_or_else(|| val.to_string()),
    }).collect()).unwrap_or_default()
}

/// Merge `[{key, value}]` inputs into a metadata JSON object (upsert by key).
pub fn merge_metadata(base: &serde_json::Value, inputs: &[MetadataInput]) -> serde_json::Value {
    let mut map = base.as_object().cloned().unwrap_or_default();
    for i in inputs {
        map.insert(i.key.clone(), serde_json::Value::String(i.value.clone()));
    }
    serde_json::Value::Object(map)
}

#[derive(SimpleObject, Clone)]
pub struct GqlError {
    pub field: Option<String>,
    pub message: String,
    pub code: String,
}

pub fn to_gql_errors(err: String) -> Vec<GqlError> {
    vec![GqlError { field: None, message: err, code: "UNKNOWN".into() }]
}

pub fn encode_cursor(i: usize) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(format!("cursor:{i}"))
}
/// Public media URL. Browsers resolve relative URLs against the DASHBOARD
/// origin (:80), not the API (:8000) — so the default is absolute.
/// Override with SALEOR_MEDIA_URL for other deployments.
pub fn media_url(path: &str) -> String {
    let base = std::env::var("SALEOR_MEDIA_URL")
        .unwrap_or_else(|_| "http://localhost:8000/media".into());
    format!("{}/{path}", base.trim_end_matches('/'))
}

pub fn decode_cursor(s: &str) -> Option<usize> {
    use base64::Engine;
    let b = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
    let t = String::from_utf8(b).ok()?;
    t.strip_prefix("cursor:")?.parse().ok()
}

/// Saleor global ID: base64(`Type:pk`), e.g. `UHJvZHVjdDoyOA==`.
/// We mint these on every object id so dashboard round-trips carry their
/// type (required for `updateMetadata` and all ID-taking writes).
pub fn gid(typename: &str, pk: impl std::fmt::Display) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(format!("{typename}:{pk}"))
}

/// Split a global ID into `(Type, raw_pk)`; plain ints/UUIDs return None.
pub fn split_gid(s: &str) -> Option<(String, String)> {
    use base64::Engine;
    let b = base64::engine::general_purpose::STANDARD.decode(s.trim()).ok()?;
    let t = String::from_utf8(b).ok()?;
    let (ty, pk) = t.split_once(':')?;
    if ty.is_empty() || pk.is_empty() || ty.contains(|c: char| !c.is_alphanumeric()) {
        return None;
    }
    Some((ty.to_string(), pk.to_string()))
}

/// Lenient ID split: global IDs, legacy raw `Type:pk` mintings (Shop:1,
/// Channel:5 — predating the global-ID migration), but never bare ints
/// (typeless ids would risk writing the wrong table).
pub fn split_gid_or_raw(s: &str) -> Option<(String, String)> {
    if let Some(t) = split_gid(s) {
        return Some(t);
    }
    let s = s.trim();
    let (ty, pk) = s.split_once(':')?;
    if ty.is_empty() || pk.is_empty() || ty.contains(|c: char| !c.is_alphanumeric()) {
        return None;
    }
    Some((ty.to_string(), pk.to_string()))
}

/// Undo one layer of accidental double-encoding (`base64("User:<global>")`,
/// seen live from dashboard round-trips of old-cached ids). Returns the
/// inner `(Type, pk)` when the raw part itself decodes to one.
pub fn unsplit_double(raw: &str) -> Option<(String, String)> {
    split_gid(raw)
}

/// Parse a UUID-typed ID: plain UUID or global (`T3JkZXI6...`).
pub fn parse_uuid_gid(s: &str) -> Option<uuid::Uuid> {
    let s = s.trim();
    if let Ok(u) = s.parse::<uuid::Uuid>() {
        return Some(u);
    }
    let (_, pk) = split_gid(s)?;
    pk.parse::<uuid::Uuid>().ok()
}
