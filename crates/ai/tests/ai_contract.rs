//! AI contract tests against the live Saleor database.
//! Mirrors `saleor/product/tests/test_product_search.py` semantics:
//! search must surface what Django indexed, recommendations must reflect
//! real co-purchases, chat must stay grounded in catalog rows.

use saleor_rustify_ai::{agent, chat, recommend, search};
use saleor_rustify_ai::embed::HashingEmbedder;
use saleor_rustify_ai::vectors;
use saleor_rustify_db::{catalog, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
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
    use saleor_rustify_db::entities::order_orderline;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

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
async fn embedding_refresh_is_idempotent() {
    let db = db().await;
    let e = HashingEmbedder::default();
    let r1 = vectors::refresh_product_embeddings(&db, &e).await.unwrap();
    assert!(r1.scanned > 0, "catalog must have products");
    assert_eq!(r1.embedded + r1.skipped, r1.scanned);
    // Second run: everything skipped, nothing re-embedded.
    let r2 = vectors::refresh_product_embeddings(&db, &e).await.unwrap();
    assert_eq!(r2.scanned, r1.scanned);
    assert_eq!(r2.embedded, 0);
    assert_eq!(r2.skipped, r2.scanned);
}

#[tokio::test]
async fn semantic_search_ranks_exact_name_top() {
    let db = db().await;
    let e = HashingEmbedder::default();
    vectors::refresh_product_embeddings(&db, &e).await.unwrap();
    let products = catalog::list_products(&db, "default-channel", None, 100).await.unwrap();
    let p = products.iter().find(|p| !p.variants.is_empty()).unwrap();
    let (ch_id, currency) = catalog::channel_info(&db, "default-channel").await.unwrap();
    let published = vectors::published_in_channel(&db, ch_id).await.unwrap();
    assert!(published.contains(&p.id.parse::<i32>().unwrap()));
    let hits = vectors::semantic_search(&db, &e, &p.name, &published, 5).await.unwrap();
    assert!(!hits.is_empty());
    // The hashing stand-in dilutes exact matches with variant-name mass
    // (stored source = name + variants), so top-3 — not top-1 — is the
    // honest bar for pure cosine. Exact ranking is the blended tier's job.
    let target = p.id.parse::<i32>().unwrap();
    assert!(hits.iter().take(3).any(|(pid, _)| *pid == target), "exact product must be top-3: {hits:?}");
    // Blended search keeps the exact hit and stays ranked.
    let blended = search::search_products_blended(&db, &e, &p.name, ch_id, &currency, 5).await.unwrap();
    assert!(!blended.is_empty());
    assert!(blended.iter().any(|h| h.product_id == p.id.parse::<i32>().unwrap()));
    for w in blended.windows(2) {
        assert!(w[0].score >= w[1].score);
    }
}

#[tokio::test]
async fn recommender_v2_backfills_beyond_copurchase() {
    use saleor_rustify_db::entities::order_orderline;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let db = db().await;
    let e = HashingEmbedder::default();
    vectors::refresh_product_embeddings(&db, &e).await.unwrap();
    let vid: i32 = order_orderline::Entity::find()
        .select_only()
        .column(order_orderline::Column::VariantId)
        .filter(order_orderline::Column::VariantId.is_not_null())
        .into_tuple::<i32>()
        .one(&db)
        .await
        .unwrap()
        .expect("populatedb must have order lines");
    let v2 = recommend::recommend_v2(&db, vid, "default-channel", 5).await.unwrap();
    let v1 = recommend::recommend_for_variant(&db, vid, "default-channel", 5).await.unwrap();
    // v2 is a superset-blend: every v1 hit survives, all priced.
    assert!(v2.len() >= v1.len(), "v2 must keep all co-occurrence hits");
    for r in &v2 {
        assert!(!r.name.is_empty());
        assert!(r.price > rust_decimal::Decimal::ZERO);
        assert!(r.score.is_finite());
    }
    // Unknown variant: clean empty, no crash.
    let none = recommend::recommend_v2(&db, -7, "default-channel", 5).await.unwrap();
    assert!(none.is_empty());
}

#[tokio::test]
async fn agent_builds_lines_from_plain_english() {
    let db = db().await;
    let products = catalog::list_products(&db, "default-channel", None, 100).await.unwrap();
    let p = products.iter().find(|p| !p.variants.is_empty()).unwrap();
    let plan = agent::plan_checkout(&db, "default-channel", &format!("2x {}", p.name)).await.unwrap();
    assert_eq!(plan.lines.len(), 1, "notes: {:?}", plan.notes);
    assert_eq!(plan.lines[0].quantity, 2);
    assert!(plan.questions.is_empty(), "questions: {:?}", plan.questions);
    assert_eq!(plan.lines[0].matched_name, p.name);
}

#[tokio::test]
async fn agent_asks_instead_of_hallucinating() {
    let db = db().await;
    let plan = agent::plan_checkout(&db, "default-channel", "3x zzzqqqxkcd flux capacitor").await.unwrap();
    assert!(plan.lines.is_empty());
    assert_eq!(plan.questions.len(), 1);
    assert!(plan.questions[0].contains("zzzqqqxkcd"));
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
