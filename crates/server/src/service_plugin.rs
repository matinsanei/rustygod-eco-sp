//! PluginService: register/call WASM extensions (Phase 3).
//!
//! Registry is in-memory; the flat-rate-tax reference plugin is built in.
//! All verbs are staff-gated (`manage_apps`). extism calls are blocking —
//! they run on tokio's blocking pool so the async runtime never stalls.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use rustygod_plugins::{PluginManifest, PluginPackage};
use rustygod_proto::plugin::{
    plugin_service_server::PluginService, CalculateTaxRequest, CalculateTaxResponse,
    CallPluginRequest, CallPluginResponse, ListPluginsRequest, ListPluginsResponse,
    PluginManifestInfo, RegisterPluginRequest, RegisterPluginResponse,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};

#[derive(Default)]
struct Registry {
    plugins: HashMap<String, PluginPackage>,
}

pub struct PluginServiceImpl {
    db: Option<DatabaseConnection>,
    registry: Arc<RwLock<Registry>>,
}

impl PluginServiceImpl {
    pub fn new(db: Option<DatabaseConnection>) -> Self {
        let registry = Arc::new(RwLock::new(Registry::default()));
        if let Ok(tax) = rustygod_plugins::reference_tax_plugin() {
            registry.write().unwrap().plugins.insert(tax.manifest.name.clone(), tax);
        }
        Self { db, registry }
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

    async fn gate(&self, req: &Request<impl Sized>) -> Result<(), Status> {
        // borrow the metadata before any move; every verb is staff-only.
        let db = self.db()?;
        crate::access::authorize(db, req.metadata(), crate::access::MANAGE_APPS).await?;
        Ok(())
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
        let pkg = PluginPackage {
            manifest: PluginManifest {
                name: m.name.clone(),
                version: m.version.clone(),
                extension_points: m.extension_points.clone(),
                capabilities: m.capabilities.clone(),
                config: r.config.into_iter().collect(),
            },
            wasm: r.wasm,
        };
        // Validate now (capabilities + wasm parses + imports linkable enough
        // to instantiate) instead of failing on first call.
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
        self.registry.write().unwrap().plugins.insert(m.name, pkg);
        Ok(Response::new(RegisterPluginResponse { registered: true, errors: vec![] }))
    }

    async fn call_plugin(
        &self,
        req: Request<CallPluginRequest>,
    ) -> Result<Response<CallPluginResponse>, Status> {
        self.gate(&req).await?;
        let r = req.into_inner();
        let pkg = self
            .registry
            .read()
            .unwrap()
            .plugins
            .get(&r.name)
            .cloned();
        let Some(pkg) = pkg else {
            return Ok(Response::new(CallPluginResponse {
                output_json: String::new(),
                events: vec![],
                errors: vec![Self::err("NOT_FOUND", format!("unknown plugin: {}", r.name))],
            }));
        };
        let payload: serde_json::Value = serde_json::from_str(&r.payload_json)
            .unwrap_or(serde_json::Value::Null);
        let function = r.function.clone();
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
        self.gate(&req).await?;
        let r = req.into_inner();
        let tax = self
            .registry
            .read()
            .unwrap()
            .plugins
            .get("flat-rate-tax")
            .cloned();
        let Some(tax) = tax else {
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
