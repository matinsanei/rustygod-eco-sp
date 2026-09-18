//! PaymentService: TransactionItem lifecycle over Django's tables +
//! manual gateway actions (authorize/charge/refund/cancel).

use rust_decimal::Decimal;
use rustygod_db::payments;
use rustygod_proto::payment::{
    payment_service_server::PaymentService, CreateTransactionRequest, CreateTransactionResponse,
    GatewayActionRequest, GatewayActionResponse, GetTransactionRequest, GetTransactionResponse,
    TransactionInfo,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub struct PaymentServiceImpl {
    db: Option<DatabaseConnection>,
}

impl PaymentServiceImpl {
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

    fn view(v: &payments::TxnView) -> TransactionInfo {
        TransactionInfo {
            id: v.id.to_string(),
            token: v.token.to_string(),
            currency: v.currency.clone(),
            authorized_amount: v.authorized.to_string(),
            charged_amount: v.charged.to_string(),
            refunded_amount: v.refunded.to_string(),
            canceled_amount: v.canceled.to_string(),
            available_actions: v.available_actions.clone(),
            psp_reference: v.psp_reference.clone().unwrap_or_default(),
        }
    }

    async fn gateway(
        &self,
        req: GatewayActionRequest,
        op: &'static str,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        let db = self.db()?;
        let id: i32 = req.transaction_id.parse().unwrap_or(-1);
        let amount: Decimal = req
            .amount
            .parse()
            .map_err(|_| Status::invalid_argument("amount must be a decimal string"))?;
        let out = match op {
            "authorize" => payments::authorize(db, id, amount, &req.idempotency_key).await,
            "charge" => payments::charge(db, id, amount, &req.idempotency_key).await,
            "refund" => payments::refund(db, id, amount, &req.idempotency_key).await,
            "cancel" => payments::cancel(db, id, amount, &req.idempotency_key).await,
            _ => return Err(Status::invalid_argument("unknown action")),
        };
        match out {
            Ok(v) => Ok(Response::new(GatewayActionResponse {
                transaction: Some(Self::view(&v)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(GatewayActionResponse {
                transaction: None,
                errors: vec![Self::err("REJECTED", e.to_string())],
            })),
        }
    }
}

#[tonic::async_trait]
impl PaymentService for PaymentServiceImpl {
    async fn create_transaction(
        &self,
        request: Request<CreateTransactionRequest>,
    ) -> Result<Response<CreateTransactionResponse>, Status> {
        let req = request.into_inner();
        let checkout_id = if req.checkout_id.is_empty() {
            None
        } else {
            req.checkout_id.parse::<Uuid>().ok()
        };
        let order_id = if req.order_id.is_empty() {
            None
        } else {
            req.order_id.parse::<Uuid>().ok()
        };
        if req.currency.len() != 3 {
            return Ok(Response::new(CreateTransactionResponse {
                transaction: None,
                errors: vec![Self::err("VALIDATION", "currency must be ISO-4217".into())],
            }));
        }
        match payments::create_transaction(
            self.db()?,
            &payments::NewTransaction {
                checkout_id,
                order_id,
                currency: req.currency,
                name: req.name,
                app_identifier: if req.app_identifier.is_empty() {
                    None
                } else {
                    Some(req.app_identifier)
                },
                idempotency_key: if req.idempotency_key.is_empty() {
                    None
                } else {
                    Some(req.idempotency_key)
                },
                available_actions: vec!["authorize".to_string()],
            },
        )
        .await
        {
            Ok(v) => Ok(Response::new(CreateTransactionResponse {
                transaction: Some(Self::view(&v)),
                errors: vec![],
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        let id: i32 = request.into_inner().id.parse().unwrap_or(-1);
        match payments::view(self.db()?, id).await {
            Ok(v) => Ok(Response::new(GetTransactionResponse {
                transaction: Some(Self::view(&v)),
                errors: vec![],
            })),
            Err(_) => Ok(Response::new(GetTransactionResponse {
                transaction: None,
                errors: vec![Self::err("NOT_FOUND", "transaction not found".into())],
            })),
        }
    }

    async fn authorize(
        &self,
        request: Request<GatewayActionRequest>,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        self.gateway(request.into_inner(), "authorize").await
    }

    async fn charge(
        &self,
        request: Request<GatewayActionRequest>,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        self.gateway(request.into_inner(), "charge").await
    }

    async fn refund(
        &self,
        request: Request<GatewayActionRequest>,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        self.gateway(request.into_inner(), "refund").await
    }

    async fn cancel(
        &self,
        request: Request<GatewayActionRequest>,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        self.gateway(request.into_inner(), "cancel").await
    }
}
