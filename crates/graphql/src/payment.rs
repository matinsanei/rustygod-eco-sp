//! Payment GraphQL: transactions, PSP callbacks, granted refunds.
//! Mirrors `PaymentService` + `OrderService` grant RPCs — same DB calls.
use async_graphql::*;

use uuid::Uuid;

use sea_orm::EntityTrait;

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
pub struct GqlPaymentError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlTransactionUpdateError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlTransactionEventReportError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

fn perr(message: String) -> GqlPaymentError {
    GqlPaymentError { field: None, message: Some(message), code: None }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "StoredPaymentMethodRequestDelete")]
pub struct GqlStoredPaymentMethodRequestDelete {
    pub result: gen::StoredPaymentMethodRequestDeleteResult,
    pub errors: Vec<GqlPaymentMethodRequestDeleteError>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentMethodRequestDeleteError")]
pub struct GqlPaymentMethodRequestDeleteError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentGatewayConfigError")]
pub struct GqlPaymentGatewayConfigError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: GqlPaymentGatewayConfigErrorCode,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
#[graphql(name = "PaymentGatewayConfigErrorCode")]
pub enum GqlPaymentGatewayConfigErrorCode {
    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,
    #[graphql(name = "INVALID")]
    INVALID,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentGatewayConfig")]
pub struct GqlPaymentGatewayConfig {
    pub id: String,
    pub data: Option<serde_json::Value>,
    pub errors: Vec<GqlPaymentGatewayConfigError>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentGatewayInitialize")]
pub struct GqlPaymentGatewayInitialize {
    #[graphql(name = "gatewayConfigs")]
    pub gateway_configs: Vec<GqlPaymentGatewayConfig>,
    pub errors: Vec<GqlPaymentGatewayInitializeError>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentGatewayInitializeError")]
pub struct GqlPaymentGatewayInitializeError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
#[graphql(name = "PaymentGatewayInitializeTokenizationResult")]
pub enum GqlPaymentGatewayInitializeTokenizationResult {
    #[graphql(name = "SUCCESSFULLY_INITIALIZED")]
    SUCCESSFULLYINITIALIZED,
    #[graphql(name = "FAILED_TO_INITIALIZE")]
    FAILEDTOINITIALIZE,
    #[graphql(name = "FAILED_TO_DELIVER")]
    FAILEDTODELIVER,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentGatewayInitializeTokenizationError")]
pub struct GqlPaymentGatewayInitializeTokenizationError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentGatewayInitializeTokenization")]
pub struct GqlPaymentGatewayInitializeTokenization {
    pub result: GqlPaymentGatewayInitializeTokenizationResult,
    pub data: Option<serde_json::Value>,
    pub errors: Vec<GqlPaymentGatewayInitializeTokenizationError>,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
#[graphql(name = "PaymentMethodTokenizationResult")]
pub enum GqlPaymentMethodTokenizationResult {
    #[graphql(name = "SUCCESSFULLY_TOKENIZED")]
    SUCCESSFULLYTOKENIZED,
    #[graphql(name = "PENDING")]
    PENDING,
    #[graphql(name = "ADDITIONAL_ACTION_REQUIRED")]
    ADDITIONALACTIONREQUIRED,
    #[graphql(name = "FAILED_TO_TOKENIZE")]
    FAILEDTOTOKENIZE,
    #[graphql(name = "FAILED_TO_DELIVER")]
    FAILEDTODELIVER,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentMethodInitializeTokenizationError")]
pub struct GqlPaymentMethodInitializeTokenizationError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentMethodInitializeTokenization")]
pub struct GqlPaymentMethodInitializeTokenization {
    pub result: GqlPaymentMethodTokenizationResult,
    pub id: Option<String>,
    pub data: Option<serde_json::Value>,
    pub errors: Vec<GqlPaymentMethodInitializeTokenizationError>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentMethodProcessTokenizationError")]
pub struct GqlPaymentMethodProcessTokenizationError {
    pub field: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentMethodProcessTokenization")]
pub struct GqlPaymentMethodProcessTokenization {
    pub result: GqlPaymentMethodTokenizationResult,
    pub id: Option<String>,
    pub data: Option<serde_json::Value>,
    pub errors: Vec<GqlPaymentMethodProcessTokenizationError>,
}

fn tuerr(message: String) -> GqlTransactionUpdateError {
    GqlTransactionUpdateError { field: None, message: Some(message), code: None }
}

fn tererr(message: String) -> GqlTransactionEventReportError {
    GqlTransactionEventReportError { field: None, message: Some(message), code: None }
}

fn money_of(amount: rust_decimal::Decimal, currency: &str) -> crate::common::Money {
    crate::common::Money { amount: amount.to_string(), currency: currency.to_string(), fraction_digits: None }
}

/// Full TransactionItem assembly (amounts, actions, audit events).
pub(crate) async fn assemble_item(db: &sea_orm::DatabaseConnection, tid: i32) -> Result<gen::TransactionItem, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
    let m = saleor_rustify_db::entities::payment_transactionitem::Entity::find_by_id(tid)
        .one(db).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "transaction not found".to_string())?;
    let evs = saleor_rustify_db::entities::payment_transactionevent::Entity::find()
        .filter(saleor_rustify_db::entities::payment_transactionevent::Column::TransactionId.eq(tid))
        .order_by_asc(saleor_rustify_db::entities::payment_transactionevent::Column::Id)
        .all(db).await.map_err(|e| e.to_string())?;
    let cur = m.currency.clone();
    Ok(gen::TransactionItem {
        id: Some(ID(crate::common::gid("TransactionItem", m.id))),
        private_metadata: crate::common::json_to_metadata_items(&serde_json::to_value(&m.private_metadata).unwrap_or(serde_json::Value::Null)),
        metadata: crate::common::json_to_metadata_items(&serde_json::to_value(&m.metadata).unwrap_or(serde_json::Value::Null)),
        created_at: Some(m.created_at.into()),
        actions: m.available_actions.clone(),
        authorized_amount: Some(money_of(m.authorized_value, &cur)),
        authorize_pending_amount: Some(money_of(m.authorize_pending_value, &cur)),
        refunded_amount: Some(money_of(m.refunded_value, &cur)),
        refund_pending_amount: Some(money_of(m.refund_pending_value, &cur)),
        canceled_amount: Some(money_of(m.canceled_value, &cur)),
        cancel_pending_amount: Some(money_of(m.cancel_pending_value, &cur)),
        charged_amount: Some(money_of(m.charged_value, &cur)),
        charge_pending_amount: Some(money_of(m.charge_pending_value, &cur)),
        name: m.name.clone(),
        psp_reference: m.psp_reference.clone(),
        events: evs.into_iter().map(|e| gen::TransactionEvent {
            id: Some(ID(crate::common::gid("TransactionEvent", e.id))),
            created_at: Some(e.created_at.into()),
            psp_reference: e.psp_reference.clone(),
            message: e.message.clone(),
            reason_reference: None,
            external_url: e.external_url.clone(),
            amount: Some(money_of(e.amount_value, &e.currency)),
            r#type: Some(e.r#type.clone()),
            created_by: None,
        }).collect(),
        created_by: None,
        external_url: None,
        payment_method_details: None,
    })
}

async fn resolve_tid(db: &sea_orm::DatabaseConnection, id: Option<ID>, token: Option<String>) -> Result<i32, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    if let Some(i) = id {
        if let Some(t) = saleor_rustify_db::catalog::parse_gid(&i.0) {
            return Ok(t);
        }
    }
    if let Some(t) = token.and_then(|s| s.parse::<uuid::Uuid>().ok()) {
        if let Some(tid) = saleor_rustify_db::entities::payment_transactionitem::Entity::find()
            .select_only().column(saleor_rustify_db::entities::payment_transactionitem::Column::Id)
            .filter(saleor_rustify_db::entities::payment_transactionitem::Column::Token.eq(t))
            .into_tuple::<i32>().one(db).await.map_err(|e| format!("{e:?}"))? {
            return Ok(tid);
        }
    }
    Err("transaction id or token is required".to_string())
}

/// Owner-or-permission gate (Django `check_if_requestor_has_access`: the
/// owning user passes, everyone else needs HANDLE_PAYMENTS).
async fn txn_access(ctx: &Context<'_>, db: &sea_orm::DatabaseConnection, tid: i32) -> Result<i32, String> {
    use sea_orm::EntityTrait;
    let (uid, _) = crate::account::requester(ctx, db).await.map_err(|e| format!("{e:?}"))?;
    let owner: Option<Option<i32>> = saleor_rustify_db::entities::payment_transactionitem::Entity::find_by_id(tid)
        .one(db).await.map_err(|e| format!("{e:?}"))?.map(|m| m.user_id);
    match owner {
        Some(Some(o)) if o == uid => Ok(uid),
        _ => crate::account::require_perm(ctx, "handle_payments").await.map_err(|e| format!("{e:?}")),
    }
}

/// Resolve the PSP and execute one requested action (shared by the
/// plain and grant-linked refund paths).
async fn execute_with_psp(
    db: &sea_orm::DatabaseConnection,
    tid: i32,
    action: saleor_rustify_core::psp::PspAction,
    amt: rust_decimal::Decimal,
    key: &str,
    app: Option<&str>,
    message: Option<String>,
) -> Result<(), String> {
    let psp = psp_for(app)?;
    saleor_rustify_db::payments::request_action(db, tid, action, amt, key, psp.as_ref(), message)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// PSP choice for live execution: Stripe when the owning app names it and
/// a key is configured, manual ledger otherwise (documented fallback).
fn psp_for(app_identifier: Option<&str>) -> Result<Box<dyn saleor_rustify_core::psp::Psp>, String> {
    let wants_stripe = app_identifier.is_some_and(|a| a.to_lowercase().contains("stripe"));
    if wants_stripe {
        if let Some(s) = saleor_rustify_psp::StripePsp::from_env() {
            return Ok(Box::new(s));
        }
        return Err("stripe app selected but STRIPE_SECRET_KEY is unset".to_string());
    }
    Ok(Box::new(saleor_rustify_core::psp::ManualPsp))
}

fn legacy_payment_node(v: &saleor_rustify_db::payments::LegacyView) -> gen::Payment {
    gen::Payment {
        id: Some(ID(crate::common::gid("Payment", v.id))),
        private_metadata: vec![],
        metadata: vec![],
        gateway: Some(v.gateway.clone()),
        is_active: Some(v.is_active),
        modified: None,
        payment_method_type: None,
        actions: vec![],
        total: Some(money(v.total, &v.currency)),
        captured_amount: Some(money(v.captured, &v.currency)),
        transactions: vec![],
        available_capture_amount: Some(money((v.total - v.captured).max(rust_decimal::Decimal::ZERO), &v.currency)),
        available_refund_amount: Some(money(v.refunded, &v.currency)),
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlTransactionUpdate {
    pub transaction: Option<gen::TransactionItem>,
    pub errors: Vec<GqlTransactionUpdateError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlTransactionEventReport {
    #[graphql(name = "alreadyProcessed")]
    pub already_processed: Option<bool>,
    pub transaction: Option<gen::TransactionItem>,
    #[graphql(name = "transactionEvent")]
    pub transaction_event: Option<gen::TransactionEvent>,
    pub errors: Vec<GqlTransactionEventReportError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlPaymentCapture {
    pub payment: Option<gen::Payment>,
    pub errors: Vec<GqlPaymentError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlPaymentRefund {
    pub payment: Option<gen::Payment>,
    pub errors: Vec<GqlPaymentError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlPaymentVoid {
    pub payment: Option<gen::Payment>,
    pub errors: Vec<GqlPaymentError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlPaymentCheckBalance {
    pub data: Option<serde_json::Value>,
    pub errors: Vec<GqlPaymentError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlTransactionInitialize {
    pub transaction: Option<gen::TransactionItem>,
    #[graphql(name = "transactionEvent")]
    pub transaction_event: Option<gen::TransactionEvent>,
    pub data: Option<serde_json::Value>,
    pub errors: Vec<GqlTransactionUpdateError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlTransactionProcess {
    pub transaction: Option<gen::TransactionItem>,
    #[graphql(name = "transactionEvent")]
    pub transaction_event: Option<gen::TransactionEvent>,
    pub data: Option<serde_json::Value>,
    pub errors: Vec<GqlTransactionUpdateError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlPaymentInitialized {
    pub gateway: String,
    pub name: String,
    pub data: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlPaymentInitialize {
    #[graphql(name = "initializedPayment")]
    pub initialized_payment: Option<GqlPaymentInitialized>,
    pub errors: Vec<GqlPaymentError>,
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

fn payment_node(m: &saleor_rustify_db::entities::payment_payment::Model) -> gen::Payment {
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
        match saleor_rustify_db::payments::view(db, tid).await {
            Ok(v) => Ok(Some(GqlTransaction { id: ID(crate::common::gid("TransactionItem", v.id)), currency: v.currency, authorized: v.authorized.to_string(), charged: v.charged.to_string(), refunded: v.refunded.to_string(), canceled: v.canceled.to_string() })),
            Err(_) => Ok(None),
        }
    }

    /// Legacy payment by id (Django `payment`: MANAGE_ORDERS).
    async fn payment(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::Payment>> {
        crate::account::require_perm(ctx, "manage_orders").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let Some(pid) = saleor_rustify_db::catalog::parse_gid(&id.0) else { return Ok(None) };
        use sea_orm::EntityTrait;
        Ok(saleor_rustify_db::entities::payment_payment::Entity::find_by_id(pid)
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
        use saleor_rustify_db::entities::payment_payment::{Column as PCol, Entity as PEnt};
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
        use saleor_rustify_db::entities::payment_transactionitem::{Column as TCol, Entity as TEnt};
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
        let v = saleor_rustify_db::payments::authorize(db, tid, amt, &idempotency_key).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlTransaction { id: ID(crate::common::gid("TransactionItem", v.id)), currency: v.currency, authorized: v.authorized.to_string(), charged: v.charged.to_string(), refunded: v.refunded.to_string(), canceled: v.canceled.to_string() })
    }
    async fn transaction_charge(&self, ctx: &Context<'_>, transaction_id: ID, amount: String, idempotency_key: String) -> Result<GqlTransaction> {
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let tid: i32 = transaction_id.0.parse().map_err(|_| Error::new("transactionId must be int"))?;
        let amt: rust_decimal::Decimal = amount.parse().map_err(|_| Error::new("amount must be decimal"))?;
        let v = saleor_rustify_db::payments::charge(db, tid, amt, &idempotency_key).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(GqlTransaction { id: ID(crate::common::gid("TransactionItem", v.id)), currency: v.currency, authorized: v.authorized.to_string(), charged: v.charged.to_string(), refunded: v.refunded.to_string(), canceled: v.canceled.to_string() })
    }

    // ------------------------------------------------------------------
    // Saleor payment flows (real money movement through the event-sourced
    // ledger; PSP calls go out only via request_action with a configured
    // gateway — otherwise the manual ledger applies).
    // ------------------------------------------------------------------

    async fn transaction_update(
        &self, ctx: &Context<'_>,
        id: Option<ID>, token: Option<String>,
        transaction: Option<gen::TransactionUpdateInput>,
        #[graphql(name = "transactionEvent")] transaction_event: Option<gen::TransactionEventInput>,
    ) -> Result<GqlTransactionUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let tid = match resolve_tid(db, id, token).await {
            Ok(t) => t,
            Err(e) => return Ok(GqlTransactionUpdate { transaction: None, errors: vec![tuerr(e.to_string())] }),
        };
        let req_uid = match txn_access(ctx, db, tid).await {
            Ok(u) => u,
            Err(e) => return Ok(GqlTransactionUpdate { transaction: None, errors: vec![tuerr(e.to_string())] }),
        };
        let _ = req_uid;
        if let Some(t) = transaction.as_ref() {
            // Currency guard first (Django INCORRECT_CURRENCY).
            let cur: String = saleor_rustify_db::entities::payment_transactionitem::Entity::find_by_id(tid)
                .one(db).await.map_err(|e| Error::new(e.to_string()))?
                .ok_or_else(|| Error::new("transaction not found"))?.currency;
            let money = |m: &Option<gen::MoneyInput>| -> Result<Option<rust_decimal::Decimal>, Error> {
                match m.as_ref() {
                    None => Ok(None),
                    Some(x) => {
                        if x.currency != cur {
                            return Err(Error::new("incorrect currency"));
                        }
                        x.amount.0.parse::<rust_decimal::Decimal>().map(Some).map_err(|_| Error::new("bad amount"))
                    }
                }
            };
            let targets = saleor_rustify_db::payments::AmountTargets {
                authorized: money(&t.amount_authorized)?,
                charged: money(&t.amount_charged)?,
                refunded: money(&t.amount_refunded)?,
                canceled: money(&t.amount_canceled)?,
            };
            let patch = saleor_rustify_db::payments::ItemPatch {
                name: t.name.clone(),
                message: t.message.clone(),
                psp_reference: t.psp_reference.clone(),
                available_actions: t.available_actions.as_ref().map(|v| v.iter().map(|a| format!("{a:?}")).collect()),
                metadata: t.metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
                private_metadata: t.private_metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
            };
            if let Err(e) = saleor_rustify_db::payments::update_transaction_scalars(db, tid, &patch).await {
                return Ok(GqlTransactionUpdate { transaction: None, errors: vec![tuerr(e.to_string())] });
            }
            if let Err(e) = saleor_rustify_db::payments::apply_amount_targets(db, tid, &targets, &cur, &format!("update-{tid}")).await {
                return Ok(GqlTransactionUpdate { transaction: None, errors: vec![tuerr(e.to_string())] });
            }
            // NOTE: external_url / payment_method_details need card-brand
            // plumbing (Django updates cc_* columns); accepted-ignored here.
        }
        if let Some(ev) = transaction_event.as_ref() {
            // Companion INFO audit event (Django appends it post-update).
            use saleor_rustify_db::payments::{report_event, NewEvent};
            let cur: String = saleor_rustify_db::entities::payment_transactionitem::Entity::find_by_id(tid)
                .one(db).await.map_err(|e| Error::new(e.to_string()))?
                .ok_or_else(|| Error::new("transaction not found"))?.currency;
            if let Err(e) = report_event(db, tid, &NewEvent {
                event_type: "info".into(),
                amount: rust_decimal::Decimal::ZERO,
                currency: cur,
                psp_reference: ev.psp_reference.clone(),
                message: ev.message.clone().unwrap_or_default(),
                idempotency_key: Some(format!("update-info-{tid}")),
                include_in_calculations: false,
                related_granted_refund_id: None,
                external_url: None,
            }).await {
                return Ok(GqlTransactionUpdate { transaction: None, errors: vec![tuerr(e.to_string())] });
            }
        }
        Ok(GqlTransactionUpdate {
            transaction: Some(assemble_item(db, tid).await.map_err(Error::new)?),
            errors: vec![],
        })
    }

    async fn transaction_event_report(
        &self, ctx: &Context<'_>,
        amount: Option<gen::GenPositiveDecimal>,
        #[graphql(name = "availableActions")] available_actions: Option<Vec<gen::TransactionActionEnum>>,
        #[graphql(name = "externalUrl")] external_url: Option<String>,
        id: Option<ID>, message: Option<String>,
        #[graphql(name = "paymentMethodDetails")] payment_method_details: Option<gen::PaymentMethodDetailsInput>,
        #[graphql(name = "pspReference")] psp_reference: String,
        time: Option<chrono::DateTime<chrono::Utc>>,
        token: Option<String>,
        #[graphql(name = "transactionMetadata")] transaction_metadata: Option<Vec<crate::common::MetadataInput>>,
        #[graphql(name = "transactionPrivateMetadata")] transaction_private_metadata: Option<Vec<crate::common::MetadataInput>>,
        #[graphql(name = "type")] event_type: gen::TransactionEventTypeEnum,
    ) -> Result<GqlTransactionEventReport> {
        let _ = (payment_method_details, time);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let tid = match resolve_tid(db, id, token).await {
            Ok(t) => t,
            Err(e) => return Ok(GqlTransactionEventReport { already_processed: None, transaction: None, transaction_event: None, errors: vec![tererr(e.to_string())] }),
        };
        if let Err(e) = txn_access(ctx, db, tid).await {
            return Ok(GqlTransactionEventReport { already_processed: None, transaction: None, transaction_event: None, errors: vec![tererr(e.to_string())] });
        }
        let ty = format!("{event_type:?}").to_lowercase();
        // PSP-level idempotency first (Django alreadyProcessed).
        if saleor_rustify_db::payments::has_event(db, tid, &psp_reference, &ty).await.unwrap_or(false) {
            let item = assemble_item(db, tid).await.map_err(Error::new)?;
            return Ok(GqlTransactionEventReport { already_processed: Some(true), transaction: Some(item), transaction_event: None, errors: vec![] });
        }
        let amt = amount.as_ref().and_then(|a| a.0.parse::<rust_decimal::Decimal>().ok()).unwrap_or(rust_decimal::Decimal::ZERO);
        let cur: String = saleor_rustify_db::entities::payment_transactionitem::Entity::find_by_id(tid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("transaction not found"))?.currency;
        // Record-only unless a money family success/adjustment/back/reverse.
        let calc = ty.ends_with("success") || ty.ends_with("adjustment") || ty == "charge_back" || ty == "refund_reverse";
        use saleor_rustify_db::payments::{report_event, NewEvent};
        if let Err(e) = report_event(db, tid, &NewEvent {
            event_type: ty.clone(),
            amount: amt,
            currency: cur.clone(),
            psp_reference: Some(psp_reference.clone()),
            message: message.clone().unwrap_or_default(),
            idempotency_key: Some(format!("report-{psp_reference}-{ty}")),
            include_in_calculations: calc,
            related_granted_refund_id: None,
            external_url,
        }).await {
            return Ok(GqlTransactionEventReport { already_processed: Some(false), transaction: None, transaction_event: None, errors: vec![tererr(e.to_string())] });
        }
        if let Some(a) = available_actions.as_ref() {
            let acts: Vec<String> = a.iter().map(|x| format!("{x:?}")).collect();
            let _ = saleor_rustify_db::payments::update_transaction_scalars(db, tid, &saleor_rustify_db::payments::ItemPatch {
                available_actions: Some(acts),
                metadata: transaction_metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
                private_metadata: transaction_private_metadata.as_ref().map(|v| crate::common::merge_metadata(&serde_json::Value::Null, v)),
                ..Default::default()
            }).await;
        }
        let item = assemble_item(db, tid).await.map_err(Error::new)?;
        let ev = item.events.iter().find(|e| e.psp_reference.as_deref() == Some(psp_reference.as_str())).cloned();
        Ok(GqlTransactionEventReport { already_processed: Some(false), transaction: Some(item), transaction_event: ev, errors: vec![] })
    }

    async fn transaction_request_action(
        &self, ctx: &Context<'_>,
        #[graphql(name = "actionType")] action_type: gen::TransactionActionEnum,
        amount: Option<gen::GenPositiveDecimal>,
        id: Option<ID>,
        #[graphql(name = "refundReason")] refund_reason: Option<String>,
        #[graphql(name = "refundReasonReference")] refund_reason_reference: Option<ID>,
        token: Option<String>,
    ) -> Result<gen::TransactionRequestAction> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let terr = |m: String| gen::TransactionRequestAction { transaction: None, errors: vec![gen::TransactionRequestActionError { field: None, message: Some(m), code: None }] };
        let tid = match resolve_tid(db, id, token).await {
            Ok(t) => t,
            Err(e) => return Ok(terr(e)),
        };
        if let Err(e) = txn_access(ctx, db, tid).await {
            return Ok(terr(e));
        }
        let amt = amount.as_ref().and_then(|a| a.0.parse::<rust_decimal::Decimal>().ok()).unwrap_or(rust_decimal::Decimal::ZERO);
        let (action, need_amount) = match format!("{action_type:?}").as_str() {
            "REFUND" => (saleor_rustify_core::psp::PspAction::Refund, true),
            "CANCEL" => (saleor_rustify_core::psp::PspAction::Cancel, false),
            _ => (saleor_rustify_core::psp::PspAction::Charge, true),
        };
        if need_amount && amt <= rust_decimal::Decimal::ZERO {
            return Ok(gen::TransactionRequestAction { transaction: None, errors: vec![gen::TransactionRequestActionError { field: Some("amount".into()), message: Some("amount must be > 0".into()), code: None }] });
        }
        let app: Option<String> = saleor_rustify_db::entities::payment_transactionitem::Entity::find_by_id(tid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("transaction not found"))?.app_identifier;
        let key = format!("req-{}-{}-{}", tid, format!("{action_type:?}").to_lowercase(), amt);
        // Granted-refund linkage (Django `refund_request_refund_for_granted_refund`):
        // manual PSP links the grant id into the refund event.
        let grant_id = refund_reason_reference.as_ref().and_then(|i| saleor_rustify_db::catalog::parse_gid(&i.0));
        let is_manual = !app.as_deref().is_some_and(|a| a.to_lowercase().contains("stripe"));
        let out = if matches!(action, saleor_rustify_core::psp::PspAction::Refund) {
            if let (Some(gid), true) = (grant_id, is_manual) {
                saleor_rustify_db::payments::refund_for_grant(db, tid, amt, &key, gid).await
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            } else {
                execute_with_psp(db, tid, action, amt, &key, app.as_deref(), refund_reason.clone()).await
            }
        } else {
            execute_with_psp(db, tid, action, amt, &key, app.as_deref(), refund_reason.clone()).await
        };
        match out {
            Ok(_) => Ok(gen::TransactionRequestAction {
                transaction: Some(assemble_item(db, tid).await.map_err(Error::new)?),
                errors: vec![],
            }),
            Err(e) => Ok(terr(e)),
        }
    }

    async fn payment_capture(&self, ctx: &Context<'_>, amount: Option<gen::GenPositiveDecimal>, #[graphql(name = "paymentId")] payment_id: ID) -> Result<GqlPaymentCapture> {
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlPaymentCapture { payment: None, errors: vec![perr(m)] };
        let Some(pid) = saleor_rustify_db::catalog::parse_gid(&payment_id.0) else {
            return Ok(err("bad payment id".into()));
        };
        let amt = amount.as_ref().and_then(|a| a.0.parse::<rust_decimal::Decimal>().ok()).unwrap_or(rust_decimal::Decimal::ZERO);
        // No amount = capture the full remainder (dashboard sends explicit
        // amounts; the default keeps one-click capture working).
        let amt = if amt <= rust_decimal::Decimal::ZERO {
            match saleor_rustify_db::entities::payment_payment::Entity::find_by_id(pid).one(db).await.map_err(|e| Error::new(e.to_string()))? {
                Some(p) => (p.total - p.captured_amount).max(rust_decimal::Decimal::ZERO),
                None => return Ok(err("payment not found".into())),
            }
        } else { amt };
        match saleor_rustify_db::payments::capture_legacy_payment(db, pid, amt).await {
            Ok(v) => Ok(GqlPaymentCapture { payment: Some(legacy_payment_node(&v)), errors: vec![] }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn payment_refund(&self, ctx: &Context<'_>, amount: Option<gen::GenPositiveDecimal>, #[graphql(name = "paymentId")] payment_id: ID) -> Result<GqlPaymentRefund> {
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlPaymentRefund { payment: None, errors: vec![perr(m)] };
        let Some(pid) = saleor_rustify_db::catalog::parse_gid(&payment_id.0) else {
            return Ok(err("bad payment id".into()));
        };
        let Some(amt) = amount.as_ref().and_then(|a| a.0.parse::<rust_decimal::Decimal>().ok()).filter(|a| *a > rust_decimal::Decimal::ZERO) else {
            return Ok(err("amount must be > 0".into()));
        };
        match saleor_rustify_db::payments::refund_legacy_payment(db, pid, amt).await {
            Ok(v) => Ok(GqlPaymentRefund { payment: Some(legacy_payment_node(&v)), errors: vec![] }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn payment_void(&self, ctx: &Context<'_>, #[graphql(name = "paymentId")] payment_id: ID) -> Result<GqlPaymentVoid> {
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlPaymentVoid { payment: None, errors: vec![perr(m)] };
        let Some(pid) = saleor_rustify_db::catalog::parse_gid(&payment_id.0) else {
            return Ok(err("bad payment id".into()));
        };
        match saleor_rustify_db::payments::void_legacy_payment(db, pid).await {
            Ok(v) => Ok(GqlPaymentVoid { payment: Some(legacy_payment_node(&v)), errors: vec![] }),
            Err(e) => Ok(err(e.to_string())),
        }
    }

    async fn payment_check_balance(&self, ctx: &Context<'_>, input: gen::PaymentCheckBalanceInput) -> Result<GqlPaymentCheckBalance> {
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        // No gateway in this backend implements balance checks (Django
        // delegates to the plugin and errors without one) — honest error,
        // never a fabricated balance.
        let _ = input;
        Ok(GqlPaymentCheckBalance { data: None, errors: vec![perr("balance check is not supported by any configured gateway".into())] })
    }

    /// Start a transaction session (Django `transactionInitialize`): attach
    /// to a checkout or order, default the amount to total-minus-processed,
    /// emit the *_REQUEST event, and hand back gateway data (Stripe intent
    /// client secret when live, {} for manual).
    async fn transaction_initialize(
        &self, ctx: &Context<'_>,
        action: Option<gen::TransactionFlowStrategyEnum>,
        amount: Option<gen::GenPositiveDecimal>,
        #[graphql(name = "customerIpAddress")] customer_ip_address: Option<String>,
        id: ID,
        #[graphql(name = "idempotencyKey")] idempotency_key: Option<String>,
        #[graphql(name = "paymentGateway")] payment_gateway: Option<gen::PaymentGatewayToInitialize>,
    ) -> Result<GqlTransactionInitialize> {
        let _ = customer_ip_address;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlTransactionInitialize { transaction: None, transaction_event: None, data: None, errors: vec![tuerr(m)] };
        let Some(uuid) = crate::common::parse_uuid_gid(&id.0) else {
            return Ok(err("bad checkout/order id".into()));
        };
        // Checkout or order?
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        let co = saleor_rustify_db::entities::checkout_checkout::Entity::find_by_id(uuid).one(db).await.map_err(|e| Error::new(e.to_string()))?;
        let od = saleor_rustify_db::entities::order_order::Entity::find_by_id(uuid).one(db).await.map_err(|e| Error::new(e.to_string()))?;
        let (checkout_id, order_id, currency, total, channel_id) = match (co, od) {
            (Some(c), _) => (Some(c.token), None, c.currency.clone(), c.total_gross_amount, c.channel_id),
            (None, Some(o)) => (None, Some(o.id), o.currency.clone(), o.total_gross_amount, o.channel_id),
            (None, None) => return Ok(err("no checkout or order with this id".into())),
        };
        // Default amount = total minus already-charged across linked items.
        let charged_sum: rust_decimal::Decimal = saleor_rustify_db::entities::payment_transactionitem::Entity::find()
            .select_only().column(saleor_rustify_db::entities::payment_transactionitem::Column::ChargedValue)
            .filter(if checkout_id.is_some() {
                saleor_rustify_db::entities::payment_transactionitem::Column::CheckoutId.eq(uuid)
            } else {
                saleor_rustify_db::entities::payment_transactionitem::Column::OrderId.eq(uuid)
            })
            .into_tuple::<rust_decimal::Decimal>().all(db).await.map_err(|e| Error::new(e.to_string()))?
            .into_iter().sum();
        let amt = amount.as_ref()
            .and_then(|a| a.0.parse::<rust_decimal::Decimal>().ok())
            .unwrap_or((total - charged_sum).max(rust_decimal::Decimal::ZERO));
        if amt <= rust_decimal::Decimal::ZERO {
            return Ok(err("nothing left to initialize: total already processed".into()));
        }
        // Flow: explicit arg wins, else the channel default.
        let charge_flow = match action.as_ref().map(|a| format!("{a:?}")).as_deref() {
            Some("CHARGE") => true,
            Some("AUTHORIZATION") => false,
            _ => {
                let strat: Option<String> = saleor_rustify_db::entities::channel_channel::Entity::find_by_id(channel_id)
                    .select_only().column(saleor_rustify_db::entities::channel_channel::Column::DefaultTransactionFlowStrategy)
                    .into_tuple::<String>().one(db).await.map_err(|e| Error::new(e.to_string()))?;
                strat.is_some_and(|s| s == "charge")
            }
        };
        let gw_id = payment_gateway.as_ref().map(|p| p.id.clone()).unwrap_or_else(|| "manual".into());
        let item = match saleor_rustify_db::payments::initialize_transaction(db, &saleor_rustify_db::payments::InitSpec {
            checkout_id, order_id, currency: currency.clone(), name: "initialize".into(),
            app_identifier: Some(gw_id.clone()), idempotency_key: idempotency_key.clone(),
            available_actions: vec!["charge".into(), "refund".into(), "cancel".into()],
            charge_flow,
        }, amt).await {
            Ok(v) => v,
            Err(e) => return Ok(err(e.to_string())),
        };
        // Gateway data: live Stripe intent, else manual empty config.
        let mut data = serde_json::json!({});
        if gw_id.to_lowercase().contains("stripe") {
            match saleor_rustify_psp::StripePsp::from_env() {
                Some(psp) => match psp.create_manual_intent(amt, &currency, &idempotency_key.clone().unwrap_or_else(|| format!("init-{}", item.id))).await {
                    Ok((pi, secret)) => { data = serde_json::json!({"id": pi, "clientSecret": secret}); }
                    Err(e) => return Ok(err(format!("stripe: {e}"))),
                },
                None => return Ok(err("stripe selected but STRIPE_SECRET_KEY is unset".into())),
            }
        }
        let full = assemble_item(db, item.id).await.map_err(Error::new)?;
        let ev = full.events.last().cloned();
        Ok(GqlTransactionInitialize { transaction: Some(full), transaction_event: ev, data: Some(data), errors: vec![] })
    }

    /// Run the transaction's next step through its PSP (Django
    /// `transactionProcess`): authorize the outstanding (or charge the
    /// remainder), with the frontend-supplied gateway data.
    async fn transaction_process(
        &self, ctx: &Context<'_>,
        #[graphql(name = "customerIpAddress")] customer_ip_address: Option<String>,
        data: Option<serde_json::Value>,
        id: Option<ID>, token: Option<String>,
    ) -> Result<GqlTransactionProcess> {
        let _ = customer_ip_address;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let err = |m: String| GqlTransactionProcess { transaction: None, transaction_event: None, data: None, errors: vec![tuerr(m)] };
        let tid = match resolve_tid(db, id, token).await {
            Ok(t) => t,
            Err(e) => return Ok(err(e)),
        };
        if let Err(e) = txn_access(ctx, db, tid).await {
            return Ok(err(e));
        }
        let (amount, action, currency) = match saleor_rustify_db::payments::outstanding(db, tid).await {
            Ok(v) => v,
            Err(e) => return Ok(err(e.to_string())),
        };
        let app: Option<String> = saleor_rustify_db::entities::payment_transactionitem::Entity::find_by_id(tid)
            .one(db).await.map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("transaction not found"))?.app_identifier;
        let psp = match psp_for(app.as_deref()) {
            Ok(p) => p,
            Err(e) => return Ok(err(e)),
        };
        let data_str = data.map(|v| v.to_string());
        let key = format!("process-{tid}-{amount}");
        let outcome = saleor_rustify_db::payments::execute_via(
            db, tid, action, amount, &key, psp.as_ref(), None, data_str.as_deref(),
        )
        .await;
        let _ = currency;
        match outcome {
            Ok(out) => {
                let full = assemble_item(db, tid).await.map_err(Error::new)?;
                let ev = full.events.last().cloned();
                let data = serde_json::json!({
                    "actionRequired": out.action_required,
                    "redirectUrl": out.redirect_url,
                    "charged": out.txn.charged.to_string(),
                    "authorized": out.txn.authorized.to_string(),
                });
                Ok(GqlTransactionProcess { transaction: Some(full), transaction_event: ev, data: Some(data), errors: vec![] })
            }
            Err(e) => Ok(err(e.to_string())),
        }
    }

    /// Legacy gateway init (Django `paymentInitialize`): manual returns an
    /// empty config (nothing to configure); anything else needs a live key.
    async fn payment_initialize(
        &self, ctx: &Context<'_>,
        channel: Option<String>, gateway: String, #[graphql(name = "paymentData")] payment_data: Option<String>,
    ) -> Result<GqlPaymentInitialize> {
        let _ = (channel, payment_data);
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        let err = |m: String| GqlPaymentInitialize { initialized_payment: None, errors: vec![perr(m)] };
        match gateway.as_str() {
            "manual" => Ok(GqlPaymentInitialize {
                initialized_payment: Some(GqlPaymentInitialized { gateway: "manual".into(), name: "Manual".into(), data: Some("{}".into()) }),
                errors: vec![],
            }),
            "stripe" => match std::env::var("STRIPE_SECRET_KEY").ok().filter(|k| !k.trim().is_empty()) {
                Some(_) => Ok(GqlPaymentInitialize {
                    initialized_payment: Some(GqlPaymentInitialized { gateway: "stripe".into(), name: "Stripe".into(), data: Some("{}".into()) }),
                    errors: vec![],
                }),
                None => Ok(err("stripe selected but STRIPE_SECRET_KEY is unset".into())),
            },
            other => Ok(err(format!("gateway {other} is not configured"))),
        }
    }

    /// App transaction creation (Django `transactionCreate`): books the item
    /// on a checkout or order, then records any nonzero opening buckets as
    /// `*_success` events (same recalc path as live PSP reports). An
    /// optional `transactionEvent` lands as a math-excluded INFO record.
    async fn transaction_create(
        &self, ctx: &Context<'_>,
        id: ID,
        transaction: gen::TransactionCreateInput,
        #[graphql(name = "transactionEvent")] transaction_event: Option<gen::TransactionEventInput>,
    ) -> Result<gen::TransactionCreate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        let err = |m: String| gen::TransactionCreate {
            transaction: None,
            errors: vec![gen::TransactionCreateError { field: None, message: Some(m), code: None }],
        };
        // Parent: checkout xor order (Django accepts both id flavors).
        let raw = crate::common::parse_uuid_gid(&id.0);
        let (checkout_id, order_id) = match raw {
            Some(u) => {
                use sea_orm::EntityTrait;
                if saleor_rustify_db::entities::checkout_checkout::Entity::find_by_id(u).one(db).await.map_err(|e| Error::new(e.to_string()))?.is_some() {
                    (Some(u), None)
                } else if saleor_rustify_db::entities::order_order::Entity::find_by_id(u).one(db).await.map_err(|e| Error::new(e.to_string()))?.is_some() {
                    (None, Some(u))
                } else {
                    return Ok(err("checkout or order not found".into()));
                }
            }
            None => return Ok(err("bad id".into())),
        };
        let money = |m: &Option<gen::MoneyInput>| -> Option<(rust_decimal::Decimal, String)> {
            m.as_ref().and_then(|x| x.amount.0.parse::<rust_decimal::Decimal>().ok().map(|a| (a, x.currency.clone())))
        };
        let auth = money(&transaction.amount_authorized);
        let charged = money(&transaction.amount_charged);
        let refunded = money(&transaction.amount_refunded);
        let canceled = money(&transaction.amount_canceled);
        for (a, _) in [auth.clone(), charged.clone(), refunded.clone(), canceled.clone()].into_iter().flatten() {
            if a < rust_decimal::Decimal::ZERO {
                return Ok(err("opening amounts cannot be negative".into()));
            }
        }
        let currency = auth.clone().or(charged.clone()).or(refunded.clone()).or(canceled.clone()).map(|(_, c)| c).unwrap_or_else(|| "USD".to_string());
        let actions: Vec<String> = transaction.available_actions.clone().unwrap_or_default().into_iter().map(|a| match a {
            gen::TransactionActionEnum::CHARGE => "charge".to_string(),
            gen::TransactionActionEnum::REFUND => "refund".to_string(),
            gen::TransactionActionEnum::CANCEL => "cancel".to_string(),
        }).collect();
        let created = match saleor_rustify_db::payments::create_transaction(db, &saleor_rustify_db::payments::NewTransaction {
            checkout_id,
            order_id,
            currency: currency.clone(),
            name: transaction.name.clone().unwrap_or_default(),
            app_identifier: None,
            idempotency_key: None,
            available_actions: actions,
        }).await {
            Ok(v) => v,
            Err(e) => return Ok(err(e.to_string())),
        };
        // Extras the ledger ctor doesn't take (columns exist on the item).
        {
            use sea_orm::{ActiveModelTrait, EntityTrait, Set};
            if let Some(row) = saleor_rustify_db::entities::payment_transactionitem::Entity::find_by_id(created.id).one(db).await.map_err(|e| Error::new(e.to_string()))? {
                let mut am: saleor_rustify_db::entities::payment_transactionitem::ActiveModel = row.into();
                if transaction.psp_reference.is_some() { am.psp_reference = Set(transaction.psp_reference.clone()); }
                if transaction.message.is_some() { am.message = Set(transaction.message.clone()); }
                if transaction.external_url.is_some() { am.external_url = Set(transaction.external_url.clone()); }
                am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            }
        }
        let report = |typ: &str, amt: rust_decimal::Decimal, cur: String| {
            saleor_rustify_db::payments::NewEvent {
                event_type: typ.to_string(),
                amount: amt,
                currency: cur,
                psp_reference: transaction.psp_reference.clone(),
                message: transaction.message.clone().unwrap_or_default(),
                idempotency_key: None,
                include_in_calculations: true,
                related_granted_refund_id: None,
                external_url: transaction.external_url.clone(),
            }
        };
        // report_event is async; run the steps inline instead of a closure.
        let mut failed: Option<String> = None;
        if let Some((a, c)) = auth { if a > rust_decimal::Decimal::ZERO && failed.is_none() {
            if let Err(e) = saleor_rustify_db::payments::report_event(db, created.id, &report("authorization_success", a, c)).await { failed = Some(e.to_string()); } } }
        if let Some((a, c)) = charged { if a > rust_decimal::Decimal::ZERO && failed.is_none() {
            if let Err(e) = saleor_rustify_db::payments::report_event(db, created.id, &report("charge_success", a, c)).await { failed = Some(e.to_string()); } } }
        if let Some((a, c)) = refunded { if a > rust_decimal::Decimal::ZERO && failed.is_none() {
            if let Err(e) = saleor_rustify_db::payments::report_event(db, created.id, &report("refund_success", a, c)).await { failed = Some(e.to_string()); } } }
        if let Some((a, c)) = canceled { if a > rust_decimal::Decimal::ZERO && failed.is_none() {
            if let Err(e) = saleor_rustify_db::payments::report_event(db, created.id, &report("cancel_success", a, c)).await { failed = Some(e.to_string()); } } }
        if let Some(ev) = transaction_event.as_ref() {
            if failed.is_none() {
                let info = saleor_rustify_db::payments::NewEvent {
                    event_type: "info".to_string(),
                    amount: rust_decimal::Decimal::ZERO,
                    currency: currency.clone(),
                    psp_reference: ev.psp_reference.clone(),
                    message: ev.message.clone().unwrap_or_default(),
                    idempotency_key: None,
                    include_in_calculations: false,
                    related_granted_refund_id: None,
                    external_url: None,
                };
                if let Err(e) = saleor_rustify_db::payments::report_event(db, created.id, &info).await { failed = Some(e.to_string()); }
            }
        }
        if let Some(m) = failed {
            return Ok(err(m));
        }
        match assemble_item(db, created.id).await {
            Ok(t) => Ok(gen::TransactionCreate { transaction: Some(t), errors: vec![] }),
            Err(e) => Ok(err(e)),
        }
    }

    /// Stored payment-method delete request (Django
    /// `storedPaymentMethodRequestDelete`): fans out the sync webhook to
    /// payment apps. No stored-method vault exists here, so with no
    /// subscriber the honest result is FAILED_TO_DELIVER.
    async fn stored_payment_method_request_delete(
        &self, ctx: &Context<'_>,
        channel: String,
        id: ID,
    ) -> Result<GqlStoredPaymentMethodRequestDelete> {
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let payload = serde_json::json!({
            "payment_method_id": id.0,
            "channel": channel,
        })
        .to_string();
        let deliveries = match saleor_rustify_db::webhooks::trigger_event(
            db,
            "stored_payment_method_delete_requested",
            Some(&channel),
            &payload,
        )
        .await
        {
            Ok(d) => d,
            Err(e) => {
                return Ok(GqlStoredPaymentMethodRequestDelete {
                    result: gen::StoredPaymentMethodRequestDeleteResult::FAILEDTODELIVER,
                    errors: vec![GqlPaymentMethodRequestDeleteError {
                        field: None,
                        message: Some(e.to_string()),
                        code: None,
                    }],
                })
            }
        };
        if deliveries.is_empty() {
            return Ok(GqlStoredPaymentMethodRequestDelete {
                result: gen::StoredPaymentMethodRequestDeleteResult::FAILEDTODELIVER,
                errors: vec![],
            });
        }
        Ok(GqlStoredPaymentMethodRequestDelete {
            result: gen::StoredPaymentMethodRequestDeleteResult::SUCCESSFULLYDELETED,
            errors: vec![],
        })
    }

    /// Gateway initialization (Django `paymentGatewayInitialize`): without a
    /// gateway plugin every gateway reports NOT_FOUND, like Django.
    async fn payment_gateway_initialize(
        &self,
        amount: Option<gen::GenPositiveDecimal>,
        id: ID,
        #[graphql(name = "paymentGateways")] payment_gateways: Option<Vec<gen::PaymentGatewayToInitialize>>,
    ) -> Result<GqlPaymentGatewayInitialize> {
        let _ = (amount, id);
        let mut configs = vec![];
        for gw in payment_gateways.unwrap_or_default() {
            configs.push(GqlPaymentGatewayConfig {
                id: gw.id.clone(),
                data: None,
                errors: vec![GqlPaymentGatewayConfigError {
                    field: None,
                    message: Some(format!("gateway {} is not configured", gw.id)),
                    code: GqlPaymentGatewayConfigErrorCode::NOTFOUND,
                }],
            });
        }
        Ok(GqlPaymentGatewayInitialize { gateway_configs: configs, errors: vec![] })
    }

    /// Gateway session tokenization init (Django
    /// `paymentGatewayInitializeTokenization`): no session apps installed.
    async fn payment_gateway_initialize_tokenization(
        &self, amount: Option<gen::GenPositiveDecimal>, id: ID,
        #[graphql(name = "paymentGateways")] payment_gateways: Option<Vec<gen::PaymentGatewayToInitialize>>,
    ) -> Result<GqlPaymentGatewayInitializeTokenization> {
        let _ = (amount, id, payment_gateways);
        Ok(GqlPaymentGatewayInitializeTokenization {
            result: GqlPaymentGatewayInitializeTokenizationResult::FAILEDTODELIVER,
            data: None,
            errors: vec![GqlPaymentGatewayInitializeTokenizationError {
                field: None,
                message: Some("no payment gateway with session support is configured".to_string()),
                code: None,
            }],
        })
    }

    /// Payment-method tokenization init (Django
    /// `paymentMethodInitializeTokenization`): no vault apps installed.
    async fn payment_method_initialize_tokenization(
        &self, channel: String, data: gen::GenJSONString, id: String,
        #[graphql(name = "paymentFlowToSupport")] payment_flow_to_support: gen::TokenizedPaymentFlowEnum,
    ) -> Result<GqlPaymentMethodInitializeTokenization> {
        let _ = (channel, data, id, payment_flow_to_support);
        Ok(GqlPaymentMethodInitializeTokenization {
            result: GqlPaymentMethodTokenizationResult::FAILEDTODELIVER,
            id: None,
            data: None,
            errors: vec![GqlPaymentMethodInitializeTokenizationError {
                field: None,
                message: Some("no payment app supports stored payment methods".to_string()),
                code: None,
            }],
        })
    }

    /// Payment-method tokenization process (Django
    /// `paymentMethodProcessTokenization`): no vault apps installed.
    async fn payment_method_process_tokenization(
        &self, channel: String, data: gen::GenJSONString, id: String,
    ) -> Result<GqlPaymentMethodProcessTokenization> {
        let _ = (channel, data, id);
        Ok(GqlPaymentMethodProcessTokenization {
            result: GqlPaymentMethodTokenizationResult::FAILEDTODELIVER,
            id: None,
            data: None,
            errors: vec![GqlPaymentMethodProcessTokenizationError {
                field: None,
                message: Some("no payment app supports stored payment methods".to_string()),
                code: None,
            }],
        })
    }

    /// Execute a granted refund's money move (Django
    /// `transactionRequestRefundForGrantedRefund`): resolves the transaction
    /// (arg wins, else the grant's link) and refunds the granted amount with
    /// the event linked back to the grant.
    async fn transaction_request_refund_for_granted_refund(
        &self, ctx: &Context<'_>,
        #[graphql(name = "grantedRefundId")] granted_refund_id: ID,
        id: Option<ID>,
        token: Option<String>,
    ) -> Result<gen::TransactionRequestRefundForGrantedRefund> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let _ = crate::account::require_perm(ctx, "handle_payments").await?;
        let err = |m: String| gen::TransactionRequestRefundForGrantedRefund {
            transaction: None,
            errors: vec![gen::TransactionRequestRefundForGrantedRefundError { field: None, message: Some(m), code: None }],
        };
        let gid = saleor_rustify_db::catalog::parse_gid(&granted_refund_id.0).unwrap_or(-1);
        let grant = match saleor_rustify_db::granted_refunds::view(db, gid).await {
            Ok(v) => v,
            Err(e) => return Ok(err(e.to_string())),
        };
        let tid = match resolve_tid(db, id, token).await {
            Ok(t) => t,
            Err(_) => match grant.transaction_item_id {
                Some(t) => t,
                None => return Ok(err("pass transaction id/token or link one on the grant".into())),
            },
        };
        if let Err(e) = txn_access(ctx, db, tid).await {
            return Ok(err(e));
        }
        let key = format!("grant-exec-{gid}-{tid}");
        match saleor_rustify_db::payments::refund_for_grant(db, tid, grant.amount, &key, gid).await {
            Ok(_) => match assemble_item(db, tid).await {
                Ok(t) => Ok(gen::TransactionRequestRefundForGrantedRefund { transaction: Some(t), errors: vec![] }),
                Err(e) => Ok(err(e)),
            },
            Err(e) => Ok(err(e.to_string())),
        }
    }
}
