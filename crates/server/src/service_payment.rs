//! PaymentService: TransactionItem lifecycle over Django's tables +
//! PSP-driven gateway actions (authorize/charge/refund/cancel via the
//! configured PSP, async callbacks, 3DS challenges, adjustments).
//! Money RPCs demand MANAGE_PAYMENTS.

use rust_decimal::Decimal;
use rustygod_core::psp::{ChallengePsp, ManualPsp, Psp, PspAction, ScriptedPsp};
use rustygod_db::payments;
use rustygod_proto::payment::{
    payment_service_server::PaymentService, AdjustAuthorizationRequest, CreateTransactionRequest,
    CreateTransactionResponse, GatewayActionRequest, GatewayActionResponse, GetTransactionRequest,
    GetTransactionResponse, PspCallbackRequest, PspCallbackResponse, TransactionInfo,
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
        req: Request<GatewayActionRequest>,
        op: &'static str,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, req.metadata(), crate::access::MANAGE_PAYMENTS).await?;
        let req = req.into_inner();
        let id: i32 = req.transaction_id.parse().unwrap_or(-1);
        let amount: Decimal = req
            .amount
            .parse()
            .map_err(|_| Status::invalid_argument("amount must be a decimal string"))?;
        let action = match op {
            "authorize" => PspAction::Authorize,
            "charge" => PspAction::Charge,
            "refund" => PspAction::Refund,
            "cancel" => PspAction::Cancel,
            _ => return Err(Status::invalid_argument("unknown action")),
        };
        // PSP selector. async-sim is pending-by-default: settle via PspCallback.
        // stripe is real PaymentIntents HTTP (STRIPE_SECRET_KEY); without a
        // key it rejects cleanly instead of failing mid-flow.
        let domain = std::env::var("RUSTYGOD_DOMAIN").unwrap_or_else(|_| "localhost".into());
        let manual = ManualPsp;
        let challenge = ChallengePsp::new(domain);
        let async_sim = ScriptedPsp::pending("async-sim");
        let stripe = rustygod_psp::StripePsp::from_env();
        let psp: &dyn Psp = match req.gateway.as_str() {
            "" | "manual" => &manual,
            "challenge" => &challenge,
            "async-sim" => &async_sim,
            "stripe" => match stripe.as_ref() {
                Some(s) => s,
                None => {
                    return Ok(Response::new(GatewayActionResponse {
                        transaction: None,
                        action_required: false,
                        redirect_url: String::new(),
                        errors: vec![Self::err(
                            "REJECTED",
                            "stripe selected but STRIPE_SECRET_KEY is unset".to_string(),
                        )],
                    }))
                }
            },
            other => {
                return Ok(Response::new(GatewayActionResponse {
                    transaction: None,
                    action_required: false,
                    redirect_url: String::new(),
                    errors: vec![Self::err(
                        "REJECTED",
                        format!("unknown gateway {other:?}: want manual|challenge|async-sim|stripe"),
                    )],
                }))
            }
        };
        let ret = if req.return_url.is_empty() { None } else { Some(req.return_url.as_str()) };
        let data = if req.data.is_empty() { None } else { Some(req.data.as_str()) };
        match payments::execute_via(db, id, action, amount, &req.idempotency_key, psp, ret, data).await {
            Ok(out) => Ok(Response::new(GatewayActionResponse {
                transaction: Some(Self::view(&out.txn)),
                action_required: out.action_required,
                redirect_url: out.redirect_url.unwrap_or_default(),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(GatewayActionResponse {
                transaction: None,
                action_required: false,
                redirect_url: String::new(),
                errors: vec![Self::err("REJECTED", e.to_string())],
            })),
        }
    }

    async fn psp_callback(
        &self,
        request: Request<PspCallbackRequest>,
    ) -> Result<Response<PspCallbackResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_PAYMENTS).await?;
        let r = request.into_inner();
        let fail = |code: &str, message: String| {
            Response::new(PspCallbackResponse {
                transaction: None,
                replayed: false,
                errors: vec![Self::err(code, message)],
            })
        };
        let id: i32 = r.transaction_id.parse().unwrap_or(-1);
        let action = match r.action.as_str() {
            "authorize" => PspAction::Authorize,
            "charge" => PspAction::Charge,
            "refund" => PspAction::Refund,
            "cancel" => PspAction::Cancel,
            _ => return Ok(fail("INVALID", "action must be authorize|charge|refund|cancel".into())),
        };
        if r.psp_reference.is_empty() || r.idempotency_key.is_empty() {
            return Ok(fail("INVALID", "psp_reference and idempotency_key are required".into()));
        }
        match payments::psp_callback(
            db, id, action, &r.psp_reference, r.success, &r.message, &r.idempotency_key,
        )
        .await
        {
            Ok(out) => Ok(Response::new(PspCallbackResponse {
                transaction: Some(Self::view(&out.view)),
                replayed: out.replayed,
                errors: vec![],
            })),
            Err(e) => {
                let msg = e.to_string();
                let code = if msg.contains("UNKNOWN_REQUEST") {
                    "NOT_FOUND"
                } else if msg.contains("TERMINAL") {
                    "ALREADY_SETTLED"
                } else {
                    "REJECTED"
                };
                Ok(fail(code, msg))
            }
        }
    }

    async fn adjust_authorization(
        &self,
        request: Request<AdjustAuthorizationRequest>,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_PAYMENTS).await?;
        let r = request.into_inner();
        let id: i32 = r.transaction_id.parse().unwrap_or(-1);
        let amount: Decimal = r
            .amount
            .parse()
            .map_err(|_| Status::invalid_argument("amount must be a decimal string"))?;
        match payments::adjust_authorization(db, id, amount, &r.idempotency_key).await {
            Ok(v) => Ok(Response::new(GatewayActionResponse {
                transaction: Some(Self::view(&v)),
                action_required: false,
                redirect_url: String::new(),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(GatewayActionResponse {
                transaction: None,
                action_required: false,
                redirect_url: String::new(),
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
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_PAYMENTS).await?;
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
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_PAYMENTS).await?;
        let id: i32 = request.into_inner().id.parse().unwrap_or(-1);
        match payments::view(db, id).await {
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
        self.gateway(request, "authorize").await
    }

    async fn charge(
        &self,
        request: Request<GatewayActionRequest>,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        self.gateway(request, "charge").await
    }

    async fn refund(
        &self,
        request: Request<GatewayActionRequest>,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        self.gateway(request, "refund").await
    }

    async fn cancel(
        &self,
        request: Request<GatewayActionRequest>,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        self.gateway(request, "cancel").await
    }

    async fn psp_callback(
        &self,
        request: Request<PspCallbackRequest>,
    ) -> Result<Response<PspCallbackResponse>, Status> {
        Self::psp_callback(self, request).await
    }

    async fn adjust_authorization(
        &self,
        request: Request<AdjustAuthorizationRequest>,
    ) -> Result<Response<GatewayActionResponse>, Status> {
        Self::adjust_authorization(self, request).await
    }
}
