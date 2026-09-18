//! PluginService: register/call WASM extensions (Phase 3).
//!
//! Registry is persistent: plugins live in the `rustygod_plugin` table
//! (ours — Django ignores it) and load on first use; the three reference
//! plugins are built in. extism calls are blocking — they run on tokio's
//! blocking pool so the async runtime never stalls.
//!
//! Auth: register/list/unregister demand `manage_apps`. `CallPlugin` (and
//! the `CalculateTax` convenience) is open **only** for extension points
//! named in `RUSTYGOD_PUBLIC_POINTS` (comma-separated, e.g.
//! `calculate_tax,checkout.validate`) — the storefront path. Everything
//! else stays staff-only.

use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::{AtomicBool, Ordering}, Arc, RwLock},
};

use rustygod_db::plugin_store;
use rustygod_plugins::{PluginManifest, PluginPackage};
use rustygod_proto::plugin::{
    plugin_service_server::PluginService, CalculateTaxRequest, CalculateTaxResponse,
    CallPluginRequest, CallPluginResponse, ListPluginsRequest, ListPluginsResponse,
    PluginManifestInfo, RegisterPluginRequest, RegisterPluginResponse, UnregisterPluginRequest,
    UnregisterPluginResponse,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};

#[derive(Default)]
struct Registry {
    plugins: HashMap<String, PluginPackage>,
    loaded: AtomicBool,
}

pub struct PluginServiceImpl {
    db: Option<DatabaseConnection>,
    registry: Arc<RwLock<Registry>>,
    public_points: HashSet<String>,
}

fn builtin_plugins() -> Vec<PluginPackage> {
    [
        rustygod_plugins::reference_tax_plugin(),
        rustygod_plugins::reference_validator_plugin(),
        rustygod_plugins::reference_notifier_plugin(),
    ]
    .into_iter()
    .filter_map(|r| r.ok())
    .collect()
}

fn to_package(name: String, m: PluginManifestInfo, config: HashMap<String, String>, wasm: Vec<u8>) -> PluginPackage {
    PluginPackage {
        manifest: PluginManifest {
            name,
            version: m.version,
            extension_points: m.extension_points,
            capabilities: m.capabilities,
            config,
        },
        wasm,
    }
}

impl PluginServiceImpl {
    pub fn new(db: Option<DatabaseConnection>) -> Self {
        let public_points: HashSet<String> = std::env::var("RUSTYGOD_PUBLIC_POINTS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let registry = Arc::new(RwLock::new(Registry::default()));
        for pkg in builtin_plugins() {
            registry.write().unwrap().plugins.insert(pkg.manifest.name.clone(), pkg);
        }
        Self { db, registry, public_points }
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

    fn info(m: &PluginManifest) -> PluginManifestInfo {
        PluginManifestInfo {
            name: m.name.clone(),
            version: m.version.clone(),
            extension_points: m.extension_points.clone(),
            capabilities: m.capabilities.clone(),
        }
    }

    /// Load persisted plugins once (builtins stay pinned — they overwrite
    /// same-named rows so reference behavior can't drift).
    async fn ensure_loaded(&self) -> Result<(), Status> {
        if self.registry.read().unwrap().loaded.load(Ordering::SeqCst) {
            return Ok(());
        }
        let Some(db) = self.db.as_ref() else {
            self.registry.read().unwrap().loaded.store(true, Ordering::SeqCst);
            return Ok(());
        };
        plugin_store::ensure_table(db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let stored = plugin_store::load_all(db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let mut reg = self.registry.write().unwrap();
        for s in stored {
            if reg.plugins.contains_key(&s.name) {
                continue; // builtin pinned
            }
            reg.plugins.insert(
                s.name.clone(),
                to_package(
                    s.name,
                    PluginManifestInfo {
                        name: String::new(),
                        version: s.version,
                        extension_points: s.extension_points,
                        capabilities: s.capabilities,
                    },
                    s.config.as_object().map(|o| {
                        o.iter().filter_map(|(k, v)| v.as_str().map(|x| (k.clone(), x.to_string()))).collect()
                    }).unwrap_or_default(),
                    s.wasm,
                ),
            );
        }
        reg.loaded.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn gate(&self, req: &Request<impl Sized>) -> Result<(), Status> {
        self.ensure_loaded().await?;
        let db = self.db()?;
        crate::access::authorize(db, req.metadata(), crate::access::MANAGE_APPS).await?;
        Ok(())
    }

    /// Open only when the *function* is a configured public point.
    async fn gate_call(&self, req: &Request<impl Sized>, function: &str) -> Result<(), Status> {
        self.ensure_loaded().await?;
        if self.public_points.contains(function) {
            return Ok(());
        }
        let db = self.db()?;
        crate::access::authorize(db, req.metadata(), crate::access::MANAGE_APPS).await?;
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<PluginPackage> {
        self.registry.read().unwrap().plugins.get(name).cloned()
    }
}

#[tonic::async_trait]
impl PluginService for PluginServiceImpl {
    async fn register_plugin(
        &self,
        req: Request<RegisterPluginRequest>,
    ) -> Result<Response<RegisterPluginResponse>, Status> {
        self.gate(&req).await?;
        let r = req.into_inner();
        let m = r.manifest.unwrap_or(PluginManifestInfo {
            name: String::new(),
            version: String::new(),
            extension_points: vec![],
            capabilities: vec![],
        });
        if m.name.is_empty() || r.wasm.is_empty() {
            return Ok(Response::new(RegisterPluginResponse {
                registered: false,
                errors: vec![Self::err("INVALID", "plugin needs a name and wasm bytes".into())],
            }));
        }
        let pkg = to_package(m.name.clone(), m, r.config.into_iter().collect(), r.wasm);
        let probe = pkg.clone();
        let valid = tokio::task::spawn_blocking(move || {
            rustygod_plugins::validate_package(&probe)
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
        if let Err(e) = valid {
            return Ok(Response::new(RegisterPluginResponse {
                registered: false,
                errors: vec![Self::err("REJECTED", e.to_string())],
            }));
        }
        if let Some(db) = self.db.as_ref() {
            let stored = plugin_store::StoredPlugin {
                name: pkg.manifest.name.clone(),
                version: pkg.manifest.version.clone(),
                extension_points: pkg.manifest.extension_points.clone(),
                capabilities: pkg.manifest.capabilities.clone(),
                config: serde_json::to_value(&pkg.manifest.config).unwrap_or_default(),
                wasm: pkg.wasm.clone(),
            };
            plugin_store::save_plugin(db, &stored)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
        }
        self.registry.write().unwrap().plugins.insert(pkg.manifest.name.clone(), pkg);
        Ok(Response::new(RegisterPluginResponse { registered: true, errors: vec![] }))
    }

    async fn unregister_plugin(
        &self,
        req: Request<UnregisterPluginRequest>,
    ) -> Result<Response<UnregisterPluginResponse>, Status> {
        self.gate(&req).await?;
        let name = req.into_inner().name;
        if ["flat-rate-tax", "min-order-validator", "order-notifier"].contains(&name.as_str()) {
            return Ok(Response::new(UnregisterPluginResponse {
                unregistered: false,
                errors: vec![Self::err("REJECTED", "builtin plugins cannot be unregistered".into())],
            }));
        }
        let removed = self.registry.write().unwrap().plugins.remove(&name).is_some();
        if let Some(db) = self.db.as_ref() {
            plugin_store::delete_plugin(db, &name)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
        }
        Ok(Response::new(UnregisterPluginResponse {
            unregistered: removed,
            errors: vec![],
        }))
    }

    async fn call_plugin(
        &self,
        req: Request<CallPluginRequest>,
    ) -> Result<Response<CallPluginResponse>, Status> {
        let function = req.get_ref().function.clone();
        self.gate_call(&req, &function).await?;
        let r = req.into_inner();
        let Some(pkg) = self.lookup(&r.name) else {
            return Ok(Response::new(CallPluginResponse {
                output_json: String::new(),
                events: vec![],
                errors: vec![Self::err("NOT_FOUND", format!("unknown plugin: {}", r.name))],
            }));
        };
        let payload: serde_json::Value = serde_json::from_str(&r.payload_json)
            .unwrap_or(serde_json::Value::Null);
        let out = tokio::task::spawn_blocking(move || {
            rustygod_plugins::call(&pkg, &function, &payload)
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
        match out {
            Ok((value, events)) => Ok(Response::new(CallPluginResponse {
                output_json: value.to_string(),
                events,
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(CallPluginResponse {
                output_json: String::new(),
                events: vec![],
                errors: vec![Self::err("PLUGIN_ERROR", e.to_string())],
            })),
        }
    }

    async fn list_plugins(
        &self,
        req: Request<ListPluginsRequest>,
    ) -> Result<Response<ListPluginsResponse>, Status> {
        self.gate(&req).await?;
        let plugins = self
            .registry
            .read()
            .unwrap()
            .plugins
            .values()
            .map(|p| Self::info(&p.manifest))
            .collect();
        Ok(Response::new(ListPluginsResponse { plugins }))
    }

    async fn calculate_tax(
        &self,
        req: Request<CalculateTaxRequest>,
    ) -> Result<Response<CalculateTaxResponse>, Status> {
        self.gate_call(&req, "calculate_tax").await?;
        let r = req.into_inner();
        let Some(tax) = self.lookup("flat-rate-tax") else {
            return Ok(Response::new(CalculateTaxResponse {
                tax_cents: 0,
                total_cents: 0,
                errors: vec![Self::err("NOT_FOUND", "flat-rate-tax not registered".into())],
            }));
        };
        let payload = serde_json::json!({
            "subtotal_cents": r.subtotal_cents,
            "rate_bps": r.rate_bps,
        });
        let out = tokio::task::spawn_blocking(move || {
            rustygod_plugins::call(&tax, "calculate_tax", &payload)
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
        match out {
            Ok((v, _)) => Ok(Response::new(CalculateTaxResponse {
                tax_cents: v["tax_cents"].as_i64().unwrap_or(0),
                total_cents: v["total_cents"].as_i64().unwrap_or(0),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(CalculateTaxResponse {
                tax_cents: 0,
                total_cents: 0,
                errors: vec![Self::err("PLUGIN_ERROR", e.to_string())],
            })),
        }
    }
}
