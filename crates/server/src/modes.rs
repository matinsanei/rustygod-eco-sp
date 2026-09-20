//! Runtime modes mirroring Saleor's process layout (`saleor-core`):
//!
//! - `api` (default): `uvicorn saleor.asgi` → gRPC core + GraphQL BFF + metrics.
//! - `worker`: `celery worker` → webhook outbox delivery loop (foreground, no ports).
//! - `beat`: `celery beat` → periodic tasks from `CELERY_BEAT_SCHEDULE`
//!   (`saleor/settings.py`). Implemented jobs run for real; the rest are
//!   honest deferred stubs (debug log) until their domain logic lands.
//! - `check`: one-shot readiness probe (DB connectivity + key tables), exit code.
//!
//! No CLI dependency: `rustygod-server [api|worker|beat|check] [--help]`.
//! `RUSTYGOD_BEAT_SCALE` (float, default 1.0) multiplies every beat interval,
//! so daily jobs can be exercised in dev (`RUSTYGOD_BEAT_SCALE=0.01`).

use sea_orm::DatabaseConnection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Api,
    Worker,
    Beat,
    Check,
}

impl Mode {
    pub fn parse() -> Self {
        let arg = std::env::args().nth(1).unwrap_or_default();
        match arg.as_str() {
            "" | "api" => Mode::Api,
            "worker" => Mode::Worker,
            "beat" | "scheduler" => Mode::Beat,
            "check" => Mode::Check,
            "-h" | "--help" | "help" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown mode {other:?}; expected api|worker|beat|check");
                std::process::exit(2);
            }
        }
    }
}

fn print_help() {
    println!(
        "rustygod-server [MODE]\n\
         \n\
         Modes (mirror Saleor's processes):\n  \
         api     gRPC core + GraphQL BFF + metrics (default)\n  \
         worker  webhook outbox delivery loop (celery worker)\n  \
         beat    periodic scheduler (celery beat, CELERY_BEAT_SCHEDULE)\n  \
         check   readiness probe, exits 0/1\n\
         \n\
         Env: RUSTYGOD_DATABASE_URL, RUSTYGOD_ADDR, RUSTYGOD_GRAPHQL_ADDR,\n  \
         RUSTYGOD_METRICS_ADDR, RUSTYGOD_DOMAIN, RUSTYGOD_SWEEP_SECS,\n  \
         RUSTYGOD_WORKER_SECS (default 10), RUSTYGOD_BEAT_SCALE (default 1.0)"
    );
}

fn beat_scale() -> f64 {
    std::env::var("RUSTYGOD_BEAT_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|f: &f64| f.is_finite() && *f > 0.0)
        .unwrap_or(1.0)
}

/// Connect to the shared Saleor PostgreSQL. Worker/beat/check refuse to run
/// without it (Saleor's worker/beat are meaningless without a database).
pub async fn require_db() -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let pool = rustygod_db::connect(&rustygod_db::database_url()).await?;
    tracing::info!("connected to Saleor PostgreSQL");
    Ok(pool)
}

// ---------------------------------------------------------------------------
// worker: celery worker equivalent (webhook outbox delivery)
// ---------------------------------------------------------------------------

/// Foreground delivery loop. Same `tick` the embedded sweeper uses
/// (single-flight via `claim_delivery`), so api+worker can run side by side.
pub async fn run_worker(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    let secs: u64 = std::env::var("RUSTYGOD_WORKER_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let domain = std::env::var("RUSTYGOD_DOMAIN").unwrap_or_else(|_| "localhost".into());
    tracing::info!("worker: webhook outbox delivery every {secs}s (celery-worker equivalent)");
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(secs.max(1)));
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("worker: shutting down");
                return Ok(());
            }
            _ = interval.tick() => {
                crate::sweeper::tick_once(&db, &domain).await;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// beat: celery beat equivalent (CELERY_BEAT_SCHEDULE)
// ---------------------------------------------------------------------------

struct BeatEntry {
    /// Beat schedule key from Saleor's CELERY_BEAT_SCHEDULE.
    name: &'static str,
    /// Canonical Saleor task path (for operators comparing with Django).
    saleor_task: &'static str,
    interval_secs: u64,
    job: BeatJob,
}

enum BeatJob {
    /// Runs for real against the shared DB.
    Real(fn(&DatabaseConnection) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>>),
    /// Domain logic not ported yet; logs at debug so the schedule is visible
    /// without spamming (info on first fire).
    Deferred,
}

fn beat_entries() -> Vec<BeatEntry> {
    use BeatJob::{Deferred, Real};
    // Saleor: saleor.warehouse.tasks.delete_expired_reservations_task, daily.
    // Ours runs every 60s (same cadence as the api-embedded sweeper).
    fn sweep_expired_reservations(
        db: &DatabaseConnection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            match rustygod_db::commerce::sweep_expired_reservations(db).await {
                Ok(0) => {}
                Ok(n) => tracing::info!("beat[delete-expired-reservations]: dropped {n}"),
                Err(e) => tracing::warn!("beat[delete-expired-reservations] failed: {e}"),
            }
        })
    }
    vec![
        BeatEntry { name: "delete-expired-reservations", saleor_task: "saleor.warehouse.tasks.delete_expired_reservations_task", interval_secs: 60, job: Real(sweep_expired_reservations) },
        // --- deferred: schedule present, logic lands with its domain port ---
        BeatEntry { name: "delete-empty-allocations", saleor_task: "saleor.warehouse.tasks.delete_empty_allocations_task", interval_secs: 24 * 3600, job: Deferred },
        BeatEntry { name: "deactivate-preorder-for-variants", saleor_task: "saleor.product.tasks.deactivate_preorder_for_variants_task", interval_secs: 3600, job: Deferred },
        BeatEntry { name: "delete-expired-checkouts", saleor_task: "saleor.checkout.tasks.delete_expired_checkouts", interval_secs: 24 * 3600, job: Deferred },
        BeatEntry { name: "delete_expired_orders", saleor_task: "saleor.order.tasks.delete_expired_orders_task", interval_secs: 24 * 3600, job: Deferred },
        BeatEntry { name: "delete-outdated-event-data", saleor_task: "saleor.core.tasks.delete_event_payloads_task", interval_secs: 24 * 3600, job: Deferred },
        BeatEntry { name: "deactivate-expired-gift-cards", saleor_task: "saleor.giftcard.tasks.deactivate_expired_cards_task", interval_secs: 24 * 3600, job: Deferred },
        BeatEntry { name: "update-stocks-quantity-allocated", saleor_task: "saleor.warehouse.tasks.update_stocks_quantity_allocated_task", interval_secs: 24 * 3600, job: Deferred },
        BeatEntry { name: "delete-old-export-files", saleor_task: "saleor.csv.tasks.delete_old_export_files", interval_secs: 24 * 3600, job: Deferred },
        // Saleor: DB-driven promotion-toggle schedule; we poll daily until ported.
        BeatEntry { name: "handle-promotion-toggle", saleor_task: "saleor.discount.tasks.handle_promotion_toggle", interval_secs: 24 * 3600, job: Deferred },
        // Saleor: every ~60s (search-index rebuild checks).
        BeatEntry { name: "update-products-search-vectors", saleor_task: "saleor.product.tasks.update_products_search_vector_task", interval_secs: 60, job: Deferred },
        BeatEntry { name: "update-gift-cards-search-vectors", saleor_task: "saleor.giftcard.tasks.update_gift_cards_search_vector_task", interval_secs: 60, job: Deferred },
        BeatEntry { name: "update-pages-search-vectors", saleor_task: "saleor.page.tasks.update_pages_search_vector_task", interval_secs: 60, job: Deferred },
        BeatEntry { name: "update-checkouts-search-vectors", saleor_task: "saleor.checkout.tasks.update_checkouts_search_vector_task", interval_secs: 60, job: Deferred },
        // Saleor: BEAT_EXPIRE_ORDERS_AFTER_TIMEDELTA, default 5 minutes.
        BeatEntry { name: "expire-orders", saleor_task: "saleor.order.tasks.expire_orders_task", interval_secs: 5 * 60, job: Deferred },
        BeatEntry { name: "remove-apps-marked-as-removed", saleor_task: "saleor.app.tasks.remove_apps_task", interval_secs: 24 * 3600, job: Deferred },
        // Saleor: every 10 minutes.
        BeatEntry { name: "release-funds-for-abandoned-checkouts", saleor_task: "saleor.payment.tasks.transaction_release_funds_for_checkout_task", interval_secs: 10 * 60, job: Deferred },
        // Saleor: BEAT_PRICE_RECALCULATION_SCHEDULE, default 30 seconds.
        BeatEntry { name: "recalculate-promotion-rules", saleor_task: "saleor.product.tasks.update_variant_relations_for_active_promotion_rules_task", interval_secs: 30, job: Deferred },
        BeatEntry { name: "recalculate-discounted-price-for-products", saleor_task: "saleor.product.tasks.recalculate_discounted_price_for_products_task", interval_secs: 30, job: Deferred },
        // Saleor: every ~60s (automatic checkout completion check).
        BeatEntry { name: "checkout-automatic-completion", saleor_task: "saleor.checkout.tasks.trigger_automatic_checkout_completion_task", interval_secs: 60, job: Deferred },
    ]
}

pub async fn run_beat(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    let scale = beat_scale();
    tracing::info!("beat: scheduler started (celery-beat equivalent, scale={scale})");
    let mut handles = Vec::new();
    for e in beat_entries() {
        let secs = ((e.interval_secs as f64) * scale).max(1.0) as u64;
        let name = e.name;
        let task = e.saleor_task;
        tracing::info!("beat: every {secs}s {name} ({task})");
        match e.job {
            BeatJob::Real(run) => {
                let db = db.clone();
                handles.push(tokio::spawn(async move {
                    let mut interval =
                        tokio::time::interval(std::time::Duration::from_secs(secs));
                    loop {
                        interval.tick().await;
                        run(&db).await;
                    }
                }));
            }
            BeatJob::Deferred => {
                handles.push(tokio::spawn(async move {
                    let mut interval =
                        tokio::time::interval(std::time::Duration::from_secs(secs));
                    let mut first = true;
                    loop {
                        interval.tick().await;
                        if first {
                            tracing::info!(
                                "beat[{name}]: deferred (not ported yet: {task})"
                            );
                            first = false;
                        } else {
                            tracing::debug!("beat[{name}]: deferred tick");
                        }
                    }
                }));
            }
        }
    }
    tokio::signal::ctrl_c().await?;
    tracing::info!("beat: shutting down");
    for h in handles {
        h.abort();
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// check: readiness probe
// ---------------------------------------------------------------------------

/// One-shot probe: connectivity + row counts on the tables the api needs.
/// Prints `ok` lines; exits non-zero on any failure (for compose `depends_on`
/// or k8s readiness).
pub async fn run_check(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    use sea_orm::{ConnectionTrait, Statement};
    let probe = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1 AS one".to_string(),
        ))
        .await?
        .ok_or("SELECT 1 returned no rows")?;
    let one: i32 = probe.try_get("", "one").map_err(|e| e.to_string())?;
    if one != 1 {
        return Err("connectivity probe failed".into());
    }
    println!("ok postgres connectivity");
    for table in [
        "django_site",
        "account_user",
        "channel_channel",
        "product_product",
        "order_order",
        "checkout_checkout",
    ] {
        let row = db
            .query_one(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT COUNT(*) AS c FROM {table}"),
            ))
            .await?;
        match row {
            Some(r) => {
                let c: i64 = r.try_get("", "c").unwrap_or(-1);
                println!("ok {table}: {c} rows");
            }
            None => println!("missing {table}"),
        }
    }
    Ok(())
}
