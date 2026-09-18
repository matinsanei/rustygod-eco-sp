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


fn test_key() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../db/tests/testdata/test_rsa.pem"
    ))
    .expect("test RSA key must exist")
}

/// Staff request: populatedb admin JWT in metadata.
async fn staff_req<T>(msg: T) -> Request<T> {
    std::env::set_var("RSA_PRIVATE_KEY", test_key());
    let db = rustygod_db::connect(&rustygod_db::database_url()).await.unwrap();
    let (user, _) = rustygod_db::auth::find_for_login(&db, "admin@example.com")
        .await
        .unwrap()
        .unwrap();
    let pair = rustygod_core::auth::mint_tokens_with_key(
        &test_key(),
        "test",
        &user.email,
        user.id,
        user.is_staff,
        &user.jwt_token_key,
    )
    .unwrap();
    let mut req = Request::new(msg);
    req.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", pair.access).parse().unwrap(),
    );
    req
}

#[tokio::test]
async fn grpc_invoice_lifecycle() {
    let svc = svc().await;
    let order_id = billable_order().await;

    let inv = svc
        .request_invoice(staff_req(RequestInvoiceRequest {
            order_id: order_id.clone(),
            number: "FV/9/2026".into(),
        }).await)
        .await
        .unwrap()
        .into_inner()
        .invoice
        .unwrap();
    assert_eq!(inv.status, "pending");
    assert_eq!(inv.number, "FV/9/2026");

    // Draft order is rejected over the wire too.
    let denied = svc
        .request_invoice(staff_req(RequestInvoiceRequest {
            order_id: Uuid::new_v4().to_string(),
            number: String::new(),
        }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(denied.invoice.is_none());
    assert!(!denied.errors.is_empty());

    let inv = svc
        .fulfill_invoice(staff_req(FulfillInvoiceRequest {
            id: inv.id,
            number: "FV/9/2026".into(),
            url: "https://cdn.example/fv9.pdf".into(),
        }).await)
        .await
        .unwrap()
        .into_inner()
        .invoice
        .unwrap();
    assert_eq!(inv.status, "success");
    assert_eq!(inv.url, "https://cdn.example/fv9.pdf");

    let ready = svc
        .list_ready(staff_req(ListReadyInvoicesRequest { order_id }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(ready.errors.is_empty());
    assert!(ready.invoices.iter().any(|i| i.id == inv.id));

    let sent = svc
        .send_invoice(staff_req(SendInvoiceRequest {
            id: inv.id,
            email: "buyer@example.com".into(),
        }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(sent.errors.is_empty());
    assert_eq!(sent.invoice.unwrap().status, "success");

    // Deletion roundtrip.
    let pending = svc
        .request_deletion(staff_req(InvoiceIdRequest { id: inv.id }).await)
        .await
        .unwrap()
        .into_inner()
        .invoice
        .unwrap();
    assert_eq!(pending.status, "pending");
    let gone = svc
        .delete_invoice(staff_req(InvoiceIdRequest { id: inv.id }).await)
        .await
        .unwrap()
        .into_inner()
        .invoice
        .unwrap();
    assert_eq!(gone.status, "deleted");
}
