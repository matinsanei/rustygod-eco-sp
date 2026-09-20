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

impl From<rustygod_core::money::Money> for Money {
    fn from(m: rustygod_core::money::Money) -> Self {
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
pub fn decode_cursor(s: &str) -> Option<usize> {
    use base64::Engine;
    let b = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
    let t = String::from_utf8(b).ok()?;
    t.strip_prefix("cursor:")?.parse().ok()
}
