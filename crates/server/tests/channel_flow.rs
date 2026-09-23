//! ChannelService writes over gRPC: create/update/delete, listings,
//! prices — staff-gated, on real tables.

use saleor_rustify_db::database_url;
use saleor_rustify_proto::commerce::{
    channel_service_server::ChannelService, CreateChannelRequest, DeleteChannelRequest,
    SetProductListingRequest, SetVariantPriceRequest, UpdateChannelRequest,
};
use saleor_rustify_server::service_commerce::ChannelServiceImpl;
use tonic::Request;
use uuid::Uuid;

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

async fn svc() -> ChannelServiceImpl {
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    ChannelServiceImpl::new(Some(db))
}

#[tokio::test]
async fn grpc_channel_lifecycle() {
    std::env::set_var("RSA_PRIVATE_KEY", test_key());
    let svc = svc().await;

    let err = svc
        .create_channel(Request::new(CreateChannelRequest {
            name: "X".into(),
            slug: "x".into(),
            currency_code: "USD".into(),
            default_country: "US".into(),
            allocation_strategy: String::new(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);

    let slug = format!("ci-grpc-{}", &Uuid::new_v4().to_string()[..8]);
    let ch = svc
        .create_channel(
            staff_req(CreateChannelRequest {
                name: "CI gRPC".into(),
                slug: slug.clone(),
                currency_code: "EUR".into(),
                default_country: "DE".into(),
                allocation_strategy: String::new(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner()
        .channel
        .unwrap();
    assert_eq!(ch.slug, slug);
    assert_eq!(ch.currency_code, "EUR");

    let upd = svc
        .update_channel(
            staff_req(UpdateChannelRequest {
                slug: slug.clone(),
                name: "CI gRPC 2".into(),
                is_active: false,
                set_active: true,
                default_country: String::new(),
                allocation_strategy: String::new(),
                auto_confirm: false,
                set_auto_confirm: false,
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner()
        .channel
        .unwrap();
    assert!(!upd.is_active);

    // Listings on the live channel (use a non-353 variant; restore after).
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    let products = saleor_rustify_db::catalog::list_products(&db, "default-channel", None, 50)
        .await
        .unwrap();
    let (pid, vid, orig) = products
        .iter()
        .flat_map(|p| p.variants.iter().map(|v| (p.id.clone(), v)))
        .filter(|(_, v)| v.id != "353")
        .map(|(pid, v)| (pid.parse::<i32>().unwrap(), v.id.parse::<i32>().unwrap(), v.price.amount))
        .next()
        .unwrap();
    let res = svc
        .set_product_listing(
            staff_req(SetProductListingRequest {
                channel: "default-channel".into(),
                product_id: pid.to_string(),
                is_published: true,
                visible_in_listings: true,
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(res.errors.is_empty());

    let res = svc
        .set_variant_price(
            staff_req(SetVariantPriceRequest {
                channel: "default-channel".into(),
                variant_id: vid.to_string(),
                price: "11.11".into(),
                cost_price: String::new(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(res.errors.is_empty());
    let pricing = saleor_rustify_db::catalog::checkout_pricing(&db, "default-channel", &[vid])
        .await
        .unwrap();
    assert_eq!(
        pricing[&vid].0.amount,
        "11.11".parse::<rust_decimal::Decimal>().unwrap()
    );
    // Restore shared data.
    saleor_rustify_db::channels::set_variant_price(&db, "default-channel", vid, Some(orig), None)
        .await
        .unwrap();

    // Fresh channel deletes; live one refuses (covered at db level).
    let del = svc
        .delete_channel(staff_req(DeleteChannelRequest { slug: slug.clone() }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(del.ok);
}
