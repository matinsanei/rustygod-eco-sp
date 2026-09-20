//! Shared scalars & page type.

use async_graphql::*;

#[derive(SimpleObject, Clone)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct Money {
    pub amount: String,
    pub currency: String,
}

impl From<rustygod_core::money::Money> for Money {
    fn from(m: rustygod_core::money::Money) -> Self {
        Self { amount: m.amount.to_string(), currency: m.currency }
    }
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
