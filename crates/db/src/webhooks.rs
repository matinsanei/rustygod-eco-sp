//! Webhook dispatch over Django's tables
//! (`webhook_webhook`, `webhook_webhookevent`, `core_eventpayload`,
//! `core_eventdelivery`, `core_eventdeliveryattempt`).
//!
//! Mirrors `saleor/webhook/` + `saleor/core/models.py`:
//! - trigger fans out to active webhooks subscribed to the event type,
//!   honoring `filterable_channel_slugs` (empty = all channels);
//! - HMAC-SHA256 signature per delivery (secret required; Django falls back
//!   to JWS without one, Rust refuses — secrets are mandatory here);
//! - attempts record request/response like Django's transport, delivery goes
//!   success on 2xx, failed after `MAX_DELIVERY_RETRIES` attempts, with
//!   `backoff * 2^n` retry dues queryable via `due_deliveries`.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::{
    entities::{
        core_eventdelivery, core_eventdeliveryattempt, core_eventpayload, webhook_webhook,
        webhook_webhookevent,
    },
    DbError, Result,
};

pub struct DeliveryView {
    pub id: i32,
    pub status: String,
    pub event_type: String,
    pub webhook_id: i32,
    pub target_url: String,
    pub payload: String,
    pub attempts: u64,
}

pub struct AttemptView {
    pub id: i32,
    pub status: String,
    pub response_status_code: Option<i16>,
    pub duration_secs: Option<f64>,
}

/// Fan-out: create one pending delivery per matching webhook.
/// Returns delivery ids. Entirely transactional.
pub async fn trigger_event(
    db: &DatabaseConnection,
    event_type: &str,
    channel_slug: Option<&str>,
    payload_json: &str,
) -> Result<Vec<i32>> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let out = trigger_event_tx(&txn, event_type, channel_slug, payload_json).await?;
    txn.commit().await?;
    Ok(out)
}

/// Transactional core of [`trigger_event`]: the OUTBOX write. Call this
/// INSIDE the business transaction (checkout complete does) so deliveries
/// are atomic with money: crash before commit → nothing; crash after →
/// rows stay `pending` for the sweeper / retry worker (R8). Never send
/// HTTP inside the transaction.
pub async fn trigger_event_tx(
    txn: &impl ConnectionTrait,
    event_type: &str,
    channel_slug: Option<&str>,
    payload_json: &str,
) -> Result<Vec<i32>> {
    if event_type.is_empty() {
        return Err(DbError::SeaOrm(sea_orm::DbErr::Custom(
            "event_type is required".into(),
        )));
    }
    let webhook_ids: Vec<i32> = webhook_webhookevent::Entity::find()
        .select_only()
        .column(webhook_webhookevent::Column::WebhookId)
        .filter(webhook_webhookevent::Column::EventType.eq(event_type))
        .into_tuple::<i32>()
        .all(txn)
        .await?;
    let payload = core_eventpayload::ActiveModel {
        payload: Set(payload_json.to_string()),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    let mut out = Vec::new();
    for wid in webhook_ids {
        let Some(wh) = webhook_webhook::Entity::find_by_id(wid).one(txn).await? else {
            continue;
        };
        if !wh.is_active {
            continue;
        }
        // Channel filter: empty list = all channels (Django semantics).
        if let Some(ch) = channel_slug {
            if !wh.filterable_channel_slugs.is_empty()
                && !wh.filterable_channel_slugs.iter().any(|s| s == ch)
            {
                continue;
            }
        }
        if wh.secret_key.as_deref().unwrap_or("").is_empty() {
            continue;
        }
        let d = core_eventdelivery::ActiveModel {
            created_at: Set(Utc::now().into()),
            status: Set("pending".to_string()),
            event_type: Set(event_type.to_string()),
            payload_id: Set(Some(payload.id)),
            webhook_id: Set(wid),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        out.push(d.id);
    }
    Ok(out)
}

/// Claim one pending delivery for sending: flips `pending` → `sending`
/// atomically and returns true only to the winner. Multi-instance sweepers
/// race here; `FOR UPDATE SKIP LOCKED` semantics via the status predicate —
/// exactly one sender proceeds, the rest see false and move on.
pub async fn claim_delivery(
    db: &DatabaseConnection,
    delivery_id: i32,
) -> Result<bool> {
    let res = db
        .execute(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "UPDATE core_eventdelivery SET status = 'sending' \
             WHERE id = $1 AND status = 'pending'"
                .to_string(),
            [delivery_id.into()],
        ))
        .await?;
    Ok(res.rows_affected() == 1)
}
/// Record an attempt and roll the delivery status forward, mirroring
/// Django's transport outcome handling:
///
/// - 2xx → attempt + delivery `success`;
/// - else → attempt `failed`; delivery stays `pending` until
///   `MAX_DELIVERY_RETRIES` attempts, then `failed`.
pub async fn record_attempt(
    db: &DatabaseConnection,
    delivery_id: i32,
    status_code: Option<i16>,
    duration_secs: Option<f64>,
    request_headers: &str,
    response_body: &str,
    response_headers: &str,
) -> Result<DeliveryView> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let ok = matches!(status_code, Some(200..=299));
    core_eventdeliveryattempt::ActiveModel {
        created_at: Set(Utc::now().into()),
        delivery_id: Set(Some(delivery_id)),
        status: Set(if ok { "success".into() } else { "failed".into() }),
        response_status_code: Set(status_code),
        duration: Set(duration_secs),
        request_headers: Set(Some(request_headers.to_string())),
        response: Set(Some(response_body.to_string())),
        response_headers: Set(Some(response_headers.to_string())),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let attempts: u64 = core_eventdeliveryattempt::Entity::find()
        .filter(core_eventdeliveryattempt::Column::DeliveryId.eq(delivery_id))
        .count(&txn)
        .await?;
    let status = if ok {
        "success"
    } else if attempts >= rustygod_core::webhooks::MAX_DELIVERY_RETRIES as u64 {
        "failed"
    } else {
        "pending"
    };
    if let Some(d) = core_eventdelivery::Entity::find_by_id(delivery_id)
        .one(&txn)
        .await?
    {
        let mut am: core_eventdelivery::ActiveModel = d.into();
        am.status = Set(status.to_string());
        am.update(&txn).await?;
    }
    txn.commit().await?;
    view(db, delivery_id).await
}

pub async fn view(db: &impl sea_orm::ConnectionTrait, delivery_id: i32) -> Result<DeliveryView> {
    let d = core_eventdelivery::Entity::find_by_id(delivery_id)
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(delivery_id.to_string())))?;
    let wh = webhook_webhook::Entity::find_by_id(d.webhook_id)
        .one(db)
        .await?
        .ok_or_else(|| DbError::SeaOrm(sea_orm::DbErr::RecordNotFound(d.webhook_id.to_string())))?;
    let payload = match d.payload_id {
        Some(pid) => core_eventpayload::Entity::find_by_id(pid)
            .one(db)
            .await?
            .map(|p| p.payload)
            .unwrap_or_default(),
        None => String::new(),
    };
    let attempts = core_eventdeliveryattempt::Entity::find()
        .filter(core_eventdeliveryattempt::Column::DeliveryId.eq(delivery_id))
        .count(db)
        .await?;
    Ok(DeliveryView {
        id: d.id,
        status: d.status,
        event_type: d.event_type,
        webhook_id: d.webhook_id,
        target_url: wh.target_url,
        payload,
        attempts,
    })
}

pub async fn list_attempts(
    db: &impl sea_orm::ConnectionTrait,
    delivery_id: i32,
) -> Result<Vec<AttemptView>> {
    Ok(core_eventdeliveryattempt::Entity::find()
        .filter(core_eventdeliveryattempt::Column::DeliveryId.eq(delivery_id))
        .order_by_asc(core_eventdeliveryattempt::Column::CreatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(|a| AttemptView {
            id: a.id,
            status: a.status,
            response_status_code: a.response_status_code,
            duration_secs: a.duration,
        })
        .collect())
}

/// Deliveries due for (re)try: pending, under the retry cap, whose last
/// attempt is older than `backoff * 2^attempts` (or never attempted).
/// Mirrors the celery beat/retry selection.
pub async fn due_deliveries(db: &DatabaseConnection, limit: u64) -> Result<Vec<i32>> {
    let pending: Vec<(i32, sea_orm::prelude::DateTimeWithTimeZone)> = core_eventdelivery::Entity::find()
        .select_only()
        .column(core_eventdelivery::Column::Id)
        .column(core_eventdelivery::Column::CreatedAt)
        .filter(core_eventdelivery::Column::Status.eq("pending"))
        .into_tuple()
        .all(db)
        .await?;
    let now = Utc::now();
    let mut out = Vec::new();
    for (id, created) in pending.into_iter().take(limit as usize * 4) {
        let attempts: u64 = core_eventdeliveryattempt::Entity::find()
            .filter(core_eventdeliveryattempt::Column::DeliveryId.eq(id))
            .count(db)
            .await?;
        if attempts >= rustygod_core::webhooks::MAX_DELIVERY_RETRIES as u64 {
            continue;
        }
        // Last attempt time, else delivery creation time.
        let last: Option<sea_orm::prelude::DateTimeWithTimeZone> = core_eventdeliveryattempt::Entity::find()
            .select_only()
            .column(core_eventdeliveryattempt::Column::CreatedAt)
            .filter(core_eventdeliveryattempt::Column::DeliveryId.eq(id))
            .order_by_desc(core_eventdeliveryattempt::Column::CreatedAt)
            .into_tuple()
            .one(db)
            .await?
            .flatten();
        let since = last.unwrap_or(created);
        let wait = chrono::Duration::seconds(
            rustygod_core::webhooks::retry_countdown_secs(
                rustygod_core::webhooks::DELIVERY_BACKOFF_SECS,
                attempts as u32,
            ) as i64,
        );
        let since_utc = since.with_timezone(&Utc);
        if now - since_utc >= wait {
            out.push(id);
        }
        if out.len() as u64 >= limit {
            break;
        }
    }
    Ok(out)
}

/// Signed request headers for a delivery (Saleor-Domain/Event/Signature).
pub fn signed_headers(
    domain: &str,
    event_type: &str,
    payload: &str,
    secret: &str,
) -> Vec<(String, String)> {
    let sig = rustygod_core::webhooks::sign_payload(payload.as_bytes(), secret);
    rustygod_core::webhooks::delivery_headers(domain, event_type, &sig)
}

pub async fn webhook_secret(
    db: &impl sea_orm::ConnectionTrait,
    webhook_id: i32,
) -> Result<Option<String>> {
    Ok(webhook_webhook::Entity::find_by_id(webhook_id)
        .one(db)
        .await?
        .and_then(|w| w.secret_key))
}
