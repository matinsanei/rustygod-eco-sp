//! InvoiceService: request/fulfill/send/delete over Django's tables.

use rustygod_db::{entities::invoice_invoice, invoices};
use rustygod_proto::invoice::{
    invoice_service_server::InvoiceService, FulfillInvoiceRequest, FulfillInvoiceResponse,
    InvoiceIdRequest, InvoiceInfo, InvoiceResponse, ListReadyInvoicesRequest,
    ListReadyInvoicesResponse, RequestInvoiceRequest, RequestInvoiceResponse, SendInvoiceRequest,
    SendInvoiceResponse,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub struct InvoiceServiceImpl {
    db: Option<DatabaseConnection>,
}

impl InvoiceServiceImpl {
    pub fn new(db: Option<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn db(&self) -> Result<&DatabaseConnection, Status> {
        self.db
            .as_ref()
            .ok_or_else(|| Status::unavailable("postgres unavailable"))
    }

    fn err(code: &str, message: String) -> rustygod_proto::common::Error {
        rustygod_proto::common::Error {
            code: code.into(),
            message,
            field: String::new(),
        }
    }

    fn info(r: &invoice_invoice::Model) -> InvoiceInfo {
        let url = if r.invoice_file.is_empty() {
            r.external_url.clone().unwrap_or_default()
        } else {
            r.invoice_file.clone()
        };
        InvoiceInfo {
            id: r.id,
            order_id: r.order_id.map(|o| o.to_string()).unwrap_or_default(),
            status: r.status.clone(),
            number: r.number.clone().unwrap_or_default(),
            url,
        }
    }

    fn oid(s: &str) -> Result<Uuid, Status> {
        s.parse()
            .map_err(|_| Status::invalid_argument("order_id must be a UUID"))
    }
}

fn db_err(e: rustygod_db::DbError) -> rustygod_proto::common::Error {
    use rustygod_db::DbError as E;
    let code = match e {
        E::Invoice(_) => "NOT_APPLICABLE",
        _ => "DB_ERROR",
    };
    InvoiceServiceImpl::err(code, e.to_string())
}

#[tonic::async_trait]
impl InvoiceService for InvoiceServiceImpl {
    async fn request_invoice(
        &self,
        req: Request<RequestInvoiceRequest>,
    ) -> Result<Response<RequestInvoiceResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        let oid = Self::oid(&r.order_id)?;
        let number = (!r.number.is_empty()).then_some(r.number);
        match invoices::request_invoice(db, oid, number, None).await {
            Ok(row) => Ok(Response::new(RequestInvoiceResponse {
                invoice: Some(Self::info(&row)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(RequestInvoiceResponse {
                invoice: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn fulfill_invoice(
        &self,
        req: Request<FulfillInvoiceRequest>,
    ) -> Result<Response<FulfillInvoiceResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        match invoices::fulfill_invoice(db, r.id, &r.number, &r.url, None).await {
            Ok(row) => Ok(Response::new(FulfillInvoiceResponse {
                invoice: Some(Self::info(&row)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(FulfillInvoiceResponse {
                invoice: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn send_invoice(
        &self,
        req: Request<SendInvoiceRequest>,
    ) -> Result<Response<SendInvoiceResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        match invoices::send_invoice(db, r.id, &r.email, None).await {
            Ok(row) => Ok(Response::new(SendInvoiceResponse {
                invoice: Some(Self::info(&row)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(SendInvoiceResponse {
                invoice: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn request_deletion(
        &self,
        req: Request<InvoiceIdRequest>,
    ) -> Result<Response<InvoiceResponse>, Status> {
        let db = self.db()?;
        match invoices::request_deletion(db, req.into_inner().id, None).await {
            Ok(row) => Ok(Response::new(InvoiceResponse {
                invoice: Some(Self::info(&row)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(InvoiceResponse {
                invoice: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn delete_invoice(
        &self,
        req: Request<InvoiceIdRequest>,
    ) -> Result<Response<InvoiceResponse>, Status> {
        let db = self.db()?;
        match invoices::delete_invoice(db, req.into_inner().id, None).await {
            Ok(row) => Ok(Response::new(InvoiceResponse {
                invoice: Some(Self::info(&row)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(InvoiceResponse {
                invoice: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn list_ready(
        &self,
        req: Request<ListReadyInvoicesRequest>,
    ) -> Result<Response<ListReadyInvoicesResponse>, Status> {
        let db = self.db()?;
        let oid = Self::oid(&req.into_inner().order_id)?;
        match invoices::ready_invoices(db, oid).await {
            Ok(rows) => Ok(Response::new(ListReadyInvoicesResponse {
                invoices: rows.iter().map(Self::info).collect(),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(ListReadyInvoicesResponse {
                invoices: vec![],
                errors: vec![db_err(e)],
            })),
        }
    }
}
