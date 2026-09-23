//! DB-mode plugin flow over gRPC handlers: builtin tax plugin answers,
//! registration validates, unknown plugins 404 — all staff-gated.

use saleor_rustify_db::database_url;
use saleor_rustify_proto::plugin::{
    plugin_service_server::PluginService, CalculateTaxRequest, CallPluginRequest,
    ListPluginsRequest, PluginManifestInfo, RegisterPluginRequest, UnregisterPluginRequest,
};
use saleor_rustify_server::service_plugin::PluginServiceImpl;
use tonic::Request;

fn test_key() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../db/tests/testdata/test_rsa.pem"
    ))
    .expect("test RSA key must exist")
}

async fn staff_req<T>(msg: T) -> Request<T> {
    std::env::set_var("RSA_PRIVATE_KEY", test_key());
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    let (user, _) = saleor_rustify_db::auth::find_for_login(&db, "admin@example.com")
        .await
        .unwrap()
        .unwrap();
    let pair = saleor_rustify_core::auth::mint_tokens_with_key(
        &test_key(),
        "test",
        &user.email,
        user.id,
        user.is_staff,
        &user.jwt_token_key,
    )
    .unwrap();
    let mut req = Request::new(msg);
    req.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", pair.access).parse().unwrap(),
    );
    req
}

async fn svc() -> PluginServiceImpl {
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    PluginServiceImpl::new(Some(db))
}

#[tokio::test]
async fn grpc_plugin_lifecycle() {
    std::env::set_var("RSA_PRIVATE_KEY", test_key());
    let svc = svc().await;

    // Anonymous is rejected before anything runs.
    let err = svc
        .calculate_tax(Request::new(CalculateTaxRequest {
            subtotal_cents: 10000,
            rate_bps: 900,
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);

    // Builtin flat-rate-tax answers through the service.
    let tax = svc
        .calculate_tax(
            staff_req(CalculateTaxRequest { subtotal_cents: 10000, rate_bps: 900 }).await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(tax.errors.is_empty());
    assert_eq!(tax.tax_cents, 900);
    assert_eq!(tax.total_cents, 10900);

    // It shows up in the registry.
    let list = svc
        .list_plugins(staff_req(ListPluginsRequest {}).await)
        .await
        .unwrap()
        .into_inner();
    assert!(list.plugins.iter().any(|p| p.name == "flat-rate-tax"));

    // Unknown plugin → NOT_FOUND error, not a crash.
    let missing = svc
        .call_plugin(
            staff_req(CallPluginRequest {
                name: "nope".into(),
                function: "x".into(),
                payload_json: "{}".into(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(!missing.errors.is_empty());
    assert_eq!(missing.errors[0].code, "NOT_FOUND");

    // Garbage wasm is rejected at registration, never stored.
    let bad = svc
        .register_plugin(
            staff_req(RegisterPluginRequest {
                manifest: Some(PluginManifestInfo {
                    name: "junk".into(),
                    version: "1".into(),
                    extension_points: vec!["run".into()],
                    capabilities: vec![],
                }),
                wasm: vec![0, 1, 2, 3],
                config: Default::default(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(!bad.registered);
    assert!(!bad.errors.is_empty());

    let list = svc
        .list_plugins(staff_req(ListPluginsRequest {}).await)
        .await
        .unwrap()
        .into_inner();
    assert!(!list.plugins.iter().any(|p| p.name == "junk"));
}

#[tokio::test]
async fn grpc_plugin_persistence_and_public_points() {
    std::env::set_var("RSA_PRIVATE_KEY", test_key());
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    let svc = PluginServiceImpl::new(Some(db.clone()));

    // Register a validator copy under a new name (persisted to postgres).
    let v = saleor_rustify_plugins::reference_validator_plugin().unwrap();
    let reg = svc
        .register_plugin(
            staff_req(RegisterPluginRequest {
                manifest: Some(PluginManifestInfo {
                    name: "ci-validator".into(),
                    version: v.manifest.version.clone(),
                    extension_points: v.manifest.extension_points.clone(),
                    capabilities: v.manifest.capabilities.clone(),
                }),
                wasm: v.wasm.clone(),
                config: Default::default(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(reg.registered, "{:?}", reg.errors);

    // A FRESH service (empty memory) serves it from the database.
    let svc2 = PluginServiceImpl::new(Some(db.clone()));
    let out = svc2
        .call_plugin(
            staff_req(CallPluginRequest {
                name: "ci-validator".into(),
                function: "checkout.validate".into(),
                payload_json: r#"{"total_cents":100,"min_cents":500}"#.into(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(out.errors.is_empty());
    assert!(out.output_json.contains("\"MIN_ORDER\""), "{}", out.output_json);

    // Builtins can't be unregistered; customs can (memory + row).
    let denied = svc
        .unregister_plugin(
            staff_req(UnregisterPluginRequest { name: "flat-rate-tax".into() }).await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(!denied.unregistered);

    let gone = svc
        .unregister_plugin(
            staff_req(UnregisterPluginRequest { name: "ci-validator".into() }).await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(gone.unregistered);

    // Public points: storefront calls without auth when env-listed.
    std::env::set_var("RUSTIFY_PUBLIC_POINTS", "calculate_tax");
    let svc3 = PluginServiceImpl::new(Some(db));
    let open = svc3
        .calculate_tax(Request::new(CalculateTaxRequest {
            subtotal_cents: 2000,
            rate_bps: 1000,
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(open.errors.is_empty());
    assert_eq!((open.tax_cents, open.total_cents), (200, 2200));

    // ...but unlisted points still demand staff.
    let err = svc3
        .call_plugin(Request::new(CallPluginRequest {
            name: "min-order-validator".into(),
            function: "checkout.validate".into(),
            payload_json: "{}".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
    std::env::remove_var("RUSTIFY_PUBLIC_POINTS");
}
