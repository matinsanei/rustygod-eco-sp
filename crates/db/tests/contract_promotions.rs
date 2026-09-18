//! Promotion engine contract vs Django's tables and Django-written rows.
//! Mirrors `saleor/discount/tests/` catalogue-promotion and voucher flows:
//! evaluation writes the same discount rows Django writes, totals drop by
//! the same amounts, voucher usage increments on completion.

use rust_decimal::Decimal;
use rustygod_db::{catalog, checkout_store, database_url, promotions};
use sea_orm::{ConnectionTrait, DatabaseConnection};

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn catalogue_rule_evaluates_on_real_variant() {
    use rustygod_db::entities::product_productvariant;
    use sea_orm::{EntityTrait, QuerySelect, SelectorTrait};

    let db = db().await;
    // Variant 353: USD 40.00 with a live 30% catalogue rule (populatedb data).
    let pid: i32 = product_productvariant::Entity::find_by_id(353)
        .select_only()
        .column(product_productvariant::Column::ProductId)
        .into_tuple()
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let eval = promotions::evaluate_line(&db, 1, 353, pid, Decimal::new(4000, 2), 2, "USD")
        .await
        .unwrap()
        .expect("rule must apply");
    assert_eq!(eval.reward_value_type, "percentage");
    // 30% of 40.00 = 12.00 per unit x 2 = 24.00 whole-line amount.
    assert_eq!(eval.amount, Decimal::new(2400, 2));
}

#[tokio::test]
async fn checkout_totals_drop_by_promotion() {
    use rustygod_db::checkout_store::NewLine;

    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "")
        .await
        .unwrap();
    checkout_store::add_lines_tx(
        &db,
        token,
        ch_id,
        &currency,
        &[NewLine { variant_id: 353, quantity: 1, unit_price: Decimal::new(4000, 2), price_override: None }],
    )
    .await
    .unwrap();

    let (co, lines) = checkout_store::load_checkout(&db, token).await.unwrap().unwrap();
    // Undiscounted line stays 40.00; checkout total drops by the 12.00 discount.
    assert_eq!(lines[0].total_price_gross_amount, Decimal::new(4000, 2));
    assert_eq!(co.total_gross_amount, Decimal::new(2800, 2));

    // The discount row Django would write exists.
    use rustygod_db::entities::discount_checkoutlinediscount;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let d = discount_checkoutlinediscount::Entity::find()
        .filter(discount_checkoutlinediscount::Column::LineId.eq(lines[0].id))
        .filter(discount_checkoutlinediscount::Column::UniqueType.eq("promotion"))
        .one(&db)
        .await
        .unwrap()
        .expect("promotion discount row must exist");
    assert_eq!(d.amount_value, Decimal::new(1200, 2));
    assert_eq!(d.r#type, "promotion");

    checkout_store::delete_checkout_row(&db, token).await.unwrap();
    // Discount rows cascade with lines (deleted above via checkout delete).
    assert!(
        discount_checkoutlinediscount::Entity::find()
            .filter(discount_checkoutlinediscount::Column::LineId.eq(lines[0].id))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn voucher_entire_order_applies_and_increments_on_complete() {
    use rustygod_db::{checkout_store::NewLine, order_store};
    use rustygod_db::entities::discount_vouchercode;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let token = checkout_store::create_checkout_row(&db, ch_id, &currency, "buyer@example.com")
        .await
        .unwrap();
    checkout_store::add_lines_tx(
        &db,
        token,
        ch_id,
        &currency,
        &[NewLine { variant_id: 353, quantity: 1, unit_price: Decimal::new(4000, 2), price_override: None }],
    )
    .await
    .unwrap();

    let before: i32 = discount_vouchercode::Entity::find()
        .filter(discount_vouchercode::Column::Code.eq("DISCOUNT"))
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .used;

    // DISCOUNT = entire_order fixed 25.00 USD. Base after 30% promo: 28.00.
    let applied = promotions::apply_voucher(&db, token, "DISCOUNT", "default-channel")
        .await
        .unwrap();
    assert_eq!(applied.voucher_type, "entire_order");
    assert_eq!(applied.amount, Decimal::new(2500, 2));
    checkout_store::refresh_totals(&db, token).await.unwrap();
    let (co, _) = checkout_store::load_checkout(&db, token).await.unwrap().unwrap();
    assert_eq!(co.total_gross_amount, Decimal::new(300, 2)); // 28 - 25
    assert_eq!(co.voucher_code.as_deref(), Some("DISCOUNT"));

    // Complete path increments usage (same call complete_checkout makes).
    promotions::increase_usage(&db, "DISCOUNT", Some("buyer@example.com"))
        .await
        .unwrap();
    let after: i32 = discount_vouchercode::Entity::find()
        .filter(discount_vouchercode::Column::Code.eq("DISCOUNT"))
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .used;
    assert_eq!(after, before + 1);

    // Roll usage back (test hygiene on shared data) + cleanup checkout.
    {
        use sea_orm::Statement;
        db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "UPDATE discount_vouchercode SET used = $1 WHERE code = 'DISCOUNT'",
            vec![before.into()],
        ))
        .await
        .unwrap();
        use rustygod_db::entities::discount_vouchercustomer;
        discount_vouchercustomer::Entity::delete_many()
            .filter(discount_vouchercustomer::Column::CustomerEmail.eq("buyer@example.com"))
            .exec(&db)
            .await
            .unwrap();
    }
    // Remove voucher discount rows + checkout.
    {
        use rustygod_db::entities::discount_checkoutdiscount;
        discount_checkoutdiscount::Entity::delete_many()
            .filter(discount_checkoutdiscount::Column::CheckoutId.eq(token))
            .exec(&db)
            .await
            .unwrap();
    }
    checkout_store::delete_checkout_row(&db, token).await.unwrap();
    let _ = order_store::next_number(&db).await.unwrap();
}
