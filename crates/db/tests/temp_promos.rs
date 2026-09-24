use tokio;
use rust_decimal::Decimal;
use saleor_rustify_db::{promotions, database_url};

#[tokio::test]
async fn test_promos() {
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    use sea_orm::{EntityTrait, QuerySelect, ColumnTrait, QueryFilter};
    let ppvcl = saleor_rustify_db::entities::product_productvariantchannellisting::Entity::find()
        .filter(saleor_rustify_db::entities::product_productvariantchannellisting::Column::VariantId.eq(332))
        .filter(saleor_rustify_db::entities::product_productvariantchannellisting::Column::ChannelId.eq(1))
        .one(&db).await.unwrap().unwrap();
    println!("Variant 332 price: {:?}", ppvcl.price_amount);
}
