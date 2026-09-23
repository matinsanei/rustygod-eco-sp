//! Runtime modes mirroring Saleor's process layout (`saleor-core`):
//!
//! - `api` (default): `uvicorn saleor.asgi` → gRPC core + GraphQL BFF + metrics.
//! - `worker`: `celery worker` → webhook outbox delivery loop (foreground, no ports).
//! - `beat`: `celery beat` → periodic tasks from `CELERY_BEAT_SCHEDULE`
//!   (`saleor/settings.py`). Implemented jobs run for real; the rest are
//!   honest deferred stubs (debug log) until their domain logic lands.
//! - `check`: one-shot readiness probe (DB connectivity + key tables), exit code.
//! - `migrate` / `seed` / `createsuperuser`: thin wrappers over saleor-core's
//!   own `manage.py`, so the DDL and the mock data come from Saleor itself —
//!   never re-guessed in Rust. `SALEOR_CORE_DIR` overrides the checkout
//!   location (default `../saleor/saleor-core` next to this repo).
//!
//! No CLI dependency: `rustygod-server [MODE] [extra args...]`.
//! `RUSTYGOD_BEAT_SCALE` (float, default 1.0) multiplies every beat interval,
//! so daily jobs can be exercised in dev (`RUSTYGOD_BEAT_SCALE=0.01`).

use sea_orm::DatabaseConnection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Api,
    Worker,
    Beat,
    Check,
    Manage(ManageCmd),
}

/// Subcommands delegated 1:1 to saleor-core `manage.py`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageCmd {
    /// `manage.py migrate` — Saleor's own Django migration history is the DDL.
    Migrate,
    /// `manage.py populatedb` — Saleor's own mock catalogue/orders/channels.
    Seed,
    /// `manage.py createsuperuser` — Saleor's own superuser creation.
    CreateSuperuser,
}

impl Mode {
    /// Returns the mode plus any extra argv (passed through to `manage.py`
    /// for the `Manage` modes).
    pub fn parse() -> (Self, Vec<String>) {
        let mut args = std::env::args().skip(1);
        let first = args.next().unwrap_or_default();
        let rest: Vec<String> = args.collect();
        let mode = match first.as_str() {
            "" | "api" => Mode::Api,
            "worker" => Mode::Worker,
            "beat" | "scheduler" => Mode::Beat,
            "check" => Mode::Check,
            "migrate" => Mode::Manage(ManageCmd::Migrate),
            "seed" | "populatedb" => Mode::Manage(ManageCmd::Seed),
            "createsuperuser" => Mode::Manage(ManageCmd::CreateSuperuser),
            "-h" | "--help" | "help" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown mode {other:?}; see --help");
                std::process::exit(2);
            }
        };
        (mode, rest)
    }
}

fn print_help() {
    println!(
        "rustygod-server [MODE] [args...]\n\
         \n\
         Modes (mirror Saleor's processes):\n  \
         api     gRPC core + GraphQL BFF + metrics (default)\n  \
         worker  webhook outbox delivery loop (celery worker)\n  \
         beat    periodic scheduler (celery beat, CELERY_BEAT_SCHEDULE)\n  \
         check   readiness probe, exits 0/1\n\
         \n\
         Saleor's own Django management (DDL + data come from saleor-core,\n \
         extra args pass straight through to manage.py):\n  \
         migrate [args]            manage.py migrate (e.g. --check dry-run)\n  \
         seed [args]               manage.py populatedb (e.g. --createsuperuser\n                                   --superuser_password=admin --withoutimages)\n  \
         createsuperuser [args]    manage.py createsuperuser\n\
         \n\
         Env: RUSTYGOD_DATABASE_URL (mapped to DATABASE_URL for manage.py),\n  \
         SALEOR_CORE_DIR (default ../saleor/saleor-core), RUSTYGOD_ADDR,\n  \
         RUSTYGOD_GRAPHQL_ADDR, RUSTYGOD_METRICS_ADDR, RUSTYGOD_DOMAIN,\n  \
         RUSTYGOD_SWEEP_SECS, RUSTYGOD_WORKER_SECS (default 10),\n  \
         RUSTYGOD_BEAT_SCALE (default 1.0)"
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
    fn checkout_automatic_completion(
        db: &DatabaseConnection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            match rustygod_db::checkout_store::auto_complete_expired_checkouts(db).await {
                Ok(r) if r.attempted == 0 => {}
                Ok(r) => tracing::info!(
                    "beat[checkout-automatic-completion]: attempted {} completed {} failed {}",
                    r.attempted, r.completed, r.failed
                ),
                Err(e) => tracing::warn!("beat[checkout-automatic-completion] failed: {e}"),
            }
        })
    }
    fn sweep_expired_checkouts(
        db: &DatabaseConnection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            match rustygod_db::checkout_store::sweep_expired_checkouts(db).await {
                Ok(0) => {}
                Ok(n) => tracing::info!("beat[delete-expired-checkouts]: dropped {n}"),
                Err(e) => tracing::warn!("beat[delete-expired-checkouts] failed: {e}"),
            }
        })
    }
    fn refresh_product_embeddings(
        db: &DatabaseConnection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let e = rustygod_ai::embed::HashingEmbedder::default();
            match rustygod_ai::vectors::refresh_product_embeddings(db, &e).await {
                Ok(r) if r.embedded == 0 => {}
                Ok(r) => tracing::info!(
                    "beat[refresh-product-embeddings]: scanned {} embedded {} skipped {}",
                    r.scanned, r.embedded, r.skipped
                ),
                Err(e) => tracing::warn!("beat[refresh-product-embeddings] failed: {e}"),
            }
        })
    }
    fn prune_security_tables(
        db: &DatabaseConnection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            match rustygod_db::account_writes::prune_security_tables(db).await {
                Ok(0) => {}
                Ok(n) => tracing::info!("beat[prune-security-tables]: dropped {n}"),
                Err(e) => tracing::warn!("beat[prune-security-tables] failed: {e}"),
            }
        })
    }
    vec![
        BeatEntry { name: "delete-expired-reservations", saleor_task: "saleor.warehouse.tasks.delete_expired_reservations_task", interval_secs: 60, job: Real(sweep_expired_reservations) },
        // --- deferred: schedule present, logic lands with its domain port ---
        BeatEntry { name: "delete-empty-allocations", saleor_task: "saleor.warehouse.tasks.delete_empty_allocations_task", interval_secs: 24 * 3600, job: Deferred },
        BeatEntry { name: "deactivate-preorder-for-variants", saleor_task: "saleor.product.tasks.deactivate_preorder_for_variants_task", interval_secs: 3600, job: Deferred },
        BeatEntry { name: "delete-expired-checkouts", saleor_task: "saleor.checkout.tasks.delete_expired_checkouts", interval_secs: 24 * 3600, job: Real(sweep_expired_checkouts) },
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
        BeatEntry { name: "checkout-automatic-completion", saleor_task: "saleor.checkout.tasks.trigger_automatic_checkout_completion_task", interval_secs: 60, job: Real(checkout_automatic_completion) },
        // Ours: vector-tier refresh (skip-unchanged, cheap on re-runs).
        BeatEntry { name: "refresh-product-embeddings", saleor_task: "rustygod.vectors.refresh_product_embeddings", interval_secs: 24 * 3600, job: Real(refresh_product_embeddings) },
        // Ours: prune expired one-time account tokens + stale login-throttle rows.
        BeatEntry { name: "prune-security-tables", saleor_task: "rustygod.account.prune_security_tables", interval_secs: 24 * 3600, job: Real(prune_security_tables) },
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

// ---------------------------------------------------------------------------
// saleor-core delegation: migrate / seed / createsuperuser
// ---------------------------------------------------------------------------

/// Locate the saleor-core checkout: `SALEOR_CORE_DIR`, else
/// `../saleor/saleor-core` next to the current directory.
fn saleor_core_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let dir = match std::env::var("SALEOR_CORE_DIR") {
        Ok(d) => std::path::PathBuf::from(d),
        Err(_) => std::env::current_dir()?.join("../saleor/saleor-core"),
    };
    let dir = dir.canonicalize().map_err(|_| {
        format!(
            "saleor-core not found at {} (set SALEOR_CORE_DIR)",
            dir.display()
        )
    })?;
    if !dir.join("manage.py").exists() {
        return Err(format!("no manage.py in {} (set SALEOR_CORE_DIR)", dir.display()).into());
    }
    Ok(dir)
}

/// Prefer saleor-core's own `.venv` python, fall back to `python3`.
fn saleor_python(dir: &std::path::Path) -> String {
    let venv = dir.join(".venv/bin/python");
    if venv.exists() {
        venv.to_string_lossy().into_owned()
    } else {
        "python3".to_string()
    }
}

/// Run `manage.py <argv>` with inherited stdio; propagate its exit code.
async fn manage_py(extra: &[String], cmd: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let dir = saleor_core_dir()?;
    let py = saleor_python(&dir);
    let mut args: Vec<&str> = vec!["manage.py"];
    args.extend_from_slice(cmd);
    tracing::info!("manage: {} {}", py, args.join(" "));
    // Point Django at the same database we use (explicit DATABASE_URL wins).
    let status = tokio::process::Command::new(&py)
        .arg("manage.py")
        .args(cmd)
        .args(extra)
        .current_dir(&dir)
        .env("DJANGO_SETTINGS_MODULE", "saleor.settings")
        .env(
            "DATABASE_URL",
            std::env::var("DATABASE_URL")
                .or_else(|_| std::env::var("RUSTYGOD_DATABASE_URL"))
                .unwrap_or_default(),
        )
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .map_err(|e| format!("failed to launch {py}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("manage.py exited with {status}").into())
    }
}

/// `migrate`: Saleor's own Django migration history is the DDL.
/// Extra args pass through (e.g. `--check` for a no-change dry run).
pub async fn run_migrate(extra: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    manage_py(extra, &["migrate"]).await
}

/// `seed`: Saleor's own `populatedb` (mock catalogue/orders/channels).
/// Typical: `seed --createsuperuser --superuser_password=admin --withoutimages`.
pub async fn run_seed(extra: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    manage_py(extra, &["populatedb"]).await
}

/// `createsuperuser`: Saleor's own superuser creation (args pass through).
pub async fn run_createsuperuser(extra: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    manage_py(extra, &["createsuperuser"]).await
}
