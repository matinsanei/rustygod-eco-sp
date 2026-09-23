//! Checkout GraphQL: create / get / addLines / complete + voucher/promotion.
//! Every resolver is a thin call into `db::checkout_store` / `db::promotions`
//! / `db::order_promotions` — the same functions gRPC uses.

use async_graphql::*;
use uuid::Uuid;

use crate::{common::*, context::GqlContext, gen};

#[derive(SimpleObject, Clone)]
pub struct GqlCheckoutLine {
    pub variant_id: ID,
    pub quantity: i32,
    pub unit_price: Money,
    pub total_price: Money,
    pub is_gift: bool,
}

#[derive(SimpleObject, Clone)]
pub struct GqlCheckout {
    pub id: ID,
    pub channel: String,
    pub email: String,
    pub currency: String,
    pub total: Money,
    pub lines: Vec<GqlCheckoutLine>,
}

fn to_gql_checkout(
    co: &saleor_rustify_db::entities::checkout_checkout::Model,
    lines: &[saleor_rustify_db::entities::checkout_checkoutline::Model],
    channel: &str,
) -> GqlCheckout {
    let d = saleor_rustify_db::checkout_store::to_domain(co, lines, channel);
    let total = Money { amount: co.total_gross_amount.to_string(), currency: co.currency.clone(), fraction_digits: None };
    GqlCheckout {
        id: ID(crate::common::gid("Checkout", &d.id)),
        channel: d.channel,
        email: d.email,
        currency: d.currency.clone(),
        total: total.clone(),
        lines: d.lines.into_iter().map(|l| {
            let tot = if l.is_gift { rust_decimal::Decimal::ZERO } else { l.unit_price.amount * rust_decimal::Decimal::from(l.quantity) };
            let currency = l.unit_price.currency.clone();
            GqlCheckoutLine {
                variant_id: ID(crate::common::gid("ProductVariant", &l.variant_id)),
                quantity: l.quantity,
                unit_price: l.unit_price.into(),
                total_price: Money { amount: tot.to_string(), currency, fraction_digits: None },
                is_gift: l.is_gift,
            }
        }).collect(),
    }
}

#[derive(InputObject)]
pub struct AddLineInput {
    pub variant_id: ID,
    pub quantity: i32,
}

#[derive(SimpleObject, Clone)]
pub struct GqlCheckoutLineEdge { pub node: Option<GqlCheckoutLine> }

#[derive(SimpleObject, Clone)]
pub struct GqlCheckoutLineConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlCheckoutLineEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: crate::common::PageInfo,
}

#[derive(SimpleObject, Clone)]
pub struct AgentCheckoutResult {
    pub checkout: GqlCheckout,
    pub notes: Vec<String>,
    pub questions: Vec<String>,
}

/// Saleor new-checkout-API error shape (field/message/code-as-string).
#[derive(SimpleObject, Clone)]
pub struct GqlCheckoutError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

macro_rules! new_api_payload {
    ($name:ident) => {
        #[derive(SimpleObject, Clone)]
        pub struct $name {
            pub checkout: Option<GqlCheckout>,
            pub errors: Vec<GqlCheckoutError>,
        }
    };
}

new_api_payload!(CheckoutCreate);
new_api_payload!(CheckoutLinesAdd);
new_api_payload!(CheckoutLinesUpdate);
new_api_payload!(CheckoutLineDelete);
new_api_payload!(CheckoutDelete);
new_api_payload!(CheckoutEmailUpdate);
new_api_payload!(CheckoutCustomerAttach);
new_api_payload!(CheckoutCustomerDetach);
new_api_payload!(CheckoutCustomerNoteUpdate);
new_api_payload!(CheckoutShippingAddressUpdate);
new_api_payload!(CheckoutBillingAddressUpdate);
new_api_payload!(CheckoutShippingMethodUpdate);
new_api_payload!(CheckoutDeliveryMethodUpdate);
new_api_payload!(CheckoutLanguageCodeUpdate);
new_api_payload!(CheckoutAddPromoCode);
new_api_payload!(CheckoutRemovePromoCode);

#[derive(InputObject)]
pub struct CheckoutCreateInput {
    pub channel: Option<String>,
    pub lines: Vec<gen::CheckoutLineInput>,
    pub email: Option<String>,
    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<gen::AddressInput>,
    #[graphql(name = "billingAddress")]
    pub billing_address: Option<gen::AddressInput>,
    #[graphql(name = "languageCode")]
    pub language_code: Option<gen::LanguageCodeEnum>,
    pub metadata: Option<Vec<crate::common::MetadataInput>>,
}

fn ckerr(message: String) -> Vec<GqlCheckoutError> {
    vec![GqlCheckoutError { field: None, message: Some(message), code: None }]
}

/// Resolve the checkout token from the new-API id soup (`id` preferred,
/// deprecated `checkoutId`/`token` honored).
fn resolve_token(id: Option<ID>, checkout_id: Option<ID>, token: Option<String>) -> std::result::Result<Uuid, String> {
    for cand in [id, checkout_id] {
        if let Some(i) = cand {
            if let Some(t) = crate::common::parse_uuid_gid(&i.0) {
                return Ok(t);
            }
        }
    }
    if let Some(t) = token.and_then(|s| s.parse::<Uuid>().ok()) {
        return Ok(t);
    }
    Err("checkout id is required".to_string())
}

fn parse_price(p: &Option<gen::GenPositiveDecimal>) -> Option<rust_decimal::Decimal> {
    p.as_ref().and_then(|g| g.0.parse().ok())
}

async fn load_gql(    db: &sea_orm::DatabaseConnection,
    token: Uuid,
    ch: &str,
) -> Result<GqlCheckout> {
    let (co, lines) = saleor_rustify_db::checkout_store::load_checkout(db, token)
        .await.map_err(|e| Error::new(e.to_string()))?
        .ok_or_else(|| Error::new("checkout not found"))?;
    Ok(to_gql_checkout(&co, &lines, ch))
}

fn channel_of(co: &saleor_rustify_db::entities::checkout_checkout::Model) -> String {
    match co.channel_id { 2 => "channel-pln".to_string(), _ => "default-channel".to_string() }
}

/// Persist a storefront address input onto a checkout side (Django creates
/// an owned address row; validation/skipping rules honored minimally).
async fn save_checkout_address(
    db: &sea_orm::DatabaseConnection,
    token: Uuid,
    billing: bool,
    a: &gen::AddressInput,
) -> Result<(), String> {
    let (Some(first), Some(last), Some(street), Some(city), Some(postal), Some(country)) = (
        a.first_name.clone(), a.last_name.clone(), a.street_address1.clone(),
        a.city.clone(), a.postal_code.clone(), a.country.as_ref().map(|c| format!("{c:?}")),
    ) else {
        return Err("first name, last name, street, city, postal code and country are required".into());
    };
    saleor_rustify_db::checkout_store::set_checkout_address(db, token, billing, &first, &last, &street, &city, &postal, &country)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Price lines through channel listings; app price overrides honored only
/// with `handle_checkouts` (Django `CheckoutLineInput.price` rule).
async fn unit_prices(
    ctx: &Context<'_>,
    db: &sea_orm::DatabaseConnection,
    ch: &str,
    items: &[(i32, Option<rust_decimal::Decimal>)],
) -> Result<std::collections::HashMap<i32, rust_decimal::Decimal>> {
    let vids: Vec<i32> = items.iter().map(|(v, _)| *v).collect();
    let pricing = saleor_rustify_db::catalog::checkout_pricing(db, ch, &vids).await.map_err(|e| Error::new(e.to_string()))?;
    let can_override = crate::account::require_perm(ctx, "handle_checkouts").await.is_ok();
    Ok(items.iter().map(|(vid, ov)| {
        let unit = match (can_override, ov) {
            (true, Some(p)) => *p,
            _ => pricing.get(vid).map(|(m, _)| m.amount).unwrap_or(rust_decimal::Decimal::ZERO),
        };
        (*vid, unit)
    }).collect())
}

#[derive(Default)]
pub struct CheckoutQuery;

#[Object]
impl CheckoutQuery {
    async fn checkout(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GqlCheckout>> {        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let token: Uuid = crate::common::parse_uuid_gid(&id.0).ok_or_else(|| Error::new("id must be UUID"))?;
        let Some((co, lines)) = saleor_rustify_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))? else { return Ok(None) };
        // Channel slug round-trip via channel_id (v1 ids 1/2 as in server).
        let ch = match co.channel_id { 2 => "channel-pln", _ => "default-channel" };
        Ok(Some(to_gql_checkout(&co, &lines, ch)))
    }

    /// Admin checkout-lines view (paginated across checkouts).
    async fn checkout_lines(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
    ) -> Result<GqlCheckoutLineConnection> {
        crate::account::require_perm(ctx, "manage_orders").await?;
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use sea_orm::{ConnectionTrait, Statement};
        let rows = db.query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT checkout_id, variant_id, quantity, currency, total_price_gross_amount, is_gift, undiscounted_unit_price_amount FROM checkout_checkoutline ORDER BY created_at DESC, id LIMIT 500".to_string(),
        )).await.map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len();
        let edges = rows.into_iter().skip(off).take(lim).map(|r| {
            let vid: i32 = r.try_get("", "variant_id").unwrap_or(0);
            let qty: i32 = r.try_get("", "quantity").unwrap_or(0);
            let cur: String = r.try_get("", "currency").unwrap_or_else(|_| "USD".into());
            let unit: rust_decimal::Decimal = r.try_get("", "undiscounted_unit_price_amount").unwrap_or(rust_decimal::Decimal::ZERO);
            let tot: rust_decimal::Decimal = r.try_get("", "total_price_gross_amount").unwrap_or(unit * rust_decimal::Decimal::from(qty));
            let gift: bool = r.try_get("", "is_gift").unwrap_or(false);
            GqlCheckoutLineEdge { node: Some(GqlCheckoutLine {
                variant_id: ID(crate::common::gid("ProductVariant", vid)),
                quantity: qty,
                unit_price: crate::common::Money { amount: unit.to_string(), currency: cur.clone(), fraction_digits: None },
                total_price: crate::common::Money { amount: tot.to_string(), currency: cur, fraction_digits: None },
                is_gift: gift,
            }) }
        }).collect();
        Ok(GqlCheckoutLineConnection {
            total_count: Some(total as i32),
            edges,
            page_info: crate::common::PageInfo { has_next_page: off + lim < total, has_previous_page: off > 0, start_cursor: None, end_cursor: None },
        })
    }
}

#[derive(Default)]
pub struct CheckoutMutation;

#[Object]
impl CheckoutMutation {
    async fn create_checkout(&self, ctx: &Context<'_>, channel: Option<String>, email: String) -> Result<GqlCheckout> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let (ch_id, currency) = saleor_rustify_db::catalog::channel_info(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        let token = saleor_rustify_db::checkout_store::create_checkout_row(db, ch_id, &currency, &email).await.map_err(|e| Error::new(e.to_string()))?;
        let (co, lines) = saleor_rustify_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("just-inserted checkout missing"))?;
        Ok(to_gql_checkout(&co, &lines, &ch))
    }

    async fn checkout_add_lines(&self, ctx: &Context<'_>, checkout_id: ID, lines: Vec<AddLineInput>) -> Result<GqlCheckout> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let token: Uuid = crate::common::parse_uuid_gid(&checkout_id.0).ok_or_else(|| Error::new("checkoutId must be UUID"))?;
        let Some((co, _)) = saleor_rustify_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))? else { return Err(Error::new("checkout not found")) };
        let ch = match co.channel_id { 2 => "channel-pln", _ => "default-channel" };
        let (ch_id, currency) = saleor_rustify_db::catalog::channel_info(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        // Price from channel listings (same as gRPC AddLines).
        let vids: Vec<i32> = lines.iter().filter_map(|l| saleor_rustify_db::catalog::parse_gid(&l.variant_id.0)).collect();
        let pricing = saleor_rustify_db::catalog::checkout_pricing(db, &ch, &vids).await.map_err(|e| Error::new(e.to_string()))?;
        let items: Vec<saleor_rustify_db::checkout_store::NewLine> = lines.into_iter().map(|l| {
            let vid: i32 = saleor_rustify_db::catalog::parse_gid(&l.variant_id.0).unwrap_or(0);
            let unit = pricing.get(&vid).map(|(m, _)| m.amount).unwrap_or(rust_decimal::Decimal::ZERO);
            saleor_rustify_db::checkout_store::NewLine { variant_id: vid, quantity: l.quantity, unit_price: unit, price_override: None }
        }).collect();
        saleor_rustify_db::checkout_store::add_lines_tx(db, token, ch_id, &currency, &items).await.map_err(|e| Error::new(e.to_string()))?;
        let (co2, ls2) = saleor_rustify_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout vanished"))?;
        Ok(to_gql_checkout(&co2, &ls2, &ch))
    }

    async fn checkout_complete(&self, ctx: &Context<'_>, checkout_id: ID) -> Result<ID> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let token: Uuid = crate::common::parse_uuid_gid(&checkout_id.0).ok_or_else(|| Error::new("checkoutId must be UUID"))?;
        let out = saleor_rustify_db::complete::complete_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(ID(crate::common::gid("Order", out.order_id)))
    }

    async fn checkout_apply_voucher(&self, ctx: &Context<'_>, checkout_id: ID, code: String) -> Result<GqlCheckout> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let token: Uuid = crate::common::parse_uuid_gid(&checkout_id.0).ok_or_else(|| Error::new("checkoutId must be UUID"))?;
        let ch = {
            let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
            match co.channel_id { 2 => "channel-pln".to_string(), _ => "default-channel".to_string() }
        };
        saleor_rustify_db::promotions::apply_voucher(db, token, &code, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        saleor_rustify_db::checkout_store::refresh_totals(db, token).await.map_err(|e| Error::new(e.to_string()))?;
        let (co2, ls2) = saleor_rustify_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout vanished"))?;
        Ok(to_gql_checkout(&co2, &ls2, &ch))
    }

    async fn refresh_order_promotion(&self, ctx: &Context<'_>, checkout_id: ID) -> Result<String> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let token: Uuid = crate::common::parse_uuid_gid(&checkout_id.0).ok_or_else(|| Error::new("checkoutId must be UUID"))?;
        let out = saleor_rustify_db::order_promotions::refresh_order_promotion(db, token).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(match out {
            saleor_rustify_db::order_promotions::RefreshOutcome::Cleared => "none".into(),
            saleor_rustify_db::order_promotions::RefreshOutcome::Discount { rule_id, amount } => format!("discount:{rule_id}:{amount}"),
            saleor_rustify_db::order_promotions::RefreshOutcome::Gift { rule_id, variant_id, .. } => format!("gift:{rule_id}:{variant_id}"),
        })
    }

    /// Agentic buying: natural-language text → a real checkout with lines.
    /// Deterministic (no LLM): quantities + product phrases resolve against
    /// the catalog; unmatched phrases come back as `questions`, never as
    /// hallucinated lines. Custom root (not in the Saleor schema, so
    /// codegen never touches it).
    // ------------------------------------------------------------------
    // Saleor new-checkout-API names (storefront parity; same tables/logic
    // as the old-style roots above). All public (Django leaves checkout
    // mutations permission-free); price overrides need handle_checkouts.
    // ------------------------------------------------------------------

    async fn checkout_create(&self, ctx: &Context<'_>, input: CheckoutCreateInput) -> Result<CheckoutCreate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let ch = input.channel.clone().unwrap_or_else(|| "default-channel".into());
        let (ch_id, currency) = saleor_rustify_db::catalog::channel_info(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        if input.lines.is_empty() {
            return Ok(CheckoutCreate { checkout: None, errors: ckerr("at least one line is required".into()) });
        }
        let token = saleor_rustify_db::checkout_store::create_checkout_row(db, ch_id, &currency, input.email.as_deref().unwrap_or("")).await.map_err(|e| Error::new(e.to_string()))?;
        let items: Vec<(i32, Option<rust_decimal::Decimal>)> = input.lines.iter()
            .filter_map(|l| saleor_rustify_db::catalog::parse_gid(&l.variant_id.0).map(|v| (v, parse_price(&l.price))))
            .collect();
        if items.is_empty() {
            return Ok(CheckoutCreate { checkout: None, errors: ckerr("unknown variants".into()) });
        }
        let units = unit_prices(ctx, db, &ch, &items).await?;
        for (i, l) in input.lines.iter().enumerate() {
            let vid = items.get(i).map(|(v, _)| *v).unwrap_or(0);
            if vid == 0 { continue; }
            let qty = l.quantity.max(1);
            let unit = units.get(&vid).copied().unwrap_or(rust_decimal::Decimal::ZERO);
            // force_new_line / metadata ride on the line row when present.
            let lid = saleor_rustify_db::checkout_store::add_line_row(db, token, vid, qty, unit, &currency, None).await.map_err(|e| Error::new(e.to_string()))?;
            let _ = (lid, l.force_new_line);
        }
        if let Some(a) = input.shipping_address.as_ref() {
            if let Err(e) = save_checkout_address(db, token, false, a).await { return Ok(CheckoutCreate { checkout: None, errors: ckerr(e) }); }
        }
        if let Some(a) = input.billing_address.as_ref() {
            if let Err(e) = save_checkout_address(db, token, true, a).await { return Ok(CheckoutCreate { checkout: None, errors: ckerr(e) }); }
        }
        if let Some(l) = input.language_code.as_ref() {
            let _ = saleor_rustify_db::checkout_store::set_language_code(db, token, crate::gen::language_code_value(l).to_string()).await;
        }
        if let Some(m) = input.metadata.as_ref() {
            let merged = crate::common::merge_metadata(&serde_json::Value::Null, m);
            let _ = saleor_rustify_db::checkout_store::set_metadata_opt(db, token, Some(merged)).await;
        }
        let _ = saleor_rustify_db::checkout_store::refresh_totals(db, token).await;
        Ok(CheckoutCreate { checkout: Some(load_gql(db, token, &ch).await?), errors: vec![] })
    }

    async fn checkout_lines_add(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, id: Option<ID>,
        lines: Vec<gen::CheckoutLineInput>, token: Option<String>,
    ) -> Result<CheckoutLinesAdd> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutLinesAdd { checkout: None, errors: ckerr(e) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        let (ch_id, currency) = saleor_rustify_db::catalog::channel_info(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        let items: Vec<(i32, i32, Option<rust_decimal::Decimal>)> = lines.iter()
            .filter_map(|l| saleor_rustify_db::catalog::parse_gid(&l.variant_id.0).map(|v| (v, l.quantity.max(1), parse_price(&l.price))))
            .collect();
        let units = unit_prices(ctx, db, &ch, &items.iter().map(|(v, _, p)| (*v, *p)).collect::<Vec<_>>()).await?;
        let nl: Vec<saleor_rustify_db::checkout_store::NewLine> = items.into_iter().map(|(vid, qty, _)| {
            saleor_rustify_db::checkout_store::NewLine { variant_id: vid, quantity: qty, unit_price: units.get(&vid).copied().unwrap_or(rust_decimal::Decimal::ZERO), price_override: None }
        }).collect();
        if let Err(e) = saleor_rustify_db::checkout_store::add_lines_tx(db, t, ch_id, &currency, &nl).await {
            return Ok(CheckoutLinesAdd { checkout: None, errors: ckerr(e.to_string()) });
        }
        let _ = saleor_rustify_db::checkout_store::refresh_totals(db, t).await;
        Ok(CheckoutLinesAdd { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_lines_update(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, id: Option<ID>,
        lines: Vec<gen::CheckoutLineUpdateInput>, token: Option<String>,
    ) -> Result<CheckoutLinesUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutLinesUpdate { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        for l in &lines {
            // lineId preferred, legacy variantId honored.
            let lid = l.line_id.as_ref().and_then(|i| crate::common::parse_uuid_gid(&i.0));
            let Some(lid) = lid else {
                return Ok(CheckoutLinesUpdate { checkout: None, errors: ckerr("lineId is required".into()) });
            };
            let qty = l.quantity.unwrap_or(1).max(0);
            if let Err(e) = saleor_rustify_db::checkout_store::update_line_quantity(db, t, lid, qty).await {
                return Ok(CheckoutLinesUpdate { checkout: None, errors: ckerr(e.to_string()) });
            }
        }
        let _ = saleor_rustify_db::checkout_store::refresh_totals(db, t).await;
        Ok(CheckoutLinesUpdate { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_line_delete(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, id: Option<ID>,
        #[graphql(name = "lineId")] line_id: Option<ID>, token: Option<String>,
    ) -> Result<CheckoutLineDelete> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutLineDelete { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        let Some(lid) = line_id.as_ref().and_then(|i| crate::common::parse_uuid_gid(&i.0)) else {
            return Ok(CheckoutLineDelete { checkout: None, errors: ckerr("lineId is required".into()) });
        };
        if let Err(e) = saleor_rustify_db::checkout_store::delete_line(db, t, lid).await {
            return Ok(CheckoutLineDelete { checkout: None, errors: ckerr(e.to_string()) });
        }
        let _ = saleor_rustify_db::checkout_store::refresh_totals(db, t).await;
        Ok(CheckoutLineDelete { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_delete(&self, ctx: &Context<'_>, id: ID) -> Result<CheckoutDelete> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(Some(id), None, None) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutDelete { checkout: None, errors: ckerr(e.to_string()) }),
        };
        if let Err(e) = saleor_rustify_db::checkout_store::delete_checkout_row(db, t).await {
            return Ok(CheckoutDelete { checkout: None, errors: ckerr(e.to_string()) });
        }
        Ok(CheckoutDelete { checkout: None, errors: vec![] })
    }

    async fn checkout_email_update(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, email: String, id: Option<ID>, token: Option<String>,
    ) -> Result<CheckoutEmailUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutEmailUpdate { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        if let Err(e) = saleor_rustify_db::checkout_store::set_email_opt(db, t, Some(email)).await {
            return Ok(CheckoutEmailUpdate { checkout: None, errors: ckerr(e.to_string()) });
        }
        Ok(CheckoutEmailUpdate { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_customer_attach(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>,
        #[graphql(name = "customerId")] customer_id: Option<ID>, id: Option<ID>, token: Option<String>,
    ) -> Result<CheckoutCustomerAttach> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutCustomerAttach { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        let uid = customer_id.as_ref().and_then(|i| saleor_rustify_db::catalog::parse_gid(&i.0));
        if let Err(e) = saleor_rustify_db::checkout_store::set_customer_opt(db, t, uid).await {
            return Ok(CheckoutCustomerAttach { checkout: None, errors: ckerr(e.to_string()) });
        }
        Ok(CheckoutCustomerAttach { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_customer_detach(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, id: Option<ID>, token: Option<String>,
    ) -> Result<CheckoutCustomerDetach> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutCustomerDetach { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        if let Err(e) = saleor_rustify_db::checkout_store::set_customer_opt(db, t, None).await {
            return Ok(CheckoutCustomerDetach { checkout: None, errors: ckerr(e.to_string()) });
        }
        Ok(CheckoutCustomerDetach { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_customer_note_update(&self, ctx: &Context<'_>, #[graphql(name = "customerNote")] customer_note: String, id: ID) -> Result<CheckoutCustomerNoteUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(Some(id), None, None) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutCustomerNoteUpdate { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        if let Err(e) = saleor_rustify_db::checkout_store::set_note_text(db, t, customer_note).await {
            return Ok(CheckoutCustomerNoteUpdate { checkout: None, errors: ckerr(e.to_string()) });
        }
        Ok(CheckoutCustomerNoteUpdate { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_shipping_address_update(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, id: Option<ID>,
        #[graphql(name = "shippingAddress")] shipping_address: gen::AddressInput, token: Option<String>,
    ) -> Result<CheckoutShippingAddressUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutShippingAddressUpdate { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        if let Err(e) = save_checkout_address(db, t, false, &shipping_address).await {
            return Ok(CheckoutShippingAddressUpdate { checkout: None, errors: ckerr(e) });
        }
        Ok(CheckoutShippingAddressUpdate { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_billing_address_update(
        &self, ctx: &Context<'_>,
        #[graphql(name = "billingAddress")] billing_address: gen::AddressInput,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, id: Option<ID>, token: Option<String>,
    ) -> Result<CheckoutBillingAddressUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutBillingAddressUpdate { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        if let Err(e) = save_checkout_address(db, t, true, &billing_address).await {
            return Ok(CheckoutBillingAddressUpdate { checkout: None, errors: ckerr(e) });
        }
        Ok(CheckoutBillingAddressUpdate { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_shipping_method_update(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, id: Option<ID>,
        #[graphql(name = "shippingMethodId")] shipping_method_id: Option<ID>, token: Option<String>,
    ) -> Result<CheckoutShippingMethodUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutShippingMethodUpdate { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        let mid = shipping_method_id.as_ref().and_then(|i| saleor_rustify_db::catalog::parse_gid(&i.0));
        if let Err(e) = saleor_rustify_db::checkout_store::set_shipping_method_opt(db, t, mid).await {
            return Ok(CheckoutShippingMethodUpdate { checkout: None, errors: ckerr(e.to_string()) });
        }
        let _ = saleor_rustify_db::checkout_store::refresh_totals(db, t).await;
        Ok(CheckoutShippingMethodUpdate { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_delivery_method_update(
        &self, ctx: &Context<'_>,
        #[graphql(name = "deliveryMethodId")] delivery_method_id: Option<ID>, id: Option<ID>, token: Option<String>,
    ) -> Result<CheckoutDeliveryMethodUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, None, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutDeliveryMethodUpdate { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        if let Some(dm) = delivery_method_id.as_ref() {
            // Shipping method (int gid) or collection point (warehouse UUID).
            if let Some(mid) = saleor_rustify_db::catalog::parse_gid(&dm.0) {
                if let Err(e) = saleor_rustify_db::checkout_store::set_shipping_method_opt(db, t, Some(mid)).await {
                    return Ok(CheckoutDeliveryMethodUpdate { checkout: None, errors: ckerr(e.to_string()) });
                }
                let _ = saleor_rustify_db::checkout_store::set_collection_point_opt(db, t, None).await;
            } else if let Some(wid) = crate::common::parse_uuid_gid(&dm.0) {
                if let Err(e) = saleor_rustify_db::checkout_store::set_collection_point_opt(db, t, Some(wid)).await {
                    return Ok(CheckoutDeliveryMethodUpdate { checkout: None, errors: ckerr(e.to_string()) });
                }
                let _ = saleor_rustify_db::checkout_store::set_shipping_method_opt(db, t, None).await;
            } else {
                return Ok(CheckoutDeliveryMethodUpdate { checkout: None, errors: ckerr("bad delivery method id".into()) });
            }
        } else {
            let _ = saleor_rustify_db::checkout_store::set_shipping_method_opt(db, t, None).await;
            let _ = saleor_rustify_db::checkout_store::set_collection_point_opt(db, t, None).await;
        }
        let _ = saleor_rustify_db::checkout_store::refresh_totals(db, t).await;
        Ok(CheckoutDeliveryMethodUpdate { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_language_code_update(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, id: Option<ID>,
        #[graphql(name = "languageCode")] language_code: gen::LanguageCodeEnum, token: Option<String>,
    ) -> Result<CheckoutLanguageCodeUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutLanguageCodeUpdate { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        if let Err(e) = saleor_rustify_db::checkout_store::set_language_code(db, t, crate::gen::language_code_value(&language_code).to_string()).await {
            return Ok(CheckoutLanguageCodeUpdate { checkout: None, errors: ckerr(e.to_string()) });
        }
        Ok(CheckoutLanguageCodeUpdate { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_add_promo_code(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, id: Option<ID>,
        #[graphql(name = "promoCode")] promo_code: String, token: Option<String>,
    ) -> Result<CheckoutAddPromoCode> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutAddPromoCode { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        // Voucher first, gift card second (Django tries both code spaces).
        let voucher_ok = saleor_rustify_db::promotions::apply_voucher(db, t, &promo_code, &ch).await.is_ok();
        if !voucher_ok {
            if let Err(e) = saleor_rustify_db::giftcards::attach_to_checkout(db, t, &promo_code, &co.currency, co.user_id).await {
                return Ok(CheckoutAddPromoCode { checkout: None, errors: ckerr(format!("invalid promo code: {e}")) });
            }
        }
        let _ = saleor_rustify_db::checkout_store::refresh_totals(db, t).await;
        Ok(CheckoutAddPromoCode { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn checkout_remove_promo_code(
        &self, ctx: &Context<'_>,
        #[graphql(name = "checkoutId")] checkout_id: Option<ID>, id: Option<ID>,
        #[graphql(name = "promoCode")] promo_code: Option<String>,
        #[graphql(name = "promoCodeId")] promo_code_id: Option<ID>, token: Option<String>,
    ) -> Result<CheckoutRemovePromoCode> {
        let _ = promo_code_id;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let t = match resolve_token(id, checkout_id, token) {
            Ok(t) => t,
            Err(e) => return Ok(CheckoutRemovePromoCode { checkout: None, errors: ckerr(e.to_string()) }),
        };
        let (co, _) = saleor_rustify_db::checkout_store::load_checkout(db, t).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
        let ch = channel_of(&co);
        let code = promo_code.clone().unwrap_or_default();
        let cleared = saleor_rustify_db::checkout_store::remove_voucher_code(db, t, if code.is_empty() { None } else { Some(&code) }).await.map_err(|e| Error::new(e.to_string()))?;
        if !cleared && !code.is_empty() {
            // Maybe a gift card: detach is idempotent, missing → error.
            if let Err(e) = saleor_rustify_db::giftcards::detach_from_checkout(db, t, &code).await {
                return Ok(CheckoutRemovePromoCode { checkout: None, errors: ckerr(format!("promo code not found: {e}")) });
            }
        }
        let _ = saleor_rustify_db::checkout_store::refresh_totals(db, t).await;
        Ok(CheckoutRemovePromoCode { checkout: Some(load_gql(db, t, &ch).await?), errors: vec![] })
    }

    async fn agent_checkout(&self, ctx: &Context<'_>, message: String, channel: Option<String>, email: Option<String>) -> Result<AgentCheckoutResult> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let ch = channel.unwrap_or_else(|| "default-channel".into());
        let plan = saleor_rustify_ai::agent::plan_checkout(db, &ch, &message).await.map_err(|e| Error::new(e.to_string()))?;
        if plan.lines.is_empty() {
            return Err(Error::new(plan.questions.first().cloned().unwrap_or_else(|| "no purchasable items found".into())));
        }
        let (ch_id, currency) = saleor_rustify_db::catalog::channel_info(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        let token = saleor_rustify_db::checkout_store::create_checkout_row(db, ch_id, &currency, email.as_deref().unwrap_or("")).await.map_err(|e| Error::new(e.to_string()))?;
        let vids: Vec<i32> = plan.lines.iter().map(|l| l.variant_id).collect();
        let pricing = saleor_rustify_db::catalog::checkout_pricing(db, &ch, &vids).await.map_err(|e| Error::new(e.to_string()))?;
        let items: Vec<saleor_rustify_db::checkout_store::NewLine> = plan.lines.iter().map(|l| {
            let unit = pricing.get(&l.variant_id).map(|(m, _)| m.amount).unwrap_or(rust_decimal::Decimal::ZERO);
            saleor_rustify_db::checkout_store::NewLine { variant_id: l.variant_id, quantity: l.quantity, unit_price: unit, price_override: None }
        }).collect();
        saleor_rustify_db::checkout_store::add_lines_tx(db, token, ch_id, &currency, &items).await.map_err(|e| Error::new(e.to_string()))?;
        saleor_rustify_db::checkout_store::refresh_totals(db, token).await.map_err(|e| Error::new(e.to_string()))?;
        let (co, lines) = saleor_rustify_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout vanished"))?;
        Ok(AgentCheckoutResult { checkout: to_gql_checkout(&co, &lines, &ch), notes: plan.notes, questions: plan.questions })
    }
}
