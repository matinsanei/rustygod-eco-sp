//! DraftOrderService: staff draft orders over Django's `order_*` tables.

use rust_decimal::Decimal;
use rustygod_db::drafts::{self, DraftLineInput, DraftOrderView};
use rustygod_proto::draft::{
    draft_order_service_server::DraftOrderService, CompleteDraftOrderResponse,
    CreateDraftOrderRequest, CreateDraftOrderResponse, DeleteDraftOrderResponse,
    DraftOrderIdRequest, DraftOrderInfo, DraftOrderLinesRequest, DraftOrderLinesResponse,
    RemoveDraftLineRequest, RemoveDraftLineResponse, SetDraftLineQuantityRequest,
    SetDraftLineQuantityResponse,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub struct DraftOrderServiceImpl {
    db: Option<DatabaseConnection>,
}

impl DraftOrderServiceImpl {
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

    fn info(v: &DraftOrderView) -> DraftOrderInfo {
        DraftOrderInfo {
            id: v.id.to_string(),
            number: v.number.to_string(),
            status: v.status.clone(),
            currency: v.currency.clone(),
            total_gross: v.total_gross.to_string(),
            lines_count: v.lines_count,
        }
    }

    fn id(s: &str, what: &str) -> Result<Uuid, Status> {
        s.parse()
            .map_err(|_| Status::invalid_argument(format!("{what} must be a UUID")))
    }

    fn lines(
        raw: Vec<rustygod_proto::draft::DraftLineInput>,
    ) -> Result<Vec<DraftLineInput>, Status> {
        raw.into_iter()
            .map(|l| {
                Ok(DraftLineInput {
                    variant_id: l
                        .variant_id
                        .parse()
                        .map_err(|_| Status::invalid_argument("variant_id must be an integer"))?,
                    quantity: l.quantity,
                    custom_price: if l.custom_price.is_empty() {
                        None
                    } else {
                        Some(l.custom_price.parse::<Decimal>().map_err(|_| {
                            Status::invalid_argument("custom_price must be a decimal string")
                        })?)
                    },
                    force_new_line: l.force_new_line,
                })
            })
            .collect()
    }
}

fn db_err(e: rustygod_db::DbError) -> rustygod_proto::common::Error {
    use rustygod_db::DbError as E;
    let code = match e {
        E::Draft(_) => "NOT_APPLICABLE",
        E::CheckoutNotFound(_) => "NOT_FOUND",
        _ => "DB_ERROR",
    };
    DraftOrderServiceImpl::err(code, e.to_string())
}

fn channel_of(req: &str) -> &str {
    if req.is_empty() {
        "default-channel"
    } else {
        req
    }
}

#[tonic::async_trait]
impl DraftOrderService for DraftOrderServiceImpl {
    async fn create_draft_order(
        &self,
        req: Request<CreateDraftOrderRequest>,
    ) -> Result<Response<CreateDraftOrderResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        match drafts::create_draft(db, channel_of(&r.channel), &r.email, None, Self::lines(r.lines)?, None).await
        {
            Ok(v) => Ok(Response::new(CreateDraftOrderResponse {
                order: Some(Self::info(&v)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(CreateDraftOrderResponse {
                order: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn add_draft_lines(
        &self,
        req: Request<DraftOrderLinesRequest>,
    ) -> Result<Response<DraftOrderLinesResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        let id = Self::id(&r.order_id, "order_id")?;
        match drafts::add_lines(db, id, channel_of(&r.channel), Self::lines(r.lines)?, None).await {
            Ok(v) => Ok(Response::new(DraftOrderLinesResponse {
                order: Some(Self::info(&v)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(DraftOrderLinesResponse {
                order: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn set_draft_line_quantity(
        &self,
        req: Request<SetDraftLineQuantityRequest>,
    ) -> Result<Response<SetDraftLineQuantityResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        let id = Self::id(&r.order_id, "order_id")?;
        let line = Self::id(&r.line_id, "line_id")?;
        match drafts::set_line_quantity(db, id, line, r.quantity).await {
            Ok(v) => Ok(Response::new(SetDraftLineQuantityResponse {
                order: Some(Self::info(&v)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(SetDraftLineQuantityResponse {
                order: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn remove_draft_line(
        &self,
        req: Request<RemoveDraftLineRequest>,
    ) -> Result<Response<RemoveDraftLineResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        let id = Self::id(&r.order_id, "order_id")?;
        let line = Self::id(&r.line_id, "line_id")?;
        match drafts::remove_line(db, id, line, None).await {
            Ok(v) => Ok(Response::new(RemoveDraftLineResponse {
                order: Some(Self::info(&v)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(RemoveDraftLineResponse {
                order: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn complete_draft_order(
        &self,
        req: Request<DraftOrderIdRequest>,
    ) -> Result<Response<CompleteDraftOrderResponse>, Status> {
        let db = self.db()?;
        let id = Self::id(&req.into_inner().order_id, "order_id")?;
        match drafts::complete_draft(db, id, None).await {
            Ok(h) => Ok(Response::new(CompleteDraftOrderResponse {
                order: Some(DraftOrderInfo {
                    id: h.id.to_string(),
                    number: h.number.to_string(),
                    status: h.status.clone(),
                    currency: h.currency.clone(),
                    total_gross: h.total_gross_amount.to_string(),
                    lines_count: 0,
                }),
                status: h.status,
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(CompleteDraftOrderResponse {
                order: None,
                status: String::new(),
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn delete_draft_order(
        &self,
        req: Request<DraftOrderIdRequest>,
    ) -> Result<Response<DeleteDraftOrderResponse>, Status> {
        let db = self.db()?;
        let id = Self::id(&req.into_inner().order_id, "order_id")?;
        match drafts::delete_draft(db, id).await {
            Ok(()) => Ok(Response::new(DeleteDraftOrderResponse { errors: vec![] })),
            Err(e) => Ok(Response::new(DeleteDraftOrderResponse {
                errors: vec![db_err(e)],
            })),
        }
    }
}
