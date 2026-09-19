use rustygod_core::order::Order;
use rustygod_db::{fulfillment, granted_refunds, order_store};
use rustygod_proto::order::{
    order_service_server::OrderService, CancelFulfillmentRequest, CancelOrderRequest,
    CancelOrderResponse, CreateFulfillmentRequest, CreateGrantedRefundRequest,
    CreateGrantedRefundResponse, ExecuteGrantedRefundRequest, ExecuteGrantedRefundResponse,
    FulfillmentInfo, FulfillmentLineInfo, FulfillmentResponse, GetGrantedRefundRequest,
    GetGrantedRefundResponse, GetOrderRequest, GetOrderResponse, GrantedRefundLineInfo,
    ListFulfillmentsRequest, ListFulfillmentsResponse, ListOrdersRequest, ListOrdersResponse,
    ReconCheck, ReconcileOrderRequest, ReconcileOrderResponse, RefundFulfillmentRequest,
    ReturnOrderLinesRequest, ReturnOrderLinesResponse,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::store::SharedStore;

pub struct OrderServiceImpl {
    store: SharedStore,
    db: Option<DatabaseConnection>,
}

impl OrderServiceImpl {
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
}

#[tonic::async_trait]
impl OrderService for OrderServiceImpl {
    async fn get_order(
        &self,
        request: Request<GetOrderRequest>,
    ) -> Result<Response<GetOrderResponse>, Status> {
        let id = request.into_inner().id;
        if let Some(db) = &self.db {
            let Ok(oid) = id.parse::<Uuid>() else {
                return Ok(Response::new(GetOrderResponse {
                    order: None,
                    errors: vec![Self::err("NOT_FOUND", "order not found".into())],
                }));
            };
            match order_store::get_order_rows(db, oid)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
            {
                Some((header, lines)) => {
                    let domain = order_store::to_domain(&header, &lines, "default-channel");
                    return Ok(Response::new(GetOrderResponse {
                        order: Some(domain.to_proto()),
                        errors: vec![],
                    }));
                }
                None => {
                    return Ok(Response::new(GetOrderResponse {
                        order: None,
                        errors: vec![Self::err("NOT_FOUND", "order not found".into())],
                    }))
                }
            }
        }
        let store = crate::store::lock(&self.store)?;
        match store.orders.get(&id) {
            Some(o) => Ok(Response::new(GetOrderResponse {
                order: Some(o.to_proto()),
                errors: vec![],
            })),
            None => Ok(Response::new(GetOrderResponse {
                order: None,
                errors: vec![Self::err("NOT_FOUND", "order not found".into())],
            })),
        }
    }

    async fn list_orders(
        &self,
        request: Request<ListOrdersRequest>,
    ) -> Result<Response<ListOrdersResponse>, Status> {
        let req = request.into_inner();
        if let Some(db) = &self.db {
            use rustygod_db::entities::order_order;
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
            let first = if req.first <= 0 { 100 } else { (req.first as u64).min(1000) };
            let mut q = order_order::Entity::find()
                .select_only()
                .column(order_order::Column::Id)
                .order_by_desc(order_order::Column::Number)
                .limit(first);
            if !req.status.is_empty() {
                q = q.filter(order_order::Column::Status.eq(req.status.clone()));
            }
            let ids: Vec<Uuid> = q
                .into_tuple::<Uuid>()
                .all(db)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            let mut orders = Vec::with_capacity(ids.len());
            for oid in ids {
                if let Some((header, lines)) = order_store::get_order_rows(db, oid)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?
                {
                    orders.push(order_store::to_domain(&header, &lines, "default-channel").to_proto());
                }
            }
            return Ok(Response::new(ListOrdersResponse {
                orders,
                page_info: Some(rustygod_proto::common::PageInfo {
                    has_next_page: false,
                    end_cursor: String::new(),
                }),
            }));
        }
        let store = crate::store::lock(&self.store)?;
        let mut orders: Vec<_> = store
            .orders
            .values()
            .filter(|o| req.status.is_empty() || o.status.as_str() == req.status)
            .map(Order::to_proto)
            .collect();
        orders.sort_by(|a, b| a.number.cmp(&b.number));
        let first = if req.first <= 0 { 100 } else { (req.first as usize).min(1000) };
        orders.truncate(first);
        Ok(Response::new(ListOrdersResponse {
            orders,
            page_info: Some(rustygod_proto::common::PageInfo {
                has_next_page: false,
                end_cursor: String::new(),
            }),
        }))
    }

    async fn create_fulfillment(
        &self,
        request: Request<CreateFulfillmentRequest>,
    ) -> Result<Response<FulfillmentResponse>, Status> {
        let db = self.db()?;
        let req = request.into_inner();
        let Ok(order_id) = req.order_id.parse::<Uuid>() else {
            return Ok(Response::new(FulfillmentResponse {
                fulfillment: None,
                errors: vec![Self::err("NOT_FOUND", "order not found".into())],
            }));
        };
        let mut items = Vec::with_capacity(req.lines.len());
        for l in &req.lines {
            let Ok(line_id) = l.order_line_id.parse::<Uuid>() else {
                return Ok(Response::new(FulfillmentResponse {
                fulfillment: None,
                errors: vec![Self::err("NOT_FOUND", "order line not found".into())],
            }));
            };
            items.push(fulfillment::FulfillItem {
                order_line_id: line_id,
                quantity: l.quantity,
                stock_id: l.stock_id.parse::<i32>().ok(),
            });
        }
        match fulfillment::create_fulfillment(db, order_id, &items, &req.tracking_number).await {
            Ok(f) => Ok(Response::new(FulfillmentResponse {
                fulfillment: Some(fulfillment_to_proto(&f)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(FulfillmentResponse {
                fulfillment: None,
                errors: vec![Self::err("REJECTED", e.to_string())],
            })),
        }
    }

    async fn cancel_fulfillment(
        &self,
        request: Request<CancelFulfillmentRequest>,
    ) -> Result<Response<FulfillmentResponse>, Status> {
        let db = self.db()?;
        let id: i32 = request.into_inner().fulfillment_id.parse().unwrap_or(-1);
        match fulfillment::cancel_fulfillment(db, id).await {
            Ok(f) => Ok(Response::new(FulfillmentResponse {
                fulfillment: Some(fulfillment_to_proto(&f)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(FulfillmentResponse {
                fulfillment: None,
                errors: vec![Self::err("REJECTED", e.to_string())],
            })),
        }
    }

    async fn refund_fulfillment(
        &self,
        request: Request<RefundFulfillmentRequest>,
    ) -> Result<Response<FulfillmentResponse>, Status> {
        let db = self.db()?;
        let req = request.into_inner();
        let Ok(order_id) = req.order_id.parse::<Uuid>() else {
            return Ok(Response::new(FulfillmentResponse {
                fulfillment: None,
                errors: vec![Self::err("NOT_FOUND", "order not found".into())],
            }));
        };
        let mut items = Vec::with_capacity(req.lines.len());
        for l in &req.lines {
            let Ok(line_id) = l.order_line_id.parse::<Uuid>() else {
                return Ok(Response::new(FulfillmentResponse {
                fulfillment: None,
                errors: vec![Self::err("NOT_FOUND", "order line not found".into())],
            }));
            };
            items.push(fulfillment::FulfillItem {
                order_line_id: line_id,
                quantity: l.quantity,
                stock_id: l.stock_id.parse::<i32>().ok(),
            });
        }
        match fulfillment::refund_fulfillment(db, order_id, &items, &req.reason).await {
            Ok(f) => Ok(Response::new(FulfillmentResponse {
                fulfillment: Some(fulfillment_to_proto(&f)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(FulfillmentResponse {
                fulfillment: None,
                errors: vec![Self::err("REJECTED", e.to_string())],
            })),
        }
    }

    async fn list_fulfillments(
        &self,
        request: Request<ListFulfillmentsRequest>,
    ) -> Result<Response<ListFulfillmentsResponse>, Status> {        let db = self.db()?;
        let Ok(order_id) = request.into_inner().order_id.parse::<Uuid>() else {
            return Ok(Response::new(ListFulfillmentsResponse { fulfillments: vec![] }));
        };
        let list = fulfillment::list_fulfillments(db, order_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListFulfillmentsResponse {
            fulfillments: list.iter().map(fulfillment_to_proto).collect(),
        }))
    }

    async fn reconcile_order(
        &self,
        request: Request<ReconcileOrderRequest>,
    ) -> Result<Response<ReconcileOrderResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_ORDERS).await?;
        let Ok(order_id) = request.into_inner().id.parse::<Uuid>() else {
            return Ok(Response::new(ReconcileOrderResponse {
                checks: vec![],
                all_ok: false,
                errors: vec![rustygod_proto::common::Error {
                    code: "INVALID".into(),
                    message: "id must be a UUID".into(),
                    field: String::new(),
                }],
            }));
        };
        match rustygod_db::reconcile::reconcile_order(db, order_id).await {
            Ok(checks) => {
                let all_ok = checks.iter().all(|c| c.ok);
                Ok(Response::new(ReconcileOrderResponse {
                    checks: checks
                        .into_iter()
                        .map(|c| ReconCheck { name: c.name, ok: c.ok, detail: c.detail })
                        .collect(),
                    all_ok,
                    errors: vec![],
                }))
            }
            Err(e) => Ok(Response::new(ReconcileOrderResponse {
                checks: vec![],
                all_ok: false,
                errors: vec![rustygod_proto::common::Error {
                    code: "NOT_FOUND".into(),
                    message: e.to_string(),
                    field: String::new(),
                }],
            })),
        }
    }

    async fn cancel_order(
        &self,
        request: Request<CancelOrderRequest>,
    ) -> Result<Response<CancelOrderResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_ORDERS).await?;
        let Ok(order_id) = request.into_inner().id.parse::<Uuid>() else {
            return Ok(Response::new(CancelOrderResponse {
                status: String::new(),
                errors: vec![rustygod_proto::common::Error {
                    code: "INVALID".into(),
                    message: "id must be a UUID".into(),
                    field: String::new(),
                }],
                refunded_amount: String::new(),
                granted_refund_ids: vec![],
            }));
        };
        match rustygod_db::cancel::cancel_order(db, order_id).await {
            Ok(out) => {
                // Post-commit fast path, same as complete (best-effort).
                if !out.delivery_ids.is_empty() {
                    let dbc = db.clone();
                    let domain = std::env::var("RUSTYGOD_DOMAIN")
                        .unwrap_or_else(|_| "localhost".into());
                    let ids = out.delivery_ids.clone();
                    tokio::spawn(async move {
                        for id in ids {
                            let _ = crate::service_webhook::deliver(&dbc, &domain, id).await;
                        }
                    });
                }
                Ok(Response::new(CancelOrderResponse {
                    status: "canceled".into(),
                    errors: vec![],
                    refunded_amount: out.refunded.to_string(),
                    granted_refund_ids: out.grant_ids,
                }))
            }
            Err(e) => {
                let msg = e.to_string();
                let code = if msg.contains("not found") {
                    "NOT_FOUND"
                } else {
                    "NOT_APPLICABLE"
                };
                Ok(Response::new(CancelOrderResponse {
                    status: String::new(),
                    errors: vec![rustygod_proto::common::Error {
                        code: code.into(),
                        message: msg,
                        field: String::new(),
                    }],
                    refunded_amount: String::new(),
                    granted_refund_ids: vec![],
                }))
            }
        }
    }

    async fn return_order_lines(
        &self,
        request: Request<ReturnOrderLinesRequest>,
    ) -> Result<Response<ReturnOrderLinesResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_ORDERS).await?;
        let r = request.into_inner();
        let fail = |code: &str, message: String| {
            Response::new(ReturnOrderLinesResponse {
                fulfillment_id: 0,
                granted_refund_id: 0,
                amount: String::new(),
                errors: vec![rustygod_proto::common::Error {
                    code: code.into(),
                    message,
                    field: String::new(),
                }],
            })
        };
        let Ok(order_id) = r.order_id.parse::<Uuid>() else {
            return Ok(fail("INVALID", "order_id must be a UUID".into()));
        };
        let mut items = vec![];
        for l in &r.lines {
            let Ok(lid) = l.order_line_id.parse::<Uuid>() else {
                return Ok(fail("INVALID", "order_line_id must be a UUID".into()));
            };
            items.push(rustygod_db::fulfillment::FulfillItem {
                order_line_id: lid,
                quantity: l.quantity,
                stock_id: (l.stock_id != 0).then_some(l.stock_id),
            });
        }
        match rustygod_db::fulfillment::return_and_refund(
            db,
            order_id,
            &items,
            &r.reason,
            r.restock,
            (r.transaction_item_id != 0).then_some(r.transaction_item_id),
        )
        .await
        {
            Ok(out) => {
                // Post-commit fast path for order_updated/fulfillment_returned.
                Ok(Response::new(ReturnOrderLinesResponse {
                    fulfillment_id: out.fulfillment_id,
                    granted_refund_id: out.granted_refund_id,
                    amount: out.amount.to_string(),
                    errors: vec![],
                }))
            }
            Err(e) => {
                let msg = e.to_string();
                let code = if msg.contains("not found") {
                    "NOT_FOUND"
                } else {
                    "NOT_APPLICABLE"
                };
                Ok(fail(code, msg))
            }
        }
    }

    async fn create_granted_refund(
        &self,
        request: Request<CreateGrantedRefundRequest>,
    ) -> Result<Response<CreateGrantedRefundResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_ORDERS).await?;
        let r = request.into_inner();
        let fail = |code: &str, message: String| {
            Response::new(CreateGrantedRefundResponse {
                granted_refund_id: 0,
                amount: String::new(),
                status: String::new(),
                lines: vec![],
                errors: vec![rustygod_proto::common::Error {
                    code: code.into(),
                    message,
                    field: String::new(),
                }],
            })
        };
        let Ok(order_id) = r.order_id.parse::<Uuid>() else {
            return Ok(fail("INVALID", "order_id must be a UUID".into()));
        };
        let mut lines = vec![];
        for l in &r.lines {
            let Ok(lid) = l.order_line_id.parse::<Uuid>() else {
                return Ok(fail("INVALID", "order_line_id must be a UUID".into()));
            };
            lines.push(granted_refunds::GrantLineInput {
                order_line_id: lid,
                quantity: l.quantity,
            });
        }
        let amount = if r.amount.trim().is_empty() {
            None
        } else {
            match r.amount.parse::<rust_decimal::Decimal>() {
                Ok(a) => Some(a),
                Err(_) => return Ok(fail("INVALID", "amount must be a decimal".into())),
            }
        };
        match granted_refunds::create_granted_refund(
            db,
            &granted_refunds::NewGrant {
                order_id,
                transaction_item_id: (r.transaction_item_id != 0)
                    .then_some(r.transaction_item_id),
                amount,
                lines,
                reason: r.reason,
                shipping_costs_included: r.shipping_costs_included,
                user_id: None,
                app_id: None,
            },
        )
        .await
        {
            Ok(g) => Ok(Response::new(CreateGrantedRefundResponse {
                granted_refund_id: g.id,
                amount: g.amount.to_string(),
                status: g.status,
                lines: g
                    .lines
                    .into_iter()
                    .map(|l| GrantedRefundLineInfo {
                        order_line_id: l.order_line_id.to_string(),
                        quantity: l.quantity,
                    })
                    .collect(),
                errors: vec![],
            })),
            Err(e) => Ok(fail("NOT_APPLICABLE", e.to_string())),
        }
    }

    async fn execute_granted_refund(
        &self,
        request: Request<ExecuteGrantedRefundRequest>,
    ) -> Result<Response<ExecuteGrantedRefundResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_ORDERS).await?;
        let r = request.into_inner();
        let fail = |code: &str, message: String| {
            Response::new(ExecuteGrantedRefundResponse {
                status: String::new(),
                replayed: false,
                errors: vec![rustygod_proto::common::Error {
                    code: code.into(),
                    message,
                    field: String::new(),
                }],
            })
        };
        if r.idempotency_key.trim().is_empty() {
            return Ok(fail("INVALID", "idempotency_key is required".into()));
        }
        match granted_refunds::execute_granted_refund(db, r.granted_refund_id, &r.idempotency_key)
            .await
        {
            Ok(out) => {
                // Post-commit fast path, same contract as complete/cancel.
                if !out.delivery_ids.is_empty() {
                    let dbc = db.clone();
                    let domain = std::env::var("RUSTYGOD_DOMAIN")
                        .unwrap_or_else(|_| "localhost".into());
                    let ids = out.delivery_ids.clone();
                    tokio::spawn(async move {
                        for id in ids {
                            let _ = crate::service_webhook::deliver(&dbc, &domain, id).await;
                        }
                    });
                }
                Ok(Response::new(ExecuteGrantedRefundResponse {
                    status: out.view.status,
                    replayed: out.replayed,
                    errors: vec![],
                }))
            }
            Err(e) => {
                let msg = e.to_string();
                let code = if msg.contains("not found") {
                    "NOT_FOUND"
                } else {
                    "NOT_APPLICABLE"
                };
                Ok(fail(code, msg))
            }
        }
    }

    async fn get_granted_refund(
        &self,
        request: Request<GetGrantedRefundRequest>,
    ) -> Result<Response<GetGrantedRefundResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_ORDERS).await?;
        let id = request.into_inner().granted_refund_id;
        match granted_refunds::view(db, id).await {
            Ok(g) => Ok(Response::new(GetGrantedRefundResponse {
                granted_refund_id: g.id,
                order_id: g.order_id.to_string(),
                amount: g.amount.to_string(),
                status: g.status,
                reason: g.reason,
                lines: g
                    .lines
                    .into_iter()
                    .map(|l| GrantedRefundLineInfo {
                        order_line_id: l.order_line_id.to_string(),
                        quantity: l.quantity,
                    })
                    .collect(),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(GetGrantedRefundResponse {
                granted_refund_id: 0,
                order_id: String::new(),
                amount: String::new(),
                status: String::new(),
                reason: String::new(),
                lines: vec![],
                errors: vec![rustygod_proto::common::Error {
                    code: "NOT_FOUND".into(),
                    message: e.to_string(),
                    field: String::new(),
                }],
            })),
        }
    }
}

impl OrderServiceImpl {
    fn db(&self) -> Result<&DatabaseConnection, Status> {
        self.db
            .as_ref()
            .ok_or_else(|| Status::unavailable("postgres unavailable"))
    }
}

fn fulfillment_to_proto(f: &fulfillment::FulfillmentView) -> FulfillmentInfo {
    FulfillmentInfo {
        id: f.id.to_string(),
        order_id: f.order_id.to_string(),
        fulfillment_order: f.fulfillment_order,
        status: f.status.clone(),
        tracking_number: f.tracking_number.clone(),
        lines: f
            .lines
            .iter()
            .map(|l| FulfillmentLineInfo {
                order_line_id: l.order_line_id.to_string(),
                quantity: l.quantity,
                stock_id: l.stock_id.map(|s| s.to_string()).unwrap_or_default(),
            })
            .collect(),
    }
}
