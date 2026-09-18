//! WebhookService: trigger fan-out, signed HTTP delivery, attempts,
//! retry dues — over Django's webhook/core tables.

use rustygod_db::webhooks;
use rustygod_proto::webhook::{
    webhook_service_server::WebhookService, AttemptInfo, DeliveryInfo, DueDeliveriesRequest,
    DueDeliveriesResponse, GetDeliveryRequest, GetDeliveryResponse, ListAttemptsRequest,
    ListAttemptsResponse, SendDeliveryRequest, SendDeliveryResponse, TriggerEventRequest,
    TriggerEventResponse,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};

pub struct WebhookServiceImpl {
    db: Option<DatabaseConnection>,
    domain: String,
}

impl WebhookServiceImpl {
    pub fn new(db: Option<DatabaseConnection>) -> Self {
        Self {
            db,
            domain: std::env::var("RUSTYGOD_DOMAIN").unwrap_or_else(|_| "localhost".into()),
        }
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

    fn info(d: &webhooks::DeliveryView) -> DeliveryInfo {
        DeliveryInfo {
            id: d.id.to_string(),
            status: d.status.clone(),
            event_type: d.event_type.clone(),
            webhook_id: d.webhook_id.to_string(),
            target_url: d.target_url.clone(),
            payload: d.payload.clone(),
            attempts: d.attempts,
        }
    }
}

#[tonic::async_trait]
impl WebhookService for WebhookServiceImpl {
    async fn trigger_event(
        &self,
        request: Request<TriggerEventRequest>,
    ) -> Result<Response<TriggerEventResponse>, Status> {
        let req = request.into_inner();
        if req.event_type.is_empty() {
            return Ok(Response::new(TriggerEventResponse {
                delivery_ids: vec![],
                errors: vec![Self::err("VALIDATION", "event_type is required".into())],
            }));
        }
        // Validate payload is JSON (Django stores text; receivers parse JSON).
        if serde_json::from_str::<serde_json::Value>(&req.payload_json).is_err() {
            return Ok(Response::new(TriggerEventResponse {
                delivery_ids: vec![],
                errors: vec![Self::err("VALIDATION", "payload_json must be JSON".into())],
            }));
        }
        let channel = if req.channel.is_empty() { None } else { Some(req.channel.as_str()) };
        match webhooks::trigger_event(self.db()?, &req.event_type, channel, &req.payload_json).await
        {
            Ok(ids) => Ok(Response::new(TriggerEventResponse {
                delivery_ids: ids.into_iter().map(|i| i.to_string()).collect(),
                errors: vec![],
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_delivery(
        &self,
        request: Request<GetDeliveryRequest>,
    ) -> Result<Response<GetDeliveryResponse>, Status> {
        let id: i32 = request.into_inner().id.parse().unwrap_or(-1);
        match webhooks::view(self.db()?, id).await {
            Ok(d) => Ok(Response::new(GetDeliveryResponse {
                delivery: Some(Self::info(&d)),
                errors: vec![],
            })),
            Err(_) => Ok(Response::new(GetDeliveryResponse {
                delivery: None,
                errors: vec![Self::err("NOT_FOUND", "delivery not found".into())],
            })),
        }
    }

    async fn list_attempts(
        &self,
        request: Request<ListAttemptsRequest>,
    ) -> Result<Response<ListAttemptsResponse>, Status> {
        let id: i32 = request.into_inner().delivery_id.parse().unwrap_or(-1);
        let attempts = webhooks::list_attempts(self.db()?, id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListAttemptsResponse {
            attempts: attempts
                .into_iter()
                .map(|a| AttemptInfo {
                    id: a.id.to_string(),
                    status: a.status,
                    response_status_code: a.response_status_code.unwrap_or(-1) as i32,
                    duration_secs: a.duration_secs.unwrap_or(0.0),
                })
                .collect(),
        }))
    }

    async fn send_delivery(
        &self,
        request: Request<SendDeliveryRequest>,
    ) -> Result<Response<SendDeliveryResponse>, Status> {
        let db = self.db()?;
        let id: i32 = request.into_inner().delivery_id.parse().unwrap_or(-1);
        let delivery = webhooks::view(db, id).await.map_err(|_| {
            Status::not_found("delivery not found")
        })?;
        if delivery.status == "success" {
            // Idempotent: delivered stays delivered, re-record nothing.
            return Ok(Response::new(SendDeliveryResponse {
                delivery: Some(Self::info(&delivery)),
                errors: vec![],
            }));
        }
        let secret = webhooks::webhook_secret(db, delivery.webhook_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .unwrap_or_default();
        if secret.is_empty() {
            return Err(Status::internal("webhook has no secret"));
        }
        let headers = webhooks::signed_headers(&self.domain, &delivery.event_type, &delivery.payload, &secret);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| Status::internal(e.to_string()))?;
        let mut req = client.post(&delivery.target_url).body(delivery.payload.clone());
        for (k, v) in &headers {
            req = req.header(k, v);
        }
        let start = std::time::Instant::now();
        let resp = req.send().await;
        let duration = start.elapsed().as_secs_f64();
        let (code, body, resp_headers): (Option<i16>, String, String) = match resp {
            Ok(r) => {
                let code = r.status().as_u16() as i16;
                let headers = format!("{:?}", r.headers());
                let body: String = r.text().await.unwrap_or_default().chars().take(4000).collect();
                (Some(code), body, headers)
            }
            Err(e) => (None, e.to_string().chars().take(4000).collect(), String::new()),
        };
        let req_headers = format!("{headers:?}");
        let updated = webhooks::record_attempt(
            db,
            id,
            code,
            Some(duration),
            &req_headers,
            &body,
            &resp_headers,
        )
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(SendDeliveryResponse {
            delivery: Some(Self::info(&updated)),
            errors: vec![],
        }))
    }

    async fn due_deliveries(
        &self,
        request: Request<DueDeliveriesRequest>,
    ) -> Result<Response<DueDeliveriesResponse>, Status> {
        let first = request.into_inner().first;
        let first = if first <= 0 { 100 } else { (first as u64).min(1000) };
        let ids = webhooks::due_deliveries(self.db()?, first)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(DueDeliveriesResponse {
            delivery_ids: ids.into_iter().map(|i| i.to_string()).collect(),
        }))
    }
}
