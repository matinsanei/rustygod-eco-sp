//! CSV exports (Django `csv` app parity, synchronous subset).
//!
//! Django references (`saleor/csv/`): exports are async file jobs writing
//! `ExportFile` rows. Here exports run synchronously (files are small at
//! this scale) into `<media>/exports/`, served by the existing media mount.
//! - scope ALL/IDS; FILTERED exports are refused honestly (the filter
//!   algebra is its own milestone);
//! - CSV only; XLSX is refused honestly (no spreadsheet writer vendored);
//! - gift cards, voucher codes, products covered (the dashboard's three
//!   export dialogs).

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

use crate::{
    entities::{
        csv_exportfile, discount_voucher, discount_vouchercode, giftcard_giftcard,
        giftcard_giftcard_tags, giftcard_giftcardtag, product_product,
    },
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::App(format!("export error: {}", msg.into()))
}

fn export_dir() -> std::path::PathBuf {
    let base = std::env::var("SALEOR_MEDIA_DIR").unwrap_or_else(|_| "../saleor/saleor-core/media".to_string());
    std::path::Path::new(&base).join("exports")
}

fn write_csv(name: &str, header: &[&str], rows: &[Vec<String>]) -> Result<String> {
    let dir = export_dir();
    std::fs::create_dir_all(&dir).map_err(|e| fail(e.to_string()))?;
    let fname = format!("{name}-{}.csv", Utc::now().timestamp_millis());
    let path = dir.join(&fname);
    let mut w = csv_writer(&path)?;
    w.push(header.join(","));
    for r in rows {
        w.push(r.iter().map(|s| escape(s)).collect::<Vec<_>>().join(","));
    }
    std::fs::write(&path, w.join("\n")).map_err(|e| fail(e.to_string()))?;
    Ok(format!("exports/{fname}"))
}

fn csv_writer(_path: &std::path::Path) -> Result<Vec<String>> {
    Ok(vec![])
}

fn escape(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Open an export job row (status pending).
pub async fn open_job(db: &sea_orm::DatabaseConnection, user_id: Option<i32>) -> Result<i32> {
    let t = Utc::now();
    let row = csv_exportfile::ActiveModel {
        status: Set("pending".to_string()),
        created_at: Set(t.into()),
        updated_at: Set(t.into()),
        content_file: Set(None),
        app_id: Set(None),
        user_id: Set(user_id),
        message: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(row.id)
}

async fn finish_job(db: &sea_orm::DatabaseConnection, id: i32, path: &str) -> Result<()> {
    if let Some(row) = csv_exportfile::Entity::find_by_id(id).one(db).await? {
        let mut am: csv_exportfile::ActiveModel = row.into();
        am.status = Set("success".to_string());
        am.content_file = Set(Some(path.to_string()));
        am.updated_at = Set(Utc::now().into());
        am.update(db).await?;
    }
    Ok(())
}

async fn fail_job(db: &sea_orm::DatabaseConnection, id: i32, message: &str) {
    if let Ok(Some(row)) = csv_exportfile::Entity::find_by_id(id).one(db).await {
        let mut am: csv_exportfile::ActiveModel = row.into();
        am.status = Set("failed".to_string());
        am.message = Set(Some(message.to_string()));
        am.updated_at = Set(Utc::now().into());
        let _ = am.update(db).await;
    }
}

/// Export gift cards (scope ALL or IDS).
pub async fn export_gift_cards(
    db: &sea_orm::DatabaseConnection,
    user_id: Option<i32>,
    ids: Option<Vec<i32>>,
) -> Result<i32> {
    let job = open_job(db, user_id).await;
    let job = match job {
        Ok(j) => j,
        Err(e) => return Err(e),
    };
    let out: Result<i32> = async {
        let mut q = giftcard_giftcard::Entity::find()
            .order_by_asc(giftcard_giftcard::Column::Id);
        if let Some(list) = ids {
            q = q.filter(giftcard_giftcard::Column::Id.is_in(list));
        }
        let cards = q.all(db).await?;
        let mut rows = Vec::with_capacity(cards.len());
        for c in &cards {
            let names = tag_names(db, c.id).await?;
            rows.push(vec![
                c.code.clone(),
                c.initial_balance_amount.to_string(),
                c.current_balance_amount.to_string(),
                c.currency.clone(),
                c.is_active.to_string(),
                c.expiry_date.map(|d| d.to_string()).unwrap_or_default(),
                names.join(";"),
            ]);
        }
        let path = write_csv(
            "gift-cards",
            &["code", "initial_balance", "current_balance", "currency", "is_active", "expiry_date", "tags"],
            &rows,
        )?;
        finish_job(db, job, &path).await?;
        Ok(job)
    }
    .await;
    if let Err(e) = &out {
        fail_job(db, job, &e.to_string()).await;
    }
    out
}

async fn tag_names(db: &impl ConnectionTrait, card_id: i32) -> Result<Vec<String>> {
    let link_ids: Vec<i32> = giftcard_giftcard_tags::Entity::find()
        .select_only()
        .column(giftcard_giftcard_tags::Column::GiftcardtagId)
        .filter(giftcard_giftcard_tags::Column::GiftcardId.eq(card_id))
        .into_tuple()
        .all(db)
        .await?;
    if link_ids.is_empty() {
        return Ok(vec![]);
    }
    Ok(giftcard_giftcardtag::Entity::find()
        .select_only()
        .column(giftcard_giftcardtag::Column::Name)
        .filter(giftcard_giftcardtag::Column::Id.is_in(link_ids))
        .into_tuple()
        .all(db)
        .await?)
}

/// Export voucher codes (one voucher or explicit code rows).
pub async fn export_voucher_codes(
    db: &sea_orm::DatabaseConnection,
    user_id: Option<i32>,
    voucher_id: Option<i32>,
    code_ids: Option<Vec<uuid::Uuid>>,
) -> Result<i32> {
    let job = open_job(db, user_id).await?;
    let out: Result<i32> = async {
        let mut q = discount_vouchercode::Entity::find()
            .order_by_asc(discount_vouchercode::Column::Code);
        if let Some(vid) = voucher_id {
            q = q.filter(discount_vouchercode::Column::VoucherId.eq(vid));
        }
        if let Some(list) = code_ids {
            q = q.filter(discount_vouchercode::Column::Id.is_in(list));
        }
        let codes = q.all(db).await?;
        let mut rows = Vec::with_capacity(codes.len());
        for c in &codes {
            let vname: Option<String> = discount_voucher::Entity::find_by_id(c.voucher_id)
                .select_only()
                .column(discount_voucher::Column::Name)
                .into_tuple()
                .one(db)
                .await?
                .unwrap_or(None);
            rows.push(vec![
                c.code.clone(),
                vname.unwrap_or_default(),
                c.used.to_string(),
                c.is_active.to_string(),
            ]);
        }
        let path = write_csv("voucher-codes", &["code", "voucher", "used", "is_active"], &rows)?;
        finish_job(db, job, &path).await?;
        Ok(job)
    }
    .await;
    if let Err(e) = &out {
        fail_job(db, job, &e.to_string()).await;
    }
    out
}

/// Export products (scope ALL or IDS).
pub async fn export_products(
    db: &sea_orm::DatabaseConnection,
    user_id: Option<i32>,
    ids: Option<Vec<i32>>,
) -> Result<i32> {
    let job = open_job(db, user_id).await?;
    let out: Result<i32> = async {
        let mut q = product_product::Entity::find()
            .select_only()
            .column(product_product::Column::Id)
            .column(product_product::Column::Name)
            .column(product_product::Column::Slug)
            .column(product_product::Column::CreatedAt)
            .order_by_asc(product_product::Column::Id);
        if let Some(list) = ids {
            q = q.filter(product_product::Column::Id.is_in(list));
        }
        let rows: Vec<(i32, String, String, chrono::DateTime<chrono::Utc>)> =
            q.into_tuple().all(db).await?;
        let out: Vec<Vec<String>> = rows
            .into_iter()
            .map(|(id, name, slug, created)| vec![id.to_string(), name, slug, created.to_rfc3339()])
            .collect();
        let path = write_csv("products", &["id", "name", "slug", "created_at"], &out)?;
        finish_job(db, job, &path).await?;
        Ok(job)
    }
    .await;
    if let Err(e) = &out {
        fail_job(db, job, &e.to_string()).await;
    }
    out
}
