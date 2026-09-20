use rustygod_proto::{
    checkout::{
        checkout_service_client::CheckoutServiceClient, AddLinesRequest, CheckoutLine,
        CompleteCheckoutRequest, CreateCheckoutRequest,
    },
    order::{order_service_client::OrderServiceClient, GetOrderRequest},
    product::{product_service_client::ProductServiceClient, ListProductsRequest},
};

fn fail(msg: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg.into()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut products = ProductServiceClient::connect("http://127.0.0.1:50051").await?;
    let list = products
        .list_products(ListProductsRequest {
            channel: "default-channel".into(),
            first: 10,
            after: String::new(),
            category_id: String::new(),
        })
        .await?
        .into_inner();
    println!("products: {}", list.products.len());
    let first = list.products.first().ok_or_else(|| fail("empty catalog"))?;
    let variant = first
        .variants
        .first()
        .ok_or_else(|| fail("product has no variants"))?
        .id
        .clone();
    if variant.is_empty() {
        return Err(fail("seed variant missing"));
    }

    let mut checkout = CheckoutServiceClient::connect("http://127.0.0.1:50051").await?;
    let co = checkout
        .create_checkout(CreateCheckoutRequest {
            channel: "default-channel".into(),
            email: "buyer@example.com".into(),
        })
        .await?
        .into_inner()
        .checkout
        .ok_or_else(|| fail("create_checkout returned no checkout"))?;
    println!("checkout: {}", co.id);

    let co = checkout
        .add_lines(AddLinesRequest {
            checkout_id: co.id.clone(),
            lines: vec![CheckoutLine {
                variant_id: variant,
                quantity: 2,
                unit_price: None,
                total_price: None,
                is_gift: false,
            }],
        })
        .await?
        .into_inner()
        .checkout
        .ok_or_else(|| fail("add_lines returned no checkout"))?;
    let total = co.total.ok_or_else(|| fail("checkout has no total"))?;
    println!("checkout total: {} {}", total.amount, total.currency);

    let done = checkout
        .complete_checkout(CompleteCheckoutRequest {
            checkout_id: co.id.clone(),
        })
        .await?
        .into_inner();
    if !done.errors.is_empty() {
        return Err(fail(format!("complete failed: {:?}", done.errors)));
    }
    println!("order id: {}", done.order_id);

    let mut orders = OrderServiceClient::connect("http://127.0.0.1:50051").await?;
    let order = orders
        .get_order(GetOrderRequest {
            id: done.order_id.clone(),
        })
        .await?
        .into_inner()
        .order
        .ok_or_else(|| fail("order not found after complete"))?;
    let order_total = order.total.ok_or_else(|| fail("order has no total"))?;
    println!(
        "order {} status={} total={} {}",
        order.number, order.status, order_total.amount, order.currency
    );
    println!("SMOKE_OK");
    Ok(())
}
