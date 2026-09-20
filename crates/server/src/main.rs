use rustygod_proto::{
    ai::{
        chat_agent_server::ChatAgentServer, recommender_server::RecommenderServer,
        semantic_search_server::SemanticSearchServer,
    },
    checkout::checkout_service_server::CheckoutServiceServer,
    commerce::{
        account_service_server::AccountServiceServer,
        channel_service_server::ChannelServiceServer,
        discount_service_server::DiscountServiceServer,
        menu_service_server::MenuServiceServer,
        page_service_server::PageServiceServer, shipping_service_server::ShippingServiceServer,
        tax_service_server::TaxServiceServer, warehouse_service_server::WarehouseServiceServer,
    },
    order::order_service_server::OrderServiceServer,
    plugin::plugin_service_server::PluginServiceServer,
    invoice::invoice_service_server::InvoiceServiceServer,
    draft::draft_order_service_server::DraftOrderServiceServer,
    giftcard::gift_card_service_server::GiftCardServiceServer,
    payment::payment_service_server::PaymentServiceServer,
    auth::auth_service_server::AuthServiceServer,
    webhook::webhook_service_server::WebhookServiceServer,
    product::product_service_server::ProductServiceServer,
};
use rustygod_server::{
    service_ai::{ChatAgentImpl, RecommenderImpl, SemanticSearchImpl},
    service_auth::AuthServiceImpl,
    service_checkout::CheckoutServiceImpl,
    service_commerce::{
        AccountServiceImpl, ChannelServiceImpl, DiscountServiceImpl,
        MenuServiceImpl, PageServiceImpl, ShippingServiceImpl, TaxServiceImpl,
        WarehouseServiceImpl,
    },
    service_draft::DraftOrderServiceImpl,
    service_invoice::InvoiceServiceImpl,
    service_plugin::PluginServiceImpl,
    service_giftcard::GiftCardServiceImpl,
    service_order::OrderServiceImpl,
    service_payment::PaymentServiceImpl,
    service_webhook::WebhookServiceImpl,
    service_product::ProductServiceImpl,
    store::{new_store, seed},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr = std::env::var("RUSTYGOD_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:50051".to_string())
        .parse()?;

    // Zero-friction mode: same PostgreSQL Django uses. Unset RUSTYGOD_DATABASE_URL
    // for the offline in-memory demo.
    let db = match rustygod_db::connect(&rustygod_db::database_url()).await {
        Ok(pool) => {
            tracing::info!("connected to Saleor PostgreSQL");
            Some(pool)
        }
        Err(e) => {
            tracing::warn!("postgres unavailable ({e}); using in-memory catalog");
            None
        }
    };

    let store = new_store();
    if db.is_none() {
        seed(&store);
    }
    let checkout_svc = match &db {
        Some(pool) => CheckoutServiceImpl::with_db(store.clone(), pool.clone()),
        None => CheckoutServiceImpl::new(store.clone()),
    };
    let order_svc = match &db {
        Some(pool) => OrderServiceImpl::with_db(store.clone(), pool.clone()),
        None => OrderServiceImpl::new(store.clone()),
    };

    tracing::info!("rustygod-saleor gRPC listening on {addr}");

    // Background sweeper (reservations expiry + webhook outbox sender).
    if let Some(pool) = db.clone() {
        rustygod_server::sweeper::spawn(pool);
    }

    // Prometheus scrape endpoint (Phase 2 observability). Serves
    // grpc_requests_total + grpc_request_duration_seconds.
    let metrics_addr: std::net::SocketAddr = std::env::var("RUSTYGOD_METRICS_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:9000".to_string())
        .parse()?;
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(metrics_addr)
        .install()
        .map_err(|e| format!("metrics listener: {e}"))?;
    tracing::info!("prometheus metrics on {metrics_addr}");

    // GraphQL BFF (enterprise): same Postgres + same domain logic, thin
    // translation for the existing Dashboard (Apollo). Saleor's
    // `API_URL=http://localhost:8000/graphql/` points here.
    let gql_addr: std::net::SocketAddr = std::env::var("RUSTYGOD_GRAPHQL_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8000".to_string())
        .parse()?;
    let gql_schema = rustygod_graphql::build_schema(db.clone());
    let gql_app = {
        use axum::{routing::get, Router, Extension, Json};
        use async_graphql::http::GraphiQLSource;
        use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
        async fn handler(
            Extension(schema): Extension<rustygod_graphql::AppSchema>,
            headers: axum::http::HeaderMap,
            Json(mut req): Json<async_graphql::Request>,
        ) -> Json<async_graphql::Response> {
            // Debug log: first 500 chars of query so Dashboard mismatches are visible
            // in backend logs (RUST_LOG=info).
            tracing::info!(query = %req.query.chars().take(500).collect::<String>(), op = ?req.operation_name);
            if let Some(bearer) = headers.get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer ").or_else(|| s.strip_prefix("bearer ")))
                .map(|s| s.to_string())
            {
                req = req.data(rustygod_graphql::context::Bearer(bearer));
            }
            Json(schema.execute(req).await)
        }
        async fn graphiql() -> axum::response::Html<String> {
            axum::response::Html(GraphiQLSource::build().endpoint("/graphql").finish())
        }
        Router::new()
            .route("/graphql", get(graphiql).post(handler))
            .layer(
                CorsLayer::new()
                    .allow_origin(AllowOrigin::mirror_request())
                    .allow_credentials(true)
                    .allow_methods(AllowMethods::mirror_request())
                    .allow_headers(AllowHeaders::mirror_request()),
            )
            .layer(Extension(gql_schema))
    };
    let gql_listener = tokio::net::TcpListener::bind(gql_addr).await?;
    tracing::info!("GraphQL BFF listening on http://{gql_addr}/graphql (playground GET /graphql)");
    tokio::spawn(async move {
        axum::serve(gql_listener, gql_app).await.unwrap();
    });

    tonic::transport::Server::builder()
        .layer(tower_http::trace::TraceLayer::new_for_grpc())
        .layer(rustygod_server::telemetry::MetricsLayer::default())
        .add_service(ProductServiceServer::new(ProductServiceImpl::new(
            store.clone(),
            db.clone(),
        )))
        .add_service(CheckoutServiceServer::new(checkout_svc))
        .add_service(OrderServiceServer::new(order_svc))
        .add_service(DraftOrderServiceServer::new(DraftOrderServiceImpl::new(db.clone())))
        .add_service(InvoiceServiceServer::new(InvoiceServiceImpl::new(db.clone())))
        .add_service(PluginServiceServer::new(PluginServiceImpl::new(db.clone())))
        .add_service(PaymentServiceServer::new(PaymentServiceImpl::new(db.clone())))
        .add_service(WebhookServiceServer::new(WebhookServiceImpl::new(db.clone())))
        .add_service(AuthServiceServer::new(AuthServiceImpl::new(db.clone())))
        .add_service(DiscountServiceServer::new(DiscountServiceImpl::new(db.clone())))
        .add_service(ShippingServiceServer::new(ShippingServiceImpl::new(db.clone())))
        .add_service(GiftCardServiceServer::new(GiftCardServiceImpl::new(db.clone())))
        .add_service(MenuServiceServer::new(MenuServiceImpl::new(db.clone())))
        .add_service(PageServiceServer::new(PageServiceImpl::new(db.clone())))
        .add_service(AccountServiceServer::new(AccountServiceImpl::new(db.clone())))
        .add_service(ChannelServiceServer::new(ChannelServiceImpl::new(db.clone())))
        .add_service(TaxServiceServer::new(TaxServiceImpl::new(db.clone())))
        .add_service(WarehouseServiceServer::new(WarehouseServiceImpl::new(db.clone())))
        .add_service(SemanticSearchServer::new(SemanticSearchImpl::new(db.clone())))
        .add_service(RecommenderServer::new(RecommenderImpl::new(db.clone())))
        .add_service(ChatAgentServer::new(ChatAgentImpl::new(db.clone())))
        .serve(addr)
        .await?;

    Ok(())
}
