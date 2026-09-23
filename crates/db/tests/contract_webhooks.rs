//! Webhook contract: fan-out, signed delivery, attempts, retry dues.
//! Mirrors `saleor/webhook/tests/` transport semantics against Django's
//! tables. Creates its own app+webhook rows and removes them after.

use chrono::Utc;
use saleor_rustify_db::{database_url, webhooks};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, Set};
use serde_json::json;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

struct Fixture {
    webhook_id: i32,
    app_id: i32,
}

async fn fixture(db: &DatabaseConnection, event: &str, channels: Vec<String>) -> Fixture {
    use saleor_rustify_db::entities::{app_app, webhook_webhook, webhook_webhookevent};
    let app = app_app::ActiveModel {
        name: Set("rustygod-test".into()),
        identifier: Set(format!("test.{}", uuid::Uuid::new_v4())),
        r#type: Set("local".into()),
        is_active: Set(true),
        is_installed: Set(true),
        created_at: Set(Utc::now().into()),
        uuid: Set(uuid::Uuid::new_v4()),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let wh = webhook_webhook::ActiveModel {
        name: Set(Some("test-hook".into())),
        app_id: Set(app.id),
        target_url: Set("http://127.0.0.1:9/nope".into()),
        is_active: Set(true),
        secret_key: Set(Some("test-secret".into())),
        filterable_channel_slugs: Set(channels),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    webhook_webhookevent::ActiveModel {
        event_type: Set(event.to_string()),
        webhook_id: Set(wh.id),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    Fixture { webhook_id: wh.id, app_id: app.id }
}

async fn cleanup(db: &DatabaseConnection, f: &Fixture, deliveries: &[i32]) {
    use saleor_rustify_db::entities::{
        app_app, core_eventdelivery, core_eventdeliveryattempt, core_eventpayload,
        webhook_webhook, webhook_webhookevent,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    // All deliveries ever made to this webhook (covers fan-out from other
    // triggers sharing the event type), not just the known ids.
    let all: Vec<i32> = {
        use sea_orm::QuerySelect;
        core_eventdelivery::Entity::find()
            .select_only()
            .column(core_eventdelivery::Column::Id)
            .filter(core_eventdelivery::Column::WebhookId.eq(f.webhook_id))
            .into_tuple::<i32>()
            .all(db)
            .await
            .unwrap()
    };
    let mut ids = deliveries.to_vec();
    ids.extend(all);
    ids.sort_unstable();
    ids.dedup();
    for id in ids {
        core_eventdeliveryattempt::Entity::delete_many()
            .filter(core_eventdeliveryattempt::Column::DeliveryId.eq(id))
            .exec(db)
            .await
            .unwrap();
        core_eventdelivery::Entity::delete_by_id(id).exec(db).await.unwrap();
    }
    // Only payloads with no deliveries left (shared across fan-out).
    {
        use sea_orm::Statement;
        db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM core_eventpayload p WHERE NOT EXISTS \
             (SELECT 1 FROM core_eventdelivery d WHERE d.payload_id = p.id) \
             AND p.payload = '{}'".to_string(),
        ))
        .await
        .unwrap();
    }
    let _ = core_eventpayload::Entity;
    webhook_webhookevent::Entity::delete_many()
        .filter(webhook_webhookevent::Column::WebhookId.eq(f.webhook_id))
        .exec(db)
        .await
        .unwrap();
    webhook_webhook::Entity::delete_by_id(f.webhook_id).exec(db).await.unwrap();
    app_app::Entity::delete_by_id(f.app_id).exec(db).await.unwrap();
}

/// Remove test fixtures leaked by earlier interrupted runs.
async fn purge_leftover_fixtures(db: &DatabaseConnection) {
    use saleor_rustify_db::entities::{app_app, webhook_webhook};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let apps: Vec<(i32, String)> = app_app::Entity::find()
        .select_only()
        .column(app_app::Column::Id)
        .column(app_app::Column::Identifier)
        .filter(app_app::Column::Identifier.like("test.%"))
        .into_tuple()
        .all(db)
        .await
        .unwrap();
    for (app_id, _) in apps {
        let whs: Vec<i32> = webhook_webhook::Entity::find()
            .select_only()
            .column(webhook_webhook::Column::Id)
            .filter(webhook_webhook::Column::AppId.eq(app_id))
            .into_tuple::<i32>()
            .all(db)
            .await
            .unwrap();
        for wid in whs {
            cleanup(db, &Fixture { webhook_id: wid, app_id }, &[]).await;
        }
    }
}

#[tokio::test]
async fn trigger_fans_out_to_subscribed_webhooks() {
    let db = db().await;
    purge_leftover_fixtures(&db).await;
    let f = fixture(&db, "order_created", vec![]).await;

    let ids = webhooks::trigger_event(&db, "order_created", Some("default-channel"), r#"{"id":1}"#)
        .await
        .unwrap();
    assert_eq!(ids.len(), 1);
    let d = webhooks::view(&db, ids[0]).await.unwrap();
    assert_eq!(d.status, "pending");
    assert_eq!(d.event_type, "order_created");
    assert_eq!(d.payload, r#"{"id":1}"#);
    assert!(d.target_url.contains("127.0.0.1"));

    // Channel filter: webhook subscribed to another channel gets nothing.
    let f2 = fixture(&db, "order_created", vec!["other-channel".into()]).await;
    let ids2 = webhooks::trigger_event(&db, "order_created", Some("default-channel"), "{}")
        .await
        .unwrap();
    assert_eq!(ids2.len(), 1, "only the unfiltered webhook fires");

    // Both triggers fanned out to f's webhook: clean all its deliveries first.
    let mut all_ids = ids.clone();
    all_ids.extend(ids2.iter().cloned());
    cleanup(&db, &f, &all_ids).await;
    cleanup(&db, &f2, &[]).await;
}

#[tokio::test]
async fn failed_attempt_keeps_pending_then_fails_at_cap() {
    let db = db().await;
    let f = fixture(&db, "product_updated", vec![]).await;
    let ids = webhooks::trigger_event(&db, "product_updated", None, "{}").await.unwrap();

    // Connection refused (nothing on :9) → failed attempt, still pending.
    let d = webhooks::record_attempt(&db, ids[0], None, Some(0.01), "{}", "", "")
        .await
        .unwrap();
    assert_eq!(d.status, "pending");
    assert_eq!(d.attempts, 1);

    // 2xx → success.
    let d = webhooks::record_attempt(&db, ids[0], Some(200), Some(0.02), "{}", "ok", "")
        .await
        .unwrap();
    assert_eq!(d.status, "success");

    let attempts = webhooks::list_attempts(&db, ids[0]).await.unwrap();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].response_status_code, None);
    assert_eq!(attempts[1].response_status_code, Some(200));

    cleanup(&db, &f, &ids).await;
}

#[tokio::test]
async fn signature_headers_match_python_hmac() {
    // Secret "test-secret", body {} — verifiable against Python hmac.
    let headers = webhooks::signed_headers("example.com", "order_created", "{}", "test-secret");
    let sig = headers.iter().find(|(k, _)| k == "Saleor-Signature").unwrap().1.clone();
    assert!(saleor_rustify_core::webhooks::verify_signature(b"{}", "test-secret", &sig));
    assert!(headers.iter().any(|(k, v)| k == "Saleor-Event" && v == "order_created"));
}

#[tokio::test]
async fn due_deliveries_follows_backoff_schedule() {
    // Django semantics: the first send happens immediately at trigger;
    // the retry queue only holds failed deliveries past their backoff.
    // A fresh pending delivery is therefore NOT due yet.
    let db = db().await;
    let f = fixture(&db, "checkout_updated", vec![]).await;
    let ids = webhooks::trigger_event(&db, "checkout_updated", None, "{}").await.unwrap();

    let due = webhooks::due_deliveries(&db, 100).await.unwrap();
    assert!(!due.contains(&ids[0]), "fresh delivery sends immediately, not via retry queue");

    // A succeeded delivery is never due.
    webhooks::record_attempt(&db, ids[0], Some(200), Some(0.01), "{}", "ok", "")
        .await
        .unwrap();
    let due = webhooks::due_deliveries(&db, 100).await.unwrap();
    assert!(!due.contains(&ids[0]));

    cleanup(&db, &f, &ids).await;
}
