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

    tonic::transport::Server::builder()
        .add_service(ProductServiceServer::new(ProductServiceImpl::new(
            store.clone(),
            db.clone(),
        )))
        .add_service(CheckoutServiceServer::new(checkout_svc))
        .add_service(OrderServiceServer::new(order_svc))
        .add_service(DraftOrderServiceServer::new(DraftOrderServiceImpl::new(db.clone())))
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
