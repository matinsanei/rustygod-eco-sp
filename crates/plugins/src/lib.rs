//! Out-of-process plugin platform (Phase 3): WASM extensions via extism.
//!
//! Saleor's answer to extensions was Python-only, in-process plugins nobody
//! could isolate or upgrade safely. Ours: sandboxed WASM modules with
//! declared **capabilities** and **extension points**.
//!
//! - A plugin is `PluginPackage { manifest, wasm }`. The manifest names the
//!   extension-point functions it implements (`calculate_tax`, …) and the
//!   host capabilities it needs (`log`, `events`).
//! - Dispatch is closed-world: calling an undeclared function is an error,
//!   and host functions are linked **only** for granted capabilities — a
//!   plugin that imports what it didn't declare fails to instantiate.
//! - Payloads are JSON (`serde_json::Value`); money crosses as integer
//!   minor units (cents), never floats — matching the core money rules.

use std::{
    collections::HashMap,
    time::Duration,
};

use extism::{Function, Manifest, Plugin, Val, ValType, Wasm, UserData};

/// Capabilities the host can grant. Closed world — anything else is rejected
/// at validation so plugins can't typo-squat their way to new powers.
pub const CAP_LOG: &str = "log";
pub const CAP_EVENTS: &str = "events";

/// Max wall-clock per plugin call (sandbox bound).
pub const CALL_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    /// Extension-point functions this plugin implements, e.g. `calculate_tax`.
    pub extension_points: Vec<String>,
    /// Host capabilities it needs, subset of `{log, events}`.
    pub capabilities: Vec<String>,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PluginPackage {
    pub manifest: PluginManifest,
    pub wasm: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("unknown capability: {0}")]
    UnknownCapability(String),
    #[error("function '{0}' is not one of the plugin's extension points")]
    NotAnExtensionPoint(String),
    #[error("wasm error: {0}")]
    Wasm(String),
    #[error("bad payload: {0}")]
    Payload(String),
}

pub type Result<T> = std::result::Result<T, PluginError>;

fn validate(m: &PluginManifest) -> Result<()> {
    for cap in &m.capabilities {
        if cap != CAP_LOG && cap != CAP_EVENTS {
            return Err(PluginError::UnknownCapability(cap.clone()));
        }
    }
    if m.name.is_empty() {
        return Err(PluginError::Payload("plugin name is empty".into()));
    }
    Ok(())
}

/// Events a plugin emitted through the `events` capability.
pub type Emitted = Vec<String>;

fn log_fn(
    _plugin: &mut extism::CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData<()>,
) -> std::result::Result<(), extism::Error> {
    let [Val::I64(off)] = inputs else {
        return Err(extism::Error::msg("log_info takes one offset"));
    };
    let Some(handle) = _plugin.memory_handle(*off as u64) else {
        return Err(extism::Error::msg("bad log offset"));
    };
    let msg = _plugin.memory_str(handle)?.to_string();
    _plugin.memory_free(handle)?;
    eprintln!("[plugin] {msg}");
    Ok(())
}

fn emit_fn(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<Emitted>,
) -> std::result::Result<(), extism::Error> {
    let [Val::I64(off), Val::I64(len)] = inputs else {
        return Err(extism::Error::msg("emit_event takes (offset, length)"));
    };
    let Some(handle) = plugin.memory_handle(*off as u64) else {
        return Err(extism::Error::msg("bad event offset"));
    };
    let bytes = plugin.memory_bytes(handle)?;
    let len = (*len as usize).min(bytes.len());
    let evt = std::str::from_utf8(&bytes[..len])
        .map_err(|_| extism::Error::msg("event is not UTF-8"))?
        .to_string();
    plugin.memory_free(handle)?;
    user_data.get()?.lock().unwrap().push(evt);
    outputs[0] = Val::I64(0);
    Ok(())
}

/// The second reference plugin: min-order validator in hand-written WAT.
pub fn reference_validator_plugin() -> Result<PluginPackage> {
    const WAT: &str = include_str!("../reference/min_order_validate.wat");
    let wasm = wat::parse_str(WAT).map_err(|e| PluginError::Payload(format!("reference WAT: {e}")))?;
    Ok(PluginPackage {
        manifest: PluginManifest {
            name: "min-order-validator".into(),
            version: "1.0.0".into(),
            extension_points: vec!["checkout.validate".into()],
            capabilities: vec![],
            config: HashMap::new(),
        },
        wasm,
    })
}

/// The third reference plugin: order notifier in hand-written WAT.
/// Needs the `events` capability; the host delivers emitted messages
/// through the webhook system.
pub fn reference_notifier_plugin() -> Result<PluginPackage> {
    const WAT: &str = include_str!("../reference/order_notifier.wat");
    let wasm = wat::parse_str(WAT).map_err(|e| PluginError::Payload(format!("reference WAT: {e}")))?;
    Ok(PluginPackage {
        manifest: PluginManifest {
            name: "order-notifier".into(),
            version: "1.0.0".into(),
            extension_points: vec!["order.paid".into()],
            capabilities: vec![CAP_EVENTS.to_string()],
            config: HashMap::new(),
        },
        wasm,
    })
}

/// The auditable reference plugin: flat-rate tax in hand-written WAT.
/// Compiled at startup — no toolchain, no registry download,
/// byte-for-byte reviewable.
pub fn reference_tax_plugin() -> Result<PluginPackage> {
    const WAT: &str = include_str!("../reference/tax_flat_rate.wat");
    let wasm = wat::parse_str(WAT).map_err(|e| PluginError::Payload(format!("reference WAT: {e}")))?;
    Ok(PluginPackage {
        manifest: PluginManifest {
            name: "flat-rate-tax".into(),
            version: "1.0.0".into(),
            extension_points: vec!["calculate_tax".into()],
            capabilities: vec![],
            config: HashMap::new(),
        },
        wasm,
    })
}

/// Build a callable instance with exactly the host functions the
/// manifest's capabilities grant. Used by `call` and by registration-time
/// validation alike, so "registers OK but fails on first call" can't happen.
fn instantiate(pkg: &PluginPackage, sink: UserData<Emitted>) -> Result<Plugin> {
    let mut fns: Vec<Function> = vec![];
    if pkg.manifest.capabilities.iter().any(|c| c == CAP_LOG) {
        fns.push(Function::new(
            "log_info",
            [ValType::I64],
            [],
            UserData::new(()),
            log_fn,
        ));
    }
    if pkg.manifest.capabilities.iter().any(|c| c == CAP_EVENTS) {
        fns.push(Function::new(
            "emit_event",
            [ValType::I64, ValType::I64],
            [ValType::I64],
            sink,
            emit_fn,
        ));
    }

    let manifest = Manifest::new([Wasm::data(pkg.wasm.clone())]).with_timeout(CALL_TIMEOUT);
    Plugin::new(&manifest, fns, true).map_err(|e| PluginError::Wasm(e.to_string()))
}

/// Registration-time validation: manifest sanity + wasm parses + imports
/// link with the granted capabilities. Same instantiation path as `call`,
/// so validated plugins can't fail on first call.
pub fn validate_package(pkg: &PluginPackage) -> Result<()> {
    validate(&pkg.manifest)?;
    wat::parse_bytes(&pkg.wasm).map_err(|e| PluginError::Payload(format!("invalid wasm: {e}")))?;
    instantiate(pkg, UserData::new(vec![]))?;
    Ok(())
}

/// Call `function` on a plugin package with a JSON payload.
/// Returns `(output_json, emitted_events)`.
pub fn call(
    pkg: &PluginPackage,
    function: &str,
    payload: &serde_json::Value,
) -> Result<(serde_json::Value, Vec<String>)> {
    validate(&pkg.manifest)?;
    if !pkg.manifest.extension_points.iter().any(|f| f == function) {
        return Err(PluginError::NotAnExtensionPoint(function.to_string()));
    }
    let input = serde_json::to_vec(payload).map_err(|e| PluginError::Payload(e.to_string()))?;

    let emitted: UserData<Emitted> = UserData::new(vec![]);
    let mut plugin = instantiate(pkg, emitted.clone())?;
    let out = plugin
        .call::<_, &[u8]>(function, input)
        .map_err(|e| PluginError::Wasm(e.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_slice(out).map_err(|e| PluginError::Payload(format!("plugin returned invalid JSON: {e}")))?;
    let events = emitted.get().map_err(|e| PluginError::Wasm(e.to_string()))?.lock().unwrap().clone();
    Ok((value, events))
}
