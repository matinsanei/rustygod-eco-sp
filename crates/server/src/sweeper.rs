//! Background sweeper (reviewer priority #1): expired reservations +
//! pending webhook deliveries, every `RUSTYGOD_SWEEP_SECS` (default 60,
//! `0` disables).
//!
//! Multi-instance answer (the reviewer's question): `tokio::spawn` inside
//! the server, single-flight via the database —
//! - reservations: `DELETE WHERE expired` commutes, doubles are no-ops;
//! - deliveries: `claim_delivery` flips `pending → sending` atomically,
//!   exactly one instance wins each row (`UPDATE ... WHERE status`);
//! - indexes (`CREATE INDEX IF NOT EXISTS`) keep both scans cheap as the
//!   tables grow, so no data rot and no full scans.
//!
//! The loop never fails the process: per-tick errors are logged, never
//! propagated. HTTP sends reuse `service_webhook::deliver` (attempts +
//! backoff + retry-cap accounting live there).

use sea_orm::{ConnectionTrait, DatabaseConnection};

async fn ensure_indexes(db: &DatabaseConnection) {
    for sql in [
        "CREATE INDEX IF NOT EXISTS rustygod_res_expires_idx \
         ON warehouse_reservation (reserved_until)",
        "CREATE INDEX IF NOT EXISTS rustygod_delivery_status_idx \
         ON core_eventdelivery (status)",
    ] {
        if let Err(e) = db
            .execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql.to_string(),
            ))
            .await
        {
            tracing::warn!("sweeper index ensure failed: {e}");
        }
    }
}

async fn tick(db: &DatabaseConnection, domain: &str) {
    match rustygod_db::commerce::sweep_expired_reservations(db).await {
        Ok(0) => {}
        Ok(n) => tracing::info!("sweeper: dropped {n} expired reservations"),
        Err(e) => tracing::warn!("sweeper reservations failed: {e}"),
    }
    let due = rustygod_db::webhooks::due_deliveries(db, 100).await.unwrap_or_default();
    let mut sent = 0u64;
    for id in due {
        match rustygod_db::webhooks::claim_delivery(db, id).await {
            Ok(true) => {}
            Ok(false) => continue, // another instance won it
            Err(e) => {
                tracing::warn!("sweeper claim {id} failed: {e}");
                continue;
            }
        }
        match crate::service_webhook::deliver(db, domain, id).await {
            Ok(v) => {
                if v.status == "success" {
                    sent += 1;
                }
            }
            Err(e) => tracing::warn!("sweeper delivery {id} failed: {e}"),
        }
    }
    if sent > 0 {
        tracing::info!("sweeper: delivered {sent} webhooks");
    }
}

/// Spawn the sweep loop. Returns immediately; the task lives with the server.
pub fn spawn(db: DatabaseConnection) {
    let secs: u64 = std::env::var("RUSTYGOD_SWEEP_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    if secs == 0 {
        tracing::info!("sweeper disabled (RUSTYGOD_SWEEP_SECS=0)");
        return;
    }
    let domain = std::env::var("RUSTYGOD_DOMAIN").unwrap_or_else(|_| "localhost".into());
    tokio::spawn(async move {
        ensure_indexes(&db).await;
        tracing::info!("sweeper every {secs}s (reservations + webhook outbox)");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(secs));
        loop {
            interval.tick().await;
            tick(&db, &domain).await;
        }
    });
}
