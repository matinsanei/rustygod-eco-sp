//! WarehouseService writes over gRPC: create/update/delete, stock
//! upsert, zone links — staff-gated, on real tables.

use saleor_rustify_db::database_url;
use saleor_rustify_proto::commerce::{
    warehouse_service_server::WarehouseService, CreateWarehouseRequest, DeleteWarehouseRequest,
    ListZonesRequest, UpdateWarehouseRequest, UpsertStockRequest, ZoneLinkRequest,
};
use saleor_rustify_server::service_commerce::WarehouseServiceImpl;
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

async fn svc() -> WarehouseServiceImpl {
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    WarehouseServiceImpl::new(Some(db))
}

#[tokio::test]
async fn grpc_warehouse_writes() {
    std::env::set_var("RSA_PRIVATE_KEY", test_key());
    let svc = svc().await;

    // Anonymous create is rejected.
    let err = svc
        .create_warehouse(Request::new(CreateWarehouseRequest {
            name: "X".into(),
            slug: "x".into(),
            email: String::new(),
            street: String::new(),
            city: String::new(),
            postal_code: String::new(),
            country: "DE".into(),
            is_private: false,
            cc_option: String::new(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);

    let slug = format!("ci-grpc-{}", &Uuid::new_v4().to_string()[..8]);
    let wh = svc
        .create_warehouse(
            staff_req(CreateWarehouseRequest {
                name: "CI gRPC".into(),
                slug: slug.clone(),
                email: "g@example.com".into(),
                street: "1 Test St".into(),
                city: "Berlin".into(),
                postal_code: "10115".into(),
                country: "DE".into(),
                is_private: false,
                cc_option: "all".into(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner()
        .warehouse
        .unwrap();
    assert_eq!(wh.slug, slug);

    // Zones list + link.
    let zones = svc
        .list_zones(Request::new(ListZonesRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert!(!zones.zones.is_empty());
    let link = svc
        .assign_zone(
            staff_req(ZoneLinkRequest {
                warehouse_id: wh.id.clone(),
                zone_id: zones.zones[0].id.clone(),
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner();
    assert!(link.ok);

    // Stock upsert on the new warehouse.
    use saleor_rustify_db::entities::product_productvariant;
    use sea_orm::{EntityTrait, QuerySelect};
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    let vid: i32 = product_productvariant::Entity::find()
        .select_only()
        .column(product_productvariant::Column::Id)
        .into_tuple()
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let stock = svc
        .upsert_stock(
            staff_req(UpsertStockRequest {
                warehouse_id: wh.id.clone(),
                variant_id: vid.to_string(),
                quantity: 7,
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner()
        .stock
        .unwrap();
    assert_eq!((stock.quantity, stock.quantity_allocated), (7, 0));

    // Update + stocked-delete refusal.
    let upd = svc
        .update_warehouse(
            staff_req(UpdateWarehouseRequest {
                id: wh.id.clone(),
                name: "CI gRPC 2".into(),
                email: String::new(),
                cc_option: String::new(),
                is_private: false,
                set_private: false,
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner()
        .warehouse
        .unwrap();
    assert_eq!(upd.name, "CI gRPC 2");

    let denied = svc
        .delete_warehouse(staff_req(DeleteWarehouseRequest { id: wh.id.clone() }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(!denied.ok);

    // Cleanup via deep delete (test-only).
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    saleor_rustify_db::warehouses::delete_warehouse_deep(&db, wh.id.parse().unwrap())
        .await
        .unwrap();
}
