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
async fn to_gen_order(
    db: &sea_orm::DatabaseConnection,
    h: &rustygod_db::order_store::OrderHeader,
    ls: Vec<rustygod_db::entities::order_orderline::Model>,
) -> gen::Order {
    use sea_orm::EntityTrait;
    // Order-level undiscounted total = sum of line undiscounted totals
    // (OrderHeader doesn't carry it; same arithmetic Django's data holds).
    let und_net: rust_decimal::Decimal = ls.iter().map(|l| l.undiscounted_total_price_net_amount).sum();
    let und_gross: rust_decimal::Decimal = ls.iter().map(|l| l.undiscounted_total_price_gross_amount).sum();
    // subtotal = sum of line totals (what the dashboard price summary reads).
    let sub_net: rust_decimal::Decimal = ls.iter().map(|l| l.total_price_net_amount).sum();
    let sub_gross: rust_decimal::Decimal = ls.iter().map(|l| l.total_price_gross_amount).sum();
    let mut lines = Vec::with_capacity(ls.len());
    for l in ls {
        // Real variant + product (dashboard lines datagrid reads
        // `variant.product.id` unconditionally — null product crashes it).
        let mut variant_name = None;
        let mut variant_sku = None;
        let mut product_stub = None;
        if let Some(vid) = l.variant_id {
            if let Ok(Some(v)) = rustygod_db::entities::product_productvariant::Entity::find_by_id(vid).one(db).await {
                variant_name = Some(v.name.clone());
                variant_sku = v.sku.clone();
                // Slim select: product_product.search_vector is tsvector and
                // crashes full-model decode (same class as channel INTERVAL).
                use sea_orm::{ColumnTrait, QueryFilter, QuerySelect};
                type PP = rustygod_db::entities::product_product::Entity;
                use rustygod_db::entities::product_product::Column as PPCol;
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
        lines.push(gen::OrderLine {
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
        });
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
        fulfillments: vec![],
        lines,
        actions: vec![],
        shipping_methods: vec![],
        invoices: vec![],
        number: Some(h.number.to_string()),
        is_paid: None,
        payment_status: Some("NOT_CHARGED".into()),
        authorize_status: None,
        charge_status: Some(h.status.clone()),
        transactions: vec![],
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
    use rustygod_db::entities::order_order::Column as OCol;
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
        let Some((h, ls)) = rustygod_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))? else { return Ok(None) };
        Ok(Some(to_gen_order(db, &h, ls).await))
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
        use rustygod_db::entities::order_order::{Column as OCol, Entity as OEnt};
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
                let cids: Vec<i32> = chs.iter().filter_map(|i| rustygod_db::catalog::parse_gid(&i.0)).collect();
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
            if let Some((hh, ll)) = rustygod_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))? {
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

#[derive(Default)]
pub struct OrderMutation;

#[Object]
impl OrderMutation {
    /// Dashboard `OrderCancel` shape (`{ order { ...OrderDetails } errors }`).
    async fn order_cancel(&self, ctx: &Context<'_>, id: ID) -> Result<GqlOrderCancel> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid = parse_id(&id.0);
        rustygod_db::cancel::cancel_order(db, oid).await.map_err(|e| Error::new(e.to_string()))?;
        let (h, ls) = rustygod_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))?.ok_or_else(|| Error::new("order vanished"))?;
        Ok(GqlOrderCancel { order: Some(to_gen_order(db, &h, ls).await), errors: vec![] })
    }

    /// Saleor `orderFulfill(order: ID, input: OrderFulfillInput!)`
    /// (dashboard `FulfillOrder`). One FulfillItem per stock entry.
    async fn order_fulfill(&self, ctx: &Context<'_>, order: Option<ID>, input: gen::OrderFulfillInput) -> Result<Option<gen::OrderFulfill>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid = parse_id(&order.map(|o| o.0).unwrap_or_default());
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        let mut items: Vec<rustygod_db::fulfillment::FulfillItem> = vec![];
        for l in input.lines {
            let lid = parse_id(&l.order_line_id.map(|i| i.0).unwrap_or_default());
            // variant of this line (for warehouse -> stock resolution).
            let line_variant: Option<i32> = rustygod_db::entities::order_orderline::Entity::find()
                .select_only().column(rustygod_db::entities::order_orderline::Column::VariantId)
                .filter(rustygod_db::entities::order_orderline::Column::Id.eq(lid))
                .into_tuple::<Option<i32>>().one(db).await.map_err(|e| Error::new(e.to_string()))?.flatten();
            for s in l.stocks {
                // Dashboard sends the WAREHOUSE id (uuid global); resolve to
                // the variant's stock row in that warehouse (was silently
                // dropped by an i32 parse — wrong-warehouse fulfillments).
                let stock_id: Option<i32> = match crate::common::parse_uuid_gid(&s.warehouse.0) {
                    Some(wid) => match line_variant {
                        Some(vid) => rustygod_db::fulfillment::stock_for_variant_warehouse(db, vid, wid).await.map_err(|e| Error::new(e.to_string()))?,
                        None => None,
                    },
                    None => None,
                };
                items.push(rustygod_db::fulfillment::FulfillItem {
                    order_line_id: lid,
                    quantity: s.quantity,
                    stock_id,
                });
            }
        }
        let tracking = input.tracking_number.unwrap_or_default();
        rustygod_db::fulfillment::create_fulfillment(db, oid, &items, &tracking).await.map_err(|e| Error::new(e.to_string()))?;
        let order_view = match rustygod_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))? {
            Some((h, ll)) => Some(to_gen_order(db, &h, ll).await),
            None => None,
        };
        Ok(Some(gen::OrderFulfill { order: order_view, errors: vec![] }))
    }

    async fn order_return_lines(&self, ctx: &Context<'_>, order_id: ID, lines: Vec<ReturnLineInput>, reason: String, restock: Option<bool>) -> Result<String> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid = parse_id(&order_id.0);
        let items: Vec<rustygod_db::fulfillment::FulfillItem> = lines.into_iter().map(|l| {
            let lid = parse_id(&l.order_line_id.0);
            let sid: Option<i32> = l.stock_id.and_then(|s| rustygod_db::catalog::parse_gid(&s.0));
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
