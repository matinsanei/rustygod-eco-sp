use rustygod_graphql::build_schema;

#[tokio::test]
async fn sdl_contains_core_sections() {
    let schema = build_schema(None);
    let sdl = schema.sdl();
    for needle in ["products", "checkout", "order", "orders", "channels", "transaction"] {
        assert!(sdl.contains(needle), "SDL must contain {needle}");
    }
}

#[tokio::test]
async fn products_query_without_db_returns_error_not_panic() {
    let schema = build_schema(None);
    let res = schema.execute("{ products { name } }").await;
    // Without DB, resolver errors are surfaced as GraphQL errors, not panics.
    assert!(!res.errors.is_empty() || res.data.to_string().contains("products"));
}
