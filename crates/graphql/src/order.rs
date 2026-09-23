//! Order + fulfillment + granted-refund GraphQL.
//! Wires `order_store` / `cancel` / `fulfillment` / `granted_refunds` —
//! same transactions gRPC uses (lock order, allocation, outbox).

use async_graphql::*;
use base64::Engine as _;
use uuid::Uuid;

use crate::{common::*, context::GqlContext, gen};

#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxedMoney")]
pub struct GqlTaxedMoney {
    pub gross: Money,
    pub net: Money,
    pub tax: Option<Money>,
    pub currency: Option<String>,
}

/// Saleor `Address` — full Dashboard shape (`fragment Address` selects
/// city/country{...}/phone/streets/...). Order resolvers currently leave
/// addresses `None`; Shop `companyAddress` reuses this same type so the
/// GraphQL name `Address` is registered exactly once.
/// Implements `Node` + `ObjectWithMetadata` (gen interfaces) — metadata
/// columns exist on `account_address`, filled where loaded.
#[derive(SimpleObject, Clone)]
#[graphql(name = "Address", complex)]
pub struct GqlAddress {
    pub id: ID,
    pub city: String,
    #[graphql(name = "cityArea")]
    pub city_area: String,
    #[graphql(name = "companyName")]
    pub company_name: String,
    pub country: GqlCountryDisplay,
    #[graphql(name = "countryArea")]
    pub country_area: String,
    #[graphql(name = "firstName")]
    pub first_name: String,
    #[graphql(name = "lastName")]
    pub last_name: String,
    pub phone: Option<String>,
    #[graphql(name = "postalCode")]
    pub postal_code: String,
    #[graphql(name = "streetAddress1")]
    pub street_address_1: String,
    #[graphql(name = "streetAddress2")]
    pub street_address_2: String,
    pub metadata: Vec<crate::common::MetadataItem>,
    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,
}

#[ComplexObject]
impl GqlAddress {
    pub async fn gen_iface_id(&self) -> Option<ID> {
        Some(self.id.clone())
    }
    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {
        self.metadata.clone()
    }
    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {
        self.private_metadata.clone()
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlOrderEdge { pub node: gen::Order, pub cursor: String }

#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderCountableConnection")]
pub struct GqlOrderConnection {
    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,
    pub edges: Vec<GqlOrderEdge>,
    #[graphql(name = "pageInfo")]
    pub page_info: crate::common::PageInfo,
}

#[derive(InputObject)]
pub struct FulfillLineInput { pub order_line_id: ID, pub quantity: i32, pub stock_id: Option<ID> }

#[derive(InputObject)]
pub struct ReturnLineInput { pub order_line_id: ID, pub quantity: i32, pub stock_id: Option<ID> }

fn to_gql_money(amount: rust_decimal::Decimal, currency: String) -> Money {
    Money { amount: amount.to_string(), currency, fraction_digits: None }
}
fn to_taxed(amount: rust_decimal::Decimal, currency: String) -> GqlTaxedMoney {
    to_taxed2(amount, amount, currency)
}

fn parse_id(s: &str) -> Uuid {
    if let Ok(b) = base64::engine::general_purpose::STANDARD.decode(s) {
        if let Ok(t) = String::from_utf8(b) {
            if let Some((_, id)) = t.split_once(':') {
                if let Ok(u) = id.parse::<Uuid>() { return u; }
            }
        }
    }
    s.parse::<Uuid>().unwrap_or(Uuid::nil())
}

/// Shared order assembly: real rows where we have them (id/number/status/
 /// totals/lines/channel/variant+product/totals), `None`/`[]` elsewhere.
/// One order line with its real variant + product (dashboard lines datagrid
/// reads `variant.product.id` unconditionally — null product crashes it).
pub(crate) async fn to_gen_order_line(
    db: &sea_orm::DatabaseConnection,
    l: &saleor_rustify_db::entities::order_orderline::Model,
) -> gen::OrderLine {
    use sea_orm::EntityTrait;
    let mut variant_name = None;
    let mut variant_sku = None;
    let mut product_stub = None;
    if let Some(vid) = l.variant_id {
        if let Ok(Some(v)) = saleor_rustify_db::entities::product_productvariant::Entity::find_by_id(vid).one(db).await {
            variant_name = Some(v.name.clone());
            variant_sku = v.sku.clone();
            // Slim select: product_product.search_vector is tsvector and
            // crashes full-model decode (same class as channel INTERVAL).
            use sea_orm::{ColumnTrait, QueryFilter, QuerySelect};
            type PP = saleor_rustify_db::entities::product_product::Entity;
            use saleor_rustify_db::entities::product_product::Column as PPCol;
            let prow: Option<(i32, String, String, Option<String>, Option<String>)> = PP::find()
                .select_only()
                .column(PPCol::Id).column(PPCol::Name).column(PPCol::Slug)
                .column(PPCol::SeoTitle).column(PPCol::SeoDescription)
                .filter(PPCol::Id.eq(v.product_id))
                .into_tuple().one(db).await.unwrap_or(None);
            if let Some((pid, pname, pslug, pseo_t, pseo_d)) = prow {
                product_stub = Some(Box::new(gen::Product {
                    id: Some(ID(crate::common::gid("Product", pid))),
                    available_for_purchase_at: None,
                    private_metadata: vec![],
                    metadata: vec![],
                    seo_title: pseo_t,
                    seo_description: pseo_d,
                    name: Some(pname.clone()),
                    description: None,
                    product_type: None,
                    slug: Some(pslug),
                    category: None,
                    created: None,
                    updated_at: None,
                    weight: None,
                    default_variant: None,
                    rating: None,
                    channel_listings: vec![],
                    media: vec![],
                    collections: vec![],
                    attributes: vec![],
                    tax_class: None,
                    is_available: None,
                    is_available_for_purchase: None,
                }));
            }
        }
    }
    gen::OrderLine {
        id: Some(ID(crate::common::gid("OrderLine", l.id))),
        private_metadata: vec![],
        metadata: vec![],
        product_name: product_stub.as_ref().and_then(|p| p.name.clone()),
        variant_name: variant_name.clone(),
        product_sku: variant_sku.clone(),
        is_shipping_required: None,
        quantity: Some(l.quantity),
        quantity_fulfilled: Some(l.quantity_fulfilled),
        tax_rate: None,
        unit_price: Some(to_taxed(l.unit_price_gross_amount, l.currency.clone())),
        undiscounted_unit_price: Some(to_taxed2(l.undiscounted_unit_price_net_amount, l.undiscounted_unit_price_gross_amount, l.currency.clone())),
        unit_discount: None,
        unit_discount_reason: None,
        unit_discount_value: None,
        unit_discount_type: None,
        total_price: Some(to_taxed(l.total_price_gross_amount, l.currency.clone())),
        undiscounted_total_price: Some(to_taxed2(l.undiscounted_total_price_net_amount, l.undiscounted_total_price_gross_amount, l.currency.clone())),
        is_price_overridden: None,
        price_override_reason: None,
        variant: l.variant_id.map(|vid| gen::ProductVariant {
            id: Some(ID(crate::common::gid("ProductVariant", vid))),
            private_metadata: vec![],
            metadata: vec![],
            name: variant_name,
            sku: variant_sku,
            product: product_stub,
            track_inventory: None,
            quantity_limit_per_customer: None,
            weight: None,
            channel_listings: vec![],
            media: vec![],
            stocks: vec![],
            quantity_available: None,
            updated_at: None,
        }),
        allocations: vec![],
        quantity_to_fulfill: None,
        tax_class: None,
        voucher_code: None,
        is_gift: Some(l.is_gift),
        discounts: vec![],
    }
}

/// Fulfillment assembly for the order details page + refund/return payloads.
/// Statuses map to Django's uppercase enum; reason references resolve via
/// pages (absent → None, dashboard-tolerated).
pub(crate) async fn to_gen_fulfillment(
    db: &sea_orm::DatabaseConnection,
    f: &saleor_rustify_db::entities::order_fulfillment::Model,
    currency: &str,
) -> gen::Fulfillment {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
    let flines = saleor_rustify_db::entities::order_fulfillmentline::Entity::find()
        .filter(saleor_rustify_db::entities::order_fulfillmentline::Column::FulfillmentId.eq(f.id))
        .order_by_asc(saleor_rustify_db::entities::order_fulfillmentline::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let mut lines = Vec::with_capacity(flines.len());
    for fl in flines {
        let ol = saleor_rustify_db::entities::order_orderline::Entity::find_by_id(fl.order_line_id)
            .one(db)
            .await
            .unwrap_or(None);
        lines.push(gen::FulfillmentLine {
            id: Some(ID(crate::common::gid("FulfillmentLine", fl.id))),
            quantity: Some(fl.quantity),
            order_line: match ol {
                Some(ref l) => Some(to_gen_order_line(db, l).await),
                None => None,
            },
            reason: if fl.reason.is_empty() { None } else { Some(fl.reason.clone()) },
            reason_reference: None,
        });
    }
    gen::Fulfillment {
        id: Some(ID(crate::common::gid("Fulfillment", f.id))),
        private_metadata: vec![],
        metadata: vec![],
        fulfillment_order: Some(f.fulfillment_order),
        status: Some(f.status.to_uppercase()),
        tracking_number: Some(f.tracking_number.clone()),
        created: Some(f.created_at.into()),
        lines,
        warehouse: None,
        shipping_refunded_amount: f
            .shipping_refund_amount
            .map(|a| to_gql_money(a, currency.to_string())),
        total_refunded_amount: f
            .total_refund_amount
            .map(|a| to_gql_money(a, currency.to_string())),
        reason: if f.reason.is_empty() { None } else { Some(f.reason.clone()) },
        reason_reference: None,
    }
}

pub(crate) async fn to_gen_order(
    db: &sea_orm::DatabaseConnection,
    h: &saleor_rustify_db::order_store::OrderHeader,
    ls: Vec<saleor_rustify_db::entities::order_orderline::Model>,
) -> gen::Order {
    // Order-level undiscounted total = sum of line undiscounted totals
    // (OrderHeader doesn't carry it; same arithmetic Django's data holds).
    let und_net: rust_decimal::Decimal = ls.iter().map(|l| l.undiscounted_total_price_net_amount).sum();
    let und_gross: rust_decimal::Decimal = ls.iter().map(|l| l.undiscounted_total_price_gross_amount).sum();
    // subtotal = sum of line totals (what the dashboard price summary reads).
    let sub_net: rust_decimal::Decimal = ls.iter().map(|l| l.total_price_net_amount).sum();
    let sub_gross: rust_decimal::Decimal = ls.iter().map(|l| l.total_price_gross_amount).sum();
    let mut lines = Vec::with_capacity(ls.len());
    for l in ls {
        lines.push(to_gen_order_line(db, &l).await);
    }
    // Fulfillments (dashboard order details + returns timeline read these;
    // an empty vec renders "no fulfillments" even after fulfilling).
    let mut fulfillments = vec![];
    {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
        let frows = saleor_rustify_db::entities::order_fulfillment::Entity::find()
            .filter(saleor_rustify_db::entities::order_fulfillment::Column::OrderId.eq(h.id))
            .order_by_asc(saleor_rustify_db::entities::order_fulfillment::Column::FulfillmentOrder)
            .all(db)
            .await
            .unwrap_or_default();
        for f in &frows {
            fulfillments.push(to_gen_fulfillment(db, f, &h.currency).await);
        }
    }
    // Transactions (details page timeline; the list view doesn't select
    // this, so the per-order cost lands only where it's rendered).
    let mut transactions = vec![];
    {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        let tids: Vec<i32> = saleor_rustify_db::entities::payment_transactionitem::Entity::find()
            .select_only()
            .column(saleor_rustify_db::entities::payment_transactionitem::Column::Id)
            .filter(saleor_rustify_db::entities::payment_transactionitem::Column::OrderId.eq(h.id))
            .into_tuple()
            .all(db)
            .await
            .unwrap_or_default();
        for tid in tids {
            if let Ok(t) = crate::payment::assemble_item(db, tid).await {
                transactions.push(t);
            }
        }
    }
    gen::Order {
        id: Some(ID(crate::common::gid("Order", &h.id))),
        private_metadata: vec![],
        metadata: vec![],
        created: Some(h.created_at.into()),
        updated_at: None,
        status: Some(h.status.clone()),
        user: None,
        billing_address: None,
        shipping_address: None,
        shipping_method_name: h.shipping_method_name.clone(),
        collection_point_name: None,
        channel: Some(gen::Channel {
            id: Some(ID("Q2hhbm5lbDox".into())),
            private_metadata: vec![],
            metadata: vec![],
            slug: Some("default-channel".into()),
            name: Some("Default".into()),
            is_active: Some(true),
            currency_code: Some("USD".into()),
            has_orders: None,
            default_country: Some(crate::common::GqlCountryDisplay { code: "US".into(), country: "United States".into() }),
            warehouses: vec![],
            stock_settings: Some(crate::common::GqlStockSettings { allocation_strategy: "prioritize-sorting-order".into() }),
            // Dashboard reads channel.orderSettings.markAsPaidStrategy
            // unconditionally (Saleor default strategy for channels).
            order_settings: Some(gen::OrderSettings {
                automatically_confirm_all_new_orders: None,
                automatically_fulfill_non_shippable_gift_card: None,
                expire_orders_after: None,
                mark_as_paid_strategy: Some("PAYMENT_FLOW".into()),
                delete_expired_orders_after: None,
                allow_unpaid_orders: None,
            }),
            checkout_settings: None,
            payment_settings: None,
            tax_configuration: None,
        }),
        fulfillments,
        lines,
        actions: vec![],
        shipping_methods: vec![],
        invoices: vec![],
        number: Some(h.number.to_string()),
        is_paid: None,
        payment_status: Some("NOT_CHARGED".into()),
        authorize_status: Some(h.authorize_status.to_uppercase()),
        charge_status: Some(h.charge_status.to_uppercase()),
        transactions,
        payments: vec![],
        total: Some(to_taxed(h.total_gross_amount, h.currency.clone())),
        undiscounted_total: Some(to_taxed2(und_net, und_gross, h.currency.clone())),
        shipping_method: None,
        shipping_price: Some(to_taxed2(h.shipping_price_net_amount, h.shipping_price_gross_amount, h.currency.clone())),
        voucher: None,
        voucher_code: None,
        gift_cards: vec![],
        customer_note: None,
        subtotal: Some(to_taxed2(sub_net, sub_gross, h.currency.clone())),
        total_authorized: None,
        total_charged: None,
        total_canceled: None,
        events: vec![],
        total_balance: None,
        user_email: Some(h.user_email.clone()),
        is_shipping_required: None,
        delivery_method: None,
        discounts: vec![],
        display_gross_prices: None,
        granted_refunds: vec![],
        total_granted_refund: None,
        total_refunded: None,
        total_refund_pending: None,
        total_authorize_pending: None,
        total_charge_pending: None,
        total_cancel_pending: None,
        total_remaining_grant: None,
    }
}

/// Taxed money with distinct net/gross; tax = gross − net (exact from the
/// rows — dashboard datagrids read `tax.amount` unconditionally, so a null
/// tax crashes the page while a zero one renders).
fn to_taxed2(net: rust_decimal::Decimal, gross: rust_decimal::Decimal, currency: String) -> GqlTaxedMoney {
    GqlTaxedMoney { gross: to_gql_money(gross, currency.clone()), net: to_gql_money(net, currency.clone()), tax: Some(to_gql_money(gross - net, currency.clone())), currency: Some(currency) }
}

/// Saleor `OrderStatus` enum name → DB status string (stored lowercase).
fn order_status_db(s: &gen::OrderStatus) -> &'static str {
    match s {
        gen::OrderStatus::DRAFT => "draft",
        gen::OrderStatus::UNCONFIRMED => "unconfirmed",
        gen::OrderStatus::UNFULFILLED => "unfulfilled",
        gen::OrderStatus::PARTIALLYFULFILLED => "partially_fulfilled",
        gen::OrderStatus::PARTIALLYRETURNED => "partially_returned",
        gen::OrderStatus::RETURNED => "returned",
        gen::OrderStatus::FULFILLED => "fulfilled",
        gen::OrderStatus::CANCELED => "canceled",
        gen::OrderStatus::EXPIRED => "expired",
    }
}

/// Deprecated `OrderStatusFilter` pseudo-statuses → closest DB status
/// (Saleor derives these from payment state; ours is hollow, documented).
fn order_status_filter_db(s: &gen::OrderStatusFilter) -> &'static str {
    match s {
        gen::OrderStatusFilter::READYTOFULFILL => "unfulfilled",
        gen::OrderStatusFilter::READYTOCAPTURE => "unfulfilled",
        gen::OrderStatusFilter::UNFULFILLED => "unfulfilled",
        gen::OrderStatusFilter::UNCONFIRMED => "unconfirmed",
        gen::OrderStatusFilter::PARTIALLYFULFILLED => "partially_fulfilled",
        gen::OrderStatusFilter::FULFILLED => "fulfilled",
        gen::OrderStatusFilter::CANCELED => "canceled",
    }
}

/// Parse an order ID: raw UUID or Saleor global ID (`T3JkZXI6...` → `Order:<uuid>`).
fn parse_order_uuid(s: &str) -> Option<Uuid> {
    let s = s.trim();
    if let Ok(u) = s.parse::<Uuid>() {
        return Some(u);
    }
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
    let t = String::from_utf8(bytes).ok()?;
    t.split_once(':')?.1.parse::<Uuid>().ok()
}

/// `number:<n>` / email substring search shared by `search` + deprecated filter.
fn search_condition(s: &str) -> sea_orm::Condition {
    use saleor_rustify_db::entities::order_order::Column as OCol;
    use sea_orm::{ColumnTrait, Condition};
    let s = s.trim();
    let like = format!("%{s}%");
    let mut any = Condition::any().add(OCol::UserEmail.like(like));
    if let Ok(n) = s.parse::<i32>() {
        any = any.add(OCol::Number.eq(n));
    }
    any
}

#[derive(Default)]
pub struct OrderQuery;

#[Object]
impl OrderQuery {
    async fn order(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::Order>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let oid = parse_id(&id.0);
        let Some((h, ls)) = saleor_rustify_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))? else { return Ok(None) };
        Ok(Some(to_gen_order(db, &h, ls).await))
    }

    /// Storefront order lookup by token (Django `orderByToken`: the token is
    /// the secret, so this stays permission-free).
    async fn order_by_token(&self, ctx: &Context<'_>, token: Uuid) -> Result<Option<gen::Order>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let oid: Uuid = token;
        let Some((h, ls)) = saleor_rustify_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))? else { return Ok(None) };
        Ok(Some(to_gen_order(db, &h, ls).await))
    }

    /// Global order settings (Django resolves these from the channel +
    /// site rows; we read the default channel, same values Django serves).
    async fn order_settings(&self, ctx: &Context<'_>) -> Result<Option<gen::OrderSettings>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        use saleor_rustify_db::entities::channel_channel::{Column as CCol, Entity as CEnt};
        // Slim select: the row carries an INTERVAL column SeaORM cannot decode.
        let row: Option<(Option<bool>, Option<bool>, Option<i32>, String, bool, Option<i32>)> = CEnt::find()
            .select_only()
            .column(CCol::AutomaticallyConfirmAllNewOrders)
            .column(CCol::AutomaticallyFulfillNonShippableGiftCard)
            .column(CCol::ExpireOrdersAfter)
            .column(CCol::OrderMarkAsPaidStrategy)
            .column(CCol::AllowUnpaidOrders)
            .column(CCol::DraftOrderLinePriceFreezePeriod)
            .filter(CCol::Slug.eq("default-channel"))
            .into_tuple()
            .one(db).await.map_err(|e| Error::new(e.to_string()))?;
        let Some((confirm, fulfill, expire, strategy, unpaid, _freeze)) = row else {
            return Err(Error::new("default channel missing"));
        };
        // INTERVAL → days via EXTRACT (never decode it into Rust).
        use sea_orm::{ConnectionTrait, Statement};
        let days: i32 = db.query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT EXTRACT(DAY FROM delete_expired_orders_after)::int AS d FROM channel_channel WHERE slug = 'default-channel'".to_string(),
        )).await.map_err(|e| Error::new(e.to_string()))?
        .and_then(|r| r.try_get::<i32>("", "d").ok()).unwrap_or(30);
        let strat = match strategy.as_str() {
            "payment_flow" => "PAYMENT_FLOW".to_string(),
            _ => "TRANSACTION_FLOW".to_string(),
        };
        Ok(Some(gen::OrderSettings {
            automatically_confirm_all_new_orders: confirm,
            automatically_fulfill_non_shippable_gift_card: fulfill,
            expire_orders_after: expire,
            mark_as_paid_strategy: Some(strat),
            delete_expired_orders_after: Some(days),
            allow_unpaid_orders: Some(unpaid),
        }))
    }

    /// Top-selling variants in a period (Django `reportProductSales`):
    /// order lines summed per variant, most units first. Saleor additionally
    /// stamps `quantityOrdered` on each node; our ProductVariant has no such
    /// field, so the ranking itself is the payload (documented).
    async fn report_product_sales(
        &self, ctx: &Context<'_>,
        period: gen::ReportingPeriod, channel: String,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
    ) -> Result<gen::ProductVariantCountableConnection> {
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| crate::common::decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        let (ch_id, _) = saleor_rustify_db::catalog::channel_info(db, &channel).await.map_err(|e| Error::new(e.to_string()))?;
        let days: i64 = match format!("{period:?}").as_str() {
            "TODAY" => 1,
            _ => 30,
        };
        use sea_orm::{ConnectionTrait, Statement};
        let rows = db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT l.variant_id AS vid, SUM(l.quantity)::bigint AS sold \
             FROM order_orderline l JOIN order_order o ON o.id = l.order_id \
             WHERE o.channel_id = $1 AND l.variant_id IS NOT NULL AND o.created_at > now() - make_interval(days => $2) \
             GROUP BY l.variant_id ORDER BY sold DESC LIMIT 200",
            [ch_id.into(), (days as i32).into()],
        )).await.map_err(|e| Error::new(e.to_string()))?;
        let vids: Vec<i32> = rows.into_iter().filter_map(|r| r.try_get::<i32>("", "vid").ok()).collect();
        let total = vids.len();
        let page: Vec<i32> = vids.into_iter().skip(off).take(lim).collect();
        let b = crate::catalog::load_variant_batches(db, &page).await.map_err(|e| Error::new(format!("{e:?}")))?;
        let mut edges = vec![];
        for (i, vid) in page.iter().enumerate() {
            let (sku, name, _, _) = b.vbase.get(vid).cloned().unwrap_or((None, String::new(), true, None));
            let _ = off + i;
            edges.push(gen::ProductVariantCountableEdge {
                node: Some(crate::catalog::build_variant(&b, *vid, name, sku.unwrap_or_default(), 0)),
            });
        }
        Ok(gen::ProductVariantCountableConnection {
            page_info: Some(crate::common::PageInfo { has_next_page: off + lim < total, has_previous_page: off > 0, start_cursor: None, end_cursor: None }),
            edges,
            total_count: Some(total as i32),
        })
    }

    /// Dashboard `OrderList`/`GlobalSearch` — filter/where/search/sortBy are
    /// applied server-side with Saleor semantics; payment-state-derived
    /// pseudo filters stay approximated (see `order_status_filter_db`).
    async fn orders(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::OrderSortingInput>, filter: Option<gen::OrderFilterInput>, #[graphql(name = "where")] where_input: Option<gen::OrderWhereInput>, search: Option<String>,
    ) -> Result<GqlOrderConnection> {
        let _ = (before, last);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use saleor_rustify_db::entities::order_order::{Column as OCol, Entity as OEnt};
        use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
        let mut cond = Condition::all();
        // --- where input ---
        if let Some(w) = where_input.as_ref() {
            if let Some(st) = w.status.as_ref() {
                let mut vals = vec![];
                if let Some(eq) = st.eq.as_ref() { vals.push(order_status_db(eq).to_string()); }
                vals.extend(st.one_of.clone().unwrap_or_default().into_iter().map(|s| order_status_db(&s).to_string()));
                if !vals.is_empty() { cond = cond.add(OCol::Status.is_in(vals)); }
            }
            let wids: Vec<Uuid> = w.ids.clone().unwrap_or_default().into_iter().filter_map(|i| parse_order_uuid(&i.0)).collect();
            if !wids.is_empty() { cond = cond.add(OCol::Id.is_in(wids)); }
            if let Some(num) = w.number.as_ref() {
                if let Some(eq) = num.eq { cond = cond.add(OCol::Number.eq(eq)); }
                if let Some(one) = num.one_of.clone() { if !one.is_empty() { cond = cond.add(OCol::Number.is_in(one)); } }
            }
            if let Some(em) = w.user_email.as_ref() {
                let mut any = Condition::any();
                if let Some(eq) = em.eq.as_ref() { any = any.add(OCol::UserEmail.like(format!("%{eq}%"))); }
                for v in em.one_of.clone().unwrap_or_default() { any = any.add(OCol::UserEmail.like(format!("%{v}%"))); }
                cond = cond.add(any);
            }
            if let Some(r) = w.created_at.as_ref() {
                if let Some(gte) = r.gte { cond = cond.add(OCol::CreatedAt.gte(gte)); }
                if let Some(lte) = r.lte { cond = cond.add(OCol::CreatedAt.lte(lte)); }
            }
            // AND/OR nesting on orders: dashboard list pages send flat where;
            // nested branches stay accepted-ignored (documented).
        }
        // --- deprecated filter input (GlobalSearch/Navigator send search) ---
        if let Some(flt) = filter.as_ref() {
            if let Some(s) = flt.search.as_ref() {
                cond = cond.add(search_condition(s));
            }
            if let Some(sts) = flt.status.as_ref() {
                if !sts.is_empty() {
                    cond = cond.add(OCol::Status.is_in(sts.iter().map(|s| order_status_filter_db(s).to_string()).collect::<Vec<_>>()));
                }
            }
            let fids: Vec<Uuid> = flt.ids.clone().unwrap_or_default().into_iter().filter_map(|i| parse_order_uuid(&i.0)).collect();
            if !fids.is_empty() { cond = cond.add(OCol::Id.is_in(fids)); }
            let fnums: Vec<i32> = flt.numbers.clone().unwrap_or_default().into_iter().filter_map(|n| n.parse::<i32>().ok()).collect();
            if !fnums.is_empty() { cond = cond.add(OCol::Number.is_in(fnums)); }
            if let Some(c) = flt.customer.as_ref() {
                cond = cond.add(OCol::UserEmail.like(format!("%{c}%")));
            }
            if let Some(chs) = flt.channels.as_ref() {
                let cids: Vec<i32> = chs.iter().filter_map(|i| saleor_rustify_db::catalog::parse_gid(&i.0)).collect();
                if !cids.is_empty() { cond = cond.add(OCol::ChannelId.is_in(cids)); }
            }
            if let Some(r) = flt.created.as_ref() {
                if let Some(gte) = r.gte { cond = cond.add(OCol::CreatedAt.gte(gte)); }
                if let Some(lte) = r.lte { cond = cond.add(OCol::CreatedAt.lte(lte)); }
            }
        }
        if let Some(s) = search.as_ref() {
            cond = cond.add(search_condition(s));
        }
        let mut q = OEnt::find()
            .select_only().column(OCol::Id)
            .filter(cond);
        // Sort (Saleor OrderSortingInput; default = newest first, as before).
        let asc = sort_by.as_ref().map(|s| matches!(s.direction, gen::OrderDirection::ASC)).unwrap_or(false);
        q = match sort_by.as_ref().map(|s| &s.field) {
            Some(gen::OrderSortField::NUMBER) => if asc { q.order_by_asc(OCol::Number) } else { q.order_by_desc(OCol::Number) },
            Some(gen::OrderSortField::STATUS) => if asc { q.order_by_asc(OCol::Status) } else { q.order_by_desc(OCol::Status) },
            _ => if asc { q.order_by_asc(OCol::CreatedAt) } else { q.order_by_desc(OCol::CreatedAt) },
        };
        let ids: Vec<Uuid> = q.into_tuple::<Uuid>().all(db).await.map_err(|e| Error::new(e.to_string()))?;
        let total = ids.len() as i32;
        let mut out = Vec::new();
        for oid in ids.into_iter().skip(off).take(lim) {
            if let Some((hh, ll)) = saleor_rustify_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))? {
                out.push(to_gen_order(db, &hh, ll).await);
            }
        }
        let edges = out.into_iter().enumerate().map(|(i, node)| GqlOrderEdge { node, cursor: encode_cursor(off + i) }).collect();
        Ok(GqlOrderConnection { total_count: Some(total), edges, page_info: crate::common::PageInfo { has_next_page: off + lim < total as usize, has_previous_page: off > 0, start_cursor: None, end_cursor: None } })
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderCancel")]
pub struct GqlOrderCancel {
    pub order: Option<gen::Order>,
    pub errors: Vec<gen::OrderError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlOrderBulkCancel {
    pub count: i32,
    pub errors: Vec<gen::OrderError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlDraftLinesBulkDelete {
    pub count: i32,
    pub errors: Vec<gen::OrderError>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlOrderCreateFromCheckout {
    pub order: Option<gen::Order>,
    pub errors: Vec<gen::OrderError>,
}

fn oerr(message: String) -> gen::OrderError {
    gen::OrderError { field: None, message: Some(message), code: None, warehouse: None, order_lines: vec![], address_type: None }
}

#[derive(Default)]
pub struct OrderMutation;

#[Object]
impl OrderMutation {
    /// Dashboard `OrderCancel` shape (`{ order { ...OrderDetails } errors }`).
    async fn order_cancel(&self, ctx: &Context<'_>, id: ID) -> Result<GqlOrderCancel> {        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid = parse_id(&id.0);
        saleor_rustify_db::cancel::cancel_order(db, oid).await.map_err(|e| Error::new(e.to_string()))?;
        let (h, ls) = saleor_rustify_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("order vanished"))?;
        Ok(GqlOrderCancel { order: Some(to_gen_order(db, &h, ls).await), errors: vec![] })
    }

    /// Saleor `orderFulfill(order: ID, input: OrderFulfillInput!)`
    /// (dashboard `FulfillOrder`). One FulfillItem per stock entry.
    async fn order_fulfill(&self, ctx: &Context<'_>, order: Option<ID>, input: gen::OrderFulfillInput) -> Result<Option<gen::OrderFulfill>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid = parse_id(&order.map(|o| o.0).unwrap_or_default());
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        let mut items: Vec<saleor_rustify_db::fulfillment::FulfillItem> = vec![];
        for l in input.lines {
            let lid = parse_id(&l.order_line_id.map(|i| i.0).unwrap_or_default());
            // variant of this line (for warehouse -> stock resolution).
            let line_variant: Option<i32> = saleor_rustify_db::entities::order_orderline::Entity::find()
                .select_only().column(saleor_rustify_db::entities::order_orderline::Column::VariantId)
                .filter(saleor_rustify_db::entities::order_orderline::Column::Id.eq(lid))
                .into_tuple::<Option<i32>>().one(db).await.map_err(|e| Error::new(e.to_string()))?.flatten();
            for s in l.stocks {
                // Dashboard sends the WAREHOUSE id (uuid global); resolve to
                // the variant's stock row in that warehouse (was silently
                // dropped by an i32 parse — wrong-warehouse fulfillments).
                let stock_id: Option<i32> = match crate::common::parse_uuid_gid(&s.warehouse.0) {
                    Some(wid) => match line_variant {
                        Some(vid) => saleor_rustify_db::fulfillment::stock_for_variant_warehouse(db, vid, wid).await.map_err(|e| Error::new(e.to_string()))?,
                        None => None,
                    },
                    None => None,
                };
                items.push(saleor_rustify_db::fulfillment::FulfillItem {
                    order_line_id: lid,
                    quantity: s.quantity,
                    stock_id,
                });
            }
        }
        let tracking = input.tracking_number.unwrap_or_default();
        saleor_rustify_db::fulfillment::create_fulfillment(db, oid, &items, &tracking).await.map_err(|e| Error::new(e.to_string()))?;
        let order_view = match saleor_rustify_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))? {
            Some((h, ll)) => Some(to_gen_order(db, &h, ll).await),
            None => None,
        };
        Ok(Some(gen::OrderFulfill { order: order_view, errors: vec![] }))
    }

    async fn order_return_lines(&self, ctx: &Context<'_>, order_id: ID, lines: Vec<ReturnLineInput>, reason: String, restock: Option<bool>) -> Result<String> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid = parse_id(&order_id.0);
        let items: Vec<saleor_rustify_db::fulfillment::FulfillItem> = lines.into_iter().map(|l| {
            let lid = parse_id(&l.order_line_id.0);
            let sid: Option<i32> = l.stock_id.and_then(|s| saleor_rustify_db::catalog::parse_gid(&s.0));
            saleor_rustify_db::fulfillment::FulfillItem { order_line_id: lid, quantity: l.quantity, stock_id: sid }
        }).collect();
        let out = saleor_rustify_db::fulfillment::return_and_refund(db, oid, &items, &reason, restock.unwrap_or(true), None).await.map_err(|e| Error::new(e.to_string()))?;
        Ok(format!("fulfillment:{} grant:{}", out.fulfillment_id, out.granted_refund_id))
    }

    /// Staff order note (Django `orderNoteAdd` → `note_added` event row).
    /// NOTE: schema name is `orderNoteAdd` (not `orderAddNote`); the explicit
    /// rename keeps codegen from emitting a shadowing stub.
    #[graphql(name = "orderNoteAdd")]
    async fn order_note_add(&self, ctx: &Context<'_>, order: ID, input: gen::OrderNoteInput) -> Result<gen::OrderNoteAdd> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await?;
        let oid = parse_id(&order.0);
        if input.message.trim().is_empty() {
            return Ok(gen::OrderNoteAdd { order: None, errors: vec![gen::OrderNoteAddError { field: None, message: Some("message is required".into()), code: None }] });
        }
        if let Err(e) = saleor_rustify_db::order_store::add_order_note(db, oid, Some(uid), input.message.trim()).await {
            return Ok(gen::OrderNoteAdd { order: None, errors: vec![gen::OrderNoteAddError { field: None, message: Some(e.to_string()), code: None }] });
        }
        let (h, ls) = saleor_rustify_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("order vanished"))?;
        Ok(gen::OrderNoteAdd { order: Some(to_gen_order(db, &h, ls).await), errors: vec![] })
    }

    /// Bulk cancel (per-order guards; survivors always commit).
    async fn order_bulk_cancel(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<GqlOrderBulkCancel> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let mut n = 0;
        for i in &ids {
            if saleor_rustify_db::cancel::cancel_order(db, parse_id(&i.0)).await.is_ok() {
                n += 1;
            }
        }
        Ok(GqlOrderBulkCancel { count: n, errors: vec![] })
    }

    /// Bulk-delete draft order lines (line ids; order resolved per row).
    async fn draft_order_lines_bulk_delete(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<GqlDraftLinesBulkDelete> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await?;
        let mut n = 0;
        for i in &ids {
            let lid = parse_id(&i.0);
            use sea_orm::EntityTrait;
            let oid: Option<Uuid> = saleor_rustify_db::entities::order_orderline::Entity::find_by_id(lid)
                .one(db).await.map_err(|e| Error::new(e.to_string()))?.map(|l| l.order_id);
            if let Some(oid) = oid {
                if saleor_rustify_db::drafts::remove_line(db, oid, lid, Some(uid)).await.is_ok() {
                    n += 1;
                }
            }
        }
        Ok(GqlDraftLinesBulkDelete { count: n, errors: vec![] })
    }

    /// Create an order from a checkout (Django `orderCreateFromCheckout`:
    /// same atomic path as `checkoutComplete`, optional metadata carry).
    async fn order_create_from_checkout(
        &self, ctx: &Context<'_>, id: ID,
        metadata: Option<Vec<crate::common::MetadataInput>>,
        #[graphql(name = "privateMetadata")] private_metadata: Option<Vec<crate::common::MetadataInput>>,
        #[graphql(name = "removeCheckout")] remove_checkout: Option<bool>,
    ) -> Result<GqlOrderCreateFromCheckout> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        if remove_checkout == Some(false) {
            // Our completion always consumes the checkout (same end state as
            // the default); keeping it usable is not supported honestly.
            return Ok(GqlOrderCreateFromCheckout { order: None, errors: vec![oerr("removeCheckout=false is not supported".into())] });
        }
        let token = crate::common::parse_uuid_gid(&id.0).ok_or_else(|| Error::new("bad checkout id"))?;
        let out = saleor_rustify_db::complete::complete_checkout(db, token).await.map_err(|e| Error::new(e.to_string()))?;
        if metadata.is_some() || private_metadata.is_some() {
            use sea_orm::{ActiveModelTrait, EntityTrait, Set};
            if let Some(row) = saleor_rustify_db::entities::order_order::Entity::find_by_id(out.order_id).one(db).await.map_err(|e| Error::new(e.to_string()))? {
                let cur_md = serde_json::to_value(&row.metadata).unwrap_or(serde_json::Value::Null);
                let cur_pmd = serde_json::to_value(&row.private_metadata).unwrap_or(serde_json::Value::Null);
                let mut am: saleor_rustify_db::entities::order_order::ActiveModel = row.into();
                if let Some(m) = metadata.as_ref() {
                    am.metadata = Set(crate::common::merge_metadata(&cur_md, m));
                }
                if let Some(m) = private_metadata.as_ref() {
                    am.private_metadata = Set(crate::common::merge_metadata(&cur_pmd, m));
                }
                am.update(db).await.map_err(|e| Error::new(e.to_string()))?;
            }
        }
        let (h, ls) = saleor_rustify_db::order_store::get_order_rows(db, out.order_id).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("order vanished"))?;
        Ok(GqlOrderCreateFromCheckout { order: Some(to_gen_order(db, &h, ls).await), errors: vec![] })
    }

    async fn order_confirm(&self, ctx: &Context<'_>, id: ID) -> Result<gen::OrderConfirm> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let oid = parse_id(&id.0);
        if let Err(e) = saleor_rustify_db::order_ops::confirm_order(db, oid, Some(uid)).await {
            return Ok(gen::OrderConfirm { order: None, errors: vec![oerr(e.to_string())] });
        }
        Ok(gen::OrderConfirm { order: order_view(db, oid).await?, errors: vec![] })
    }

    async fn order_capture(&self, ctx: &Context<'_>, id: ID, amount: gen::GenPositiveDecimal) -> Result<gen::OrderCapture> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let oid = parse_id(&id.0);
        let amt = amount.0.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO);
        if let Err(e) = saleor_rustify_db::order_ops::capture_order(db, oid, amt, Some(uid)).await {
            return Ok(gen::OrderCapture { order: None, errors: vec![oerr(e.to_string())] });
        }
        Ok(gen::OrderCapture { order: order_view(db, oid).await?, errors: vec![] })
    }

    async fn order_refund(&self, ctx: &Context<'_>, id: ID, amount: gen::GenPositiveDecimal) -> Result<gen::OrderRefund> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let oid = parse_id(&id.0);
        let amt = amount.0.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO);
        if let Err(e) = saleor_rustify_db::order_ops::refund_order(db, oid, amt, Some(uid)).await {
            return Ok(gen::OrderRefund { order: None, errors: vec![oerr(e.to_string())] });
        }
        Ok(gen::OrderRefund { order: order_view(db, oid).await?, errors: vec![] })
    }

    async fn order_void(&self, ctx: &Context<'_>, id: ID) -> Result<gen::OrderVoid> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let oid = parse_id(&id.0);
        if let Err(e) = saleor_rustify_db::order_ops::void_order(db, oid, Some(uid)).await {
            return Ok(gen::OrderVoid { order: None, errors: vec![oerr(e.to_string())] });
        }
        Ok(gen::OrderVoid { order: order_view(db, oid).await?, errors: vec![] })
    }

    async fn order_mark_as_paid(&self, ctx: &Context<'_>, id: ID, #[graphql(name = "transactionReference")] transaction_reference: Option<String>) -> Result<gen::OrderMarkAsPaid> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let oid = parse_id(&id.0);
        if let Err(e) = saleor_rustify_db::order_ops::mark_order_as_paid(db, oid, transaction_reference, Some(uid)).await {
            return Ok(gen::OrderMarkAsPaid { order: None, errors: vec![oerr(e.to_string())] });
        }
        Ok(gen::OrderMarkAsPaid { order: order_view(db, oid).await?, errors: vec![] })
    }

    async fn order_update(&self, ctx: &Context<'_>, #[graphql(name = "externalReference")] external_reference: Option<String>, id: ID, input: gen::OrderUpdateInput) -> Result<gen::OrderUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let oid = parse_id(&id.0);
        let out = saleor_rustify_db::order_ops::update_order(
            db, oid,
            input.user_email,
            external_reference,
            input.language_code.as_ref().map(|c| format!("{c:?}")),
            input.billing_address.as_ref().map(addr_input),
            input.shipping_address.as_ref().map(addr_input),
            Some(uid),
        )
        .await;
        // 3.23 OrderUpdateInput carries no customerNote (notes go through
        // orderNoteAdd); metadata stays on the metadata mutations.
        if let Err(e) = out {
            return Ok(gen::OrderUpdate { errors: vec![oerr(e.to_string())], order: None });
        }
        Ok(gen::OrderUpdate { errors: vec![], order: order_view(db, oid).await? })
    }

    async fn order_update_shipping(&self, ctx: &Context<'_>, order: ID, input: gen::OrderUpdateShippingInput) -> Result<gen::OrderUpdateShipping> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let oid = parse_id(&order.0);
        let mid = input.shipping_method.as_ref().and_then(|m| saleor_rustify_db::catalog::parse_gid(&m.0));
        if let Err(e) = saleor_rustify_db::order_ops::update_order_shipping(db, oid, mid, Some(uid)).await {
            return Ok(gen::OrderUpdateShipping { order: None, errors: vec![oerr(e.to_string())] });
        }
        Ok(gen::OrderUpdateShipping { order: order_view(db, oid).await?, errors: vec![] })
    }

    async fn order_note_update(&self, ctx: &Context<'_>, note: ID, input: gen::OrderNoteInput) -> Result<gen::OrderNoteUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let eid = saleor_rustify_db::catalog::parse_gid(&note.0).unwrap_or(-1);
        match saleor_rustify_db::order_ops::update_note(db, eid, &input.message).await {
            Ok(oid) => Ok(gen::OrderNoteUpdate { order: order_view(db, oid).await?, errors: vec![] }),
            Err(e) => Ok(gen::OrderNoteUpdate {
                order: None,
                errors: vec![gen::OrderNoteUpdateError { field: None, message: Some(e.to_string()), code: None }],
            }),
        }
    }

    async fn order_line_update(&self, ctx: &Context<'_>, id: ID, input: gen::OrderLineInput) -> Result<gen::OrderLineUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let lid = parse_id(&id.0);
        match line_order(db, lid).await {
            Ok(oid) => {
                if let Err(e) = saleor_rustify_db::order_ops::update_line_quantity(db, oid, lid, input.quantity, Some(uid)).await {
                    return Ok(gen::OrderLineUpdate { order: None, errors: vec![oerr(e.to_string())], order_line: None });
                }
                Ok(gen::OrderLineUpdate { order: order_view(db, oid).await?, errors: vec![], order_line: None })
            }
            Err(e) => Ok(gen::OrderLineUpdate { order: None, errors: vec![oerr(e)], order_line: None }),
        }
    }

    async fn order_lines_create(&self, ctx: &Context<'_>, id: ID, input: Vec<gen::OrderLineCreateInput>) -> Result<gen::OrderLinesCreate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let oid = parse_id(&id.0);
        let mut lines = Vec::with_capacity(input.len());
        for l in input {
            let vid = saleor_rustify_db::catalog::parse_gid(&l.variant_id.0).unwrap_or(-1);
            if vid < 0 {
                return Ok(gen::OrderLinesCreate { order: None, errors: vec![oerr("bad variant id".into())] });
            }
            lines.push(saleor_rustify_db::order_ops::NewOrderLine { variant_id: vid, quantity: l.quantity });
        }
        if let Err(e) = saleor_rustify_db::order_ops::create_lines(db, oid, lines, Some(uid)).await {
            return Ok(gen::OrderLinesCreate { order: None, errors: vec![oerr(e.to_string())] });
        }
        Ok(gen::OrderLinesCreate { order: order_view(db, oid).await?, errors: vec![] })
    }

    async fn order_line_delete(&self, ctx: &Context<'_>, id: ID) -> Result<gen::OrderLineDelete> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let lid = parse_id(&id.0);
        match line_order(db, lid).await {
            Ok(oid) => {
                if let Err(e) = saleor_rustify_db::order_ops::delete_line(db, oid, lid, Some(uid)).await {
                    return Ok(gen::OrderLineDelete { order: None, errors: vec![oerr(e.to_string())] });
                }
                Ok(gen::OrderLineDelete { order: order_view(db, oid).await?, errors: vec![] })
            }
            Err(e) => Ok(gen::OrderLineDelete { order: None, errors: vec![oerr(e)] }),
        }
    }

    async fn order_discount_add(&self, ctx: &Context<'_>, input: gen::OrderDiscountCommonInput, #[graphql(name = "orderId")] order_id: ID) -> Result<gen::OrderDiscountAdd> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let oid = parse_id(&order_id.0);
        let vt = if matches!(input.value_type, gen::DiscountValueTypeEnum::PERCENTAGE) { "percentage" } else { "fixed" };
        let vv = input.value.0.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO);
        if let Err(e) = saleor_rustify_db::order_ops::discount_add(db, oid, input.reason, vt, vv, Some(uid)).await {
            return Ok(gen::OrderDiscountAdd { order: None, errors: vec![oerr(e.to_string())] });
        }
        Ok(gen::OrderDiscountAdd { order: order_view(db, oid).await?, errors: vec![] })
    }

    async fn order_discount_update(&self, ctx: &Context<'_>, #[graphql(name = "discountId")] discount_id: ID, input: gen::OrderDiscountCommonInput) -> Result<gen::OrderDiscountUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let did = parse_id(&discount_id.0);
        let vt = if matches!(input.value_type, gen::DiscountValueTypeEnum::PERCENTAGE) { "percentage" } else { "fixed" };
        let vv = input.value.0.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO);
        match saleor_rustify_db::order_ops::discount_update(db, did, input.reason, Some(vt.into()), Some(vv), Some(uid)).await {
            Ok(oid) => Ok(gen::OrderDiscountUpdate { order: order_view(db, oid).await?, errors: vec![] }),
            Err(e) => Ok(gen::OrderDiscountUpdate { order: None, errors: vec![oerr(e.to_string())] }),
        }
    }

    async fn order_discount_delete(&self, ctx: &Context<'_>, #[graphql(name = "discountId")] discount_id: ID) -> Result<gen::OrderDiscountDelete> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let did = parse_id(&discount_id.0);
        match saleor_rustify_db::order_ops::discount_delete(db, did, Some(uid)).await {
            Ok(oid) => Ok(gen::OrderDiscountDelete { order: order_view(db, oid).await?, errors: vec![] }),
            Err(e) => Ok(gen::OrderDiscountDelete { order: None, errors: vec![oerr(e.to_string())] }),
        }
    }

    async fn order_line_discount_update(&self, ctx: &Context<'_>, input: gen::OrderDiscountCommonInput, #[graphql(name = "orderLineId")] order_line_id: ID) -> Result<gen::OrderLineDiscountUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let lid = parse_id(&order_line_id.0);
        let vt = if matches!(input.value_type, gen::DiscountValueTypeEnum::PERCENTAGE) { "percentage" } else { "fixed" };
        let vv = input.value.0.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO);
        match saleor_rustify_db::order_ops::line_discount_update(db, lid, vt, vv, input.reason, Some(uid)).await {
            Ok(oid) => Ok(gen::OrderLineDiscountUpdate { order: order_view(db, oid).await?, errors: vec![] }),
            Err(e) => Ok(gen::OrderLineDiscountUpdate { order: None, errors: vec![oerr(e.to_string())] }),
        }
    }

    async fn order_line_discount_remove(&self, ctx: &Context<'_>, #[graphql(name = "orderLineId")] order_line_id: ID) -> Result<gen::OrderLineDiscountRemove> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let lid = parse_id(&order_line_id.0);
        match saleor_rustify_db::order_ops::line_discount_remove(db, lid, Some(uid)).await {
            Ok(oid) => Ok(gen::OrderLineDiscountRemove { order: order_view(db, oid).await?, errors: vec![] }),
            Err(e) => Ok(gen::OrderLineDiscountRemove { order: None, errors: vec![oerr(e.to_string())] }),
        }
    }

    async fn order_fulfillment_approve(
        &self, ctx: &Context<'_>, id: ID,
        #[graphql(name = "notifyCustomer")] notify_customer: bool,
        #[graphql(name = "allowStockToBeExceeded")] allow_stock_to_be_exceeded: Option<bool>,
    ) -> Result<gen::FulfillmentApprove> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let _ = notify_customer; // customer email is SMTP-side (documented gap)
        let fid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        match saleor_rustify_db::fulfillment::approve_fulfillment(db, fid, allow_stock_to_be_exceeded.unwrap_or(false)).await {
            Ok(v) => Ok(gen::FulfillmentApprove { order: order_view(db, v.order_id).await?, errors: vec![] }),
            Err(e) => Ok(gen::FulfillmentApprove { order: None, errors: vec![oerr(e.to_string())] }),
        }
    }

    async fn order_fulfillment_cancel(
        &self, ctx: &Context<'_>, id: ID, input: gen::FulfillmentCancelInput,
    ) -> Result<gen::FulfillmentCancel> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let fid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        let wid = input.warehouse_id.as_ref().and_then(|w| crate::common::parse_uuid_gid(&w.0));
        match saleor_rustify_db::fulfillment::cancel_fulfillment_to(db, fid, wid).await {
            Ok(v) => Ok(gen::FulfillmentCancel { order: order_view(db, v.order_id).await?, errors: vec![] }),
            Err(e) => Ok(gen::FulfillmentCancel { order: None, errors: vec![oerr(e.to_string())] }),
        }
    }

    async fn order_fulfillment_update_tracking(
        &self, ctx: &Context<'_>, id: ID, input: gen::FulfillmentUpdateTrackingInput,
    ) -> Result<gen::FulfillmentUpdateTracking> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let fid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        let tracking = input.tracking_number.clone().unwrap_or_default();
        match saleor_rustify_db::fulfillment::update_tracking(db, fid, &tracking).await {
            Ok(v) => Ok(gen::FulfillmentUpdateTracking { order: order_view(db, v.order_id).await?, errors: vec![] }),
            Err(e) => Ok(gen::FulfillmentUpdateTracking { order: None, errors: vec![oerr(e.to_string())] }),
        }
    }

    async fn order_fulfillment_refund_products(
        &self, ctx: &Context<'_>, order: ID, input: gen::OrderRefundProductsInput,
    ) -> Result<gen::FulfillmentRefundProducts> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid = parse_id(&order.0);
        let items = match refund_items(db, &input.order_lines, &input.fulfillment_lines).await {            Ok(i) => i,
            Err(e) => return Ok(gen::FulfillmentRefundProducts { fulfillment: None, order: None, errors: vec![oerr(e)] }),
        };
        if items.is_empty() {
            return Ok(gen::FulfillmentRefundProducts { fulfillment: None, order: None, errors: vec![oerr("provide orderLines or fulfillmentLines".into())] });
        }
        let amount = input.amount_to_refund.as_ref().and_then(|a| a.0.parse::<rust_decimal::Decimal>().ok());
        // Refunds cover unfulfilled lines: no shelf move (Django deallocates
        // instead; same net stock effect).
        match saleor_rustify_db::fulfillment::return_and_refund_full(            db, oid, &items, "", false, None,
            input.include_shipping_costs.unwrap_or(false), amount,
        ).await {
            Ok(out) => {
                use sea_orm::EntityTrait;
                let frow = saleor_rustify_db::entities::order_fulfillment::Entity::find_by_id(out.fulfillment_id)
                    .one(db).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("fulfillment vanished"))?;
                let cur = order_currency(db, oid).await?;
                Ok(gen::FulfillmentRefundProducts {
                    fulfillment: Some(to_gen_fulfillment(db, &frow, &cur).await),
                    order: order_view(db, oid).await?,
                    errors: vec![],
                })
            }
            Err(e) => Ok(gen::FulfillmentRefundProducts { fulfillment: None, order: None, errors: vec![oerr(e.to_string())] }),
        }
    }

    async fn order_fulfillment_return_products(
        &self, ctx: &Context<'_>, order: ID, input: gen::OrderReturnProductsInput,
    ) -> Result<gen::FulfillmentReturnProducts> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid = parse_id(&order.0);
        // Per-line `replace` spins a replacement order (draft) in Django —
        // separate milestone, refused honestly instead of half-created.
        let wants_replace = input.order_lines.clone().unwrap_or_default().into_iter().any(|l| l.replace.unwrap_or(false))
            || input.fulfillment_lines.clone().unwrap_or_default().into_iter().any(|l| l.replace.unwrap_or(false));
        if wants_replace {
            return Ok(gen::FulfillmentReturnProducts { order: None, replace_order: None, errors: vec![oerr("replacement orders are not supported".into())] });
        }
        let items = match refund_items_return(db, &input.order_lines, &input.fulfillment_lines).await {
            Ok(i) => i,
            Err(e) => return Ok(gen::FulfillmentReturnProducts { order: None, replace_order: None, errors: vec![oerr(e)] }),
        };
        if items.is_empty() {
            return Ok(gen::FulfillmentReturnProducts { order: None, replace_order: None, errors: vec![oerr("provide orderLines or fulfillmentLines".into())] });
        }
        let reason = input.reason.clone().unwrap_or_default();
        if reason.trim().is_empty() {
            return Ok(gen::FulfillmentReturnProducts { order: None, replace_order: None, errors: vec![oerr("reason is required".into())] });
        }
        let amount = input.amount_to_refund.as_ref().and_then(|a| a.0.parse::<rust_decimal::Decimal>().ok());
        // refund=false → shelf move only, no money (Django's return flow
        // separates the stock move from the money the same way).
        let out = if input.refund == Some(false) {
            match saleor_rustify_db::fulfillment::refund_fulfillment(db, oid, &items, &reason).await {
                Ok(f) => saleor_rustify_db::fulfillment::ReturnRefundView { fulfillment_id: f.id, granted_refund_id: 0, amount: rust_decimal::Decimal::ZERO },
                Err(e) => return Ok(gen::FulfillmentReturnProducts { order: None, replace_order: None, errors: vec![oerr(e.to_string())] }),
            }
        } else {
            match saleor_rustify_db::fulfillment::return_and_refund_full(
                db, oid, &items, &reason, true, None,
                input.include_shipping_costs.unwrap_or(false), amount,
            ).await {
                Ok(o) => o,
                Err(e) => return Ok(gen::FulfillmentReturnProducts { order: None, replace_order: None, errors: vec![oerr(e.to_string())] }),
            }
        };
        let _ = out;
        Ok(gen::FulfillmentReturnProducts { order: order_view(db, oid).await?, replace_order: None, errors: vec![] })
    }

    async fn order_grant_refund_create(
        &self, ctx: &Context<'_>, id: ID, input: gen::OrderGrantRefundCreateInput,
    ) -> Result<gen::OrderGrantRefundCreate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let (uid, _) = crate::account::requester(ctx, db).await.unwrap_or((0, String::new()));
        let oid = parse_id(&id.0);
        let mut lines = vec![];
        for l in input.lines.clone().unwrap_or_default() {
            let lid = parse_id(&l.id.0);
            if lid.is_nil() {
                return Ok(gen::OrderGrantRefundCreate { order: None, granted_refund: None, errors: vec![grant_err("bad order line id".into())] });
            }
            lines.push(saleor_rustify_db::granted_refunds::GrantLineInput { order_line_id: lid, quantity: l.quantity });
        }
        let tid = saleor_rustify_db::catalog::parse_gid(&input.transaction_id.0)
            .ok_or_else(|| ())
            .and_then(|t| if t > 0 { Ok(t) } else { Err(()) });
        let tid = match tid {
            Ok(t) => t,
            Err(()) => return Ok(gen::OrderGrantRefundCreate { order: None, granted_refund: None, errors: vec![grant_err("bad transaction id".into())] }),
        };
        let amount = input.amount.as_ref().and_then(|a| a.0.parse::<rust_decimal::Decimal>().ok());
        let reason = input.reason.clone().unwrap_or_default();
        match saleor_rustify_db::granted_refunds::create_granted_refund(db, &saleor_rustify_db::granted_refunds::NewGrant {
            order_id: oid,
            transaction_item_id: Some(tid),
            amount,
            lines,
            reason,
            shipping_costs_included: input.grant_refund_for_shipping.unwrap_or(false),
            user_id: Some(uid),
            app_id: None,
        }).await {
            Ok(v) => {
                let cur = order_currency(db, oid).await?;
                Ok(gen::OrderGrantRefundCreate {
                    order: order_view(db, oid).await?,
                    granted_refund: Some(granted_refund_view(db, &v, &cur).await?),
                    errors: vec![],
                })
            }
            Err(e) => Ok(gen::OrderGrantRefundCreate { order: None, granted_refund: None, errors: vec![grant_err(e.to_string())] }),
        }
    }

    async fn order_grant_refund_update(
        &self, ctx: &Context<'_>, id: ID, input: gen::OrderGrantRefundUpdateInput,
    ) -> Result<gen::OrderGrantRefundUpdate> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let gid = saleor_rustify_db::catalog::parse_gid(&id.0).unwrap_or(-1);
        let mut add_lines = vec![];
        for l in input.add_lines.clone().unwrap_or_default() {
            let lid = parse_id(&l.id.0);
            if lid.is_nil() {
                return Ok(gen::OrderGrantRefundUpdate { order: None, errors: vec![grant_upd_err("bad order line id".into())] });
            }
            add_lines.push(saleor_rustify_db::granted_refunds::GrantLineInput { order_line_id: lid, quantity: l.quantity });
        }
        let mut remove_line_ids = vec![];
        for r in input.remove_lines.clone().unwrap_or_default() {
            remove_line_ids.push(saleor_rustify_db::catalog::parse_gid(&r.0).unwrap_or(-1));
        }
        let amount = input.amount.as_ref().and_then(|a| a.0.parse::<rust_decimal::Decimal>().ok());
        let tid_opt = input.transaction_id.as_ref().map(|t| saleor_rustify_db::catalog::parse_gid(&t.0));
        match saleor_rustify_db::granted_refunds::update_granted_refund(db, gid, &saleor_rustify_db::granted_refunds::UpdateGrant {
            amount,
            reason: input.reason.clone(),
            transaction_item_id: tid_opt,
            grant_refund_for_shipping: input.grant_refund_for_shipping.unwrap_or(false),
            add_lines,
            remove_line_ids,
        }).await {
            Ok(v) => Ok(gen::OrderGrantRefundUpdate { order: order_view(db, v.order_id).await?, errors: vec![] }),
            Err(e) => Ok(gen::OrderGrantRefundUpdate { order: None, errors: vec![grant_upd_err(e.to_string())] }),
        }
    }
}

/// Order lines + fulfillment lines → money/stock items for the
/// refund/return flows.
async fn refund_items(
    db: &sea_orm::DatabaseConnection,
    order_lines: &Option<Vec<gen::OrderRefundLineInput>>,
    fulfillment_lines: &Option<Vec<gen::OrderRefundFulfillmentLineInput>>,
) -> std::result::Result<Vec<saleor_rustify_db::fulfillment::FulfillItem>, String> {
    use sea_orm::EntityTrait;
    let mut items = vec![];
    for l in order_lines.clone().unwrap_or_default() {
        items.push(saleor_rustify_db::fulfillment::FulfillItem {
            order_line_id: parse_id(&l.order_line_id.0),
            quantity: l.quantity,
            stock_id: None,
        });
    }
    for fl in fulfillment_lines.clone().unwrap_or_default() {
        let fid = saleor_rustify_db::catalog::parse_gid(&fl.fulfillment_line_id.0).unwrap_or(-1);
        let row = saleor_rustify_db::entities::order_fulfillmentline::Entity::find_by_id(fid)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "fulfillment line not found".to_string())?;
        items.push(saleor_rustify_db::fulfillment::FulfillItem {
            order_line_id: row.order_line_id,
            quantity: fl.quantity,
            stock_id: row.stock_id,
        });
    }
    Ok(items)
}

/// Same, for the return input shapes (superset with replace/reason).
async fn refund_items_return(
    db: &sea_orm::DatabaseConnection,
    order_lines: &Option<Vec<gen::OrderReturnLineInput>>,
    fulfillment_lines: &Option<Vec<gen::OrderReturnFulfillmentLineInput>>,
) -> std::result::Result<Vec<saleor_rustify_db::fulfillment::FulfillItem>, String> {
    use sea_orm::EntityTrait;
    let mut items = vec![];
    for l in order_lines.clone().unwrap_or_default() {
        items.push(saleor_rustify_db::fulfillment::FulfillItem {
            order_line_id: parse_id(&l.order_line_id.0),
            quantity: l.quantity,
            stock_id: None,
        });
    }
    for fl in fulfillment_lines.clone().unwrap_or_default() {
        let fid = saleor_rustify_db::catalog::parse_gid(&fl.fulfillment_line_id.0).unwrap_or(-1);
        let row = saleor_rustify_db::entities::order_fulfillmentline::Entity::find_by_id(fid)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "fulfillment line not found".to_string())?;
        items.push(saleor_rustify_db::fulfillment::FulfillItem {
            order_line_id: row.order_line_id,
            quantity: fl.quantity,
            stock_id: row.stock_id,
        });
    }
    Ok(items)
}

async fn order_currency(db: &sea_orm::DatabaseConnection, oid: Uuid) -> Result<String> {
    use sea_orm::{EntityTrait, QuerySelect};
    Ok(saleor_rustify_db::entities::order_order::Entity::find_by_id(oid)
        .select_only()
        .column(saleor_rustify_db::entities::order_order::Column::Currency)
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .unwrap_or_else(|| "USD".to_string()))
}

/// Granted-refund decision → dashboard payload (lines embed full order lines).
async fn granted_refund_view(
    db: &sea_orm::DatabaseConnection,
    v: &saleor_rustify_db::granted_refunds::GrantView,
    currency: &str,
) -> Result<gen::OrderGrantedRefund> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let rows = saleor_rustify_db::entities::order_ordergrantedrefundline::Entity::find()
        .filter(saleor_rustify_db::entities::order_ordergrantedrefundline::Column::GrantedRefundId.eq(v.id))
        .all(db)
        .await
        .map_err(|e| Error::new(e.to_string()))?;
    let mut lines = Vec::with_capacity(rows.len());
    for r in rows {
        let ol = saleor_rustify_db::entities::order_orderline::Entity::find_by_id(r.order_line_id)
            .one(db)
            .await
            .map_err(|e| Error::new(e.to_string()))?;
        lines.push(gen::OrderGrantedRefundLine {
            id: Some(ID(crate::common::gid("OrderGrantedRefundLine", r.id))),
            quantity: Some(r.quantity),
            order_line: match ol {
                Some(ref l) => Some(to_gen_order_line(db, l).await),
                None => None,
            },
            reason: r.reason.clone(),
            reason_reference: None,
        });
    }
    Ok(gen::OrderGrantedRefund {
        id: Some(ID(crate::common::gid("OrderGrantedRefund", v.id))),
        created_at: None,
        amount: Some(to_gql_money(v.amount, currency.to_string())),
        reason: Some(v.reason.clone()),
        reason_reference: None,
        user: None,
        app: None,
        shipping_costs_included: Some(false),
        lines,
        status: Some(v.status.to_uppercase()),
        transaction_events: vec![],
        transaction: None,
    })
}

fn grant_err(message: String) -> gen::OrderGrantRefundCreateError {
    gen::OrderGrantRefundCreateError { field: None, message: Some(message), code: None, lines: vec![] }
}

fn grant_upd_err(message: String) -> gen::OrderGrantRefundUpdateError {
    gen::OrderGrantRefundUpdateError { field: None, message: Some(message), code: None, add_lines: vec![], remove_lines: vec![] }
}

async fn order_view(db: &sea_orm::DatabaseConnection, oid: Uuid) -> Result<Option<gen::Order>> {
    match saleor_rustify_db::order_store::get_order_rows(db, oid).await {
        Ok(Some((h, ls))) => Ok(Some(to_gen_order(db, &h, ls).await)),
        Ok(None) => Ok(None),
        Err(e) => Err(Error::new(e.to_string())),
    }
}

async fn line_order(db: &sea_orm::DatabaseConnection, lid: Uuid) -> std::result::Result<Uuid, String> {
    use sea_orm::{EntityTrait, QuerySelect};
    saleor_rustify_db::entities::order_orderline::Entity::find_by_id(lid)
        .select_only()
        .column(saleor_rustify_db::entities::order_orderline::Column::OrderId)
        .into_tuple::<Uuid>()
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "order line not found".to_string())
}

fn addr_input(a: &gen::AddressInput) -> saleor_rustify_db::order_ops::OrderAddressInput {
    saleor_rustify_db::order_ops::OrderAddressInput {
        first_name: a.first_name.clone().unwrap_or_default(),
        last_name: a.last_name.clone().unwrap_or_default(),
        street1: a.street_address1.clone().unwrap_or_default(),
        street2: a.street_address2.clone().unwrap_or_default(),
        city: a.city.clone().unwrap_or_default(),
        postal_code: a.postal_code.clone().unwrap_or_default(),
        country: a.country.as_ref().map(|c| format!("{c:?}")).unwrap_or_default(),
        country_area: a.country_area.clone().unwrap_or_default(),
        phone: a.phone.clone().unwrap_or_default(),
        company_name: a.company_name.clone().unwrap_or_default(),
    }
}

async fn authorize(ctx: &Context<'_>, perm: &str) -> Result<()> {    // Was bearer-presence-only (perm ignored); now a real codename gate
    // (superusers bypass, so dashboard flows are unaffected).
    let _ = crate::account::require_perm(ctx, perm).await?;
    Ok(())
}
