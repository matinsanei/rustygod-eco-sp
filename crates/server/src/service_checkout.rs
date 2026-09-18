use rustygod_core::order::Order;
use rustygod_db::{catalog, checkout_store};
use rustygod_proto::checkout::{
    checkout_service_server::CheckoutService, AddLinesRequest, AddLinesResponse,
    ApplyVoucherRequest, ApplyVoucherResponse, CompleteCheckoutRequest, CompleteCheckoutResponse,
    CreateCheckoutRequest, CreateCheckoutResponse, GetCheckoutRequest, GetCheckoutResponse,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::store::{lock, SharedStore};

pub struct CheckoutServiceImpl {
    store: SharedStore,
    db: Option<DatabaseConnection>,
}

impl CheckoutServiceImpl {
    pub fn new(store: SharedStore) -> Self {
        Self { store, db: None }
    }

    pub fn with_db(store: SharedStore, db: DatabaseConnection) -> Self {
        Self {
            store,
            db: Some(db),
        }
    }

    fn err(code: &str, message: String) -> rustygod_proto::common::Error {
        rustygod_proto::common::Error {
            code: code.into(),
            message,
            field: String::new(),
        }
    }

    fn db(&self) -> Result<&DatabaseConnection, Status> {
        self.db
            .as_ref()
            .ok_or_else(|| Status::unavailable("postgres unavailable"))
    }
}

fn parse_token(id: &str) -> Option<Uuid> {
    id.parse::<Uuid>().ok()
}

fn not_found_checkout() -> AddLinesResponse {
    AddLinesResponse {
        checkout: None,
        errors: vec![CheckoutServiceImpl::err("NOT_FOUND", "checkout not found".into())],
    }
}

/// Build the wire Checkout with the **discounted** total from the refreshed
/// Django row (Django's `checkout.total`), not the undiscounted line sum.
fn checkout_proto(
    co: &rustygod_db::entities::checkout_checkout::Model,
    lines: &[rustygod_db::entities::checkout_checkoutline::Model],
    channel: &str,
) -> rustygod_proto::checkout::Checkout {
    let domain = checkout_store::to_domain(co, lines, channel);
    let mut proto = domain.to_proto();
    proto.total = Some(
        rustygod_core::money::Money::new(co.total_gross_amount, co.currency.clone()).into(),
    );
    proto
}

#[tonic::async_trait]
impl CheckoutService for CheckoutServiceImpl {
    async fn create_checkout(
        &self,
        request: Request<CreateCheckoutRequest>,
    ) -> Result<Response<CreateCheckoutResponse>, Status> {
        let req = request.into_inner();
        let channel = if req.channel.is_empty() {
            "default-channel".to_string()
        } else {
            req.channel
        };
        if self.db.is_some() {
            let db = self.db()?;
            // Persisted on Django's `checkout_checkout` — Django reads the same row.
            let (ch_id, currency) = catalog::channel_info(db, &channel)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            let token = checkout_store::create_checkout_row(db, ch_id, &currency, &req.email)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            let (co, lines) = checkout_store::load_checkout(db, token)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::internal("just-inserted checkout missing"))?;
            let proto = checkout_proto(&co, &lines, &channel);
            return Ok(Response::new(CreateCheckoutResponse {
                checkout: Some(proto),
                errors: vec![],
            }));
        }
        // Offline memory mode.
        let checkout = rustygod_core::checkout::Checkout::new(channel, req.email, "USD");
        let proto = checkout.to_proto();
        lock(&self.store)?
            .checkouts
            .insert(checkout.id.clone(), checkout);
        Ok(Response::new(CreateCheckoutResponse {
            checkout: Some(proto),
            errors: vec![],
        }))
    }

    async fn get_checkout(
        &self,
        request: Request<GetCheckoutRequest>,
    ) -> Result<Response<GetCheckoutResponse>, Status> {
        let id = request.into_inner().id;
        if self.db.is_some() {
            let db = self.db()?;
            let Some(token) = parse_token(&id) else {
                return Ok(Response::new(GetCheckoutResponse {
                    checkout: None,
                    errors: vec![Self::err("NOT_FOUND", "checkout not found".into())],
                }));
            };
            // Channel slug is stored only by id; default-channel covers v1.
            // (Multi-channel slug round-trip lands with the channel-cache milestone.)
            match checkout_store::load_checkout(db, token)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
            {
                Some((co, lines)) => {
                    let ch_slug = channel_slug_for(&co.channel_id);
                    let proto = checkout_proto(&co, &lines, &ch_slug);
                    return Ok(Response::new(GetCheckoutResponse {
                        checkout: Some(proto),
                        errors: vec![],
                    }));
                }
                None => {
                    return Ok(Response::new(GetCheckoutResponse {
                        checkout: None,
                        errors: vec![Self::err("NOT_FOUND", "checkout not found".into())],
                    }))
                }
            }
        }
        match lock(&self.store)?.checkouts.get(&id) {
            Some(c) => Ok(Response::new(GetCheckoutResponse {
                checkout: Some(c.to_proto()),
                errors: vec![],
            })),
            None => Ok(Response::new(GetCheckoutResponse {
                checkout: None,
                errors: vec![Self::err("NOT_FOUND", "checkout not found".into())],
            })),
        }
    }

    async fn add_lines(
        &self,
        request: Request<AddLinesRequest>,
    ) -> Result<Response<AddLinesResponse>, Status> {
        let req = request.into_inner();
        if self.db.is_some() {
            let db = self.db()?;
            let Some(token) = parse_token(&req.checkout_id) else {
                return Ok(Response::new(not_found_checkout()));
            };
            let Some((co, _)) = checkout_store::load_checkout(db, token)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
            else {
                return Ok(Response::new(not_found_checkout()));
            };
            let ch_slug = channel_slug_for(&co.channel_id);
            let ids: Vec<i32> = req
                .lines
                .iter()
                .filter_map(|l| l.variant_id.parse::<i32>().ok())
                .collect();
            let pricing = catalog::checkout_pricing(db, &ch_slug, &ids)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            let mut items = Vec::with_capacity(req.lines.len());
            for line in &req.lines {
                let vid = line.variant_id.parse::<i32>().ok();
                let price = vid.and_then(|id| pricing.get(&id));
                let Some((unit, _)) = price else {
                    return Ok(Response::new(AddLinesResponse {
                        checkout: None,
                        errors: vec![Self::err(
                            "NOT_FOUND",
                            format!("variant {} not found", line.variant_id),
                        )],
                    }));
                };
                if unit.currency != co.currency {
                    return Ok(Response::new(AddLinesResponse {
                        checkout: None,
                        errors: vec![Self::err(
                            "VALIDATION",
                            "line currency must match checkout currency".into(),
                        )],
                    }));
                }
                if line.quantity < 1 {
                    return Ok(Response::new(AddLinesResponse {
                        checkout: None,
                        errors: vec![Self::err(
                            "VALIDATION",
                            "quantity must be positive".into(),
                        )],
                    }));
                };
                let Some(v) = vid else {
                    return Ok(Response::new(AddLinesResponse {
                        checkout: None,
                        errors: vec![Self::err(
                            "NOT_FOUND",
                            format!("variant {} not found", line.variant_id),
                        )],
                    }));
                };
                items.push(checkout_store::NewLine {
                    variant_id: v,
                    quantity: line.quantity,
                    unit_price: unit.amount,
                    price_override: None,
                });
            }
            // One transaction: merge + insert + totals refresh.
            checkout_store::add_lines_tx(db, token, co.channel_id, &co.currency, &items)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            let (co, lines) = checkout_store::load_checkout(db, token)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::internal("checkout vanished mid-request"))?;
            let proto = checkout_proto(&co, &lines, &ch_slug);
            return Ok(Response::new(AddLinesResponse {
                checkout: Some(proto),
                errors: vec![],
            }));
        }
        // Offline memory mode.
        let prices: Vec<(String, rustygod_core::money::Money)> = {
            let store = lock(&self.store)?;
            let mut out = Vec::new();
            for line in &req.lines {
                match store
                    .products
                    .values()
                    .flat_map(|p| &p.variants)
                    .find(|v| v.id == line.variant_id)
                {
                    Some(v) => out.push((line.variant_id.clone(), v.price.clone())),
                    None => {
                        return Ok(Response::new(AddLinesResponse {
                            checkout: None,
                            errors: vec![Self::err(
                                "NOT_FOUND",
                                format!("variant {} not found", line.variant_id),
                            )],
                        }))
                    }
                }
            }
            out
        };
        let mut store = lock(&self.store)?;
        let checkout = match store.checkouts.get_mut(&req.checkout_id) {
            Some(c) => c,
            None => return Ok(Response::new(not_found_checkout())),
        };
        for (i, line) in req.lines.iter().enumerate() {
            if let Err(e) =
                checkout.add_line(line.variant_id.clone(), line.quantity, prices[i].1.clone())
            {
                return Ok(Response::new(AddLinesResponse {
                    checkout: None,
                    errors: vec![Self::err("VALIDATION", e.to_string())],
                }));
            }
        }
        let proto = checkout.to_proto();
        Ok(Response::new(AddLinesResponse {
            checkout: Some(proto),
            errors: vec![],
        }))
    }

    async fn apply_voucher(
        &self,
        request: Request<ApplyVoucherRequest>,
    ) -> Result<Response<ApplyVoucherResponse>, Status> {
        let req = request.into_inner();
        let db = self.db()?;
        let Some(token) = parse_token(&req.checkout_id) else {
            return Ok(Response::new(ApplyVoucherResponse {
                checkout: None,
                voucher_type: String::new(),
                value_type: String::new(),
                value: String::new(),
                currency: String::new(),
                amount: String::new(),
                errors: vec![Self::err("NOT_FOUND", "checkout not found".into())],
            }));
        };
        use sea_orm::TransactionTrait;
        // Validate + write discount rows + refresh totals atomically.
        let txn = db.begin().await.map_err(|e| Status::internal(e.to_string()))?;
        let applied = rustygod_db::promotions::apply_voucher(&txn, token, &req.code, "default-channel")
            .await
            .map_err(|e| Status::internal(e.to_string()));
        let applied = match applied {
            Ok(a) => a,
            Err(status) => {
                txn.rollback().await.map_err(|e| Status::internal(e.to_string()))?;
                // Domain failures surface as clean client errors, not transport errors.
                return Ok(Response::new(ApplyVoucherResponse {
                    checkout: None,
                    voucher_type: String::new(),
                    value_type: String::new(),
                    value: String::new(),
                    currency: String::new(),
                    amount: String::new(),
                    errors: vec![Self::err("NOT_APPLICABLE", status.to_string())],
                }));
            }
        };
        checkout_store::refresh_totals(&txn, token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        txn.commit().await.map_err(|e| Status::internal(e.to_string()))?;
        let (co, lines) = checkout_store::load_checkout(db, token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::internal("checkout vanished mid-request"))?;
        let proto = checkout_proto(&co, &lines, "default-channel");
        Ok(Response::new(ApplyVoucherResponse {
            checkout: Some(proto),
            voucher_type: applied.voucher_type,
            value_type: applied.value_type,
            value: applied.value.to_string(),
            currency: applied.currency,
            amount: applied.amount.to_string(),
            errors: vec![],
        }))
    }

    async fn complete_checkout(
        &self,
        request: Request<CompleteCheckoutRequest>,
    ) -> Result<Response<CompleteCheckoutResponse>, Status> {
        let req = request.into_inner();
        if self.db.is_some() {
            use sea_orm::TransactionTrait;
            let db = self.db()?;
            let Some(token) = parse_token(&req.checkout_id) else {
                return Ok(Response::new(CompleteCheckoutResponse {
                    order_id: String::new(),
                    errors: vec![Self::err("NOT_FOUND", "checkout not found".into())],
                }));
            };
            // Whole completion is ONE transaction: mint order + delete checkout
            // commit together, or neither does (no orphan orders, no lost checkouts).
            let txn = db.begin().await.map_err(|e| Status::internal(e.to_string()))?;
            enum Fail {
                Client(&'static str, String),
                Internal(Status),
            }
            let result: Result<String, Fail> = async {
                let Some((co, lines)) = checkout_store::load_checkout(&txn, token)
                    .await
                    .map_err(|e| Fail::Internal(Status::internal(e.to_string())))?
                else {
                    return Err(Fail::Client("NOT_FOUND", "checkout not found".into()));
                };
                if lines.is_empty() {
                    return Err(Fail::Client("EMPTY_CHECKOUT", "checkout has no lines".into()));
                }
                let ch_slug = channel_slug_for(&co.channel_id);
                let domain = checkout_store::to_domain(&co, &lines, &ch_slug);
                let ids: Vec<i32> = lines.iter().map(|l| l.variant_id).collect();
                let pricing = catalog::checkout_pricing(&txn, &ch_slug, &ids)
                    .await
                    .map_err(|e| Fail::Internal(Status::internal(e.to_string())))?;
                let order = rustygod_db::order_store::mint_from_checkout(
                    &txn,
                    &domain,
                    co.channel_id,
                    &ch_slug,
                    &pricing,
                )
                .await
                .map_err(|e| Fail::Internal(Status::internal(e.to_string())))?;
                let order_id = order.id.clone();
                // Voucher usage increments with completion (Django:
                // increase_voucher_usage), inside the same transaction.
                if let Some(code) = co.voucher_code.clone() {
                    let email = if co.email.clone().unwrap_or_default().is_empty() {
                        None
                    } else {
                        co.email.clone()
                    };
                    rustygod_db::promotions::increase_usage(
                        &txn,
                        &code,
                        email.as_deref(),
                    )
                    .await
                    .map_err(|e| Fail::Internal(Status::internal(e.to_string())))?;
                }
                checkout_store::delete_checkout_row(&txn, token)
                    .await
                    .map_err(|e| Fail::Internal(Status::internal(e.to_string())))?;
                Ok(order_id)
            }
            .await;
            match result {
                Ok(order_id) => {
                    txn.commit().await.map_err(|e| Status::internal(e.to_string()))?;
                    return Ok(Response::new(CompleteCheckoutResponse {
                        order_id,
                        errors: vec![],
                    }));
                }
                Err(Fail::Client(code, message)) => {
                    txn.rollback().await.map_err(|e| Status::internal(e.to_string()))?;
                    return Ok(Response::new(CompleteCheckoutResponse {
                        order_id: String::new(),
                        errors: vec![Self::err(code, message)],
                    }));
                }
                Err(Fail::Internal(status)) => {
                    txn.rollback().await.map_err(|e| Status::internal(e.to_string()))?;
                    return Err(status);
                }
            }
        }
        // Offline memory mode (unchanged semantics, panic-free locks).
        let (checkout, seq) = {
            let mut store = lock(&self.store)?;
            let checkout = match store.checkouts.remove(&req.checkout_id) {
                Some(c) => c,
                None => {
                    return Ok(Response::new(CompleteCheckoutResponse {
                        order_id: String::new(),
                        errors: vec![Self::err("NOT_FOUND", "checkout not found".into())],
                    }))
                }
            };
            if checkout.lines.is_empty() {
                return Ok(Response::new(CompleteCheckoutResponse {
                    order_id: String::new(),
                    errors: vec![Self::err("EMPTY_CHECKOUT", "checkout has no lines".into())],
                }));
            }
            store.order_seq += 1;
            (checkout, store.order_seq)
        };
        let mem_names: std::collections::HashMap<String, String> =
            lock(&self.store)?.products.values().flat_map(|p| {
                p.variants
                    .iter()
                    .map(|v| (v.id.clone(), format!("{} ({})", p.name, v.name)))
            }).collect();
        let order = Order::from_checkout(
            &checkout,
            |vid| {
                mem_names
                    .get(vid)
                    .cloned()
                    .unwrap_or_else(|| vid.to_string())
            },
            || format!("RG-{seq:06}"),
        );
        let order_id = order.id.clone();
        lock(&self.store)?.orders.insert(order_id.clone(), order);
        Ok(Response::new(CompleteCheckoutResponse {
            order_id,
            errors: vec![],
        }))
    }
}

/// v1 channel mapping: ids 1/2 are the populatedb channels.
/// (Full channel-slug round-trip lands with the channel-cache milestone.)
fn channel_slug_for(channel_id: &i32) -> String {
    match channel_id {
        2 => "channel-pln".to_string(),
        _ => "default-channel".to_string(),
    }
}
