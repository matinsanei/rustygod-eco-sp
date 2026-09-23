//! Payment GraphQL: transactions, PSP callbacks, granted refunds.
//! Mirrors `PaymentService` + `OrderService` grant RPCs — same DB calls.
use async_graphql::*;

use uuid::Uuid;

use crate::{context::GqlContext, gen};

#[derive(SimpleObject, Clone)]
pub struct GqlTransaction {
    pub id: ID,
    pub currency: String,
    pub authorized: String,
    pub charged: String,
    pub refunded: String,
    pub canceled: String,
}

#[derive(SimpleObject, Clone)]
pub struct GqlTransactionEdge { pub node: Option<GqlTransaction> }

#[derive(SimpleObject, Clone)]
pub struct GqlTransactionConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlTransactionEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: crate::common::PageInfo,
}

#[derive(SimpleObject, Clone)]
pub struct GqlPaymentEdge { pub node: Option<gen::Payment> }

#[derive(SimpleObject, Clone)]
pub struct GqlPaymentConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlPaymentEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: crate::common::PageInfo,
}

fn money(amount: rust_decimal::Decimal, currency: &str) -> crate::common::Money {
    crate::common::Money { amount: amount.to_string(), currency: currency.to_string(), fraction_digits: None }
}

fn payment_node(m: &rustygod_db::entities::payment_payment::Model) -> gen::Payment {
    let avail_capture = (m.total - m.captured_amount).max(rust_decimal::Decimal::ZERO);
    gen::Payment {
        id: Some(ID(crate::common::gid("Payment", m.id))),
        private_metadata: vec![],
        metadata: vec![],
        gateway: Some(m.gateway.clone()),
        is_active: Some(m.is_active),
        modified: Some(m.modified_at.into()),
        payment_method_type: Some(m.payment_method_type.clone()),
        actions: vec![],
        total: Some(money(m.total, &m.currency)),
        captured_amount: Some(money(m.captured_amount, &m.currency)),
        transactions: vec![],
        available_capture_amount: Some(money(avail_capture, &m.currency)),
        available_refund_amount: Some(money(m.captured_amount, &m.currency)),
    }
}

#[derive(Default)]
pub struct PaymentQuery;

#[Object]
impl PaymentQuery {
    async fn transaction(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GqlTransaction>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let tid: i32 = id.0.parse().map_err(|_| Error::new("id must be int"))?;
        match rustygod_db::payments::view(db, tid).await {
            Ok(v) => Ok(Some(GqlTransaction { id: ID(crate::common::gid("TransactionItem", v.id)), currency: v.currency, authorized: v.authorized.to_string(), charged: v.charged.to_string(), refunded: v.refunded.to_string(), canceled: v.canceled.to_string() })),
            Err(_) => Ok(None),
        }
    }

    /// Legacy payment by id (Django `payment`: MANAGE_ORDERS).
    async fn payment(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::Payment>> {
        crate::account::require_perm(ctx, "manage_orders").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(pid) = rustygod_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        use sea_orm::EntityTrait;
        Ok(rustygod_db::entities::payment_payment::Entity::find_by_id(pid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?.map(|m| payment_node(&m)))
    }

    async fn payments(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        filter: Option<gen::PaymentFilterInput>,
    ) -> Result<Option<GqlPaymentConnection>> {
        crate::account::require_perm(ctx, "manage_orders").await?;
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder};
        use rustygod_db::entities::payment_payment::{Column as PCol, Entity as PEnt};
        let mut cond = Condition::all();
        if let Some(f) = filter.as_ref() {
            let ids = crate::catalog::gid_vec(f.ids.clone());
            if !ids.is_empty() { cond = cond.add(PCol::Id.is_in(ids)); }
            // Checkout ids arrive as UUID gids; match the link column.
            let cok: Vec<Uuid> = f.checkouts.as_ref().map(|v| v.as_slice()).unwrap_or(&[])
                .iter().filter_map(|i| crate::common::parse_uuid_gid(&i.0)).collect();
            if !cok.is_empty() { cond = cond.add(PCol::CheckoutId.is_in(cok)); }
        }
        let rows = PEnt::find().filter(cond).order_by_desc(PCol::Id)
            .all(db).await.map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len();
        let edges = rows.into_iter().skip(off).take(lim)
            .map(|m| GqlPaymentEdge { node: Some(payment_node(&m)) }).collect();
        Ok(Some(GqlPaymentConnection {
            total_count: Some(total as i32),
            edges,
            page_info: crate::common::PageInfo { has_next_page: off + lim < total, has_previous_page: off > 0, start_cursor: None, end_cursor: None },
        }))
    }

    /// Transaction items with where/sort (Django `transactions`).
    async fn transactions(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        #[graphql(name = "where")] where_input: Option<gen::TransactionWhereInput>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::TransactionSortingInput>,
    ) -> Result<Option<GqlTransactionConnection>> {
        crate::account::require_any_perm(ctx, &["manage_orders", "handle_payments"]).await?;
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
        use rustygod_db::entities::payment_transactionitem::{Column as TCol, Entity as TEnt};
        fn apply_where(cond: Condition, w: &gen::TransactionWhereInput) -> Condition {
            let mut c = cond;
            let ids = crate::catalog::gid_vec(w.ids.clone());
            if !ids.is_empty() { c = c.add(TCol::Id.is_in(ids)); }
            let (eq, one) = crate::catalog::str_filter(w.psp_reference.clone());
            if let Some(e) = eq { c = c.add(TCol::PspReference.eq(e)); }
            if !one.is_empty() { c = c.add(TCol::PspReference.is_in(one)); }
            let (aeq, aone) = crate::catalog::str_filter(w.app_identifier.clone());
            if let Some(e) = aeq { c = c.add(TCol::AppIdentifier.eq(e)); }
            if !aone.is_empty() { c = c.add(TCol::AppIdentifier.is_in(aone)); }
            if let Some(r) = w.created_at.as_ref() {
                if let Some(gte) = r.gte { c = c.add(TCol::CreatedAt.gte(gte)); }
                if let Some(lte) = r.lte { c = c.add(TCol::CreatedAt.lte(lte)); }
            }
            c
        }
        let mut cond = Condition::all();
        if let Some(w) = where_input.as_ref() { cond = apply_where(cond, w); }
        let mut q = TEnt::find().filter(cond);
        let desc = sort_by.as_ref().map(|s| s.direction == gen::OrderDirection::DESC).unwrap_or(true);
        q = match sort_by.as_ref().map(|s| &s.field) {
            Some(gen::TransactionSortField::MODIFIEDAT) => if desc { q.order_by_desc(TCol::ModifiedAt) } else { q.order_by_asc(TCol::ModifiedAt) },
            _ => if desc { q.order_by_desc(TCol::CreatedAt) } else { q.order_by_asc(TCol::CreatedAt) },
        };
        // Slim select (full rows carry JSON blobs; list needs money + refs).
        let rows: Vec<(i32, String, rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal)> = q
            .select_only()
            .column(TCol::Id).column(TCol::Currency)
            .column(TCol::AuthorizedValue).column(TCol::ChargedValue)
            .column(TCol::RefundedValue).column(TCol::CanceledValue)
            .into_tuple().all(db).await.map_err(|e| Error::new(e.to_string()))?;
        let total = rows.len();
        let edges = rows.into_iter().skip(off).take(lim).map(|(id, cur, a, ch, rf, cx)| {
            GqlTransactionEdge { node: Some(GqlTransaction {
                id: ID(crate::common::gid("TransactionItem", id)),
                currency: cur,
                authorized: a.to_string(), charged: ch.to_string(),
                refunded: rf.to_string(), canceled: cx.to_string(),
            }) }
        }).collect();
        Ok(Some(GqlTransactionConnection {
            total_count: Some(total as i32),
            edges,
            page_info: crate::common::PageInfo { has_next_page: off + lim < total, has_previous_page: off > 0, start_cursor: None, end_cursor: None },
        }))
    }
}

#[derive(Default)]
pub struct PaymentMutation;

#[Object]
impl PaymentMutation {
    async fn transaction_authorize(&self, ctx: &Context<'_>, transaction_id: ID, amount: String, idempotency_key: String) -> Result<GqlTransaction> {
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let tid: i32 = transaction_id.0.parse().map_err(|_| Error::new("transactionId must be int"))?;
        let amt: rust_decimal::Decimal = amount.parse().map_err(|_| Error::new("amount must be decimal"))?;
        let v = rustygod_db::payments::authorize(db, tid, amt, &idempotency_key).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlTransaction { id: ID(crate::common::gid("TransactionItem", v.id)), currency: v.currency, authorized: v.authorized.to_string(), charged: v.charged.to_string(), refunded: v.refunded.to_string(), canceled: v.canceled.to_string() })
    }
    async fn transaction_charge(&self, ctx: &Context<'_>, transaction_id: ID, amount: String, idempotency_key: String) -> Result<GqlTransaction> {
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let tid: i32 = transaction_id.0.parse().map_err(|_| Error::new("transactionId must be int"))?;
        let amt: rust_decimal::Decimal = amount.parse().map_err(|_| Error::new("amount must be decimal"))?;
        let v = rustygod_db::payments::charge(db, tid, amt, &idempotency_key).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlTransaction { id: ID(crate::common::gid("TransactionItem", v.id)), currency: v.currency, authorized: v.authorized.to_string(), charged: v.charged.to_string(), refunded: v.refunded.to_string(), canceled: v.canceled.to_string() })
    }
}
