fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "proto/common.proto",
                "proto/product.proto",
                "proto/checkout.proto",
                "proto/order.proto",
                "proto/commerce.proto",
                "proto/ai.proto",
                "proto/payment.proto",
                "proto/webhook.proto",
                "proto/auth.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
