//! Fulfillment lifecycle + granted refunds (Django fulfill/return/grant parity).

use rust_decimal::Decimal;
use saleor_rustify_db::{catalog, database_url, drafts, fulfillment, granted_refunds, order_ops};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn stocked_variant(db: &DatabaseConnection) -> i32 {
    let products = catalog::list_products(db, "default-channel", None, 100).await.unwrap();
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

/// Confirmed, paid order with one fulfilled line. Returns (order, line, fulfillment).
async fn paid_fulfilled(db: &DatabaseConnection) -> (Uuid, Uuid, i32) {
    let vid = stocked_variant(db).await;
    let view = drafts::create_draft(
        db,
        "default-channel",
        "fulfill@example.com",
        None,
        vec![drafts::DraftLineInput { variant_id: vid, quantity: 2, custom_price: None, force_new_line: false }],
        None,
    )
    .await
    .unwrap();
    let oid = view.id;
    use saleor_rustify_db::entities::{order_order, order_orderline};
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
    let line = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(oid))
        .one(db)
        .await
        .unwrap()
        .unwrap();
    let row = order_order::Entity::find_by_id(oid).one(db).await.unwrap().unwrap();
    let mut am: order_order::ActiveModel = row.into();
    am.status = Set("unconfirmed".to_string());
    am.update(db).await.unwrap();
    order_ops::confirm_order(db, oid, None).await.unwrap();
    order_ops::mark_order_as_paid(db, oid, None, None).await.unwrap();
    let f = fulfillment::create_fulfillment(
        db,
        oid,
        &[fulfillment::FulfillItem { order_line_id: line.id, quantity: 2, stock_id: None }],
        "TRACK-1",
    )
    .await
    .unwrap();
    assert_eq!(f.status, "fulfilled");
    (oid, line.id, f.id)
}

#[tokio::test]
async fn cancel_restores_stock_and_tracking_updates() {
    let db = db().await;
    let (oid, lid, fid) = paid_fulfilled(&db).await;
    use saleor_rustify_db::entities::order_orderline;
    use sea_orm::EntityTrait;
    let before = order_orderline::Entity::find_by_id(lid).one(&db).await.unwrap().unwrap();
    assert_eq!(before.quantity_fulfilled, 2);

    fulfillment::update_tracking(&db, fid, "TRACK-2").await.unwrap();
    let v = fulfillment::view(&db, fid).await.unwrap();
    assert_eq!(v.tracking_number, "TRACK-2");

    let c = fulfillment::cancel_fulfillment_to(&db, fid, None).await.unwrap();
    assert_eq!(c.status, "canceled");
    let after = order_orderline::Entity::find_by_id(lid).one(&db).await.unwrap().unwrap();
    assert_eq!(after.quantity_fulfilled, 0);
    // Second cancel is idempotent.
    fulfillment::cancel_fulfillment_to(&db, fid, None).await.unwrap();
    let _ = oid;
}

#[tokio::test]
async fn approve_waiting_fulfillment() {
    let db = db().await;
    let (_, _, fid) = paid_fulfilled(&db).await;
    // Simulate a Django-shared waiting row.
    use saleor_rustify_db::entities::order_fulfillment;
    use sea_orm::{ActiveModelTrait, EntityTrait};
    let f = order_fulfillment::Entity::find_by_id(fid).one(&db).await.unwrap().unwrap();
    let mut am: order_fulfillment::ActiveModel = f.into();
    use sea_orm::Set;
    am.status = Set("waiting_for_approval".to_string());
    am.update(&db).await.unwrap();
    let v = fulfillment::approve_fulfillment(&db, fid, false).await.unwrap();
    assert_eq!(v.status, "fulfilled");
    // Approving a fulfilled row fails honestly.
    assert!(fulfillment::approve_fulfillment(&db, fid, false).await.is_err());
}

#[tokio::test]
async fn return_and_refund_moves_money_and_stock() {
    let db = db().await;
    let (oid, lid, _) = paid_fulfilled(&db).await;
    let out = fulfillment::return_and_refund_full(
        &db,
        oid,
        &[fulfillment::FulfillItem { order_line_id: lid, quantity: 1, stock_id: None }],
        "customer return",
        true,
        None,
        false,
        None,
    )
    .await
    .unwrap();
    assert!(out.amount > Decimal::ZERO);
    let g = granted_refunds::view(&db, out.granted_refund_id).await.unwrap();
    assert_eq!(g.status, "success");
    // Charged bucket decreased by the refund.
    use saleor_rustify_db::entities::order_order;
    use sea_orm::EntityTrait;
    let row = order_order::Entity::find_by_id(oid).one(&db).await.unwrap().unwrap();
    assert!(row.total_charged_amount < row.total_gross_amount);
}

#[tokio::test]
async fn grant_create_update_guards() {
    let db = db().await;
    let (oid, lid, _) = paid_fulfilled(&db).await;
    let v = granted_refunds::create_granted_refund(
        &db,
        &granted_refunds::NewGrant {
            order_id: oid,
            transaction_item_id: None,
            amount: Some(Decimal::new(5, 0)),
            lines: vec![],
            reason: "goodwill".into(),
            shipping_costs_included: false,
            user_id: None,
            app_id: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(v.status, "none");
    // Empty update rejected.
    assert!(granted_refunds::update_granted_refund(
        &db,
        v.id,
        &granted_refunds::UpdateGrant {
            amount: None, reason: None, transaction_item_id: None,
            grant_refund_for_shipping: false, add_lines: vec![], remove_line_ids: vec![],
        },
    )
    .await
    .is_err());
    // Reason-only + amount + line add.
    let u = granted_refunds::update_granted_refund(
        &db,
        v.id,
        &granted_refunds::UpdateGrant {
            amount: Some(Decimal::new(7, 0)),
            reason: Some("goodwill+ship".into()),
            transaction_item_id: None,
            grant_refund_for_shipping: true,
            add_lines: vec![granted_refunds::GrantLineInput { order_line_id: lid, quantity: 1 }],
            remove_line_ids: vec![],
        },
    )
    .await
    .unwrap();
    assert_eq!(u.amount, Decimal::new(7, 0));
    assert_eq!(u.lines.len(), 1);
}
