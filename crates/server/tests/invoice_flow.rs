//! DB-mode invoice flow over gRPC handlers: request → fulfill →
//! list-ready → send, against real Postgres.

use rustygod_db::database_url;
use rustygod_proto::invoice::{
    invoice_service_server::InvoiceService, FulfillInvoiceRequest, InvoiceIdRequest,
    ListReadyInvoicesRequest, RequestInvoiceRequest, SendInvoiceRequest,
};
use rustygod_server::service_invoice::InvoiceServiceImpl;
use tonic::Request;
use uuid::Uuid;

async fn svc() -> InvoiceServiceImpl {
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    InvoiceServiceImpl::new(Some(db))
}

async fn billable_order() -> String {
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    use rustygod_db::entities::order_order;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    order_order::Entity::find()
        .select_only()
        .column(order_order::Column::Id)
        .filter(order_order::Column::BillingAddressId.is_not_null())
        .filter(order_order::Column::Status.is_not_in(["draft", "unconfirmed", "expired"]))
        .into_tuple::<Uuid>()
        .one(&db)
        .await
        .unwrap()
        .expect("populatedb must have a billable order")
        .to_string()
}

#[tokio::test]
async fn grpc_invoice_lifecycle() {
    let svc = svc().await;
    let order_id = billable_order().await;

    let inv = svc
        .request_invoice(Request::new(RequestInvoiceRequest {
            order_id: order_id.clone(),
            number: "FV/9/2026".into(),
        }))
        .await
        .unwrap()
        .into_inner()
        .invoice
        .unwrap();
    assert_eq!(inv.status, "pending");
    assert_eq!(inv.number, "FV/9/2026");

    // Draft order is rejected over the wire too.
    let denied = svc
        .request_invoice(Request::new(RequestInvoiceRequest {
            order_id: Uuid::new_v4().to_string(),
            number: String::new(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(denied.invoice.is_none());
    assert!(!denied.errors.is_empty());

    let inv = svc
        .fulfill_invoice(Request::new(FulfillInvoiceRequest {
            id: inv.id,
            number: "FV/9/2026".into(),
            url: "https://cdn.example/fv9.pdf".into(),
        }))
        .await
        .unwrap()
        .into_inner()
        .invoice
        .unwrap();
    assert_eq!(inv.status, "success");
    assert_eq!(inv.url, "https://cdn.example/fv9.pdf");

    let ready = svc
        .list_ready(Request::new(ListReadyInvoicesRequest { order_id }))
        .await
        .unwrap()
        .into_inner();
    assert!(ready.errors.is_empty());
    assert!(ready.invoices.iter().any(|i| i.id == inv.id));

    let sent = svc
        .send_invoice(Request::new(SendInvoiceRequest {
            id: inv.id,
            email: "buyer@example.com".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(sent.errors.is_empty());
    assert_eq!(sent.invoice.unwrap().status, "success");

    // Deletion roundtrip.
    let pending = svc
        .request_deletion(Request::new(InvoiceIdRequest { id: inv.id }))
        .await
        .unwrap()
        .into_inner()
        .invoice
        .unwrap();
    assert_eq!(pending.status, "pending");
    let gone = svc
        .delete_invoice(Request::new(InvoiceIdRequest { id: inv.id }))
        .await
        .unwrap()
        .into_inner()
        .invoice
        .unwrap();
    assert_eq!(gone.status, "deleted");
}
