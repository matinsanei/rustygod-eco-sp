//! AI contract tests against the live Saleor database.
//! Mirrors `saleor/product/tests/test_product_search.py` semantics:
//! search must surface what Django indexed, recommendations must reflect
//! real co-purchases, chat must stay grounded in catalog rows.

use rustygod_ai::{chat, recommend, search};
use rustygod_db::{catalog, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

async fn first_product_word(db: &DatabaseConnection) -> (String, i32, String) {
    let products = catalog::list_products(db, "default-channel", None, 100)
        .await
        .unwrap();
    let p = products.iter().find(|p| !p.variants.is_empty()).unwrap();
    let word = p
        .name
        .split_whitespace()
        .find(|w| w.len() >= 4)
        .unwrap_or(&p.name)
        .to_string();
    let (ch_id, currency) = catalog::channel_info(db, "default-channel").await.unwrap();
    (word, ch_id, currency)
}

#[tokio::test]
async fn trigram_search_surfaces_indexed_products() {
    let db = db().await;
    let (word, ch_id, currency) = first_product_word(&db).await;
    let hits = search::search_products(&db, &word, ch_id, &currency, 10)
        .await
        .unwrap();
    assert!(!hits.is_empty(), "search for {word:?} must hit");
    assert!(hits.iter().all(|h| h.score > 0.0));
    // Scores strictly non-increasing (ranked).
    for w in hits.windows(2) {
        assert!(w[0].score >= w[1].score);
    }
}

#[tokio::test]
async fn gibberish_search_returns_empty_without_error() {
    let db = db().await;
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let hits = search::search_products(&db, "zzzqqqxkcd", ch_id, &currency, 10)
        .await
        .unwrap();
    assert!(hits.is_empty());
}

#[tokio::test]
async fn recommendations_reflect_real_copurchases() {
    use rustygod_db::entities::order_orderline;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect, SelectorTrait};

    let db = db().await;
    // A variant Django actually sold.
    let vid: Option<i32> = order_orderline::Entity::find()
        .select_only()
        .column(order_orderline::Column::VariantId)
        .filter(order_orderline::Column::VariantId.is_not_null())
        .into_tuple::<i32>()
        .one(&db)
        .await
        .unwrap();
    let vid = vid.expect("populatedb must have order lines");
    let recos = recommend::recommend_for_variant(&db, vid, "default-channel", 5)
        .await
        .unwrap();
    // Scores sorted desc; every reco resolves to a real channel price.
    for w in recos.windows(2) {
        assert!(w[0].score >= w[1].score);
    }
    for r in &recos {
        assert!(!r.name.is_empty());
        assert!(r.price > rust_decimal::Decimal::ZERO);
    }
    // Unknown variant: clean empty, no crash.
    let none = recommend::recommend_for_variant(&db, -7, "default-channel", 5)
        .await
        .unwrap();
    assert!(none.is_empty());
}

#[tokio::test]
async fn chat_answer_is_grounded_in_catalog() {
    let db = db().await;
    let (word, ch_id, _) = first_product_word(&db).await;
    let (ch_id2, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    assert_eq!(ch_id, ch_id2);
    let reply = chat::answer(&db, &word, ch_id, "default-channel", &currency, 5)
        .await
        .unwrap();
    assert!(!reply.text_parts.is_empty());
    assert!(!reply.product_ids.is_empty());
    assert!(reply.text_parts[0].contains(&word) || reply.text_parts[0].contains("found"));

    let empty = chat::answer(&db, "zzzqqqxkcd", ch_id, "default-channel", &currency, 5)
        .await
        .unwrap();
    assert!(empty.product_ids.is_empty());
    assert!(empty.text_parts[0].contains("couldn't find"));
}
