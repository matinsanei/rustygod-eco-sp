//! CSV exports: gift cards, voucher codes, products.

use saleor_rustify_db::{database_url, exports};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn export_jobs_produce_files() {
    let db = db().await;
    let g = exports::export_gift_cards(&db, None, None).await.unwrap();
    let v = exports::export_voucher_codes(&db, None, None, None).await.unwrap();
    let p = exports::export_products(&db, None, None).await.unwrap();
    use saleor_rustify_db::entities::csv_exportfile;
    use sea_orm::EntityTrait;
    for job in [g, v, p] {
        let row = csv_exportfile::Entity::find_by_id(job).one(&db).await.unwrap().unwrap();
        assert_eq!(row.status, "success");
        let path = row.content_file.clone().expect("export must store a path");
        assert!(path.ends_with(".csv"));
        // Cleanup file + row.
        let base = std::env::var("SALEOR_MEDIA_DIR").unwrap_or_else(|_| "../saleor/saleor-core/media".to_string());
        let _ = std::fs::remove_file(std::path::Path::new(&base).join(&path));
        use sea_orm::ActiveModelTrait;
        let am: csv_exportfile::ActiveModel = row.into();
        am.delete(&db).await.unwrap();
    }
}
