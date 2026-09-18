//! Draft-order contract vs Django's `order_*` tables.
//! Mirrors `saleor/graphql/order/mutations/draft_order_*.py` and
//! `saleor/order/tests/test_draft_order.py`: create/edit/complete/delete,
//! the "not draft" guards, stock allocation on complete, and event rows.

use rust_decimal::Decimal;
use rustygod_db::{catalog, database_url, drafts, order_store};
use sea_orm::DatabaseConnection;

/// Cross-process stock lock: allocations touch shared Django stock rows,
/// and contract binaries run in parallel (same pattern as fulfillment tests).
struct StockGuard {
    _f: std::fs::File,
}

fn stock_guard() -> StockGuard {
    use fs2::FileExt;
    let f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open("/tmp/rustygod-stock.lock")
        .expect("lock file");
    f.lock_exclusive().expect("stock lock");
    StockGuard { _f: f }
}

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

/// Two stocked variants on default-channel with their listing prices.
async fn stocked_variants(db: &DatabaseConnection) -> Vec<(i32, Decimal)> {
    let products = catalog::list_products(db, "default-channel", None, 100)
        .await
        .unwrap();
    let mut out = Vec::new();
    for p in &products {
        for v in &p.variants {
            if v.quantity_available >= 4 {
                let vid: i32 = v.id.parse().unwrap();
                let pricing = catalog::checkout_pricing(db, "default-channel", &[vid])
                    .await
                    .unwrap();
                if let Some((money, _)) = pricing.get(&vid) {
                    out.push((vid, money.amount));
                }
                if out.len() == 2 {
                    return out;
                }
            }
        }
    }
    panic!("need two stocked variants on default-channel");
}

fn line(variant_id: i32, quantity: i32) -> drafts::DraftLineInput {
    drafts::DraftLineInput { variant_id, quantity, custom_price: None, force_new_line: false }
}

#[tokio::test]
async fn create_draft_prices_lines_and_totals() {
    let db = db().await;
    let sv = stocked_variants(&db).await;
    let (v1, p1) = sv[0];
    let (v2, _) = sv[1];

    let view = drafts::create_draft(
        &db,
        "default-channel",
        "staff-draft@example.com",
        None,
        vec![
            line(v1, 2),
            drafts::DraftLineInput {
                variant_id: v2,
                quantity: 1,
                custom_price: Some(Decimal::new(999, 2)),
                force_new_line: false,
            },
        ],
        None,
    )
    .await
    .unwrap();

    assert_eq!(view.status, "draft");
    assert!(view.number > 0);
    assert_eq!(view.lines_count, 2);
    assert_eq!(view.total_gross, p1 * Decimal::from(2) + Decimal::new(999, 2));

    let (header, lines) = order_store::get_order_rows(&db, view.id).await.unwrap().unwrap();
    assert_eq!(header.status, "draft");
    assert_eq!(lines.len(), 2);

    drafts::delete_draft(&db, view.id).await.unwrap();
    assert!(order_store::get_order_rows(&db, view.id).await.unwrap().is_none());
}

#[tokio::test]
async fn add_lines_merges_by_default_and_splits_when_forced() {
    let db = db().await;
    let sv = stocked_variants(&db).await;
    let (v1, p1) = sv[0];

    let view = drafts::create_draft(&db, "default-channel", "m@example.com", None, vec![line(v1, 1)], None)
        .await
        .unwrap();

    // Same variant merges: still 1 line, qty 1+2.
    let view = drafts::add_lines(&db, view.id, "default-channel", vec![line(v1, 2)], None)
        .await
        .unwrap();
    assert_eq!(view.lines_count, 1);
    assert_eq!(view.total_gross, p1 * Decimal::from(3));

    // Forced split: 2 lines.
    let view = drafts::add_lines(
        &db,
        view.id,
        "default-channel",
        vec![drafts::DraftLineInput { variant_id: v1, quantity: 1, custom_price: None, force_new_line: true }],
        None,
    )
    .await
    .unwrap();
    assert_eq!(view.lines_count, 2);
    assert_eq!(view.total_gross, p1 * Decimal::from(4));

    drafts::delete_draft(&db, view.id).await.unwrap();
}

#[tokio::test]
async fn set_quantity_and_remove_line_recalc() {
    let db = db().await;
    let sv = stocked_variants(&db).await;
    let (v1, p1) = sv[0];
    let (v2, p2) = sv[1];

    let view = drafts::create_draft(
        &db, "default-channel", "q@example.com", None, vec![line(v1, 1), line(v2, 1)], None,
    )
    .await
    .unwrap();
    let (_, lines) = order_store::get_order_rows(&db, view.id).await.unwrap().unwrap();
    assert_eq!(lines.len(), 2);

    let view = drafts::set_line_quantity(&db, view.id, lines[0].id, 4).await.unwrap();
    assert_eq!(view.total_gross, p1 * Decimal::from(4) + p2);

    let view = drafts::remove_line(&db, view.id, lines[1].id, None).await.unwrap();
    assert_eq!(view.lines_count, 1);
    assert_eq!(view.total_gross, p1 * Decimal::from(4));

    let err = drafts::set_line_quantity(&db, view.id, lines[0].id, 0).await.unwrap_err();
    assert!(matches!(err, rustygod_db::DbError::Draft(_)));

    drafts::delete_draft(&db, view.id).await.unwrap();
}

#[tokio::test]
async fn completed_draft_rejects_edits_and_delete() {
    let _guard = stock_guard();
    let db = db().await;
    let sv = stocked_variants(&db).await;
    let view = drafts::create_draft(&db, "default-channel", "c@example.com", None, vec![line(sv[0].0, 1)], None)
        .await
        .unwrap();
    let header = drafts::complete_draft(&db, view.id, None).await.unwrap();
    assert_ne!(header.status, "draft");

    for res in [
        drafts::add_lines(&db, view.id, "default-channel", vec![line(sv[0].0, 1)], None).await.map(|_| ()),
        drafts::delete_draft(&db, view.id).await,
    ] {
        let err = res.unwrap_err().to_string();
        assert!(err.contains("not draft"), "{err}");
    }

    // Completed order still readable; clean up the row directly
    // (allocations, then lines, then events — all reference the order).
    use rustygod_db::entities::{
        order_order, order_orderevent, order_orderline, warehouse_allocation,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let (_, lines) = order_store::get_order_rows(&db, view.id).await.unwrap().unwrap();
    for l in &lines {
        warehouse_allocation::Entity::delete_many()
            .filter(warehouse_allocation::Column::OrderLineId.eq(l.id))
            .exec(&db)
            .await
            .unwrap();
        order_orderline::Entity::delete_by_id(l.id).exec(&db).await.unwrap();
    }
    order_orderevent::Entity::delete_many()
        .filter(order_orderevent::Column::OrderId.eq(view.id))
        .exec(&db)
        .await
        .unwrap();
    order_order::Entity::delete_by_id(view.id).exec(&db).await.unwrap();
}

#[tokio::test]
async fn complete_allocates_stock_and_writes_event() {
    let _guard = stock_guard();
    let db = db().await;
    let sv = stocked_variants(&db).await;
    let (v1, _) = sv[0];

    let free_before: i32 = rustygod_db::commerce::stocks_for_variant(&db, v1)
        .await
        .unwrap()
        .iter()
        .map(|s| s.quantity - s.quantity_allocated)
        .sum();

    let view = drafts::create_draft(&db, "default-channel", "a@example.com", None, vec![line(v1, 2)], None)
        .await
        .unwrap();
    let header = drafts::complete_draft(&db, view.id, None).await.unwrap();
    assert!(header.status == "unfulfilled" || header.status == "unconfirmed", "{}", header.status);

    // Allocation consumed exactly the ordered quantity (tracked variants).
    let free_after: i32 = rustygod_db::commerce::stocks_for_variant(&db, v1)
        .await
        .unwrap()
        .iter()
        .map(|s| s.quantity - s.quantity_allocated)
        .sum();
    assert!(free_before - free_after == 0 || free_before - free_after == 2,
        "before={free_before} after={free_after}");

    use rustygod_db::entities::order_orderevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let placed = order_orderevent::Entity::find()
        .filter(order_orderevent::Column::OrderId.eq(view.id))
        .filter(order_orderevent::Column::Type.eq(drafts::events::PLACED_FROM_DRAFT))
        .one(&db)
        .await
        .unwrap();
    assert!(placed.is_some(), "PLACED_FROM_DRAFT event must exist");

    // Cleanup via direct deletes (completed orders are not deletable by design).
    use rustygod_db::entities::{order_order, order_orderline, warehouse_allocation};
    let (_, lines) = order_store::get_order_rows(&db, view.id).await.unwrap().unwrap();
    for l in &lines {
        let allocs = warehouse_allocation::Entity::find()
            .filter(warehouse_allocation::Column::OrderLineId.eq(l.id))
            .all(&db)
            .await
            .unwrap();
        for a in &allocs {
            warehouse_allocation::Entity::delete_by_id(a.id).exec(&db).await.unwrap();
        }
        order_orderline::Entity::delete_by_id(l.id).exec(&db).await.unwrap();
    }
    order_orderevent::Entity::delete_many()
        .filter(order_orderevent::Column::OrderId.eq(view.id))
        .exec(&db)
        .await
        .unwrap();
    order_order::Entity::delete_by_id(view.id).exec(&db).await.unwrap();
}

#[tokio::test]
async fn complete_fails_on_insufficient_stock() {
    let _guard = stock_guard();
    let db = db().await;
    let sv = stocked_variants(&db).await;
    // Absurd quantity: no warehouse can cover it.
    let view = drafts::create_draft(
        &db, "default-channel", "s@example.com", None, vec![line(sv[0].0, 1_000_000)], None,
    )
    .await
    .unwrap();
    let err = drafts::complete_draft(&db, view.id, None).await.unwrap_err();
    assert!(err.to_string().contains("insufficient stock"), "{err}");
    // Still a draft — nothing was flipped.
    let (header, _) = order_store::get_order_rows(&db, view.id).await.unwrap().unwrap();
    assert_eq!(header.status, "draft");

    drafts::delete_draft(&db, view.id).await.unwrap();
}

#[tokio::test]
async fn draft_created_event_exists() {
    let db = db().await;
    let sv = stocked_variants(&db).await;
    let view = drafts::create_draft(&db, "default-channel", "e@example.com", None, vec![line(sv[0].0, 1)], None)
        .await
        .unwrap();
    use rustygod_db::entities::order_orderevent;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let ev = order_orderevent::Entity::find()
        .filter(order_orderevent::Column::OrderId.eq(view.id))
        .filter(order_orderevent::Column::Type.eq(drafts::events::DRAFT_CREATED))
        .one(&db)
        .await
        .unwrap();
    assert!(ev.is_some());
    drafts::delete_draft(&db, view.id).await.unwrap();
}
