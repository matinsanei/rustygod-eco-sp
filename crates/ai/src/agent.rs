//! Agentic buying: natural-language text → checkout lines, deterministic.
//!
//! No LLM, no network: the message is split into segments, each segment
//! yields an optional quantity + a product phrase, phrases resolve against
//! the catalog (SKU exact → name ILIKE → trigram search), and anything
//! unresolved comes back as a clarifying question instead of a hallucinated
//! line. The GraphQL `agentCheckout` mutation (and any future voice/tool
//! caller) builds the checkout from `AgentPlan`.

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::collections::HashMap;

use crate::{search, Result};

pub struct AgentLine {
    pub variant_id: i32,
    pub quantity: i32,
    /// Catalog product name actually matched (transparency for the buyer).
    pub matched_name: String,
}

pub struct AgentPlan {
    pub lines: Vec<AgentLine>,
    /// Human-readable match notes ("2 × 'shirt' → 'Blue T-Shirt'").
    pub notes: Vec<String>,
    /// Clarifying questions for segments that resolved to nothing.
    pub questions: Vec<String>,
}

fn number_word(s: &str) -> Option<i32> {
    match s {
        "one" => Some(1), "two" => Some(2), "three" => Some(3), "four" => Some(4),
        "five" => Some(5), "six" => Some(6), "seven" => Some(7), "eight" => Some(8),
        "nine" => Some(9), "ten" => Some(10), "eleven" => Some(11), "twelve" => Some(12),
        _ => None,
    }
}

/// Split a message into purchasable segments.
fn split_segments(message: &str) -> Vec<String> {
    let lower = message.to_lowercase().replace(" plus ", "+");
    let mut segs = vec![lower];
    for sep in [";", "\n", "+", ","] {
        segs = segs.into_iter().flat_map(|s| s.split(sep).map(str::to_string).collect::<Vec<_>>()).collect();
    }
    segs.into_iter()
        .flat_map(|s| s.split(" and ").map(str::trim).map(str::to_string).collect::<Vec<_>>())
        .map(|s| {
            let mut s = s;
            for p in ["please ", "add ", "buy ", "get ", "order ", "i want ", "i'd like ", "i need "] {
                if let Some(rest) = s.strip_prefix(p) {
                    s = rest.to_string();
                    break;
                }
            }
            s.strip_suffix(" please").map(str::to_string).unwrap_or(s)
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// `(quantity, phrase)` from one segment. Defaults to 1 × whole segment.
fn parse_segment(seg: &str) -> (i32, String) {
    let words: Vec<&str> = seg.split_whitespace().collect();
    if words.len() >= 2 {
        // "2x shirt", "2 x shirt", "2 shirt".
        let (num, rest) = if words[0].ends_with(['x', '×']) && words[0].len() > 1 {
            (&words[0][..words[0].len() - 1], &words[1..])
        } else if words[0].chars().all(|c| c.is_ascii_digit()) {
            let rest = if words[1].eq_ignore_ascii_case("x") { &words[2.min(words.len() - 1)..] } else { &words[1..] };
            (words[0], rest)
        } else if number_word(words[0]).is_some() {
            (words[0], &words[1..])
        } else {
            ("", &words[0..])
        };
        if !num.is_empty() {
            let qty = num.parse::<i32>().ok().or_else(|| number_word(num)).unwrap_or(1).clamp(1, 99);
            let phrase = rest.join(" ").trim().to_string();
            if !phrase.is_empty() {
                return (qty, phrase);
            }
        }
    }
    (1, seg.to_string())
}

/// Resolve one phrase to a priced variant id + product name.
/// SKU exact → product-name ILIKE → trigram search top-1.
async fn resolve_phrase(
    db: &DatabaseConnection,
    channel_slug: &str,
    channel_id: i32,
    currency: &str,
    phrase: &str,
) -> Result<Option<(i32, String)>> {
    // 1. SKU exact (case-insensitive).
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT v.id, p.name FROM product_productvariant v \
             JOIN product_product p ON p.id = v.product_id \
             WHERE v.sku ILIKE $1 LIMIT 2".to_string(),
            [phrase.into()],
        ))
        .await?;
    if let Some(r) = rows.first() {
        return Ok(Some((r.try_get("", "id")?, r.try_get("", "name")?)));
    }
    // 2. Product name substring, published in channel, best trigram first.
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT p.id, p.name FROM product_product p \
             WHERE p.name ILIKE $1 \
               AND p.id IN (SELECT product_id FROM product_productchannellisting WHERE channel_id = $2 AND is_published) \
             ORDER BY similarity(p.name, $3) DESC LIMIT 3".to_string(),
            [format!("%{phrase}%").into(), channel_id.into(), phrase.into()],
        ))
        .await?;
    if let Some(r) = rows.first() {
        let pid: i32 = r.try_get("", "id")?;
        let name: String = r.try_get("", "name")?;
        if let Some(vid) = first_priced_variant(db, channel_slug, pid).await? {
            return Ok(Some((vid, name)));
        }
    }
    // 3. Trigram search fallback (typos, partial words).
    let hits = search::search_products(db, phrase, channel_id, currency, 1).await?;
    if let Some(h) = hits.first() {
        if let Some(vid) = first_priced_variant(db, channel_slug, h.product_id).await? {
            return Ok(Some((vid, h.name.clone())));
        }
    }
    Ok(None)
}

/// Lowest-id channel-priced variant of a product (representative pick).
async fn first_priced_variant(
    db: &DatabaseConnection,
    channel_slug: &str,
    product_id: i32,
) -> Result<Option<i32>> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT id FROM product_productvariant WHERE product_id = $1 ORDER BY id".to_string(),
            [product_id.into()],
        ))
        .await?;
    let vids: Vec<i32> = rows.iter().filter_map(|r| r.try_get("", "id").ok()).collect();
    if vids.is_empty() {
        return Ok(None);
    }
    let pricing = saleor_rustify_db::catalog::checkout_pricing(db, channel_slug, &vids).await?;
    Ok(vids.into_iter().find(|v| pricing.contains_key(v)))
}

/// Turn a natural-language message into checkout lines.
#[tracing::instrument(skip(db))]
pub async fn plan_checkout(
    db: &DatabaseConnection,
    channel_slug: &str,
    message: &str,
) -> Result<AgentPlan> {
    let (channel_id, currency) = saleor_rustify_db::catalog::channel_info(db, channel_slug).await?;
    let mut plan = AgentPlan { lines: vec![], notes: vec![], questions: vec![] };
    // Batch the pricing lookup across all matched variants at the end is
    // overkill here; per-phrase resolution keeps the flow obvious and the
    // catalog small. Merge duplicate variants into one line.
    let mut merged: HashMap<i32, (i32, String)> = HashMap::new();
    for seg in split_segments(message) {
        let (qty, phrase) = parse_segment(&seg);
        match resolve_phrase(db, channel_slug, channel_id, &currency, &phrase).await? {
            Some((vid, name)) => {
                plan.notes.push(format!("{qty} × '{phrase}' → '{name}'"));
                merged.entry(vid).and_modify(|e| e.0 = (e.0 + qty).min(99)).or_insert((qty, name));
            }
            None => plan.questions.push(format!("I couldn't find '{phrase}' in the catalog — which product did you mean?")),
        }
    }
    plan.lines = merged
        .into_iter()
        .map(|(variant_id, (quantity, matched_name))| AgentLine { variant_id, quantity, matched_name })
        .collect();
    plan.lines.sort_by_key(|l| l.variant_id);
    Ok(plan)
}
