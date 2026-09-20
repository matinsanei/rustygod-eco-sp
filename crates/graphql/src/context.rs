//! Request context: DB handle + raw Authorization header.

use sea_orm::DatabaseConnection;

pub const MANAGE_ORDERS: &str = "manage_orders";

#[derive(Clone, Debug)]
pub struct Bearer(pub String);

#[derive(Clone)]
pub struct GqlContext {
    pub db: Option<DatabaseConnection>,
    pub bearer: Option<String>,
}

impl GqlContext {
    pub fn db(&self) -> Result<&DatabaseConnection, String> {
        self.db.as_ref().ok_or_else(|| "postgres unavailable".into())
    }
}
