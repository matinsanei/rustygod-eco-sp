//! Payment GraphQL: transactions, PSP callbacks, granted refunds.
//! Mirrors `PaymentService` + `OrderService` grant RPCs — same DB calls.
use async_graphql::*;

use crate::context::GqlContext;

#[derive(SimpleObject, Clone)]
pub struct GqlTransaction {
    pub id: ID,
    pub currency: String,
    pub authorized: String,
    pub charged: String,
    pub refunded: String,
    pub canceled: String,
}

#[derive(Default)]
pub struct PaymentQuery;

#[Object]
impl PaymentQuery {
    async fn transaction(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GqlTransaction>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let tid: i32 = id.0.parse().map_err(|_| Error::new("id must be int"))?;
        match rustygod_db::payments::view(db, tid).await {
            Ok(v) => Ok(Some(GqlTransaction { id: ID(v.id.to_string()), currency: v.currency, authorized: v.authorized.to_string(), charged: v.charged.to_string(), refunded: v.refunded.to_string(), canceled: v.canceled.to_string() })),
            Err(_) => Ok(None),
        }
    }
}

#[derive(Default)]
pub struct PaymentMutation;

#[Object]
impl PaymentMutation {
    async fn transaction_authorize(&self, ctx: &Context<'_>, transaction_id: ID, amount: String, idempotency_key: String) -> Result<GqlTransaction> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let tid: i32 = transaction_id.0.parse().map_err(|_| Error::new("transactionId must be int"))?;
        let amt: rust_decimal::Decimal = amount.parse().map_err(|_| Error::new("amount must be decimal"))?;
        let v = rustygod_db::payments::authorize(db, tid, amt, &idempotency_key).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlTransaction { id: ID(v.id.to_string()), currency: v.currency, authorized: v.authorized.to_string(), charged: v.charged.to_string(), refunded: v.refunded.to_string(), canceled: v.canceled.to_string() })
    }
    async fn transaction_charge(&self, ctx: &Context<'_>, transaction_id: ID, amount: String, idempotency_key: String) -> Result<GqlTransaction> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let tid: i32 = transaction_id.0.parse().map_err(|_| Error::new("transactionId must be int"))?;
        let amt: rust_decimal::Decimal = amount.parse().map_err(|_| Error::new("amount must be decimal"))?;
        let v = rustygod_db::payments::charge(db, tid, amt, &idempotency_key).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlTransaction { id: ID(v.id.to_string()), currency: v.currency, authorized: v.authorized.to_string(), charged: v.charged.to_string(), refunded: v.refunded.to_string(), canceled: v.canceled.to_string() })
    }
}
