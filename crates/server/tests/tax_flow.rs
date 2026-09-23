//! TaxService over gRPC: rate lookup + taxed lines on real data.

use saleor_rustify_db::database_url;
use saleor_rustify_proto::commerce::{
    tax_service_server::TaxService, CalculateTaxesRequest, GetTaxRateRequest, TaxLineInput,
};
use saleor_rustify_server::service_commerce::TaxServiceImpl;
use tonic::Request;

async fn svc() -> TaxServiceImpl {
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    TaxServiceImpl::new(Some(db))
}

#[tokio::test]
async fn grpc_tax_rate_and_calculation() {
    let svc = svc().await;

    let rate = svc
        .get_tax_rate(Request::new(GetTaxRateRequest {
            variant_id: String::new(),
            country: "DE".into(),
            channel: "default-channel".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(rate.errors.is_empty());
    assert_eq!(rate.rate, "19");
    assert!(rate.charge_taxes);

    let unknown = svc
        .get_tax_rate(Request::new(GetTaxRateRequest {
            variant_id: String::new(),
            country: "XX".into(),
            channel: "default-channel".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(unknown.rate, "0");

    // Tax a real listed variant: 2 × unit @ 19% DE.
    let db = saleor_rustify_db::connect(&database_url()).await.unwrap();
    let products = saleor_rustify_db::catalog::list_products(&db, "default-channel", None, 5)
        .await
        .unwrap();
    let vid: i32 = products.iter().flat_map(|p| &p.variants).next().unwrap().id.parse().unwrap();
    let pricing = saleor_rustify_db::catalog::checkout_pricing(&db, "default-channel", &[vid])
        .await
        .unwrap();
    let unit = pricing[&vid].0.amount.to_string();

    let calc = svc
        .calculate_taxes(Request::new(CalculateTaxesRequest {
            channel: "default-channel".into(),
            country: "DE".into(),
            lines: vec![TaxLineInput { variant_id: vid.to_string(), unit_price: unit, quantity: 2 }],
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(calc.errors.is_empty());
    assert_eq!(calc.lines.len(), 1);
    assert_eq!(calc.lines[0].tax_rate, "19");
    assert_eq!(calc.total_net, calc.lines[0].total_net);
    assert_eq!(calc.total_gross, calc.lines[0].total_gross);
    assert!(calc.total_gross.parse::<rust_decimal::Decimal>().unwrap()
        > calc.total_net.parse::<rust_decimal::Decimal>().unwrap());
}
