//! Invoice contract vs Django's `invoice_*` tables.
//! Mirrors `saleor/graphql/invoice/tests/`: request guards (status +
//! billing address), pending → success → sent lifecycle, deletion flow,
//! and the event rows.

use rustygod_db::{database_url, invoices};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

/// A Django order that can legally take an invoice: billed, non-draft.
async fn billable_order(db: &DatabaseConnection) -> Uuid {
    use rustygod_db::entities::order_order;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    order_order::Entity::find()
        .select_only()
        .column(order_order::Column::Id)
        .filter(order_order::Column::BillingAddressId.is_not_null())
        .filter(order_order::Column::Status.is_not_in(["draft", "unconfirmed", "expired"]))
        .into_tuple()
        .one(db)
        .await
        .unwrap()
        .expect("populatedb must have a billed, non-draft order")
}

async fn event_types(db: &DatabaseConnection, invoice_id: i32) -> Vec<String> {
    invoices::events_of(db, invoice_id)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.r#type)
        .collect()
}

#[tokio::test]
async fn request_fulfill_send_lifecycle() {
    let db = db().await;
    let oid = billable_order(&db).await;

    let inv = invoices::request_invoice(&db, oid, Some("FV/1/2026".into()), None)
        .await
        .unwrap();
    assert_eq!(inv.status, "pending");
    assert_eq!(inv.number.as_deref(), Some("FV/1/2026"));
    assert_eq!(inv.order_id, Some(oid));

    let inv = invoices::fulfill_invoice(&db, inv.id, "FV/1/2026", "https://cdn.example/i.pdf", None)
        .await
        .unwrap();
    assert_eq!(inv.status, "success");
    assert_eq!(inv.external_url.as_deref(), Some("https://cdn.example/i.pdf"));

    let ready = invoices::ready_invoices(&db, oid).await.unwrap();
    assert!(ready.iter().any(|r| r.id == inv.id));

    invoices::send_invoice(&db, inv.id, "buyer@example.com", None).await.unwrap();

    assert_eq!(
        event_types(&db, inv.id).await,
        vec!["requested", "created", "sent"]
    );
}

#[tokio::test]
async fn request_rejects_draft_and_unbilled_orders() {
    let db = db().await;
    use rustygod_db::entities::order_order;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

    // Draft order → INVALID_STATUS.
    let draft: Option<Uuid> = order_order::Entity::find()
        .select_only()
        .column(order_order::Column::Id)
        .filter(order_order::Column::Status.eq("draft"))
        .into_tuple()
        .one(&db)
        .await
        .unwrap();
    if let Some(did) = draft {
        let err = invoices::request_invoice(&db, did, None, None).await.unwrap_err();
        assert!(err.to_string().contains("draft, unconfirmed or expired"), "{err}");
    }

    // Non-draft order without billing address → NOT_READY.
    let unbilled: Option<Uuid> = order_order::Entity::find()
        .select_only()
        .column(order_order::Column::Id)
        .filter(order_order::Column::BillingAddressId.is_null())
        .filter(order_order::Column::Status.is_not_in(["draft", "unconfirmed", "expired"]))
        .into_tuple()
        .one(&db)
        .await
        .unwrap();
    if let Some(uid) = unbilled {
        let err = invoices::request_invoice(&db, uid, None, None).await.unwrap_err();
        assert!(err.to_string().contains("without billing address"), "{err}");
    }

    // Unknown order → not found.
    let err = invoices::request_invoice(&db, Uuid::new_v4(), None, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not found"), "{err}");
}

#[tokio::test]
async fn send_requires_ready_invoice() {
    let db = db().await;
    let oid = billable_order(&db).await;
    let inv = invoices::request_invoice(&db, oid, None, None).await.unwrap();
    let err = invoices::send_invoice(&db, inv.id, "buyer@example.com", None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Only ready invoices"), "{err}");
    invoices::delete_invoice(&db, inv.id, None).await.unwrap();
}

#[tokio::test]
async fn fail_and_deletion_flows() {
    let db = db().await;
    let oid = billable_order(&db).await;

    // Plugin failure path.
    let inv = invoices::request_invoice(&db, oid, None, None).await.unwrap();
    let failed = invoices::fail_invoice(&db, inv.id, "renderer exploded").await.unwrap();
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.message.as_deref(), Some("renderer exploded"));
    assert!(invoices::ready_invoices(&db, oid).await.unwrap().iter().all(|r| r.id != inv.id));
    // Failed invoices cannot be fulfilled.
    let err = invoices::fulfill_invoice(&db, inv.id, "X", "http://x", None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("only pending"), "{err}");

    // Deletion request → execute.
    let inv2 = invoices::request_invoice(&db, oid, None, None).await.unwrap();
    let pending = invoices::request_deletion(&db, inv2.id, None).await.unwrap();
    assert_eq!(pending.status, "pending");
    let gone = invoices::delete_invoice(&db, inv2.id, None).await.unwrap();
    assert_eq!(gone.status, "deleted");
    assert_eq!(
        event_types(&db, inv2.id).await,
        vec!["requested", "requested_deletion", "deleted"]
    );
}
