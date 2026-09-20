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
    let m = to_gql_money(amount, currency.clone());
    GqlTaxedMoney { gross: m.clone(), net: m, tax: None, currency: Some(currency) }
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
/// totals/lines/channel), `None`/`[]` elsewhere (enterprise stubs).
fn to_gen_order(
    h: &rustygod_db::order_store::OrderHeader,
    ls: Vec<rustygod_db::entities::order_orderline::Model>,
) -> gen::Order {
    gen::Order {
        id: Some(ID(h.id.to_string())),
        private_metadata: vec![],
        metadata: vec![],
        created: Some(h.created_at.into()),
        updated_at: None,
        status: Some(h.status.clone()),
        user: None,
        billing_address: None,
        shipping_address: None,
        shipping_method_name: None,
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
            order_settings: None,
            checkout_settings: None,
            payment_settings: None,
            tax_configuration: None,
        }),
        fulfillments: vec![],
        lines: ls.into_iter().map(|l| gen::OrderLine {
            id: Some(ID(l.id.to_string())),
            private_metadata: vec![],
            metadata: vec![],
            product_name: None,
            variant_name: None,
            product_sku: None,
            is_shipping_required: None,
            quantity: Some(l.quantity),
            quantity_fulfilled: Some(l.quantity_fulfilled),
            tax_rate: None,
            unit_price: Some(to_taxed(l.unit_price_gross_amount, l.currency.clone())),
            undiscounted_unit_price: None,
            unit_discount: None,
            unit_discount_reason: None,
            unit_discount_value: None,
            unit_discount_type: None,
            total_price: Some(to_taxed(l.total_price_gross_amount, l.currency)),
            undiscounted_total_price: None,
            is_price_overridden: None,
            price_override_reason: None,
            variant: l.variant_id.map(minimal_variant),
            allocations: vec![],
            quantity_to_fulfill: None,
            tax_class: None,
            voucher_code: None,
            is_gift: Some(l.is_gift),
            discounts: vec![],
        }).collect(),
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
        undiscounted_total: None,
        shipping_method: None,
        shipping_price: None,
        voucher: None,
        voucher_code: None,
        gift_cards: vec![],
        customer_note: None,
        subtotal: None,
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

/// Id-only variant stub (lets Dashboard normalize line → variant links).
fn minimal_variant(id: i32) -> gen::ProductVariant {
    gen::ProductVariant {
        id: Some(ID(id.to_string())),
        private_metadata: vec![],
        metadata: vec![],
        name: None,
        sku: None,
        product: None,
        track_inventory: None,
        quantity_limit_per_customer: None,
        weight: None,
        channel_listings: vec![],
        media: vec![],
        stocks: vec![],
        quantity_available: None,
        updated_at: None,
    }
}

#[derive(Default)]
pub struct OrderQuery;

#[Object]
impl OrderQuery {
    async fn order(&self, ctx: &Context<'_>, id: ID) -> Result<Option<gen::Order>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let oid = parse_id(&id.0);
        let Some((h, ls)) = rustygod_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))? else { return Ok(None) };
        Ok(Some(to_gen_order(&h, ls)))
    }

    async fn orders(
        &self, ctx: &Context<'_>,
        first: Option<i32>, after: Option<String>, before: Option<String>, last: Option<i32>,
        #[graphql(name = "sortBy")] sort_by: Option<gen::OrderSortingInput>, filter: Option<gen::OrderFilterInput>, #[graphql(name = "where")] where_input: Option<gen::OrderWhereInput>, search: Option<String>,
    ) -> Result<GqlOrderConnection> {
        let _ = (before, last, sort_by, filter, where_input, search);
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        let off = after.and_then(|c| decode_cursor(&c)).unwrap_or(0);
        let lim = first.unwrap_or(20).clamp(1, 100) as usize;
        use sea_orm::{EntityTrait, QueryOrder, QuerySelect};
        let ids: Vec<Uuid> = rustygod_db::entities::order_order::Entity::find()
            .select_only().column(rustygod_db::entities::order_order::Column::Id)
            .order_by_desc(rustygod_db::entities::order_order::Column::CreatedAt)
            .into_tuple::<Uuid>().all(db).await.map_err(|e| Error::new(e.to_string()))?;
        let total = ids.len() as i32;
        let mut out = Vec::new();
        for oid in ids.into_iter().skip(off).take(lim) {
            if let Some((hh, ll)) = rustygod_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))? {
                out.push(to_gen_order(&hh, ll));
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
        Ok(GqlOrderCancel { order: Some(to_gen_order(&h, ls)), errors: vec![] })
    }

    /// Saleor `orderFulfill(order: ID, input: OrderFulfillInput!)`
    /// (dashboard `FulfillOrder`). One FulfillItem per stock entry.
    async fn order_fulfill(&self, ctx: &Context<'_>, order: Option<ID>, input: gen::OrderFulfillInput) -> Result<Option<gen::OrderFulfill>> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid = parse_id(&order.map(|o| o.0).unwrap_or_default());
        let mut items: Vec<rustygod_db::fulfillment::FulfillItem> = vec![];
        for l in input.lines {
            let lid = parse_id(&l.order_line_id.map(|i| i.0).unwrap_or_default());
            for s in l.stocks {
                items.push(rustygod_db::fulfillment::FulfillItem {
                    order_line_id: lid,
                    quantity: s.quantity,
                    stock_id: s.warehouse.0.parse::<i32>().ok(),
                });
            }
        }
        let tracking = input.tracking_number.unwrap_or_default();
        rustygod_db::fulfillment::create_fulfillment(db, oid, &items, &tracking).await.map_err(|e| Error::new(e.to_string()))?;
        let order_view = rustygod_db::order_store::get_order_rows(db, oid).await.map_err(|e| Error::new(e.to_string()))?.map(|(h, ll)| to_gen_order(&h, ll));
        Ok(Some(gen::OrderFulfill { order: order_view, errors: vec![] }))
    }

    async fn order_return_lines(&self, ctx: &Context<'_>, order_id: ID, lines: Vec<ReturnLineInput>, reason: String, restock: Option<bool>) -> Result<String> {
        let g = ctx.data::<GqlContext>()?; let db = g.db()?;
        authorize(ctx, crate::context::MANAGE_ORDERS).await?;
        let oid = parse_id(&order_id.0);
        let items: Vec<rustygod_db::fulfillment::FulfillItem> = lines.into_iter().map(|l| {
            let lid = parse_id(&l.order_line_id.0);
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
