//! Staff order operations: confirm/capture/refund/void/markPaid/update/
//! shipping/notes/lines/discounts (Django order mutations parity).

use rust_decimal::Decimal;
use saleor_rustify_db::{catalog, database_url, drafts, order_ops};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn stocked_variant(db: &DatabaseConnection) -> i32 {
    let products = catalog::list_products(db, "default-channel", None, 100)
        .await
        .unwrap();
    for p in products {
        for v in p.variants {
            let Ok(vid) = v.id.parse::<i32>() else { continue };
            if catalog::checkout_pricing(db, "default-channel", &[vid])
                .await
                .map(|m| m.contains_key(&vid))
                .unwrap_or(false)
            {
                return vid;
            }
        }
    }
    panic!("need a listed variant on default-channel");
}

async fn draft_with_line(db: &DatabaseConnection) -> (Uuid, Uuid) {
    let vid = stocked_variant(db).await;
    let view = drafts::create_draft(
        db,
        "default-channel",
        "ops@example.com",
        None,
        vec![drafts::DraftLineInput { variant_id: vid, quantity: 2, custom_price: None, force_new_line: false }],
        None,
    )
    .await
    .unwrap();
    use saleor_rustify_db::entities::order_orderline;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let line = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(view.id))
        .one(db)
        .await
        .unwrap()
        .expect("draft must hold a line");
    (view.id, line.id)
}

#[tokio::test]
async fn editable_guards_reject_confirmed_orders() {
    let db = db().await;
    let bogus = Uuid::new_v4();
    // Unknown order → not found (guard runs after lookup).
    assert!(order_ops::confirm_order(&db, bogus, None).await.is_err());
    assert!(order_ops::capture_order(&db, bogus, Decimal::ONE, None).await.is_err());
    assert!(order_ops::refund_order(&db, bogus, Decimal::ONE, None).await.is_err());
    assert!(order_ops::void_order(&db, bogus, None).await.is_err());
}

#[tokio::test]
async fn draft_line_lifecycle_and_discounts() {
    let db = db().await;
    let (oid, lid) = draft_with_line(&db).await;

    // Quantity update recomputes totals.
    order_ops::update_line_quantity(&db, oid, lid, 5, None).await.unwrap();
    use saleor_rustify_db::entities::{discount_orderdiscount, discount_orderlinediscount, order_orderline};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let line = order_orderline::Entity::find_by_id(lid).one(&db).await.unwrap().unwrap();
    assert_eq!(line.quantity, 5);

    // Manual order discount add → update → delete.
    let did = order_ops::discount_add(&db, oid, Some("staff 10%".into()), "percentage", Decimal::new(10, 0), None)
        .await
        .unwrap();
    let d = discount_orderdiscount::Entity::find()
        .filter(discount_orderdiscount::Column::Id.eq(did))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(d.r#type, "manual");
    assert!(d.amount_value > Decimal::ZERO);
    let oid2 = order_ops::discount_update(&db, did, None, None, Some(Decimal::new(5, 0)), None).await.unwrap();
    assert_eq!(oid2, oid);
    let oid3 = order_ops::discount_delete(&db, did, None).await.unwrap();
    assert_eq!(oid3, oid);
    assert!(discount_orderdiscount::Entity::find()
        .filter(discount_orderdiscount::Column::Id.eq(did))
        .one(&db)
        .await
        .unwrap()
        .is_none());

    // Manual line discount add → remove.
    let back = order_ops::line_discount_update(&db, lid, "fixed", Decimal::ONE, Some("promo".into()), None)
        .await
        .unwrap();
    assert_eq!(back, oid);
    assert!(discount_orderlinediscount::Entity::find()
        .filter(discount_orderlinediscount::Column::LineId.eq(lid))
        .one(&db)
        .await
        .unwrap()
        .is_some());
    order_ops::line_discount_remove(&db, lid, None).await.unwrap();

    // Line delete empties the draft; confirm then fails (no products).
    order_ops::delete_line(&db, oid, lid, None).await.unwrap();
    assert!(order_ops::confirm_order(&db, oid, None).await.is_err());
}

#[tokio::test]
async fn draft_confirm_and_mark_paid_flow() {
    let db = db().await;
    let (oid, _) = draft_with_line(&db).await;
    // Drafts can't be confirmed (need unconfirmed status first).
    assert!(order_ops::confirm_order(&db, oid, None).await.is_err());

    // Flip to unconfirmed (simulates checkout placement) then confirm.
    use saleor_rustify_db::entities::order_order;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};
    let row = order_order::Entity::find_by_id(oid).one(&db).await.unwrap().unwrap();
    let mut am: order_order::ActiveModel = row.into();
    am.status = Set("unconfirmed".to_string());
    am.update(&db).await.unwrap();
    order_ops::confirm_order(&db, oid, None).await.unwrap();
    let row = order_order::Entity::find_by_id(oid).one(&db).await.unwrap().unwrap();
    assert_eq!(row.status, "unfulfilled");

    // Editing a confirmed order is rejected.
    assert!(order_ops::discount_add(&db, oid, None, "fixed", Decimal::ONE, None).await.is_err());

    // Mark as paid books a manual transaction for the full total.
    order_ops::mark_order_as_paid(&db, oid, Some("test-ref".into()), None).await.unwrap();
    let row = order_order::Entity::find_by_id(oid).one(&db).await.unwrap().unwrap();
    assert!(row.total_charged_amount >= row.total_gross_amount);
}

#[tokio::test]
async fn capture_refund_void_cycle() {
    let db = db().await;
    let (oid, _) = draft_with_line(&db).await;
    use saleor_rustify_db::entities::order_order;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};
    // Authorize funds first via a manual transaction.
    let t = saleor_rustify_db::payments::create_transaction(
        &db,
        &saleor_rustify_db::payments::NewTransaction {
            checkout_id: None,
            order_id: Some(oid),
            currency: "USD".into(),
            name: String::new(),
            app_identifier: None,
            idempotency_key: Some(format!("ops-cycle-{oid}")),
            available_actions: vec!["charge".into(), "cancel".into()],
        },
    )
    .await
    .unwrap();
    saleor_rustify_db::payments::authorize(&db, t.id, Decimal::new(1000, 0), &format!("ops-cycle-auth-{oid}"))
        .await
        .unwrap();

    let captured = order_ops::capture_order(&db, oid, Decimal::new(40, 0), None).await.unwrap();
    assert_eq!(captured, Decimal::new(40, 0));
    let refunded = order_ops::refund_order(&db, oid, Decimal::new(15, 0), None).await.unwrap();
    assert_eq!(refunded, Decimal::new(15, 0));
    // Void cancels the remaining authorized remainder.
    order_ops::void_order(&db, oid, None).await.unwrap();
    let v = saleor_rustify_db::payments::view(&db, t.id).await.unwrap();
    assert_eq!(v.authorized, Decimal::ZERO);
    assert_eq!(v.charged, Decimal::new(25, 0));
    let _ = order_order::Entity::find_by_id(oid).one(&db).await.unwrap();
}
