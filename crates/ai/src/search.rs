//! Lexical-tier product search over `product_product` using pg_trgm
//! similarity — the same gin_trgm_ops indexes Django's own search uses.
//! Only channel-published products are returned (Saleor visibility rule).

use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

use crate::Result;

pub struct SearchHit {
    pub product_id: i32,
    pub name: String,
    pub slug: String,
    pub currency: String,
    pub min_price: Decimal,
    pub score: f64,
}

#[tracing::instrument(skip(db))]
pub async fn search_products(
    db: &DatabaseConnection,
    query: &str,
    channel_id: i32,
    currency: &str,
    limit: u64,
) -> Result<Vec<SearchHit>> {
    // Saleor ranks by trigram similarity; the `%` operator uses the gin index.
    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"SELECT p.id, p.name, p.slug, similarity(p.name, $1)::float8 AS sml,
               (SELECT MIN(l.price_amount)
                  FROM product_productvariant v
                  JOIN product_productvariantchannellisting l
                    ON l.variant_id = v.id AND l.channel_id = $3
                 WHERE v.product_id = p.id) AS min_price
           FROM product_product p
          WHERE p.name % $2
            AND p.id IN (SELECT product_id FROM product_productchannellisting
                         WHERE channel_id = $3 AND is_published)
          ORDER BY sml DESC LIMIT $4"#,
        vec![
            query.into(),
            query.into(),
            channel_id.into(),
            (limit as i64).into(),
        ],
    );
    let rows = db.query_all(stmt).await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(SearchHit {
            product_id: r.try_get("", "id")?,
            name: r.try_get("", "name")?,
            slug: r.try_get("", "slug")?,
            currency: currency.to_string(),
            min_price: r
                .try_get::<Option<Decimal>>("", "min_price")?
                .unwrap_or(Decimal::ZERO),
            score: r.try_get("", "sml")?,
        });
    }
    Ok(out)
}
