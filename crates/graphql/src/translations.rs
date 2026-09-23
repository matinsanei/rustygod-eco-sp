//! Translations (Saleor `translations(kind)` / `translation(id, kind)` /
//! `*Translate` mutations) over the Django `*_translation` tables.
//!
//! Reads: per-kind entity lists wrapped in `TranslatableContent` union
//! members; `translation(languageCode:)` on entities AND content types
//! resolves via REAL_METHODS codegen entries calling the lookups here.
//! Writes: language-scoped upserts (only provided fields, Saleor parity).
//! Kinds without Django tables (PROMOTION/PROMOTION_RULE/SALE — Saleor
//! 3.24 dropped `discount_sale`) return empty lists / errors honestly.

use async_graphql::{Context, ID, Object, Result};
use sea_orm::{ConnectionTrait, Statement};

use crate::{
    common,
    context::GqlContext,
    gen,
    metadata,
};

#[derive(Default)]
pub struct TranslationQuery;

#[derive(Default)]
pub struct TranslationMutation;

fn terr(message: String) -> gen::TranslationError {
    gen::TranslationError { field: None, message: Some(message), code: None }
}

pub(crate) fn language_display(lang: &str) -> crate::commerce::GqlLanguageDisplay {
    crate::commerce::GqlLanguageDisplay {
        code: lang.to_string(),
        language: language_name(lang).unwrap_or(lang).to_string(),
    }
}

async fn tr_row(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    fk: &str,
    eid: i32,
    lang: &str,
    cols: &str,
) -> Option<sea_orm::QueryResult> {
    let st = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        format!("SELECT {cols} FROM {table} WHERE {fk} = $1 AND language_code = $2"),
        [eid.into(), lang.to_string().into()],
    );
    db.query_one(st).await.ok().flatten()
}

fn get_str(r: &sea_orm::QueryResult, c: &str) -> Option<String> {
    r.try_get::<Option<String>>("", c).ok().flatten()
}

fn get_json_text(r: &sea_orm::QueryResult, c: &str) -> Option<String> {
    r.try_get::<serde_json::Value>("", c).ok().map(|v| v.to_string())
}

fn js(s: Option<String>) -> Option<gen::GenJSONString> {
    s.map(gen::GenJSONString)
}

// ---------------------------------------------------------------------------
// lookups (called by REAL_METHODS `translation(languageCode:)` bodies)
// ---------------------------------------------------------------------------

pub(crate) async fn product_translation(
    db: &sea_orm::DatabaseConnection,
    eid: i32,
    lang: &str,
) -> Option<gen::ProductTranslation> {
    let r = tr_row(db, "product_producttranslation", "product_id", eid, lang, "id, name, slug, seo_title, seo_description, description").await?;
    Some(gen::ProductTranslation {
        id: Some(ID(common::gid("ProductTranslation", r.try_get::<i32>("", "id").ok()?))),
        language: Some(language_display(lang)),
        seo_title: get_str(&r, "seo_title"),
        seo_description: get_str(&r, "seo_description"),
        slug: get_str(&r, "slug"),
        name: get_str(&r, "name"),
        description: js(get_json_text(&r, "description")),
    })
}

pub(crate) async fn product_variant_translation(
    db: &sea_orm::DatabaseConnection,
    eid: i32,
    lang: &str,
) -> Option<gen::ProductVariantTranslation> {
    let r = tr_row(db, "product_productvarianttranslation", "product_variant_id", eid, lang, "id, name").await?;
    Some(gen::ProductVariantTranslation {
        id: Some(ID(common::gid("ProductVariantTranslation", r.try_get::<i32>("", "id").ok()?))),
        language: Some(language_display(lang)),
        name: get_str(&r, "name"),
    })
}

pub(crate) async fn category_translation(
    db: &sea_orm::DatabaseConnection,
    eid: i32,
    lang: &str,
) -> Option<gen::CategoryTranslation> {
    let r = tr_row(db, "product_categorytranslation", "category_id", eid, lang, "id, name, slug, seo_title, seo_description, description").await?;
    Some(gen::CategoryTranslation {
        id: Some(ID(common::gid("CategoryTranslation", r.try_get::<i32>("", "id").ok()?))),
        language: Some(language_display(lang)),
        seo_title: get_str(&r, "seo_title"),
        seo_description: get_str(&r, "seo_description"),
        slug: get_str(&r, "slug"),
        name: get_str(&r, "name"),
        description: js(get_json_text(&r, "description")),
    })
}

pub(crate) async fn collection_translation(
    db: &sea_orm::DatabaseConnection,
    eid: i32,
    lang: &str,
) -> Option<gen::CollectionTranslation> {
    let r = tr_row(db, "product_collectiontranslation", "collection_id", eid, lang, "id, name, slug, seo_title, seo_description, description").await?;
    Some(gen::CollectionTranslation {
        id: Some(ID(common::gid("CollectionTranslation", r.try_get::<i32>("", "id").ok()?))),
        language: Some(language_display(lang)),
        seo_title: get_str(&r, "seo_title"),
        seo_description: get_str(&r, "seo_description"),
        slug: get_str(&r, "slug"),
        name: get_str(&r, "name"),
        description: js(get_json_text(&r, "description")),
    })
}

pub(crate) async fn page_translation(
    db: &sea_orm::DatabaseConnection,
    eid: i32,
    lang: &str,
) -> Option<gen::PageTranslation> {
    let r = tr_row(db, "page_pagetranslation", "page_id", eid, lang, "id, title, slug, seo_title, seo_description, content").await?;
    Some(gen::PageTranslation {
        id: Some(ID(common::gid("PageTranslation", r.try_get::<i32>("", "id").ok()?))),
        language: Some(language_display(lang)),
        seo_title: get_str(&r, "seo_title"),
        seo_description: get_str(&r, "seo_description"),
        slug: get_str(&r, "slug"),
        title: get_str(&r, "title"),
        content: js(get_json_text(&r, "content")),
    })
}

pub(crate) async fn voucher_translation(
    db: &sea_orm::DatabaseConnection,
    eid: i32,
    lang: &str,
) -> Option<gen::VoucherTranslation> {
    let r = tr_row(db, "discount_vouchertranslation", "voucher_id", eid, lang, "id, name").await?;
    Some(gen::VoucherTranslation {
        id: Some(ID(common::gid("VoucherTranslation", r.try_get::<i32>("", "id").ok()?))),
        language: Some(language_display(lang)),
        name: get_str(&r, "name"),
    })
}

pub(crate) async fn shipping_method_translation(
    db: &sea_orm::DatabaseConnection,
    eid: i32,
    lang: &str,
) -> Option<gen::ShippingMethodTranslation> {
    let r = tr_row(db, "shipping_shippingmethodtranslation", "shipping_method_id", eid, lang, "id, name, description").await?;
    Some(gen::ShippingMethodTranslation {
        id: Some(ID(common::gid("ShippingMethodTranslation", r.try_get::<i32>("", "id").ok()?))),
        language: Some(language_display(lang)),
        name: get_str(&r, "name"),
        description: js(get_json_text(&r, "description")),
    })
}

pub(crate) async fn menu_item_translation(
    db: &sea_orm::DatabaseConnection,
    eid: i32,
    lang: &str,
) -> Option<gen::MenuItemTranslation> {
    let r = tr_row(db, "menu_menuitemtranslation", "menu_item_id", eid, lang, "id, name").await?;
    Some(gen::MenuItemTranslation {
        id: Some(ID(common::gid("MenuItemTranslation", r.try_get::<i32>("", "id").ok()?))),
        language: Some(language_display(lang)),
        name: get_str(&r, "name"),
    })
}

pub(crate) async fn attribute_translation(
    db: &sea_orm::DatabaseConnection,
    eid: i32,
    lang: &str,
) -> Option<gen::AttributeTranslation> {
    let r = tr_row(db, "attribute_attributetranslation", "attribute_id", eid, lang, "id, name").await?;
    Some(gen::AttributeTranslation {
        id: Some(ID(common::gid("AttributeTranslation", r.try_get::<i32>("", "id").ok()?))),
        name: get_str(&r, "name"),
    })
}

pub(crate) async fn attribute_value_translation(
    db: &sea_orm::DatabaseConnection,
    eid: i32,
    lang: &str,
) -> Option<gen::AttributeValueTranslation> {
    let r = tr_row(db, "attribute_attributevaluetranslation", "attribute_value_id", eid, lang, "id, name, rich_text, plain_text").await?;
    Some(gen::AttributeValueTranslation {
        id: Some(ID(common::gid("AttributeValueTranslation", r.try_get::<i32>("", "id").ok()?))),
        language: Some(language_display(lang)),
        name: get_str(&r, "name"),
        rich_text: js(get_json_text(&r, "rich_text")),
        plain_text: r.try_get::<Option<String>>("", "plain_text").ok().flatten(),
    })
}

// ---------------------------------------------------------------------------
// content assembly (list + single)
// ---------------------------------------------------------------------------

/// Slim entity row: id + name-ish columns for content wrappers.
async fn entity_rows(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    extra: &str,
    ids: Option<Vec<i32>>,
    limit: i64,
    offset: i64,
) -> Vec<sea_orm::QueryResult> {
    let mut sql = format!("SELECT id{extra} FROM {table}");
    let mut params: Vec<sea_orm::Value> = vec![];
    if let Some(ids) = ids {
        if ids.is_empty() {
            return vec![];
        }
        let list = (1..=ids.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
        sql.push_str(&format!(" WHERE id IN ({list})"));
        params.extend(ids.into_iter().map(|i| i.into()));
    }
    sql.push_str(&format!(" ORDER BY id LIMIT {} OFFSET {}", limit.max(1).min(100), offset.max(0)));
    let st = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, params);
    db.query_all(st).await.unwrap_or_default()
}

fn row_id(r: &sea_orm::QueryResult) -> Option<i32> {
    r.try_get::<i32>("", "id").ok()
}

async fn content_for(
    db: &sea_orm::DatabaseConnection,
    kind: &gen::TranslatableKinds,
    entity_id: i32,
) -> Option<gen::TranslatableItem> {
    match kind {
        gen::TranslatableKinds::PRODUCT => {
            let rows = entity_rows(db, "product_product", ", name, slug", Some(vec![entity_id]), 1, 0).await;
            let r = rows.into_iter().next()?;
            let mut p = metadata::lit_product(common::gid("Product", entity_id), vec![], vec![]);
            p.name = get_str(&r, "name");
            p.slug = get_str(&r, "slug");
            Some(gen::TranslatableItem::ProductTranslatableContent(gen::ProductTranslatableContent {
                id: Some(ID(common::gid("Product", entity_id))),
                product: Some(p),
                attribute_values: vec![],
            }))
        }
        gen::TranslatableKinds::VARIANT => {
            let rows = entity_rows(db, "product_productvariant", ", name", Some(vec![entity_id]), 1, 0).await;
            let r = rows.into_iter().next()?;
            let mut v = metadata::lit_product_variant(common::gid("ProductVariant", entity_id), vec![], vec![]);
            v.name = get_str(&r, "name");
            Some(gen::TranslatableItem::ProductVariantTranslatableContent(gen::ProductVariantTranslatableContent {
                id: Some(ID(common::gid("ProductVariant", entity_id))),
                name: get_str(&r, "name"),
                product_variant: Some(v),
                attribute_values: vec![],
            }))
        }
        gen::TranslatableKinds::CATEGORY => {
            let rows = entity_rows(db, "product_category", "", Some(vec![entity_id]), 1, 0).await;
            rows.into_iter().next()?;
            Some(gen::TranslatableItem::CategoryTranslatableContent(gen::CategoryTranslatableContent {
                id: Some(ID(common::gid("Category", entity_id))),
                category: Some(metadata::lit_category(common::gid("Category", entity_id), vec![], vec![])),
            }))
        }
        gen::TranslatableKinds::COLLECTION => {
            let rows = entity_rows(db, "product_collection", "", Some(vec![entity_id]), 1, 0).await;
            rows.into_iter().next()?;
            Some(gen::TranslatableItem::CollectionTranslatableContent(gen::CollectionTranslatableContent {
                id: Some(ID(common::gid("Collection", entity_id))),
                collection: Some(metadata::lit_collection(common::gid("Collection", entity_id), vec![], vec![])),
            }))
        }
        gen::TranslatableKinds::PAGE => {
            let rows = entity_rows(db, "page_page", ", title", Some(vec![entity_id]), 1, 0).await;
            let r = rows.into_iter().next()?;
            let mut p = metadata::lit_page(common::gid("Page", entity_id), vec![], vec![]);
            p.title = get_str(&r, "title");
            Some(gen::TranslatableItem::PageTranslatableContent(gen::PageTranslatableContent {
                id: Some(ID(common::gid("Page", entity_id))),
                page: Some(p),
                attribute_values: vec![],
            }))
        }
        gen::TranslatableKinds::VOUCHER => {
            let rows = entity_rows(db, "discount_voucher", ", name", Some(vec![entity_id]), 1, 0).await;
            let r = rows.into_iter().next()?;
            let mut v = metadata::lit_voucher(common::gid("Voucher", entity_id), vec![], vec![]);
            v.name = get_str(&r, "name");
            Some(gen::TranslatableItem::VoucherTranslatableContent(gen::VoucherTranslatableContent {
                id: Some(ID(common::gid("Voucher", entity_id))),
                name: get_str(&r, "name"),
                voucher: Some(v),
            }))
        }
        gen::TranslatableKinds::SHIPPINGMETHOD => {
            let rows = entity_rows(db, "shipping_shippingmethod", ", name", Some(vec![entity_id]), 1, 0).await;
            let r = rows.into_iter().next()?;
            let mut s = metadata::lit_shipping_method_type(common::gid("ShippingMethod", entity_id), vec![], vec![]);
            s.name = get_str(&r, "name");
            Some(gen::TranslatableItem::ShippingMethodTranslatableContent(gen::ShippingMethodTranslatableContent {
                id: Some(ID(common::gid("ShippingMethod", entity_id))),
                shipping_method_id: Some(ID(common::gid("ShippingMethod", entity_id))),
                name: get_str(&r, "name"),
                description: None,
                shipping_method: Some(s),
            }))
        }
        gen::TranslatableKinds::MENUITEM => {
            let rows = entity_rows(db, "menu_menuitem", ", name", Some(vec![entity_id]), 1, 0).await;
            let r = rows.into_iter().next()?;
            let mut m = metadata::lit_menu_item(common::gid("MenuItem", entity_id), vec![], vec![]);
            m.name = get_str(&r, "name");
            Some(gen::TranslatableItem::MenuItemTranslatableContent(gen::MenuItemTranslatableContent {
                id: Some(ID(common::gid("MenuItem", entity_id))),
                menu_item: Some(m),
            }))
        }
        gen::TranslatableKinds::ATTRIBUTE => {
            let rows = entity_rows(db, "attribute_attribute", ", name", Some(vec![entity_id]), 1, 0).await;
            let r = rows.into_iter().next()?;
            let mut a = metadata::lit_attribute(common::gid("Attribute", entity_id), vec![], vec![]);
            a.name = get_str(&r, "name");
            Some(gen::TranslatableItem::AttributeTranslatableContent(gen::AttributeTranslatableContent {
                id: Some(ID(common::gid("Attribute", entity_id))),
                name: get_str(&r, "name"),
                attribute: Some(a),
            }))
        }
        gen::TranslatableKinds::ATTRIBUTEVALUE => {
            let rows = entity_rows(db, "attribute_attributevalue", ", name", Some(vec![entity_id]), 1, 0).await;
            let r = rows.into_iter().next()?;
            let mut a = metadata::lit_attribute_value(common::gid("AttributeValue", entity_id), vec![], vec![]);
            a.name = get_str(&r, "name");
            Some(gen::TranslatableItem::AttributeValueTranslatableContent(gen::AttributeValueTranslatableContent {
                id: Some(ID(common::gid("AttributeValue", entity_id))),
                name: get_str(&r, "name"),
                rich_text: None,
                plain_text: None,
                attribute_value: Some(a),
                attribute: None,
            }))
        }
        // No Django tables behind these in 3.24 (Sale dropped legacy Sale).
        gen::TranslatableKinds::PROMOTION | gen::TranslatableKinds::PROMOTIONRULE | gen::TranslatableKinds::SALE => None,
    }
}

fn kind_table(kind: &gen::TranslatableKinds) -> Option<&'static str> {
    Some(match kind {
        gen::TranslatableKinds::PRODUCT => "product_product",
        gen::TranslatableKinds::VARIANT => "product_productvariant",
        gen::TranslatableKinds::CATEGORY => "product_category",
        gen::TranslatableKinds::COLLECTION => "product_collection",
        gen::TranslatableKinds::PAGE => "page_page",
        gen::TranslatableKinds::VOUCHER => "discount_voucher",
        gen::TranslatableKinds::SHIPPINGMETHOD => "shipping_shippingmethod",
        gen::TranslatableKinds::MENUITEM => "menu_menuitem",
        gen::TranslatableKinds::ATTRIBUTE => "attribute_attribute",
        gen::TranslatableKinds::ATTRIBUTEVALUE => "attribute_attributevalue",
        gen::TranslatableKinds::PROMOTION | gen::TranslatableKinds::PROMOTIONRULE | gen::TranslatableKinds::SALE => return None,
    })
}

#[Object]
impl TranslationQuery {
    /// Dashboard translation list pages (one `kind` per page).
    async fn translations(
        &self,
        ctx: &Context<'_>,
        kind: gen::TranslatableKinds,
        before: Option<String>,
        after: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<Option<gen::TranslatableItemConnection>> {
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let Some(table) = kind_table(&kind) else {
            return Ok(Some(gen::TranslatableItemConnection {
                page_info: Some(common::PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }),
                edges: vec![],
            }));
        };
        let off = after.and_then(|c| common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as i64;
        let rows = entity_rows(db, table, "", None, lim + 1, off as i64).await;
        let has_next = rows.len() as i64 > lim;
        let mut edges = vec![];
        for r in rows.into_iter().take(lim as usize) {
            if let Some(id) = row_id(&r) {
                if let Some(node) = content_for(db, &kind, id).await {
                    edges.push(gen::TranslatableItemEdge { node: Some(node) });
                }
            }
        }
        Ok(Some(gen::TranslatableItemConnection {
            page_info: Some(common::PageInfo {
                has_next_page: has_next,
                has_previous_page: off > 0,
                start_cursor: None,
                end_cursor: None,
            }),
            edges,
        }))
    }

    /// Dashboard translation detail pages (`translation(kind, id)`).
    async fn translation(
        &self,
        ctx: &Context<'_>,
        id: ID,
        kind: gen::TranslatableKinds,
    ) -> Result<Option<gen::TranslatableItem>> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let Some(eid) = rustygod_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        Ok(content_for(db, &kind, eid).await)
    }
}

// ---------------------------------------------------------------------------
// writes: language-scoped upserts (provided fields only, Saleor parity)
// ---------------------------------------------------------------------------

/// Upsert one translation row. `entity_table` gates on the owner existing
/// (Saleor raises on unknown ids); only `Some` columns are written.
async fn upsert_tr(
    db: &sea_orm::DatabaseConnection,
    entity_table: &str,
    tr_table: &str,
    fk: &str,
    eid: i32,
    lang: &str,
    strs: &[(&str, Option<String>)],
    jsons: &[(&str, Option<String>)],
) -> Result<(), String> {
    let exists = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        format!("SELECT 1 FROM {entity_table} WHERE id = $1 LIMIT 1"),
        [eid.into()],
    );
    let found = db.query_one(exists).await.map_err(|e| e.to_string())?.is_some();
    if !found {
        return Err("object not found".into());
    }
    let sel = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        format!("SELECT id FROM {tr_table} WHERE {fk} = $1 AND language_code = $2"),
        [eid.into(), lang.to_string().into()],
    );
    let existing: Option<i32> = db
        .query_one(sel)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|r| r.try_get::<i32>("", "id").ok());
    let wanted_strs: Vec<(&str, String)> =
        strs.iter().filter_map(|(c, v)| v.clone().map(|s| (*c, s))).collect();
    let wanted_jsons: Vec<(&str, serde_json::Value)> = jsons
        .iter()
        .filter_map(|(c, v)| {
            v.clone().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok().map(|j| (*c, j)))
        })
        .collect();
    match existing {
        Some(tid) => {
            let mut params: Vec<sea_orm::Value> = vec![tid.into()];
            let mut sets: Vec<String> = vec![];
            for (c, v) in &wanted_strs {
                params.push(v.clone().into());
                sets.push(format!("{c} = ${}", params.len()));
            }
            for (c, j) in &wanted_jsons {
                params.push(j.to_string().into());
                sets.push(format!("{c} = ${}::jsonb", params.len()));
            }
            if !sets.is_empty() {
                let upd = Statement::from_sql_and_values(
                    sea_orm::DatabaseBackend::Postgres,
                    format!("UPDATE {tr_table} SET {} WHERE id = $1", sets.join(", ")),
                    params,
                );
                db.execute(upd).await.map_err(|e| e.to_string())?;
            }
        }
        None => {
            let mut cols = vec![fk.to_string(), "language_code".to_string()];
            let mut vals: Vec<String> = vec!["$1".into(), "$2".into()];
            let mut params: Vec<sea_orm::Value> = vec![eid.into(), lang.to_string().into()];
            for (c, v) in &wanted_strs {
                params.push(v.clone().into());
                vals.push(format!("${}", params.len()));
                cols.push(c.to_string());
            }
            for (c, j) in &wanted_jsons {
                params.push(j.to_string().into());
                vals.push(format!("${}::jsonb", params.len()));
                cols.push(c.to_string());
            }
            let ins = Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                format!("INSERT INTO {tr_table} ({}) VALUES ({})", cols.join(", "), vals.join(", ")),
                params,
            );
            db.execute(ins).await.map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn gid_of(id: &ID) -> Option<i32> {
    rustygod_db::catalog::parse_gid(&id.0)
}

#[Object]
impl TranslationMutation {
    async fn product_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::TranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::ProductTranslate> {
        let _ = crate::account::require_perm(ctx, "manage_translations").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let lang = gen::language_code_value(&language_code);
        let Some(eid) = gid_of(&id) else {
            return Ok(gen::ProductTranslate { errors: vec![terr("bad id".into())], product: None });
        };
        let r = upsert_tr(
            db, "product_product", "product_producttranslation", "product_id", eid, lang,
            &[("name", input.name.clone()), ("slug", input.slug.clone()),
              ("seo_title", input.seo_title.clone()), ("seo_description", input.seo_description.clone())],
            &[("description", input.description.clone().map(|d| d.0.clone()))],
        )
        .await;
        match r {
            Ok(()) => Ok(gen::ProductTranslate {
                errors: vec![],
                product: Some(metadata::lit_product(common::gid("Product", eid), vec![], vec![])),
            }),
            Err(e) => Ok(gen::ProductTranslate { errors: vec![terr(e)], product: None }),
        }
    }

    async fn product_variant_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::NameTranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::ProductVariantTranslate> {
        let _ = crate::account::require_perm(ctx, "manage_translations").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let lang = gen::language_code_value(&language_code);
        let Some(eid) = gid_of(&id) else {
            return Ok(gen::ProductVariantTranslate { errors: vec![terr("bad id".into())], product_variant: None });
        };
        match upsert_tr(db, "product_productvariant", "product_productvarianttranslation", "product_variant_id", eid, lang,
            &[("name", input.name.clone())], &[]).await
        {
            Ok(()) => Ok(gen::ProductVariantTranslate {
                errors: vec![],
                product_variant: Some(metadata::lit_product_variant(common::gid("ProductVariant", eid), vec![], vec![])),
            }),
            Err(e) => Ok(gen::ProductVariantTranslate { errors: vec![terr(e)], product_variant: None }),
        }
    }

    async fn category_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::TranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::CategoryTranslate> {
        let _ = crate::account::require_perm(ctx, "manage_translations").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let lang = gen::language_code_value(&language_code);
        let Some(eid) = gid_of(&id) else {
            return Ok(gen::CategoryTranslate { errors: vec![terr("bad id".into())], category: None });
        };
        let r = upsert_tr(
            db, "product_category", "product_categorytranslation", "category_id", eid, lang,
            &[("name", input.name.clone()), ("slug", input.slug.clone()),
              ("seo_title", input.seo_title.clone()), ("seo_description", input.seo_description.clone())],
            &[("description", input.description.clone().map(|d| d.0.clone()))],
        )
        .await;
        match r {
            Ok(()) => Ok(gen::CategoryTranslate {
                errors: vec![],
                category: Some(metadata::lit_category(common::gid("Category", eid), vec![], vec![])),
            }),
            Err(e) => Ok(gen::CategoryTranslate { errors: vec![terr(e)], category: None }),
        }
    }

    async fn collection_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::TranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::CollectionTranslate> {
        let _ = crate::account::require_perm(ctx, "manage_translations").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let lang = gen::language_code_value(&language_code);
        let Some(eid) = gid_of(&id) else {
            return Ok(gen::CollectionTranslate { errors: vec![terr("bad id".into())], collection: None });
        };
        let r = upsert_tr(
            db, "product_collection", "product_collectiontranslation", "collection_id", eid, lang,
            &[("name", input.name.clone()), ("slug", input.slug.clone()),
              ("seo_title", input.seo_title.clone()), ("seo_description", input.seo_description.clone())],
            &[("description", input.description.clone().map(|d| d.0.clone()))],
        )
        .await;
        match r {
            Ok(()) => Ok(gen::CollectionTranslate {
                errors: vec![],
                collection: Some(metadata::lit_collection(common::gid("Collection", eid), vec![], vec![])),
            }),
            Err(e) => Ok(gen::CollectionTranslate { errors: vec![terr(e)], collection: None }),
        }
    }

    async fn page_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::PageTranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::PageTranslate> {
        let _ = crate::account::require_perm(ctx, "manage_translations").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let lang = gen::language_code_value(&language_code);
        let Some(eid) = gid_of(&id) else {
            return Ok(gen::PageTranslate { errors: vec![terr("bad id".into())], page: None });
        };
        let r = upsert_tr(
            db, "page_page", "page_pagetranslation", "page_id", eid, lang,
            &[("title", input.title.clone()), ("slug", input.slug.clone()),
              ("seo_title", input.seo_title.clone()), ("seo_description", input.seo_description.clone())],
            &[("content", input.content.clone().map(|d| d.0.clone()))],
        )
        .await;
        match r {
            Ok(()) => Ok(gen::PageTranslate {
                errors: vec![],
                page: content_for(db, &gen::TranslatableKinds::PAGE, eid).await.and_then(|n| match n {
                    gen::TranslatableItem::PageTranslatableContent(c) => Some(c),
                    _ => None,
                }),
            }),
            Err(e) => Ok(gen::PageTranslate { errors: vec![terr(e)], page: None }),
        }
    }

    async fn voucher_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::NameTranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::VoucherTranslate> {
        let _ = crate::account::require_perm(ctx, "manage_translations").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let lang = gen::language_code_value(&language_code);
        let Some(eid) = gid_of(&id) else {
            return Ok(gen::VoucherTranslate { errors: vec![terr("bad id".into())], voucher: None });
        };
        match upsert_tr(db, "discount_voucher", "discount_vouchertranslation", "voucher_id", eid, lang,
            &[("name", input.name.clone())], &[]).await
        {
            Ok(()) => Ok(gen::VoucherTranslate {
                errors: vec![],
                voucher: Some(metadata::lit_voucher(common::gid("Voucher", eid), vec![], vec![])),
            }),
            Err(e) => Ok(gen::VoucherTranslate { errors: vec![terr(e)], voucher: None }),
        }
    }

    async fn shipping_price_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::ShippingPriceTranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::ShippingPriceTranslate> {
        let _ = crate::account::require_perm(ctx, "manage_translations").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let lang = gen::language_code_value(&language_code);
        let Some(eid) = gid_of(&id) else {
            return Ok(gen::ShippingPriceTranslate { errors: vec![terr("bad id".into())], shipping_method: None });
        };
        let r = upsert_tr(
            db, "shipping_shippingmethod", "shipping_shippingmethodtranslation", "shipping_method_id", eid, lang,
            &[("name", input.name.clone())],
            &[("description", input.description.clone().map(|d| d.0.clone()))],
        )
        .await;
        match r {
            Ok(()) => Ok(gen::ShippingPriceTranslate {
                errors: vec![],
                shipping_method: Some(metadata::lit_shipping_method_type(common::gid("ShippingMethod", eid), vec![], vec![])),
            }),
            Err(e) => Ok(gen::ShippingPriceTranslate { errors: vec![terr(e)], shipping_method: None }),
        }
    }

    async fn menu_item_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::NameTranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::MenuItemTranslate> {
        let _ = crate::account::require_perm(ctx, "manage_translations").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let lang = gen::language_code_value(&language_code);
        let Some(eid) = gid_of(&id) else {
            return Ok(gen::MenuItemTranslate { errors: vec![terr("bad id".into())], menu_item: None });
        };
        match upsert_tr(db, "menu_menuitem", "menu_menuitemtranslation", "menu_item_id", eid, lang,
            &[("name", input.name.clone())], &[]).await
        {
            Ok(()) => Ok(gen::MenuItemTranslate {
                errors: vec![],
                menu_item: Some(metadata::lit_menu_item(common::gid("MenuItem", eid), vec![], vec![])),
            }),
            Err(e) => Ok(gen::MenuItemTranslate { errors: vec![terr(e)], menu_item: None }),
        }
    }

    async fn attribute_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::NameTranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::AttributeTranslate> {
        let _ = crate::account::require_perm(ctx, "manage_translations").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let lang = gen::language_code_value(&language_code);
        let Some(eid) = gid_of(&id) else {
            return Ok(gen::AttributeTranslate { errors: vec![terr("bad id".into())], attribute: None });
        };
        match upsert_tr(db, "attribute_attribute", "attribute_attributetranslation", "attribute_id", eid, lang,
            &[("name", input.name.clone())], &[]).await
        {
            Ok(()) => Ok(gen::AttributeTranslate {
                errors: vec![],
                attribute: Some(metadata::lit_attribute(common::gid("Attribute", eid), vec![], vec![])),
            }),
            Err(e) => Ok(gen::AttributeTranslate { errors: vec![terr(e)], attribute: None }),
        }
    }

    async fn attribute_value_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::AttributeValueTranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::AttributeValueTranslate> {
        let _ = crate::account::require_perm(ctx, "manage_translations").await?;
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let lang = gen::language_code_value(&language_code);
        let Some(eid) = gid_of(&id) else {
            return Ok(gen::AttributeValueTranslate { errors: vec![terr("bad id".into())], attribute_value: None });
        };
        let r = upsert_tr(
            db, "attribute_attributevalue", "attribute_attributevaluetranslation", "attribute_value_id", eid, lang,
            &[("name", input.name.clone()), ("plain_text", input.plain_text.clone())],
            &[("rich_text", input.rich_text.clone().map(|d| d.0.clone()))],
        )
        .await;
        match r {
            Ok(()) => Ok(gen::AttributeValueTranslate {
                errors: vec![],
                attribute_value: Some(metadata::lit_attribute_value(common::gid("AttributeValue", eid), vec![], vec![])),
            }),
            Err(e) => Ok(gen::AttributeValueTranslate { errors: vec![terr(e)], attribute_value: None }),
        }
    }

    /// Legacy Sale translations: the `discount_sale` table is gone in 3.24,
    /// so there is nothing to write — an honest error instead of a silent
    /// success (the stub used to claim `errors: []`).
    async fn sale_translate(
        &self, ctx: &Context<'_>, id: ID, input: gen::NameTranslationInput, #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum,
    ) -> Result<gen::SaleTranslate> {
        let _ = (ctx, id, input, language_code);
        Ok(gen::SaleTranslate { errors: vec![terr("legacy sales no longer exist in Saleor 3.24".into())], sale: None })
    }
}
/// Human language names from Saleor's own `saleor/core/languages.py`
/// (frozen — regen by re-running the dump command in STATUS).
fn language_name(code: &str) -> Option<&'static str> {
    Some(match code {
        "af" => "Afrikaans",
        "af-na" => "Afrikaans (Namibia)",
        "af-za" => "Afrikaans (South Africa)",
        "agq" => "Aghem",
        "agq-cm" => "Aghem (Cameroon)",
        "ak" => "Akan",
        "ak-gh" => "Akan (Ghana)",
        "am" => "Amharic",
        "am-et" => "Amharic (Ethiopia)",
        "ar" => "Arabic",
        "ar-ae" => "Arabic (United Arab Emirates)",
        "ar-bh" => "Arabic (Bahrain)",
        "ar-dj" => "Arabic (Djibouti)",
        "ar-dz" => "Arabic (Algeria)",
        "ar-eg" => "Arabic (Egypt)",
        "ar-eh" => "Arabic (Western Sahara)",
        "ar-er" => "Arabic (Eritrea)",
        "ar-il" => "Arabic (Israel)",
        "ar-iq" => "Arabic (Iraq)",
        "ar-jo" => "Arabic (Jordan)",
        "ar-km" => "Arabic (Comoros)",
        "ar-kw" => "Arabic (Kuwait)",
        "ar-lb" => "Arabic (Lebanon)",
        "ar-ly" => "Arabic (Libya)",
        "ar-ma" => "Arabic (Morocco)",
        "ar-mr" => "Arabic (Mauritania)",
        "ar-om" => "Arabic (Oman)",
        "ar-ps" => "Arabic (Palestinian Territories)",
        "ar-qa" => "Arabic (Qatar)",
        "ar-sa" => "Arabic (Saudi Arabia)",
        "ar-sd" => "Arabic (Sudan)",
        "ar-so" => "Arabic (Somalia)",
        "ar-ss" => "Arabic (South Sudan)",
        "ar-sy" => "Arabic (Syria)",
        "ar-td" => "Arabic (Chad)",
        "ar-tn" => "Arabic (Tunisia)",
        "ar-ye" => "Arabic (Yemen)",
        "as" => "Assamese",
        "as-in" => "Assamese (India)",
        "asa" => "Asu",
        "asa-tz" => "Asu (Tanzania)",
        "ast" => "Asturian",
        "ast-es" => "Asturian (Spain)",
        "az" => "Azerbaijani",
        "az-cyrl" => "Azerbaijani (Cyrillic)",
        "az-cyrl-az" => "Azerbaijani (Cyrillic, Azerbaijan)",
        "az-latn" => "Azerbaijani (Latin)",
        "az-latn-az" => "Azerbaijani (Latin, Azerbaijan)",
        "bas" => "Basaa",
        "bas-cm" => "Basaa (Cameroon)",
        "be" => "Belarusian",
        "be-by" => "Belarusian (Belarus)",
        "bem" => "Bemba",
        "bem-zm" => "Bemba (Zambia)",
        "bez" => "Bena",
        "bez-tz" => "Bena (Tanzania)",
        "bg" => "Bulgarian",
        "bg-bg" => "Bulgarian (Bulgaria)",
        "bm" => "Bambara",
        "bm-ml" => "Bambara (Mali)",
        "bn" => "Bangla",
        "bn-bd" => "Bangla (Bangladesh)",
        "bn-in" => "Bangla (India)",
        "bo" => "Tibetan",
        "bo-cn" => "Tibetan (China)",
        "bo-in" => "Tibetan (India)",
        "br" => "Breton",
        "br-fr" => "Breton (France)",
        "brx" => "Bodo",
        "brx-in" => "Bodo (India)",
        "bs" => "Bosnian",
        "bs-cyrl" => "Bosnian (Cyrillic)",
        "bs-cyrl-ba" => "Bosnian (Cyrillic, Bosnia & Herzegovina)",
        "bs-latn" => "Bosnian (Latin)",
        "bs-latn-ba" => "Bosnian (Latin, Bosnia & Herzegovina)",
        "ca" => "Catalan",
        "ca-ad" => "Catalan (Andorra)",
        "ca-es" => "Catalan (Spain)",
        "ca-es-valencia" => "Catalan (Spain, Valencian)",
        "ca-fr" => "Catalan (France)",
        "ca-it" => "Catalan (Italy)",
        "ccp" => "Chakma",
        "ccp-bd" => "Chakma (Bangladesh)",
        "ccp-in" => "Chakma (India)",
        "ce" => "Chechen",
        "ce-ru" => "Chechen (Russia)",
        "ceb" => "Cebuano",
        "ceb-ph" => "Cebuano (Philippines)",
        "cgg" => "Chiga",
        "cgg-ug" => "Chiga (Uganda)",
        "chr" => "Cherokee",
        "chr-us" => "Cherokee (United States)",
        "ckb" => "Central Kurdish",
        "ckb-iq" => "Central Kurdish (Iraq)",
        "ckb-ir" => "Central Kurdish (Iran)",
        "cs" => "Czech",
        "cs-cz" => "Czech (Czechia)",
        "cu" => "Church Slavic",
        "cu-ru" => "Church Slavic (Russia)",
        "cy" => "Welsh",
        "cy-gb" => "Welsh (United Kingdom)",
        "da" => "Danish",
        "da-dk" => "Danish (Denmark)",
        "da-gl" => "Danish (Greenland)",
        "dav" => "Taita",
        "dav-ke" => "Taita (Kenya)",
        "de" => "German",
        "de-at" => "German (Austria)",
        "de-be" => "German (Belgium)",
        "de-ch" => "German (Switzerland)",
        "de-de" => "German (Germany)",
        "de-it" => "German (Italy)",
        "de-li" => "German (Liechtenstein)",
        "de-lu" => "German (Luxembourg)",
        "dje" => "Zarma",
        "dje-ne" => "Zarma (Niger)",
        "dsb" => "Lower Sorbian",
        "dsb-de" => "Lower Sorbian (Germany)",
        "dua" => "Duala",
        "dua-cm" => "Duala (Cameroon)",
        "dyo" => "Jola-Fonyi",
        "dyo-sn" => "Jola-Fonyi (Senegal)",
        "dz" => "Dzongkha",
        "dz-bt" => "Dzongkha (Bhutan)",
        "ebu" => "Embu",
        "ebu-ke" => "Embu (Kenya)",
        "ee" => "Ewe",
        "ee-gh" => "Ewe (Ghana)",
        "ee-tg" => "Ewe (Togo)",
        "el" => "Greek",
        "el-cy" => "Greek (Cyprus)",
        "el-gr" => "Greek (Greece)",
        "en" => "English",
        "en-ae" => "English (United Arab Emirates)",
        "en-ag" => "English (Antigua & Barbuda)",
        "en-ai" => "English (Anguilla)",
        "en-as" => "English (American Samoa)",
        "en-at" => "English (Austria)",
        "en-au" => "English (Australia)",
        "en-bb" => "English (Barbados)",
        "en-be" => "English (Belgium)",
        "en-bi" => "English (Burundi)",
        "en-bm" => "English (Bermuda)",
        "en-bs" => "English (Bahamas)",
        "en-bw" => "English (Botswana)",
        "en-bz" => "English (Belize)",
        "en-ca" => "English (Canada)",
        "en-cc" => "English (Cocos (Keeling) Islands)",
        "en-ch" => "English (Switzerland)",
        "en-ck" => "English (Cook Islands)",
        "en-cm" => "English (Cameroon)",
        "en-cx" => "English (Christmas Island)",
        "en-cy" => "English (Cyprus)",
        "en-de" => "English (Germany)",
        "en-dg" => "English (Diego Garcia)",
        "en-dk" => "English (Denmark)",
        "en-dm" => "English (Dominica)",
        "en-er" => "English (Eritrea)",
        "en-fi" => "English (Finland)",
        "en-fj" => "English (Fiji)",
        "en-fk" => "English (Falkland Islands)",
        "en-fm" => "English (Micronesia)",
        "en-gb" => "English (United Kingdom)",
        "en-gd" => "English (Grenada)",
        "en-gg" => "English (Guernsey)",
        "en-gh" => "English (Ghana)",
        "en-gi" => "English (Gibraltar)",
        "en-gm" => "English (Gambia)",
        "en-gu" => "English (Guam)",
        "en-gy" => "English (Guyana)",
        "en-hk" => "English (Hong Kong SAR China)",
        "en-ie" => "English (Ireland)",
        "en-il" => "English (Israel)",
        "en-im" => "English (Isle of Man)",
        "en-in" => "English (India)",
        "en-io" => "English (British Indian Ocean Territory)",
        "en-je" => "English (Jersey)",
        "en-jm" => "English (Jamaica)",
        "en-ke" => "English (Kenya)",
        "en-ki" => "English (Kiribati)",
        "en-kn" => "English (St. Kitts & Nevis)",
        "en-ky" => "English (Cayman Islands)",
        "en-lc" => "English (St. Lucia)",
        "en-lr" => "English (Liberia)",
        "en-ls" => "English (Lesotho)",
        "en-mg" => "English (Madagascar)",
        "en-mh" => "English (Marshall Islands)",
        "en-mo" => "English (Macao SAR China)",
        "en-mp" => "English (Northern Mariana Islands)",
        "en-ms" => "English (Montserrat)",
        "en-mt" => "English (Malta)",
        "en-mu" => "English (Mauritius)",
        "en-mw" => "English (Malawi)",
        "en-my" => "English (Malaysia)",
        "en-na" => "English (Namibia)",
        "en-nf" => "English (Norfolk Island)",
        "en-ng" => "English (Nigeria)",
        "en-nl" => "English (Netherlands)",
        "en-nr" => "English (Nauru)",
        "en-nu" => "English (Niue)",
        "en-nz" => "English (New Zealand)",
        "en-pg" => "English (Papua New Guinea)",
        "en-ph" => "English (Philippines)",
        "en-pk" => "English (Pakistan)",
        "en-pn" => "English (Pitcairn Islands)",
        "en-pr" => "English (Puerto Rico)",
        "en-pw" => "English (Palau)",
        "en-rw" => "English (Rwanda)",
        "en-sb" => "English (Solomon Islands)",
        "en-sc" => "English (Seychelles)",
        "en-sd" => "English (Sudan)",
        "en-se" => "English (Sweden)",
        "en-sg" => "English (Singapore)",
        "en-sh" => "English (St. Helena)",
        "en-si" => "English (Slovenia)",
        "en-sl" => "English (Sierra Leone)",
        "en-ss" => "English (South Sudan)",
        "en-sx" => "English (Sint Maarten)",
        "en-sz" => "English (Eswatini)",
        "en-tc" => "English (Turks & Caicos Islands)",
        "en-tk" => "English (Tokelau)",
        "en-to" => "English (Tonga)",
        "en-tt" => "English (Trinidad & Tobago)",
        "en-tv" => "English (Tuvalu)",
        "en-tz" => "English (Tanzania)",
        "en-ug" => "English (Uganda)",
        "en-um" => "English (U.S. Outlying Islands)",
        "en-us" => "English (United States)",
        "en-vc" => "English (St. Vincent & Grenadines)",
        "en-vg" => "English (British Virgin Islands)",
        "en-vi" => "English (U.S. Virgin Islands)",
        "en-vu" => "English (Vanuatu)",
        "en-ws" => "English (Samoa)",
        "en-za" => "English (South Africa)",
        "en-zm" => "English (Zambia)",
        "en-zw" => "English (Zimbabwe)",
        "eo" => "Esperanto",
        "es" => "Spanish",
        "es-ar" => "Spanish (Argentina)",
        "es-bo" => "Spanish (Bolivia)",
        "es-br" => "Spanish (Brazil)",
        "es-bz" => "Spanish (Belize)",
        "es-cl" => "Spanish (Chile)",
        "es-co" => "Spanish (Colombia)",
        "es-cr" => "Spanish (Costa Rica)",
        "es-cu" => "Spanish (Cuba)",
        "es-do" => "Spanish (Dominican Republic)",
        "es-ea" => "Spanish (Ceuta & Melilla)",
        "es-ec" => "Spanish (Ecuador)",
        "es-es" => "Spanish (Spain)",
        "es-gq" => "Spanish (Equatorial Guinea)",
        "es-gt" => "Spanish (Guatemala)",
        "es-hn" => "Spanish (Honduras)",
        "es-ic" => "Spanish (Canary Islands)",
        "es-mx" => "Spanish (Mexico)",
        "es-ni" => "Spanish (Nicaragua)",
        "es-pa" => "Spanish (Panama)",
        "es-pe" => "Spanish (Peru)",
        "es-ph" => "Spanish (Philippines)",
        "es-pr" => "Spanish (Puerto Rico)",
        "es-py" => "Spanish (Paraguay)",
        "es-sv" => "Spanish (El Salvador)",
        "es-us" => "Spanish (United States)",
        "es-uy" => "Spanish (Uruguay)",
        "es-ve" => "Spanish (Venezuela)",
        "et" => "Estonian",
        "et-ee" => "Estonian (Estonia)",
        "eu" => "Basque",
        "eu-es" => "Basque (Spain)",
        "ewo" => "Ewondo",
        "ewo-cm" => "Ewondo (Cameroon)",
        "fa" => "Persian",
        "fa-af" => "Persian (Afghanistan)",
        "fa-ir" => "Persian (Iran)",
        "ff" => "Fulah",
        "ff-adlm" => "Fulah (Adlam)",
        "ff-adlm-bf" => "Fulah (Adlam, Burkina Faso)",
        "ff-adlm-cm" => "Fulah (Adlam, Cameroon)",
        "ff-adlm-gh" => "Fulah (Adlam, Ghana)",
        "ff-adlm-gm" => "Fulah (Adlam, Gambia)",
        "ff-adlm-gn" => "Fulah (Adlam, Guinea)",
        "ff-adlm-gw" => "Fulah (Adlam, Guinea-Bissau)",
        "ff-adlm-lr" => "Fulah (Adlam, Liberia)",
        "ff-adlm-mr" => "Fulah (Adlam, Mauritania)",
        "ff-adlm-ne" => "Fulah (Adlam, Niger)",
        "ff-adlm-ng" => "Fulah (Adlam, Nigeria)",
        "ff-adlm-sl" => "Fulah (Adlam, Sierra Leone)",
        "ff-adlm-sn" => "Fulah (Adlam, Senegal)",
        "ff-latn" => "Fulah (Latin)",
        "ff-latn-bf" => "Fulah (Latin, Burkina Faso)",
        "ff-latn-cm" => "Fulah (Latin, Cameroon)",
        "ff-latn-gh" => "Fulah (Latin, Ghana)",
        "ff-latn-gm" => "Fulah (Latin, Gambia)",
        "ff-latn-gn" => "Fulah (Latin, Guinea)",
        "ff-latn-gw" => "Fulah (Latin, Guinea-Bissau)",
        "ff-latn-lr" => "Fulah (Latin, Liberia)",
        "ff-latn-mr" => "Fulah (Latin, Mauritania)",
        "ff-latn-ne" => "Fulah (Latin, Niger)",
        "ff-latn-ng" => "Fulah (Latin, Nigeria)",
        "ff-latn-sl" => "Fulah (Latin, Sierra Leone)",
        "ff-latn-sn" => "Fulah (Latin, Senegal)",
        "fi" => "Finnish",
        "fi-fi" => "Finnish (Finland)",
        "fil" => "Filipino",
        "fil-ph" => "Filipino (Philippines)",
        "fo" => "Faroese",
        "fo-dk" => "Faroese (Denmark)",
        "fo-fo" => "Faroese (Faroe Islands)",
        "fr" => "French",
        "fr-be" => "French (Belgium)",
        "fr-bf" => "French (Burkina Faso)",
        "fr-bi" => "French (Burundi)",
        "fr-bj" => "French (Benin)",
        "fr-bl" => "French (St. Barthélemy)",
        "fr-ca" => "French (Canada)",
        "fr-cd" => "French (Congo - Kinshasa)",
        "fr-cf" => "French (Central African Republic)",
        "fr-cg" => "French (Congo - Brazzaville)",
        "fr-ch" => "French (Switzerland)",
        "fr-ci" => "French (Côte d’Ivoire)",
        "fr-cm" => "French (Cameroon)",
        "fr-dj" => "French (Djibouti)",
        "fr-dz" => "French (Algeria)",
        "fr-fr" => "French (France)",
        "fr-ga" => "French (Gabon)",
        "fr-gf" => "French (French Guiana)",
        "fr-gn" => "French (Guinea)",
        "fr-gp" => "French (Guadeloupe)",
        "fr-gq" => "French (Equatorial Guinea)",
        "fr-ht" => "French (Haiti)",
        "fr-km" => "French (Comoros)",
        "fr-lu" => "French (Luxembourg)",
        "fr-ma" => "French (Morocco)",
        "fr-mc" => "French (Monaco)",
        "fr-mf" => "French (St. Martin)",
        "fr-mg" => "French (Madagascar)",
        "fr-ml" => "French (Mali)",
        "fr-mq" => "French (Martinique)",
        "fr-mr" => "French (Mauritania)",
        "fr-mu" => "French (Mauritius)",
        "fr-nc" => "French (New Caledonia)",
        "fr-ne" => "French (Niger)",
        "fr-pf" => "French (French Polynesia)",
        "fr-pm" => "French (St. Pierre & Miquelon)",
        "fr-re" => "French (Réunion)",
        "fr-rw" => "French (Rwanda)",
        "fr-sc" => "French (Seychelles)",
        "fr-sn" => "French (Senegal)",
        "fr-sy" => "French (Syria)",
        "fr-td" => "French (Chad)",
        "fr-tg" => "French (Togo)",
        "fr-tn" => "French (Tunisia)",
        "fr-vu" => "French (Vanuatu)",
        "fr-wf" => "French (Wallis & Futuna)",
        "fr-yt" => "French (Mayotte)",
        "fur" => "Friulian",
        "fur-it" => "Friulian (Italy)",
        "fy" => "Western Frisian",
        "fy-nl" => "Western Frisian (Netherlands)",
        "ga" => "Irish",
        "ga-gb" => "Irish (United Kingdom)",
        "ga-ie" => "Irish (Ireland)",
        "gd" => "Scottish Gaelic",
        "gd-gb" => "Scottish Gaelic (United Kingdom)",
        "gl" => "Galician",
        "gl-es" => "Galician (Spain)",
        "gsw" => "Swiss German",
        "gsw-ch" => "Swiss German (Switzerland)",
        "gsw-fr" => "Swiss German (France)",
        "gsw-li" => "Swiss German (Liechtenstein)",
        "gu" => "Gujarati",
        "gu-in" => "Gujarati (India)",
        "guz" => "Gusii",
        "guz-ke" => "Gusii (Kenya)",
        "gv" => "Manx",
        "gv-im" => "Manx (Isle of Man)",
        "ha" => "Hausa",
        "ha-gh" => "Hausa (Ghana)",
        "ha-ne" => "Hausa (Niger)",
        "ha-ng" => "Hausa (Nigeria)",
        "haw" => "Hawaiian",
        "haw-us" => "Hawaiian (United States)",
        "he" => "Hebrew",
        "he-il" => "Hebrew (Israel)",
        "hi" => "Hindi",
        "hi-in" => "Hindi (India)",
        "hr" => "Croatian",
        "hr-ba" => "Croatian (Bosnia & Herzegovina)",
        "hr-hr" => "Croatian (Croatia)",
        "hsb" => "Upper Sorbian",
        "hsb-de" => "Upper Sorbian (Germany)",
        "hu" => "Hungarian",
        "hu-hu" => "Hungarian (Hungary)",
        "hy" => "Armenian",
        "hy-am" => "Armenian (Armenia)",
        "ia" => "Interlingua",
        "id" => "Indonesian",
        "id-id" => "Indonesian (Indonesia)",
        "ig" => "Igbo",
        "ig-ng" => "Igbo (Nigeria)",
        "ii" => "Sichuan Yi",
        "ii-cn" => "Sichuan Yi (China)",
        "is" => "Icelandic",
        "is-is" => "Icelandic (Iceland)",
        "it" => "Italian",
        "it-ch" => "Italian (Switzerland)",
        "it-it" => "Italian (Italy)",
        "it-sm" => "Italian (San Marino)",
        "it-va" => "Italian (Vatican City)",
        "ja" => "Japanese",
        "ja-jp" => "Japanese (Japan)",
        "jgo" => "Ngomba",
        "jgo-cm" => "Ngomba (Cameroon)",
        "jmc" => "Machame",
        "jmc-tz" => "Machame (Tanzania)",
        "jv" => "Javanese",
        "jv-id" => "Javanese (Indonesia)",
        "ka" => "Georgian",
        "ka-ge" => "Georgian (Georgia)",
        "kab" => "Kabyle",
        "kab-dz" => "Kabyle (Algeria)",
        "kam" => "Kamba",
        "kam-ke" => "Kamba (Kenya)",
        "kde" => "Makonde",
        "kde-tz" => "Makonde (Tanzania)",
        "kea" => "Kabuverdianu",
        "kea-cv" => "Kabuverdianu (Cape Verde)",
        "khq" => "Koyra Chiini",
        "khq-ml" => "Koyra Chiini (Mali)",
        "ki" => "Kikuyu",
        "ki-ke" => "Kikuyu (Kenya)",
        "kk" => "Kazakh",
        "kk-kz" => "Kazakh (Kazakhstan)",
        "kkj" => "Kako",
        "kkj-cm" => "Kako (Cameroon)",
        "kl" => "Kalaallisut",
        "kl-gl" => "Kalaallisut (Greenland)",
        "kln" => "Kalenjin",
        "kln-ke" => "Kalenjin (Kenya)",
        "km" => "Khmer",
        "km-kh" => "Khmer (Cambodia)",
        "kn" => "Kannada",
        "kn-in" => "Kannada (India)",
        "ko" => "Korean",
        "ko-kp" => "Korean (North Korea)",
        "ko-kr" => "Korean (South Korea)",
        "kok" => "Konkani",
        "kok-in" => "Konkani (India)",
        "ks" => "Kashmiri",
        "ks-arab" => "Kashmiri (Arabic)",
        "ks-arab-in" => "Kashmiri (Arabic, India)",
        "ksb" => "Shambala",
        "ksb-tz" => "Shambala (Tanzania)",
        "ksf" => "Bafia",
        "ksf-cm" => "Bafia (Cameroon)",
        "ksh" => "Colognian",
        "ksh-de" => "Colognian (Germany)",
        "ku" => "Kurdish",
        "ku-tr" => "Kurdish (Turkey)",
        "kw" => "Cornish",
        "kw-gb" => "Cornish (United Kingdom)",
        "ky" => "Kyrgyz",
        "ky-kg" => "Kyrgyz (Kyrgyzstan)",
        "lag" => "Langi",
        "lag-tz" => "Langi (Tanzania)",
        "lb" => "Luxembourgish",
        "lb-lu" => "Luxembourgish (Luxembourg)",
        "lg" => "Ganda",
        "lg-ug" => "Ganda (Uganda)",
        "lkt" => "Lakota",
        "lkt-us" => "Lakota (United States)",
        "ln" => "Lingala",
        "ln-ao" => "Lingala (Angola)",
        "ln-cd" => "Lingala (Congo - Kinshasa)",
        "ln-cf" => "Lingala (Central African Republic)",
        "ln-cg" => "Lingala (Congo - Brazzaville)",
        "lo" => "Lao",
        "lo-la" => "Lao (Laos)",
        "lrc" => "Northern Luri",
        "lrc-iq" => "Northern Luri (Iraq)",
        "lrc-ir" => "Northern Luri (Iran)",
        "lt" => "Lithuanian",
        "lt-lt" => "Lithuanian (Lithuania)",
        "lu" => "Luba-Katanga",
        "lu-cd" => "Luba-Katanga (Congo - Kinshasa)",
        "luo" => "Luo",
        "luo-ke" => "Luo (Kenya)",
        "luy" => "Luyia",
        "luy-ke" => "Luyia (Kenya)",
        "lv" => "Latvian",
        "lv-lv" => "Latvian (Latvia)",
        "mai" => "Maithili",
        "mai-in" => "Maithili (India)",
        "mas" => "Masai",
        "mas-ke" => "Masai (Kenya)",
        "mas-tz" => "Masai (Tanzania)",
        "mer" => "Meru",
        "mer-ke" => "Meru (Kenya)",
        "mfe" => "Morisyen",
        "mfe-mu" => "Morisyen (Mauritius)",
        "mg" => "Malagasy",
        "mg-mg" => "Malagasy (Madagascar)",
        "mgh" => "Makhuwa-Meetto",
        "mgh-mz" => "Makhuwa-Meetto (Mozambique)",
        "mgo" => "Metaʼ",
        "mgo-cm" => "Metaʼ (Cameroon)",
        "mi" => "Maori",
        "mi-nz" => "Maori (New Zealand)",
        "mk" => "Macedonian",
        "mk-mk" => "Macedonian (North Macedonia)",
        "ml" => "Malayalam",
        "ml-in" => "Malayalam (India)",
        "mn" => "Mongolian",
        "mn-mn" => "Mongolian (Mongolia)",
        "mni" => "Manipuri",
        "mni-beng" => "Manipuri (Bangla)",
        "mni-beng-in" => "Manipuri (Bangla, India)",
        "mr" => "Marathi",
        "mr-in" => "Marathi (India)",
        "ms" => "Malay",
        "ms-bn" => "Malay (Brunei)",
        "ms-id" => "Malay (Indonesia)",
        "ms-my" => "Malay (Malaysia)",
        "ms-sg" => "Malay (Singapore)",
        "mt" => "Maltese",
        "mt-mt" => "Maltese (Malta)",
        "mua" => "Mundang",
        "mua-cm" => "Mundang (Cameroon)",
        "my" => "Burmese",
        "my-mm" => "Burmese (Myanmar (Burma))",
        "mzn" => "Mazanderani",
        "mzn-ir" => "Mazanderani (Iran)",
        "naq" => "Nama",
        "naq-na" => "Nama (Namibia)",
        "nb" => "Norwegian Bokmål",
        "nb-no" => "Norwegian Bokmål (Norway)",
        "nb-sj" => "Norwegian Bokmål (Svalbard & Jan Mayen)",
        "nd" => "North Ndebele",
        "nd-zw" => "North Ndebele (Zimbabwe)",
        "nds" => "Low German",
        "nds-de" => "Low German (Germany)",
        "nds-nl" => "Low German (Netherlands)",
        "ne" => "Nepali",
        "ne-in" => "Nepali (India)",
        "ne-np" => "Nepali (Nepal)",
        "nl" => "Dutch",
        "nl-aw" => "Dutch (Aruba)",
        "nl-be" => "Dutch (Belgium)",
        "nl-bq" => "Dutch (Caribbean Netherlands)",
        "nl-cw" => "Dutch (Curaçao)",
        "nl-nl" => "Dutch (Netherlands)",
        "nl-sr" => "Dutch (Suriname)",
        "nl-sx" => "Dutch (Sint Maarten)",
        "nmg" => "Kwasio",
        "nmg-cm" => "Kwasio (Cameroon)",
        "nn" => "Norwegian Nynorsk",
        "nn-no" => "Norwegian Nynorsk (Norway)",
        "nnh" => "Ngiemboon",
        "nnh-cm" => "Ngiemboon (Cameroon)",
        "nus" => "Nuer",
        "nus-ss" => "Nuer (South Sudan)",
        "nyn" => "Nyankole",
        "nyn-ug" => "Nyankole (Uganda)",
        "om" => "Oromo",
        "om-et" => "Oromo (Ethiopia)",
        "om-ke" => "Oromo (Kenya)",
        "or" => "Odia",
        "or-in" => "Odia (India)",
        "os" => "Ossetic",
        "os-ge" => "Ossetic (Georgia)",
        "os-ru" => "Ossetic (Russia)",
        "pa" => "Punjabi",
        "pa-arab" => "Punjabi (Arabic)",
        "pa-arab-pk" => "Punjabi (Arabic, Pakistan)",
        "pa-guru" => "Punjabi (Gurmukhi)",
        "pa-guru-in" => "Punjabi (Gurmukhi, India)",
        "pcm" => "Nigerian Pidgin",
        "pcm-ng" => "Nigerian Pidgin (Nigeria)",
        "pl" => "Polish",
        "pl-pl" => "Polish (Poland)",
        "prg" => "Prussian",
        "ps" => "Pashto",
        "ps-af" => "Pashto (Afghanistan)",
        "ps-pk" => "Pashto (Pakistan)",
        "pt" => "Portuguese",
        "pt-ao" => "Portuguese (Angola)",
        "pt-br" => "Portuguese (Brazil)",
        "pt-ch" => "Portuguese (Switzerland)",
        "pt-cv" => "Portuguese (Cape Verde)",
        "pt-gq" => "Portuguese (Equatorial Guinea)",
        "pt-gw" => "Portuguese (Guinea-Bissau)",
        "pt-lu" => "Portuguese (Luxembourg)",
        "pt-mo" => "Portuguese (Macao SAR China)",
        "pt-mz" => "Portuguese (Mozambique)",
        "pt-pt" => "Portuguese (Portugal)",
        "pt-st" => "Portuguese (São Tomé & Príncipe)",
        "pt-tl" => "Portuguese (Timor-Leste)",
        "qu" => "Quechua",
        "qu-bo" => "Quechua (Bolivia)",
        "qu-ec" => "Quechua (Ecuador)",
        "qu-pe" => "Quechua (Peru)",
        "rm" => "Romansh",
        "rm-ch" => "Romansh (Switzerland)",
        "rn" => "Rundi",
        "rn-bi" => "Rundi (Burundi)",
        "ro" => "Romanian",
        "ro-md" => "Romanian (Moldova)",
        "ro-ro" => "Romanian (Romania)",
        "rof" => "Rombo",
        "rof-tz" => "Rombo (Tanzania)",
        "ru" => "Russian",
        "ru-by" => "Russian (Belarus)",
        "ru-kg" => "Russian (Kyrgyzstan)",
        "ru-kz" => "Russian (Kazakhstan)",
        "ru-md" => "Russian (Moldova)",
        "ru-ru" => "Russian (Russia)",
        "ru-ua" => "Russian (Ukraine)",
        "rw" => "Kinyarwanda",
        "rw-rw" => "Kinyarwanda (Rwanda)",
        "rwk" => "Rwa",
        "rwk-tz" => "Rwa (Tanzania)",
        "sah" => "Sakha",
        "sah-ru" => "Sakha (Russia)",
        "saq" => "Samburu",
        "saq-ke" => "Samburu (Kenya)",
        "sat" => "Santali",
        "sat-olck" => "Santali (Ol Chiki)",
        "sat-olck-in" => "Santali (Ol Chiki, India)",
        "sbp" => "Sangu",
        "sbp-tz" => "Sangu (Tanzania)",
        "sd" => "Sindhi",
        "sd-arab" => "Sindhi (Arabic)",
        "sd-arab-pk" => "Sindhi (Arabic, Pakistan)",
        "sd-deva" => "Sindhi (Devanagari)",
        "sd-deva-in" => "Sindhi (Devanagari, India)",
        "se" => "Northern Sami",
        "se-fi" => "Northern Sami (Finland)",
        "se-no" => "Northern Sami (Norway)",
        "se-se" => "Northern Sami (Sweden)",
        "seh" => "Sena",
        "seh-mz" => "Sena (Mozambique)",
        "ses" => "Koyraboro Senni",
        "ses-ml" => "Koyraboro Senni (Mali)",
        "sg" => "Sango",
        "sg-cf" => "Sango (Central African Republic)",
        "shi" => "Tachelhit",
        "shi-latn" => "Tachelhit (Latin)",
        "shi-latn-ma" => "Tachelhit (Latin, Morocco)",
        "shi-tfng" => "Tachelhit (Tifinagh)",
        "shi-tfng-ma" => "Tachelhit (Tifinagh, Morocco)",
        "si" => "Sinhala",
        "si-lk" => "Sinhala (Sri Lanka)",
        "sk" => "Slovak",
        "sk-sk" => "Slovak (Slovakia)",
        "sl" => "Slovenian",
        "sl-si" => "Slovenian (Slovenia)",
        "smn" => "Inari Sami",
        "smn-fi" => "Inari Sami (Finland)",
        "sn" => "Shona",
        "sn-zw" => "Shona (Zimbabwe)",
        "so" => "Somali",
        "so-dj" => "Somali (Djibouti)",
        "so-et" => "Somali (Ethiopia)",
        "so-ke" => "Somali (Kenya)",
        "so-so" => "Somali (Somalia)",
        "sq" => "Albanian",
        "sq-al" => "Albanian (Albania)",
        "sq-mk" => "Albanian (North Macedonia)",
        "sq-xk" => "Albanian (Kosovo)",
        "sr" => "Serbian",
        "sr-cyrl" => "Serbian (Cyrillic)",
        "sr-cyrl-ba" => "Serbian (Cyrillic, Bosnia & Herzegovina)",
        "sr-cyrl-me" => "Serbian (Cyrillic, Montenegro)",
        "sr-cyrl-rs" => "Serbian (Cyrillic, Serbia)",
        "sr-cyrl-xk" => "Serbian (Cyrillic, Kosovo)",
        "sr-latn" => "Serbian (Latin)",
        "sr-latn-ba" => "Serbian (Latin, Bosnia & Herzegovina)",
        "sr-latn-me" => "Serbian (Latin, Montenegro)",
        "sr-latn-rs" => "Serbian (Latin, Serbia)",
        "sr-latn-xk" => "Serbian (Latin, Kosovo)",
        "su" => "Sundanese",
        "su-latn" => "Sundanese (Latin)",
        "su-latn-id" => "Sundanese (Latin, Indonesia)",
        "sv" => "Swedish",
        "sv-ax" => "Swedish (Åland Islands)",
        "sv-fi" => "Swedish (Finland)",
        "sv-se" => "Swedish (Sweden)",
        "sw" => "Swahili",
        "sw-cd" => "Swahili (Congo - Kinshasa)",
        "sw-ke" => "Swahili (Kenya)",
        "sw-tz" => "Swahili (Tanzania)",
        "sw-ug" => "Swahili (Uganda)",
        "ta" => "Tamil",
        "ta-in" => "Tamil (India)",
        "ta-lk" => "Tamil (Sri Lanka)",
        "ta-my" => "Tamil (Malaysia)",
        "ta-sg" => "Tamil (Singapore)",
        "te" => "Telugu",
        "te-in" => "Telugu (India)",
        "teo" => "Teso",
        "teo-ke" => "Teso (Kenya)",
        "teo-ug" => "Teso (Uganda)",
        "tg" => "Tajik",
        "tg-tj" => "Tajik (Tajikistan)",
        "th" => "Thai",
        "th-th" => "Thai (Thailand)",
        "ti" => "Tigrinya",
        "ti-er" => "Tigrinya (Eritrea)",
        "ti-et" => "Tigrinya (Ethiopia)",
        "tk" => "Turkmen",
        "tk-tm" => "Turkmen (Turkmenistan)",
        "to" => "Tongan",
        "to-to" => "Tongan (Tonga)",
        "tr" => "Turkish",
        "tr-cy" => "Turkish (Cyprus)",
        "tr-tr" => "Turkish (Turkey)",
        "tt" => "Tatar",
        "tt-ru" => "Tatar (Russia)",
        "twq" => "Tasawaq",
        "twq-ne" => "Tasawaq (Niger)",
        "tzm" => "Central Atlas Tamazight",
        "tzm-ma" => "Central Atlas Tamazight (Morocco)",
        "ug" => "Uyghur",
        "ug-cn" => "Uyghur (China)",
        "uk" => "Ukrainian",
        "uk-ua" => "Ukrainian (Ukraine)",
        "ur" => "Urdu",
        "ur-in" => "Urdu (India)",
        "ur-pk" => "Urdu (Pakistan)",
        "uz" => "Uzbek",
        "uz-arab" => "Uzbek (Arabic)",
        "uz-arab-af" => "Uzbek (Arabic, Afghanistan)",
        "uz-cyrl" => "Uzbek (Cyrillic)",
        "uz-cyrl-uz" => "Uzbek (Cyrillic, Uzbekistan)",
        "uz-latn" => "Uzbek (Latin)",
        "uz-latn-uz" => "Uzbek (Latin, Uzbekistan)",
        "vai" => "Vai",
        "vai-latn" => "Vai (Latin)",
        "vai-latn-lr" => "Vai (Latin, Liberia)",
        "vai-vaii" => "Vai (Vai)",
        "vai-vaii-lr" => "Vai (Vai, Liberia)",
        "vi" => "Vietnamese",
        "vi-vn" => "Vietnamese (Vietnam)",
        "vo" => "Volapük",
        "vun" => "Vunjo",
        "vun-tz" => "Vunjo (Tanzania)",
        "wae" => "Walser",
        "wae-ch" => "Walser (Switzerland)",
        "wo" => "Wolof",
        "wo-sn" => "Wolof (Senegal)",
        "xh" => "Xhosa",
        "xh-za" => "Xhosa (South Africa)",
        "xog" => "Soga",
        "xog-ug" => "Soga (Uganda)",
        "yav" => "Yangben",
        "yav-cm" => "Yangben (Cameroon)",
        "yi" => "Yiddish",
        "yo" => "Yoruba",
        "yo-bj" => "Yoruba (Benin)",
        "yo-ng" => "Yoruba (Nigeria)",
        "yue" => "Cantonese",
        "yue-hans" => "Cantonese (Simplified)",
        "yue-hans-cn" => "Cantonese (Simplified, China)",
        "yue-hant" => "Cantonese (Traditional)",
        "yue-hant-hk" => "Cantonese (Traditional, Hong Kong SAR China)",
        "zgh" => "Standard Moroccan Tamazight",
        "zgh-ma" => "Standard Moroccan Tamazight (Morocco)",
        "zh" => "Chinese",
        "zh-hans" => "Chinese (Simplified)",
        "zh-hans-cn" => "Chinese (Simplified, China)",
        "zh-hans-hk" => "Chinese (Simplified, Hong Kong SAR China)",
        "zh-hans-mo" => "Chinese (Simplified, Macao SAR China)",
        "zh-hans-sg" => "Chinese (Simplified, Singapore)",
        "zh-hant" => "Chinese (Traditional)",
        "zh-hant-hk" => "Chinese (Traditional, Hong Kong SAR China)",
        "zh-hant-mo" => "Chinese (Traditional, Macao SAR China)",
        "zh-hant-tw" => "Chinese (Traditional, Taiwan)",
        "zu" => "Zulu",
        "zu-za" => "Zulu (South Africa)",
        _ => return None,
    })
}
