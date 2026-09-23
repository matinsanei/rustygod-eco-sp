//! Promotion + voucher writes (discount mutations parity).

use rust_decimal::Decimal;
use saleor_rustify_db::{database_url, promo_writes};
use sea_orm::DatabaseConnection;
use serde_json::json;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn rule(channels: Vec<i32>) -> promo_writes::NewRule {
    promo_writes::NewRule {
        name: Some("10% off".into()),
        description: json!({}),
        catalogue_predicate: json!({}),
        order_predicate: json!({}),
        reward_value_type: Some("percentage".into()),
        reward_value: Some(Decimal::new(10, 0)),
        reward_type: Some("subtotal_discount".into()),
        channel_ids: channels,
        gift_variant_ids: vec![],
    }
}

#[tokio::test]
async fn promotion_rule_lifecycle() {
    let db = db().await;
    let pid = promo_writes::create_promotion(&db, "Smoke Promo", "catalogue", json!({}), None, None, vec![rule(vec![])])
        .await
        .unwrap();
    assert!(promo_writes::create_promotion(&db, "  ", "catalogue", json!({}), None, None, vec![]).await.is_err());
    promo_writes::update_promotion(&db, pid, Some("Smoke Promo 2".into()), None, None, None)
        .await
        .unwrap();
    let rid = promo_writes::create_rule(&db, pid, &rule(vec![])).await.unwrap();
    promo_writes::update_rule(
        &db,
        rid,
        &promo_writes::UpdateRule {
            name: Some("edited".into()),
            reward_value: Some(Some(Decimal::new(20, 0))),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(promo_writes::update_rule(
        &db,
        rid,
        &promo_writes::UpdateRule { reward_value: Some(Some(Decimal::ZERO)), ..Default::default() },
    )
    .await
    .is_err());
    promo_writes::delete_rule(&db, rid).await.unwrap();
    promo_writes::delete_promotion(&db, pid).await.unwrap();
    use saleor_rustify_db::entities::discount_promotion;
    use sea_orm::EntityTrait;
    assert!(discount_promotion::Entity::find_by_id(pid).one(&db).await.unwrap().is_none());
}

#[tokio::test]
async fn voucher_lifecycle() {
    let db = db().await;
    let tag = format!("S{}", chrono::Utc::now().timestamp_millis() % 100000);
    let vid = promo_writes::create_voucher(
        &db,
        &promo_writes::NewVoucher {
            name: Some("Smoke".into()),
            voucher_type: "entire_order".into(),
            discount_value_type: "fixed".into(),
            codes: vec![format!("{tag}-1")],
            ..Default::default()
        },
        "USD",
    )
    .await
    .unwrap();
    assert!(promo_writes::create_voucher(
        &db,
        &promo_writes::NewVoucher { codes: vec![], ..Default::default() },
        "USD",
    )
    .await
    .is_err());
    use saleor_rustify_db::entities::product_product;
    use sea_orm::QuerySelect;
    let real_pid: i32 = product_product::Entity::find()
        .select_only()
        .column(product_product::Column::Id)
        .into_tuple()
        .one(&db)
        .await
        .unwrap()
        .expect("seed must contain products");
    promo_writes::voucher_catalogues(&db, vid, true, &[real_pid], &[], &[], &[]).await.unwrap();
    promo_writes::voucher_catalogues(&db, vid, false, &[real_pid], &[], &[], &[]).await.unwrap();
    promo_writes::voucher_channel_listings(
        &db,
        vid,
        &[promo_writes::ChannelListingInput { channel_id: 1, discount_value: Decimal::new(5, 0), min_spent: None }],
        &[],
        "USD",
    )
    .await
    .unwrap();
    promo_writes::update_voucher(
        &db,
        vid,
        &promo_writes::UpdateVoucher { add_codes: vec![format!("{tag}-2")], ..Default::default() },
        "USD",
    )
    .await
    .unwrap();
    use saleor_rustify_db::entities::discount_vouchercode;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let codes = discount_vouchercode::Entity::find()
        .filter(discount_vouchercode::Column::VoucherId.eq(vid))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(codes.len(), 2);
    let n = promo_writes::delete_voucher_codes(&db, &[codes[0].id]).await.unwrap();
    assert_eq!(n, 1);
    assert_eq!(promo_writes::bulk_delete_vouchers(&db, &[vid]).await.unwrap(), 1);
    let _ = Uuid::nil();
}
