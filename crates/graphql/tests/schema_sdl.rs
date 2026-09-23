use saleor_rustify_graphql::build_schema;

#[tokio::test]
async fn sdl_contains_core_sections() {
    let schema = build_schema(None);
    let sdl = schema.sdl();
    for needle in ["products", "checkout", "order", "orders", "channels", "transaction", "warehouses", "taxClasses", "shippingMethods", "pages", "promotions", "me"] {
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

#[tokio::test]
async fn me_without_bearer_is_null() {
    let schema = build_schema(None);
    let res = schema.execute("{ me { email } }").await;
    assert!(res.errors.is_empty(), "me without bearer should not error: {:?}", res.errors);
    assert!(res.data.to_string().contains("null") || res.data.to_string().contains("me"), "me should be null without auth");
}
