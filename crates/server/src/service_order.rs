use rustygod_core::order::Order;
use rustygod_db::{fulfillment, order_store};
use rustygod_proto::order::{
    order_service_server::OrderService, CancelFulfillmentRequest, CreateFulfillmentRequest,
    FulfillmentInfo, FulfillmentLineInfo, FulfillmentResponse, GetOrderRequest, GetOrderResponse,
    ListFulfillmentsRequest, ListFulfillmentsResponse, ListOrdersRequest, ListOrdersResponse,
    ReconCheck, ReconcileOrderRequest, ReconcileOrderResponse, RefundFulfillmentRequest,
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
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, SelectorTrait};
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
