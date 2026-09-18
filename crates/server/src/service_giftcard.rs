//! GiftCardService: issue/attach/redeem over Django's `giftcard_*` tables.

use rust_decimal::Decimal;
use rustygod_db::{
    entities::giftcard_giftcard,
    giftcards::{self, IssueInput},
};
use rustygod_proto::giftcard::{
    gift_card_service_server::GiftCardService, BalanceMutationRequest, BalanceMutationResponse,
    CheckoutBalanceRequest, CheckoutBalanceResponse, CheckoutGiftCardRequest,
    CheckoutGiftCardResponse, GetGiftCardRequest, GetGiftCardResponse, GiftCardInfo,
    IssueGiftCardRequest, IssueGiftCardResponse, RedeemGiftCardRequest, RedeemGiftCardResponse,
    SetActiveRequest, SetActiveResponse,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub struct GiftCardServiceImpl {
    db: Option<DatabaseConnection>,
}

impl GiftCardServiceImpl {
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

    fn info(c: &giftcard_giftcard::Model) -> GiftCardInfo {
        GiftCardInfo {
            code: c.code.clone(),
            currency: c.currency.clone(),
            initial_balance: c.initial_balance_amount.to_string(),
            current_balance: c.current_balance_amount.to_string(),
            is_active: c.is_active,
            expiry_date: c.expiry_date.map(|d| d.to_string()).unwrap_or_default(),
            created_by_email: c.created_by_email.clone().unwrap_or_default(),
        }
    }

    fn dec(s: &str) -> Result<Decimal, Status> {
        s.parse()
            .map_err(|_| Status::invalid_argument("amount must be a decimal string"))
    }

    fn token(s: &str) -> Result<Uuid, Status> {
        s.parse()
            .map_err(|_| Status::invalid_argument("checkout_id must be a UUID"))
    }
}

fn db_err(e: rustygod_db::DbError) -> rustygod_proto::common::Error {
    use rustygod_db::DbError as E;
    let code = match e {
        E::GiftCardNotFound(_) => "NOT_FOUND",
        E::GiftCardNotApplicable(_) => "NOT_APPLICABLE",
        E::GiftCardConflict(_) => "CONFLICT",
        _ => "DB_ERROR",
    };
    GiftCardServiceImpl::err(code, e.to_string())
}

#[tonic::async_trait]
impl GiftCardService for GiftCardServiceImpl {
    async fn issue(
        &self,
        req: Request<IssueGiftCardRequest>,
    ) -> Result<Response<IssueGiftCardResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        let balance = Self::dec(&r.initial_balance)?;
        let expiry = if r.expiry_date.is_empty() {
            None
        } else {
            Some(
                r.expiry_date
                    .parse()
                    .map_err(|_| Status::invalid_argument("expiry_date must be YYYY-MM-DD"))?,
            )
        };
        match giftcards::issue(
            db,
            IssueInput {
                initial_balance: balance,
                currency: r.currency,
                created_by_email: (!r.created_by_email.is_empty()).then_some(r.created_by_email),
                expiry_date: expiry,
                is_active: true,
            },
            None,
        )
        .await
        {
            Ok(c) => Ok(Response::new(IssueGiftCardResponse {
                gift_card: Some(Self::info(&c)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(IssueGiftCardResponse {
                gift_card: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn get_by_code(
        &self,
        req: Request<GetGiftCardRequest>,
    ) -> Result<Response<GetGiftCardResponse>, Status> {
        let db = self.db()?;
        match giftcards::get_by_code(db, &req.into_inner().code).await {
            Ok(Some(c)) => Ok(Response::new(GetGiftCardResponse {
                gift_card: Some(Self::info(&c)),
                errors: vec![],
            })),
            Ok(None) => Ok(Response::new(GetGiftCardResponse {
                gift_card: None,
                errors: vec![Self::err("NOT_FOUND", "gift card not found".into())],
            })),
            Err(e) => Ok(Response::new(GetGiftCardResponse {
                gift_card: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn attach_to_checkout(
        &self,
        req: Request<CheckoutGiftCardRequest>,
    ) -> Result<Response<CheckoutGiftCardResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        let token = Self::token(&r.checkout_id)?;
        // Currency comes from the checkout row itself.
        let currency = rustygod_db::checkout_store::load_checkout(db, token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .map(|(co, _)| co.currency)
            .unwrap_or_default();
        match giftcards::attach_to_checkout(db, token, &r.code, &currency, None).await {
            Ok(c) => Ok(Response::new(CheckoutGiftCardResponse {
                gift_card: Some(Self::info(&c)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(CheckoutGiftCardResponse {
                gift_card: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn detach_from_checkout(
        &self,
        req: Request<CheckoutGiftCardRequest>,
    ) -> Result<Response<CheckoutGiftCardResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        let token = Self::token(&r.checkout_id)?;
        match giftcards::detach_from_checkout(db, token, &r.code).await {
            Ok(()) => Ok(Response::new(CheckoutGiftCardResponse {
                gift_card: None,
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(CheckoutGiftCardResponse {
                gift_card: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn checkout_balance(
        &self,
        req: Request<CheckoutBalanceRequest>,
    ) -> Result<Response<CheckoutBalanceResponse>, Status> {
        let db = self.db()?;
        let token = Self::token(&req.into_inner().checkout_id)?;
        let currency = rustygod_db::checkout_store::load_checkout(db, token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .map(|(co, _)| co.currency)
            .unwrap_or_default();
        match giftcards::checkout_balance(db, token, &currency).await {
            Ok(b) => Ok(Response::new(CheckoutBalanceResponse {
                currency,
                balance: b.to_string(),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(CheckoutBalanceResponse {
                currency: String::new(),
                balance: String::new(),
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn redeem(
        &self,
        req: Request<RedeemGiftCardRequest>,
    ) -> Result<Response<RedeemGiftCardResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        let amount = Self::dec(&r.amount)?;
        let order_id = if r.order_id.is_empty() {
            None
        } else {
            Some(
                r.order_id
                    .parse()
                    .map_err(|_| Status::invalid_argument("order_id must be a UUID"))?,
            )
        };
        match giftcards::redeem_for_order(db, &r.code, order_id, amount, None).await {
            Ok(taken) => {
                let card = giftcards::get_by_code(db, &r.code)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
                Ok(Response::new(RedeemGiftCardResponse {
                    amount_taken: taken.to_string(),
                    gift_card: card.map(|c| Self::info(&c)),
                    errors: vec![],
                }))
            }
            Err(e) => Ok(Response::new(RedeemGiftCardResponse {
                amount_taken: String::new(),
                gift_card: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn adjust_balance(
        &self,
        req: Request<BalanceMutationRequest>,
    ) -> Result<Response<BalanceMutationResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        let balance = Self::dec(&r.amount)?;
        match giftcards::adjust_balance(db, &r.code, balance, None).await {
            Ok(c) => Ok(Response::new(BalanceMutationResponse {
                gift_card: Some(Self::info(&c)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(BalanceMutationResponse {
                gift_card: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn refund(
        &self,
        req: Request<BalanceMutationRequest>,
    ) -> Result<Response<BalanceMutationResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        let amount = Self::dec(&r.amount)?;
        let order_id = if r.order_id.is_empty() {
            None
        } else {
            Some(
                r.order_id
                    .parse()
                    .map_err(|_| Status::invalid_argument("order_id must be a UUID"))?,
            )
        };
        match giftcards::refund_to_card(db, &r.code, amount, order_id).await {
            Ok(c) => Ok(Response::new(BalanceMutationResponse {
                gift_card: Some(Self::info(&c)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(BalanceMutationResponse {
                gift_card: None,
                errors: vec![db_err(e)],
            })),
        }
    }

    async fn set_active(
        &self,
        req: Request<SetActiveRequest>,
    ) -> Result<Response<SetActiveResponse>, Status> {
        let db = self.db()?;
        let r = req.into_inner();
        match giftcards::set_active(db, &r.code, r.active, None).await {
            Ok(c) => Ok(Response::new(SetActiveResponse {
                gift_card: Some(Self::info(&c)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(SetActiveResponse {
                gift_card: None,
                errors: vec![db_err(e)],
            })),
        }
    }
}
