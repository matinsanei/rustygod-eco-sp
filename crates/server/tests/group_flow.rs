//! AccountService groups over gRPC: CRUD, members, grants —
//! staff-gated, on real tables.

use rustygod_db::database_url;
use rustygod_proto::commerce::{
    account_service_server::AccountService, CreateGroupRequest, DeleteGroupRequest,
    GroupMembersRequest, GroupPermissionsRequest,
};
use rustygod_server::service_commerce::AccountServiceImpl;
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
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let (user, _) = rustygod_db::auth::find_for_login(&db, "admin@example.com")
        .await
        .unwrap()
        .unwrap();
    let pair = rustygod_core::auth::mint_tokens_with_key(
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

async fn svc() -> AccountServiceImpl {
    let db = rustygod_db::connect(&database_url()).await.unwrap();
    AccountServiceImpl::new(Some(db))
}

#[tokio::test]
async fn grpc_group_lifecycle() {
    std::env::set_var("RSA_PRIVATE_KEY", test_key());
    let svc = svc().await;

    let err = svc
        .create_group(Request::new(CreateGroupRequest {
            name: "X".into(),
            permissions: vec![],
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);

    let name = format!("ci-g-{}", &Uuid::new_v4().to_string()[..8]);
    let g = svc
        .create_group(
            staff_req(CreateGroupRequest {
                name: name.clone(),
                permissions: vec!["manage_orders".into()],
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner()
        .group
        .unwrap();
    assert_eq!(g.permissions, vec!["manage_orders"]);

    let db = rustygod_db::connect(&database_url()).await.unwrap();
    let (admin, _) = rustygod_db::auth::find_for_login(&db, "admin@example.com")
        .await
        .unwrap()
        .unwrap();
    let g = svc
        .add_group_members(
            staff_req(GroupMembersRequest {
                id: g.id.clone(),
                user_ids: vec![admin.id.to_string()],
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner()
        .group
        .unwrap();
    assert_eq!(g.member_ids, vec![admin.id.to_string()]);

    let g = svc
        .grant_group_permissions(
            staff_req(GroupPermissionsRequest {
                id: g.id.clone(),
                codenames: vec!["manage_products".into()],
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner()
        .group
        .unwrap();
    assert!(g.permissions.contains(&"manage_products".to_string()));

    let g = svc
        .revoke_group_permissions(
            staff_req(GroupPermissionsRequest {
                id: g.id.clone(),
                codenames: vec!["manage_products".into()],
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner()
        .group
        .unwrap();
    assert!(!g.permissions.contains(&"manage_products".to_string()));

    let g = svc
        .remove_group_members(
            staff_req(GroupMembersRequest {
                id: g.id.clone(),
                user_ids: vec![admin.id.to_string()],
            })
            .await,
        )
        .await
        .unwrap()
        .into_inner()
        .group
        .unwrap();
    assert!(g.member_ids.is_empty());

    let del = svc
        .delete_group(staff_req(DeleteGroupRequest { id: g.id.clone() }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(del.ok);
}
