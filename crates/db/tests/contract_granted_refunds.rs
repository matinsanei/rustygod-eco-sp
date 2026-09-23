//! Granted-refund contract: decision vs money separation (RC2).
//! Create validates like Django (amount xor lines/shipping, line ownership,
//! quantities, charged coverage); execute moves the money with the event
//! linked back to the decision and flips status to success. Covers T1a.

use rust_decimal::Decimal;
use saleor_rustify_db::{
    catalog, checkout_store, complete, database_url, granted_refunds, order_store, payments,
};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn stock_guard() -> fs2::FileLockGuard {
    fs2::FileLockGuard::new()
}

mod fs2 {
    pub struct FileLockGuard {
        _f: std::fs::File,
    }
    impl FileLockGuard {
        pub fn new() -> Self {
            use fs2::FileExt;
            let f = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open("/tmp/rustify-stock.lock")
                .expect("stock lock");
            f.lock_exclusive().expect("stock lock");
            Self { _f: f }
        }
    }
}

struct PaidOrder {
    order_id: Uuid,
    line_id: Uuid,
    line_qty: i32,
    total: Decimal,
    currency: String,
    txn_id: i32,
}

/// Mint a real order via complete, then charge it in full on a transaction.
async fn paid_order(db: &DatabaseConnection, tag: &str) -> PaidOrder {
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    let token =
        checkout_store::create_checkout_row(db, ch_id, &currency, "grant@example.com")
            .await
            .unwrap();
    let products = catalog::list_products(db, "default-channel", None, 100)
        .await
        .unwrap();
    let vid: i32 = products
        .iter()
        .flat_map(|p| &p.variants)
        .find(|v| v.quantity_available >= 2)
        .expect("need a stocked variant")
        .id
        .parse()
        .unwrap();
    let pricing = catalog::checkout_pricing(db, "default-channel", &[vid])
        .await
        .unwrap();
    let unit = pricing[&vid].0.amount;
    checkout_store::add_line_row(db, token, vid, 2, unit, &currency, None)
        .await
        .unwrap();
    let out = complete::complete_checkout(db, token).await.unwrap();

    let (header, lines) = order_store::get_order_rows(db, out.order_id)
        .await
        .unwrap()
        .unwrap();
    let txn = payments::create_transaction(
        db,
        &payments::NewTransaction {
            checkout_id: None,
            order_id: Some(out.order_id),
            currency: header.currency.clone(),
            name: "manual".into(),
            app_identifier: Some("rustygod-manual".into()),
            idempotency_key: Some(format!("grant-{tag}-{}", Uuid::new_v4())),
            available_actions: vec!["authorize".into()],
        },
    )
    .await
    .unwrap();
    payments::authorize(db, txn.id, header.total_gross_amount, &format!("grant-auth-{tag}"))
        .await
        .unwrap();
    payments::charge(db, txn.id, header.total_gross_amount, &format!("grant-chg-{tag}"))
        .await
        .unwrap();
    PaidOrder {
        order_id: out.order_id,
        line_id: lines[0].id,
        line_qty: lines[0].quantity,
        total: header.total_gross_amount,
        currency: header.currency,
        txn_id: txn.id,
    }
}

async fn cleanup(db: &DatabaseConnection, g: &granted_refunds::GrantView, txn_id: i32) {
    use saleor_rustify_db::entities::{
        order_ordergrantedrefund, order_ordergrantedrefundline, payment_transactionevent,
        payment_transactionitem,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    // Events reference the grant (FK) → delete money first, decision last.
    payment_transactionevent::Entity::delete_many()
        .filter(payment_transactionevent::Column::TransactionId.eq(txn_id))
        .exec(db)
        .await
        .unwrap();
    order_ordergrantedrefundline::Entity::delete_many()
        .filter(order_ordergrantedrefundline::Column::GrantedRefundId.eq(g.id))
        .exec(db)
        .await
        .unwrap();
    order_ordergrantedrefund::Entity::delete_by_id(g.id)
        .exec(db)
        .await
        .unwrap();
    payment_transactionitem::Entity::delete_by_id(txn_id)
        .exec(db)
        .await
        .unwrap();
}

#[tokio::test]
async fn create_derives_amount_from_lines() {
    let _guard = stock_guard();
    let db = db().await;
    let o = paid_order(&db, "auto").await;
    let g = granted_refunds::create_granted_refund(
        &db,
        &granted_refunds::NewGrant {
            order_id: o.order_id,
            transaction_item_id: Some(o.txn_id),
            amount: None,
            lines: vec![granted_refunds::GrantLineInput {
                order_line_id: o.line_id,
                quantity: 1,
            }],
            reason: "one unit back".into(),
            shipping_costs_included: false,
            user_id: None,
            app_id: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(g.status, "none", "decision moves no money");
    assert!(g.amount > Decimal::ZERO);
    assert!(g.amount <= o.total);
    // Untouched money: transaction still fully charged.
    let v = payments::view(&db, o.txn_id).await.unwrap();
    assert_eq!(v.refunded, Decimal::ZERO);
    cleanup(&db, &g, o.txn_id).await;
}

#[tokio::test]
async fn create_rejects_amount_above_charged() {
    let _guard = stock_guard();
    let db = db().await;
    let o = paid_order(&db, "over").await;
    let err = granted_refunds::create_granted_refund(
        &db,
        &granted_refunds::NewGrant {
            order_id: o.order_id,
            transaction_item_id: Some(o.txn_id),
            amount: Some(o.total + Decimal::ONE),
            lines: vec![],
            reason: "too much".into(),
            shipping_costs_included: false,
            user_id: None,
            app_id: None,
        },
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("only"), "must cite charged coverage: {err}");
    // No row left behind.
    use saleor_rustify_db::entities::order_ordergrantedrefund;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let n = order_ordergrantedrefund::Entity::find()
        .filter(order_ordergrantedrefund::Column::OrderId.eq(o.order_id))
        .all(&db)
        .await
        .unwrap()
        .len();
    assert_eq!(n, 0);
    // Cleanup bare transaction.
    use saleor_rustify_db::entities::{payment_transactionevent, payment_transactionitem};
    payment_transactionevent::Entity::delete_many()
        .filter(payment_transactionevent::Column::TransactionId.eq(o.txn_id))
        .exec(&db)
        .await
        .unwrap();
    payment_transactionitem::Entity::delete_by_id(o.txn_id)
        .exec(&db)
        .await
        .unwrap();
}

#[tokio::test]
async fn execute_moves_money_and_flips_status() {
    let _guard = stock_guard();
    let db = db().await;
    let o = paid_order(&db, "exec").await;
    let g = granted_refunds::create_granted_refund(
        &db,
        &granted_refunds::NewGrant {
            order_id: o.order_id,
            transaction_item_id: Some(o.txn_id),
            amount: Some(o.total),
            lines: vec![],
            reason: "full return".into(),
            shipping_costs_included: false,
            user_id: None,
            app_id: None,
        },
    )
    .await
    .unwrap();
    let out = granted_refunds::execute_granted_refund(&db, g.id, "exec-key-1")
        .await
        .unwrap();
    assert!(!out.replayed);
    assert_eq!(out.view.status, "success");
    let v = payments::view(&db, o.txn_id).await.unwrap();
    assert_eq!(v.refunded, o.total, "money actually moved");
    // The money event links back to the decision.
    use saleor_rustify_db::entities::payment_transactionevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let linked = payment_transactionevent::Entity::find()
        .filter(payment_transactionevent::Column::TransactionId.eq(o.txn_id))
        .filter(payment_transactionevent::Column::RelatedGrantedRefundId.eq(g.id))
        .all(&db)
        .await
        .unwrap();
    assert!(!linked.is_empty(), "refund events must reference the grant");
    // RC2 live: decision vs money parity holds after execute.
    let checks = saleor_rustify_db::reconcile::reconcile_order(&db, o.order_id)
        .await
        .unwrap();
    let rc2 = checks.iter().find(|c| c.name == "refunded_within_charged").unwrap();
    assert!(rc2.ok, "RC2 must hold: {}", rc2.detail);
    cleanup(&db, &out.view, o.txn_id).await;
}

#[tokio::test]
async fn execute_replay_moves_no_money_twice() {
    let _guard = stock_guard();
    let db = db().await;
    let o = paid_order(&db, "replay").await;
    let g = granted_refunds::create_granted_refund(
        &db,
        &granted_refunds::NewGrant {
            order_id: o.order_id,
            transaction_item_id: Some(o.txn_id),
            amount: Some(o.total),
            lines: vec![],
            reason: "replay check".into(),
            shipping_costs_included: false,
            user_id: None,
            app_id: None,
        },
    )
    .await
    .unwrap();
    let first = granted_refunds::execute_granted_refund(&db, g.id, "replay-key")
        .await
        .unwrap();
    assert!(!first.replayed);
    let second = granted_refunds::execute_granted_refund(&db, g.id, "replay-key")
        .await
        .unwrap();
    assert!(second.replayed, "second execute must be a replay");
    let v = payments::view(&db, o.txn_id).await.unwrap();
    assert_eq!(v.refunded, o.total, "exactly once");
    let _ = o.line_qty;
    let _ = o.currency;
    cleanup(&db, &second.view, o.txn_id).await;
}

#[tokio::test]
async fn create_rejects_foreign_line() {
    let _guard = stock_guard();
    let db = db().await;
    let a = paid_order(&db, "foreign-a").await;
    let b = paid_order(&db, "foreign-b").await;
    let err = granted_refunds::create_granted_refund(
        &db,
        &granted_refunds::NewGrant {
            order_id: a.order_id,
            transaction_item_id: Some(a.txn_id),
            amount: None,
            lines: vec![granted_refunds::GrantLineInput {
                order_line_id: b.line_id,
                quantity: 1,
            }],
            reason: "sneaky".into(),
            shipping_costs_included: false,
            user_id: None,
            app_id: None,
        },
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("does not belong"), "{err}");
    // Cleanup both bare setups.
    use saleor_rustify_db::entities::{payment_transactionevent, payment_transactionitem};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    for t in [a.txn_id, b.txn_id] {
        payment_transactionevent::Entity::delete_many()
            .filter(payment_transactionevent::Column::TransactionId.eq(t))
            .exec(&db)
            .await
            .unwrap();
        payment_transactionitem::Entity::delete_by_id(t)
            .exec(&db)
            .await
            .unwrap();
    }
}
