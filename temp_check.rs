use tokio;
use rust_decimal::Decimal;
use saleor_rustify_db::{promotions, database_url};

#[tokio::main]
async fn main() {
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    let rules = promotions::active_catalogue_rules(&db, 1).await.unwrap();
    for r in &rules {
        println!("Rule: {} (Type: {}, Val: {})", r.rule_name, r.reward_value_type, r.reward_value);
    }
}
