//! Plugin engine contract: the hand-written WAT reference plugin runs
//! the raw extism ABI end to end, and capability gating holds.

use saleor_rustify_plugins::{call, PluginManifest, PluginPackage};
use std::collections::HashMap;

fn tax_pkg(caps: &[&str]) -> PluginPackage {
    let wat = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/reference/tax_flat_rate.wat"
    ))
    .expect("reference WAT must exist");
    let wasm = wat::parse_str(&wat).expect("WAT must compile");
    PluginPackage {
        manifest: PluginManifest {
            name: "flat-rate-tax".into(),
            version: "1.0.0".into(),
            extension_points: vec!["calculate_tax".into()],
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
            config: HashMap::new(),
        },
        wasm,
    }
}

#[test]
fn flat_rate_tax_computes_half_up() {
    let pkg = tax_pkg(&[]);
    let (out, events) = call(
        &pkg,
        "calculate_tax",
        &serde_json::json!({"subtotal_cents": 10000, "rate_bps": 900}),
    )
    .unwrap();
    assert_eq!(out["tax_cents"], 900);
    assert_eq!(out["total_cents"], 10900);
    assert!(events.is_empty());

    // HALF_UP: 199 * 7.5% = 14.925 -> 15.
    let (out, _) = call(
        &pkg,
        "calculate_tax",
        &serde_json::json!({"subtotal_cents": 199, "rate_bps": 750}),
    )
    .unwrap();
    assert_eq!(out["tax_cents"], 15);
    assert_eq!(out["total_cents"], 214);

    // Zero subtotal.
    let (out, _) = call(
        &pkg,
        "calculate_tax",
        &serde_json::json!({"subtotal_cents": 0, "rate_bps": 900}),
    )
    .unwrap();
    assert_eq!(out["tax_cents"], 0);
}

#[test]
fn undeclared_function_is_rejected_before_wasm() {    let pkg = tax_pkg(&[]);
    let err = call(&pkg, "refund_money", &serde_json::json!({})).unwrap_err();
    assert!(err.to_string().contains("not one of the plugin's extension points"), "{err}");
}

#[test]
fn unknown_capability_is_rejected() {
    let pkg = tax_pkg(&["root"]);
    let err = call(&pkg, "calculate_tax", &serde_json::json!({})).unwrap_err();
    assert!(err.to_string().contains("unknown capability"), "{err}");
}

/// A plugin that emits `order.paid` through the `events` capability and
/// answers `{"ok":true}`. All bytes cross via kernel calls (guest linear
/// memory is not host-visible — see the tax plugin's emit path).
fn emitter_wasm() -> Vec<u8> {
    // "order.paid" = 111,114,100,101,114,46,112,97,105,100
    // {"ok":true} = 123,34,111,107,34,58,116,114,117,101,125
    let wat = r#"(module
      (import "extism:host/env" "alloc" (func $alloc (param i64) (result i64)))
      (import "extism:host/env" "output_set" (func $oset (param i64 i64)))
      (import "extism:host/env" "store_u8" (func $store (param i64 i32)))
      (import "extism:host/user" "emit_event" (func $emit (param i64 i64) (result i64)))
      (memory (export "memory") 1)
      (func $emit_bytes (param $o i64) (param $b0 i32) (param $b1 i32)
             (param $b2 i32) (param $b3 i32) (param $b4 i32) (result i64)
        (call $store (local.get $o) (local.get $b0))
        (call $store (i64.add (local.get $o) (i64.const 1)) (local.get $b1))
        (call $store (i64.add (local.get $o) (i64.const 2)) (local.get $b2))
        (call $store (i64.add (local.get $o) (i64.const 3)) (local.get $b3))
        (call $store (i64.add (local.get $o) (i64.const 4)) (local.get $b4))
        (i64.add (local.get $o) (i64.const 5)))
      (func (export "on_order_paid") (result i32)
        (local $o i64) (local $start i64)
        ;; emit "order.paid" — needs the events cap.
        (local.set $o (call $alloc (i64.const 10)))
        (local.set $o (call $emit_bytes (local.get $o)
          (i32.const 111) (i32.const 114) (i32.const 100) (i32.const 101) (i32.const 114)))
        (local.set $o (call $emit_bytes (local.get $o)
          (i32.const 46) (i32.const 112) (i32.const 97) (i32.const 105) (i32.const 100)))
        (call $emit (i64.sub (local.get $o) (i64.const 10)) (i64.const 10)) (drop)
        ;; answer {"ok":true}
        (local.set $start (call $alloc (i64.const 11)))
        (local.set $o (call $emit_bytes (local.get $start)
          (i32.const 123) (i32.const 34) (i32.const 111) (i32.const 107) (i32.const 34)))
        (local.set $o (call $emit_bytes (local.get $o)
          (i32.const 58) (i32.const 116) (i32.const 114) (i32.const 117) (i32.const 101)))
        (call $store (local.get $o) (i32.const 125))
        (call $oset (local.get $start) (i64.const 11))
        (i32.const 0))
    )"#;
    wat::parse_str(wat).expect("emitter WAT must compile")
}

#[test]
fn events_capability_gates_host_access() {
    let mk = |caps: Vec<&str>| PluginPackage {
        manifest: PluginManifest {
            name: "emitter".into(),
            version: "1".into(),
            extension_points: vec!["on_order_paid".into()],
            capabilities: caps.into_iter().map(|s| s.to_string()).collect(),
            config: HashMap::new(),
        },
        wasm: emitter_wasm(),
    };
    // Without `events`: the import has nothing to link against.
    let err = call(&mk(vec![]), "on_order_paid", &serde_json::json!({})).unwrap_err();
    assert!(
        err.to_string().contains("emit_event"),
        "must fail on the unlinked import, got: {err}"
    );
    // With `events`: links, runs, and the event lands in the host sink.
    let (out, events) = call(&mk(vec!["events"]), "on_order_paid", &serde_json::json!({})).unwrap();
    assert_eq!(out["ok"], true);
    assert_eq!(events, vec!["order.paid"]);
}

#[test]
fn min_order_validator_passes_and_rejects() {
    let pkg = saleor_rustify_plugins::reference_validator_plugin().unwrap();
    let (out, _) = call(
        &pkg,
        "checkout.validate",
        &serde_json::json!({"total_cents": 500, "min_cents": 1000}),
    )
    .unwrap();
    assert_eq!(out["valid"], false);
    assert_eq!(out["code"], "MIN_ORDER");

    let (out, _) = call(
        &pkg,
        "checkout.validate",
        &serde_json::json!({"total_cents": 1000, "min_cents": 1000}),
    )
    .unwrap();
    assert_eq!(out["valid"], true);
    assert_eq!(out["code"], "OK");
}

#[test]
fn order_notifier_emits_slack_message() {
    let pkg = saleor_rustify_plugins::reference_notifier_plugin().unwrap();
    let (out, events) = call(
        &pkg,
        "order.paid",
        &serde_json::json!({"order_id": "3fa85f64-5717-4562-b3fc-2c963f66afa6"}),
    )
    .unwrap();
    assert_eq!(out["notified"], true);
    assert_eq!(
        events,
        vec!["{\"channel\":\"#orders\",\"text\":\"Order 3fa85f64-5717-4562-b3fc-2c963f66afa6 paid\"}"]
    );
}
