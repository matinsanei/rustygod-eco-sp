//! End-to-end behavioral contract for the checkout → order flow,
//! mirroring `saleor/checkout/tests/test_checkout_complete.py` and
//! `saleor/checkout/tests/test_order_from_checkout.py`:
//! completing a checkout consumes it exactly once and mints an order whose
//! lines and totals match the checkout.

use rustygod_proto::{
    checkout::{
        checkout_service_server::CheckoutService, AddLinesRequest, CheckoutLine,
        CompleteCheckoutRequest, CreateCheckoutRequest,
    },
    order::{order_service_server::OrderService, GetOrderRequest, ListOrdersRequest},
};
use rustygod_server::{
    service_checkout::CheckoutServiceImpl, service_order::OrderServiceImpl,
    store::{new_store, seed},
};
use tonic::Request;

fn services() -> (CheckoutServiceImpl, OrderServiceImpl) {
    let store = new_store();
    seed(&store);
    (
        CheckoutServiceImpl::new(store.clone()),
        OrderServiceImpl::new(store),
    )
}

#[tokio::test]
async fn complete_checkout_mints_matching_order() {
    let (checkout_svc, order_svc) = services();

    let co = checkout_svc
        .create_checkout(Request::new(CreateCheckoutRequest {
            channel: "default-channel".into(),
            email: "buyer@example.com".into(),
        }))
        .await
        .unwrap()
        .into_inner()
        .checkout
        .unwrap();

    let co = checkout_svc
        .add_lines(Request::new(AddLinesRequest {
            checkout_id: co.id.clone(),
            lines: vec![CheckoutLine {
                variant_id: "var_1".into(),
                quantity: 2,
                unit_price: None,
                total_price: None,
            }],
        }))
        .await
        .unwrap()
        .into_inner()
        .checkout
        .unwrap();
    assert_eq!(co.total.unwrap().amount, "59.98");

    let done = checkout_svc
        .complete_checkout(Request::new(CompleteCheckoutRequest {
            checkout_id: co.id.clone(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(done.errors.is_empty());
    assert!(!done.order_id.is_empty());

    let order = order_svc
        .get_order(Request::new(GetOrderRequest {
            id: done.order_id.clone(),
        }))
        .await
        .unwrap()
        .into_inner()
        .order
        .unwrap();
    assert_eq!(order.status, "unfulfilled");
    assert_eq!(order.total.unwrap().amount, "59.98");
    assert_eq!(order.lines.len(), 1);
    assert_eq!(order.lines[0].quantity, 2);
    assert!(order.number.starts_with("RG-"));
}

#[tokio::test]
async fn checkout_is_consumed_exactly_once() {
    // Mirrors Saleor: completing twice must fail the second time.
    let (checkout_svc, _) = services();
    let co = checkout_svc
        .create_checkout(Request::new(CreateCheckoutRequest {
            channel: "default-channel".into(),
            email: "x@example.com".into(),
        }))
        .await
        .unwrap()
        .into_inner()
        .checkout
        .unwrap();
    checkout_svc
        .add_lines(Request::new(AddLinesRequest {
            checkout_id: co.id.clone(),
            lines: vec![CheckoutLine {
                variant_id: "var_2".into(),
                quantity: 1,
                unit_price: None,
                total_price: None,
            }],
        }))
        .await
        .unwrap();

    let first = checkout_svc
        .complete_checkout(Request::new(CompleteCheckoutRequest {
            checkout_id: co.id.clone(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(first.errors.is_empty());

    let second = checkout_svc
        .complete_checkout(Request::new(CompleteCheckoutRequest {
            checkout_id: co.id.clone(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(!second.errors.is_empty());
    assert_eq!(second.errors[0].code, "NOT_FOUND");
}

#[tokio::test]
async fn unknown_variant_is_rejected() {
    // Mirrors checkout-cleaner behavior: bad lines never enter the checkout.
    let (checkout_svc, _) = services();
    let co = checkout_svc
        .create_checkout(Request::new(CreateCheckoutRequest {
            channel: "default-channel".into(),
            email: "x@example.com".into(),
        }))
        .await
        .unwrap()
        .into_inner()
        .checkout
        .unwrap();

    let res = checkout_svc
        .add_lines(Request::new(AddLinesRequest {
            checkout_id: co.id.clone(),
            lines: vec![CheckoutLine {
                variant_id: "nope".into(),
                quantity: 1,
                unit_price: None,
                total_price: None,
            }],
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(res.checkout.is_none());
    assert_eq!(res.errors[0].code, "NOT_FOUND");
}

#[tokio::test]
async fn orders_list_grows_with_completions() {
    let (checkout_svc, order_svc) = services();
    let before = order_svc
        .list_orders(Request::new(ListOrdersRequest {
            first: 100,
            after: String::new(),
            status: String::new(),
        }))
        .await
        .unwrap()
        .into_inner()
        .orders
        .len();

    for _ in 0..2 {
        let co = checkout_svc
            .create_checkout(Request::new(CreateCheckoutRequest {
                channel: "default-channel".into(),
                email: "x@example.com".into(),
            }))
            .await
            .unwrap()
            .into_inner()
            .checkout
            .unwrap();
        checkout_svc
            .add_lines(Request::new(AddLinesRequest {
                checkout_id: co.id.clone(),
                lines: vec![CheckoutLine {
                    variant_id: "var_1".into(),
                    quantity: 1,
                    unit_price: None,
                    total_price: None,
                }],
            }))
            .await
            .unwrap();
        checkout_svc
            .complete_checkout(Request::new(CompleteCheckoutRequest {
                checkout_id: co.id.clone(),
            }))
            .await
            .unwrap();
    }

    let after = order_svc
        .list_orders(Request::new(ListOrdersRequest {
            first: 100,
            after: String::new(),
            status: String::new(),
        }))
        .await
        .unwrap()
        .into_inner()
        .orders
        .len();
    assert_eq!(after, before + 2);
}
