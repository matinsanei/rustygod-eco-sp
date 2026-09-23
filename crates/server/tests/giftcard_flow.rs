//! DB-mode gift-card flow over gRPC handlers: issue → attach →
//! balance → redeem → refund, all against real Postgres, mirroring
//! `saleor/giftcard/tests/` checkout/order integration.

use saleor_rustify_db::{catalog, checkout_store, database_url};
use saleor_rustify_proto::giftcard::{
    gift_card_service_server::GiftCardService, BalanceMutationRequest, CheckoutBalanceRequest,
    CheckoutGiftCardRequest, GetGiftCardRequest, IssueGiftCardRequest, RedeemGiftCardRequest,
    SetActiveRequest,
};
use saleor_rustify_server::service_giftcard::GiftCardServiceImpl;
use tonic::Request;

fn test_key() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../db/tests/testdata/test_rsa.pem"
    ))
    .expect("test RSA key must exist")
}

/// Staff request: populatedb admin JWT in metadata (staff RPCs demand
/// MANAGE_GIFT_CARD since access control landed).
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

async fn svc() -> GiftCardServiceImpl {
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    GiftCardServiceImpl::new(Some(db))
}

async fn fresh_checkout() -> String {
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    checkout_store::create_checkout_row(&db, ch_id, &currency, "gc-buyer@example.com")
        .await
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn grpc_giftcard_lifecycle() {
    let svc = svc().await;
    let checkout_id = fresh_checkout().await;

    // Issue.
    let card = svc
        .issue(staff_req(IssueGiftCardRequest {
            initial_balance: "75.50".into(),
            currency: "USD".into(),
            created_by_email: "staff@example.com".into(),
            expiry_date: String::new(),
        })
        .await)
        .await
        .unwrap()
        .into_inner()
        .gift_card
        .unwrap();
    assert_eq!(card.current_balance, "75.500");
    assert!(card.is_active);

    // GetByCode roundtrip.
    let same = svc
        .get_by_code(Request::new(GetGiftCardRequest { code: card.code.clone() }))
        .await
        .unwrap()
        .into_inner()
        .gift_card
        .unwrap();
    assert_eq!(same.code, card.code);

    // Attach + balance.
    let attached = svc
        .attach_to_checkout(Request::new(CheckoutGiftCardRequest {
            checkout_id: checkout_id.clone(),
            code: card.code.clone(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(attached.errors.is_empty());
    let bal = svc
        .checkout_balance(Request::new(CheckoutBalanceRequest {
            checkout_id: checkout_id.clone(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(bal.currency, "USD");
    assert_eq!(bal.balance, "75.500");

    // Redeem partial.
    let redeemed = svc
        .redeem(Request::new(RedeemGiftCardRequest {
            code: card.code.clone(),
            order_id: String::new(),
            amount: "25.50".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(redeemed.errors.is_empty());
    assert_eq!(redeemed.amount_taken, "25.50");
    assert_eq!(redeemed.gift_card.unwrap().current_balance, "50.000");

    // Refund back.
    let back = svc
        .refund(Request::new(BalanceMutationRequest {
            code: card.code.clone(),
            amount: "5.50".into(),
            order_id: String::new(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(back.errors.is_empty());
    assert_eq!(back.gift_card.unwrap().current_balance, "55.500");

    // Detach, then balance drops to zero.
    let detached = svc
        .detach_from_checkout(Request::new(CheckoutGiftCardRequest {
            checkout_id: checkout_id.clone(),
            code: card.code.clone(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(detached.errors.is_empty());
    let bal = svc
        .checkout_balance(Request::new(CheckoutBalanceRequest { checkout_id }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(bal.balance, "0");

    // Deactivate blocks redeem.
    let off = svc
        .set_active(staff_req(SetActiveRequest { code: card.code.clone(), active: false }).await)
        .await
        .unwrap()
        .into_inner();
    assert!(!off.gift_card.unwrap().is_active);
    let denied = svc
        .redeem(Request::new(RedeemGiftCardRequest {
            code: card.code.clone(),
            order_id: String::new(),
            amount: "1.00".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(!denied.errors.is_empty());
    assert_eq!(denied.errors[0].code, "NOT_APPLICABLE");
}
