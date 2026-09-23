use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use saleor_rustify_core::{checkout::Checkout, money::Money, order::Order, product::Product};
use tonic::Status;

/// Shared in-memory store. Tonight's stand-in for Postgres —
/// same struct shapes as the future sqlx repositories, so the swap is mechanical.
#[derive(Debug, Default)]
pub struct Store {
    pub products: HashMap<String, Product>,
    pub checkouts: HashMap<String, Checkout>,
    pub orders: HashMap<String, Order>,
    pub order_seq: u64,
}

pub type SharedStore = Arc<Mutex<Store>>;

pub fn new_store() -> SharedStore {
    Arc::new(Mutex::new(Store::default()))
}

/// Lock the store, mapping poisoning to a gRPC error instead of panicking.
pub fn lock(store: &SharedStore) -> Result<MutexGuard<'_, Store>, Status> {
    store
        .lock()
        .map_err(|e| Status::internal(format!("store lock poisoned: {e}")))
}

fn usd(amount: rust_decimal::Decimal) -> Money {
    Money::new(amount, "USD")
}

/// Seed demo catalog (mirrors `populatedb` defaults: USD channel).
/// Amounts are const-constructed — infallible by construction.
pub fn seed(store: &SharedStore) {
    let mut s = match store.lock() {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("seed failed: store lock poisoned: {e}");
            return;
        }
    };

    let p1 = Product {
        id: "prod_1".into(),
        name: "Rusty T-Shirt".into(),
        slug: "rusty-t-shirt".into(),
        description: "Official rustygod tee".into(),
        product_type_id: "pt_apparel".into(),
        category_id: Some("cat_apparel".into()),
        is_published: true,
        default_price: usd(rust_decimal::Decimal::new(2999, 2)),
        variants: vec![saleor_rustify_core::product::ProductVariant {
            id: "var_1".into(),
            product_id: "prod_1".into(),
            name: "M / Black".into(),
            sku: "RUST-TEE-M-BLK".into(),
            price: usd(rust_decimal::Decimal::new(2999, 2)),
            quantity_available: 100,
        }],
    };
    let p2 = Product {
        id: "prod_2".into(),
        name: "gRPC Mug".into(),
        slug: "grpc-mug".into(),
        description: "No GraphQL was harmed".into(),
        product_type_id: "pt_merch".into(),
        category_id: Some("cat_merch".into()),
        is_published: true,
        default_price: usd(rust_decimal::Decimal::new(1450, 2)),
        variants: vec![saleor_rustify_core::product::ProductVariant {
            id: "var_2".into(),
            product_id: "prod_2".into(),
            name: "Standard".into(),
            sku: "GRPC-MUG-STD".into(),
            price: usd(rust_decimal::Decimal::new(1450, 2)),
            quantity_available: 250,
        }],
    };
    s.products.insert(p1.id.clone(), p1);
    s.products.insert(p2.id.clone(), p2);
}
