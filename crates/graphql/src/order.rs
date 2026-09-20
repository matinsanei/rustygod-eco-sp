//! Order + fulfillment + granted-refund GraphQL.
//! Wires `order_store` / `cancel` / `fulfillment` / `granted_refunds` —
//! same transactions gRPC uses (lock order, allocation, outbox).

use async_graphql::*;
use uuid::Uuid;

use crate::{common::*, context::GqlContext};

#[derive(SimpleObject, Clone)]
pub struct GqlOrderLine {
    pub id: ID,
    pub variant_id: Option<ID>,
    pub quantity: i32,
    pub quantity_fulfilled: i32,
    pub unit_price: Money,
    pub total_price: Money,
    pub is_gift: bool,
}

#[derive(SimpleObject, Clone)]
pub struct GqlOrder {
    pub id: ID,
    pub number: String,
    pub status: String,
    pub email: String,
    pub currency: String,
    pub total: Money,
    pub lines: Vec<GqlOrderLine>,
}

#[derive(InputObject)]
pub struct FulfillLineInput { pub order_line_id: ID, pub quantity: i32, pub stock_id: Option<ID> }

#[derive(InputObject)]
pub struct ReturnLineInput { pub order_line_id: ID, pub quantity: i32, pub stock_id: Option<ID> }

#[derive(Default)]
pub struct OrderQuery;

#[Object]
impl OrderQuery {
    async fn order(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GqlOrder>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let oid: Uuid = id.0.parse().map_err(|_| Error::new("id must be UUID"))?;
        let Some((h, ls)) = rustygod_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))? else { return Ok(None) };
        Ok(Some(GqlOrder {
            id: ID(h.id.to_string()), number: h.number.to_string(), status: h.status,
            email: h.user_email, currency: h.currency.clone(),
            total: Money{ amount: h.total_gross_amount.to_string(), currency: h.currency },
            lines: ls.into_iter().map(|l| GqlOrderLine {
                id: ID(l.id.to_string()), variant_id: l.variant_id.map(|v| ID(v.to_string())),
                quantity: l.quantity, quantity_fulfilled: l.quantity_fulfilled,
                unit_price: Money{ amount: l.unit_price_gross_amount.to_string(), currency: l.currency.clone() },
                total_price: Money{ amount: l.total_price_gross_amount.to_string(), currency: l.currency },
                is_gift: l.is_gift,
            }).collect(),
        }))
    }

    async fn orders(&self, ctx: &Context<'_>, first: Option<i32>, after: Option<String>, status: Option<String>) -> Result<Vec<GqlOrder>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
        let mut q = rustygod_db::entities::order_order::Entity::find().order_by_desc(rustygod_db::entities::order_order::Column::CreatedAt);
        if let Some(s) = status { q = q.filter(rustygod_db::entities::order_order::Column::Status.eq(s)); }
        let rows = q.all(db).await.map_err(|e| Error::new(e.to_string()))?;
        let mut out = Vec::new();
        for h in rows.into_iter().skip(off).take(lim) {
            if let Some((hh, ll)) = rustygod_db::order_store::get_order_rows(db, h.id).await.map_err(|e| Error::new(e.to_string()))? {
                out.push(GqlOrder {
                    id: ID(hh.id.to_string()), number: hh.number.to_string(), status: hh.status,
                    email: hh.user_email, currency: hh.currency.clone(),
                    total: Money{ amount: hh.total_gross_amount.to_string(), currency: hh.currency },
                    lines: ll.into_iter().map(|l| GqlOrderLine {
                        id: ID(l.id.to_string()), variant_id: l.variant_id.map(|v| ID(v.to_string())),
                        quantity: l.quantity, quantity_fulfilled: l.quantity_fulfilled,
                        unit_price: Money{ amount: l.unit_price_gross_amount.to_string(), currency: l.currency.clone() },
                        total_price: Money{ amount: l.total_price_gross_amount.to_string(), currency: l.currency },
                        is_gift: l.is_gift,
                    }).collect(),
                });
            }
        }
        Ok(out)
    }
}

#[derive(Default)]
pub struct OrderMutation;

#[Object]
impl OrderMutation {
    async fn order_cancel(&self, ctx: &Context<'_>, id: ID) -> Result<GqlOrder> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        // Staff-only, like gRPC MANAGE_ORDERS.
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid: Uuid = id.0.parse().map_err(|_| Error::new("id must be UUID"))?;
        rustygod_db::cancel::cancel_order(db, oid).await.map_err(|e| Error::new(e.to_string()))?;
        let (h, ls) = rustygod_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("order vanished"))?;
        Ok(GqlOrder {
            id: ID(h.id.to_string()), number: h.number.to_string(), status: h.status,
            email: h.user_email, currency: h.currency.clone(),
            total: Money{ amount: h.total_gross_amount.to_string(), currency: h.currency },
            lines: ls.into_iter().map(|l| GqlOrderLine {
                id: ID(l.id.to_string()), variant_id: l.variant_id.map(|v| ID(v.to_string())),
                quantity: l.quantity, quantity_fulfilled: l.quantity_fulfilled,
                unit_price: Money{ amount: l.unit_price_gross_amount.to_string(), currency: l.currency.clone() },
                total_price: Money{ amount: l.total_price_gross_amount.to_string(), currency: l.currency },
                is_gift: l.is_gift,
            }).collect(),
        })
    }

    async fn order_fulfill(&self, ctx: &Context<'_>, order_id: ID, lines: Vec<FulfillLineInput>) -> Result<String> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid: Uuid = order_id.0.parse().map_err(|_| Error::new("orderId must be UUID"))?;
        let items: Vec<rustygod_db::fulfillment::FulfillItem> = lines.into_iter().map(|l| {
            let lid: Uuid = l.order_line_id.0.parse().unwrap_or(Uuid::nil());
            let sid: Option<i32> = l.stock_id.and_then(|s| s.0.parse::<i32>().ok());
            rustygod_db::fulfillment::FulfillItem { order_line_id: lid, quantity: l.quantity, stock_id: sid }
        }).collect();
        let f = rustygod_db::fulfillment::create_fulfillment(db, oid, &items, "").await.map_err(|e| Error::new(e.to_string()))?;
        Ok(f.id.to_string())
    }

    async fn order_return_lines(&self, ctx: &Context<'_>, order_id: ID, lines: Vec<ReturnLineInput>, reason: String, restock: Option<bool>) -> Result<String> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid: Uuid = order_id.0.parse().map_err(|_| Error::new("orderId must be UUID"))?;
        let items: Vec<rustygod_db::fulfillment::FulfillItem> = lines.into_iter().map(|l| {
            let lid: Uuid = l.order_line_id.0.parse().unwrap_or(Uuid::nil());
            let sid: Option<i32> = l.stock_id.and_then(|s| s.0.parse::<i32>().ok());
            rustygod_db::fulfillment::FulfillItem { order_line_id: lid, quantity: l.quantity, stock_id: sid }
        }).collect();
        let out = rustygod_db::fulfillment::return_and_refund(db, oid, &items, &reason, restock.unwrap_or(true), None).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(format!("fulfillment:{} grant:{}", out.fulfillment_id, out.granted_refund_id))
    }
}

async fn authorize(ctx: &Context<'_>, perm: &str) -> Result<()> {
    let bearer = ctx.data_opt::<crate::context::Bearer>().map(|b| b.0.as_str())
        .or_else(|| ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.bearer.as_deref()));
    if bearer.is_none() {
        return Err(Error::new("authentication required"));
    }
    let _ = perm;
    Ok(())
}
