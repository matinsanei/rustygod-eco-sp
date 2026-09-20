//! Checkout GraphQL: create / get / addLines / complete + voucher/promotion.
//! Every resolver is a thin call into `db::checkout_store` / `db::promotions`
//! / `db::order_promotions` — the same functions gRPC uses.

use async_graphql::*;
use uuid::Uuid;

use crate::{common::*, context::GqlContext};

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
    co: &rustygod_db::entities::checkout_checkout::Model,
    lines: &[rustygod_db::entities::checkout_checkoutline::Model],
    channel: &str,
) -> GqlCheckout {
    let d = rustygod_db::checkout_store::to_domain(co, lines, channel);
    let total = Money { amount: co.total_gross_amount.to_string(), currency: co.currency.clone(), fraction_digits: None };
    GqlCheckout {
        id: ID(d.id),
        channel: d.channel,
        email: d.email,
        currency: d.currency.clone(),
        total: total.clone(),
        lines: d.lines.into_iter().map(|l| {
            let tot = if l.is_gift { rust_decimal::Decimal::ZERO } else { l.unit_price.amount * rust_decimal::Decimal::from(l.quantity) };
            let currency = l.unit_price.currency.clone();
            GqlCheckoutLine {
                variant_id: ID(l.variant_id),
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

#[derive(Default)]
pub struct CheckoutQuery;

#[Object]
impl CheckoutQuery {
    async fn checkout(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GqlCheckout>> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let token: Uuid = id.0.parse().map_err(|_| Error::new("id must be UUID"))?;
        let Some((co, lines)) = rustygod_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))? else { return Ok(None) };
        // Channel slug round-trip via channel_id (v1 ids 1/2 as in server).
        let ch = match co.channel_id { 2 => "channel-pln", _ => "default-channel" };
        Ok(Some(to_gql_checkout(&co, &lines, ch)))
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
        let (ch_id, currency) = rustygod_db::catalog::channel_info(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        let token = rustygod_db::checkout_store::create_checkout_row(db, ch_id, &currency, &email).await.map_err(|e| Error::new(e.to_string()))?;
        let (co, lines) = rustygod_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("just-inserted checkout missing"))?;
        Ok(to_gql_checkout(&co, &lines, &ch))
    }

    async fn checkout_add_lines(&self, ctx: &Context<'_>, checkout_id: ID, lines: Vec<AddLineInput>) -> Result<GqlCheckout> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let token: Uuid = checkout_id.0.parse().map_err(|_| Error::new("checkoutId must be UUID"))?;
        let Some((co, _)) = rustygod_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))? else { return Err(Error::new("checkout not found")) };
        let ch = match co.channel_id { 2 => "channel-pln", _ => "default-channel" };
        let (ch_id, currency) = rustygod_db::catalog::channel_info(db, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        // Price from channel listings (same as gRPC AddLines).
        let vids: Vec<i32> = lines.iter().filter_map(|l| l.variant_id.0.parse::<i32>().ok()).collect();
        let pricing = rustygod_db::catalog::checkout_pricing(db, &ch, &vids).await.map_err(|e| Error::new(e.to_string()))?;
        let items: Vec<rustygod_db::checkout_store::NewLine> = lines.into_iter().map(|l| {
            let vid: i32 = l.variant_id.0.parse().unwrap_or(0);
            let unit = pricing.get(&vid).map(|(m, _)| m.amount).unwrap_or(rust_decimal::Decimal::ZERO);
            rustygod_db::checkout_store::NewLine { variant_id: vid, quantity: l.quantity, unit_price: unit, price_override: None }
        }).collect();
        rustygod_db::checkout_store::add_lines_tx(db, token, ch_id, &currency, &items).await.map_err(|e| Error::new(e.to_string()))?;
        let (co2, ls2) = rustygod_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout vanished"))?;
        Ok(to_gql_checkout(&co2, &ls2, &ch))
    }

    async fn checkout_complete(&self, ctx: &Context<'_>, checkout_id: ID) -> Result<ID> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let token: Uuid = checkout_id.0.parse().map_err(|_| Error::new("checkoutId must be UUID"))?;
        let out = rustygod_db::complete::complete_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(ID(out.order_id.to_string()))
    }

    async fn checkout_apply_voucher(&self, ctx: &Context<'_>, checkout_id: ID, code: String) -> Result<GqlCheckout> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let token: Uuid = checkout_id.0.parse().map_err(|_| Error::new("checkoutId must be UUID"))?;
        let ch = {
            let (co, _) = rustygod_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout not found"))?;
            match co.channel_id { 2 => "channel-pln".to_string(), _ => "default-channel".to_string() }
        };
        rustygod_db::promotions::apply_voucher(db, token, &code, &ch).await.map_err(|e| Error::new(e.to_string()))?;
        rustygod_db::checkout_store::refresh_totals(db, token).await.map_err(|e| Error::new(e.to_string()))?;
        let (co2, ls2) = rustygod_db::checkout_store::load_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("checkout vanished"))?;
        Ok(to_gql_checkout(&co2, &ls2, &ch))
    }

    async fn refresh_order_promotion(&self, ctx: &Context<'_>, checkout_id: ID) -> Result<String> {
        let g = ctx.data::<GqlContext>()?;
        let db = g.db()?;
        let token: Uuid = checkout_id.0.parse().map_err(|_| Error::new("checkoutId must be UUID"))?;
        let out = rustygod_db::order_promotions::refresh_order_promotion(db, token).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(match out {
            rustygod_db::order_promotions::RefreshOutcome::Cleared => "none".into(),
            rustygod_db::order_promotions::RefreshOutcome::Discount { rule_id, amount } => format!("discount:{rule_id}:{amount}"),
            rustygod_db::order_promotions::RefreshOutcome::Gift { rule_id, variant_id, .. } => format!("gift:{rule_id}:{variant_id}"),
        })
    }
}
