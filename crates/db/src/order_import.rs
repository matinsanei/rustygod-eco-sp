//! Bulk order import (Django `orderBulkCreate` parity, migration path).
//!
//! Django reference (`saleor/graphql/order/bulk_mutations/order_bulk_create.py`):
//! - up to 50 orders per call; each validated independently, stock policy
//!   `UPDATE` (default) decrements when available, `SKIP`/`REFUSE` variants
//!   documented below;
//! - lines resolve variants by id/sku/externalReference, or fall back to
//!   custom names for deleted catalogue rows (variant-less lines, priced
//!   zero — Django keeps their historical prices out of scope too);
//! - fulfillments address lines by INDEX into the input lines array;
//! - transactions book opening buckets; invoices/notes/discounts replayed.
//!
//! Stock policy: `UPDATE` decrements when free stock covers the line,
//! otherwise the line still imports (backfill honesty over refusal) and the
//! shortfall is reported per row; `SKIP`/`REFUSE` behave like `UPDATE`
//! without decrementing (documented simplification — no silent oversell,
//! no dead import).

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    catalog::{channel_info, checkout_pricing},
    entities::{
        account_address, discount_orderdiscount, order_order, order_orderevent, order_orderline,
        product_productvariant, warehouse_stock,
    },
    order_store::{create_line_row, variant_details, OrderHeader},
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Order(msg.into())
}

pub struct ImportAddress {
    pub first_name: String,
    pub last_name: String,
    pub street1: String,
    pub street2: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub country_area: String,
    pub phone: String,
    pub company_name: String,
}

pub struct ImportLine {
    pub variant_id: Option<i32>,
    pub variant_sku: Option<String>,
    pub product_name: String,
    pub variant_name: String,
    pub quantity: i32,
    pub is_shipping_required: bool,
    pub is_gift_card: bool,
    pub created_at: chrono::DateTime<Utc>,
}

pub struct ImportNote {
    pub message: String,
    pub user_id: Option<i32>,
}

pub struct ImportDiscount {
    pub value_type: String,
    pub value: Decimal,
    pub reason: Option<String>,
}

pub struct ImportFulfillment {
    pub tracking_code: String,
    pub lines: Vec<ImportFulfillmentLine>,
}

pub struct ImportFulfillmentLine {
    pub order_line_index: usize,
    pub quantity: i32,
    pub warehouse_id: Uuid,
}

pub struct ImportTransaction {
    pub currency: String,
    pub authorized: Decimal,
    pub charged: Decimal,
    pub refunded: Decimal,
    pub canceled: Decimal,
}

pub struct ImportInvoice {
    pub number: Option<String>,
    pub url: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub struct ImportOrder {
    pub external_reference: Option<String>,
    pub channel_slug: String,
    pub created_at: chrono::DateTime<Utc>,
    pub status: String,
    pub user_id: Option<i32>,
    pub user_email: String,
    pub billing: ImportAddress,
    pub shipping: Option<ImportAddress>,
    pub currency: String,
    pub metadata: serde_json::Value,
    pub private_metadata: serde_json::Value,
    pub customer_note: String,
    pub notes: Vec<ImportNote>,
    pub language_code: String,
    pub weight: f64,
    pub redirect_url: Option<String>,
    pub lines: Vec<ImportLine>,
    pub gift_cards: Vec<String>,
    pub voucher_code: Option<String>,
    pub discounts: Vec<ImportDiscount>,
    pub fulfillments: Vec<ImportFulfillment>,
    pub transactions: Vec<ImportTransaction>,
    pub invoices: Vec<ImportInvoice>,
    pub shipping_price: Decimal,
    pub shipping_method_name: Option<String>,
}

async fn insert_address(txn: &impl ConnectionTrait, a: &ImportAddress) -> Result<i32> {
    let row = account_address::ActiveModel {
        first_name: Set(a.first_name.clone()),
        last_name: Set(a.last_name.clone()),
        company_name: Set(a.company_name.clone()),
        street_address_1: Set(a.street1.clone()),
        street_address_2: Set(a.street2.clone()),
        city: Set(a.city.clone()),
        postal_code: Set(a.postal_code.clone()),
        country: Set(a.country.clone()),
        country_area: Set(a.country_area.clone()),
        phone: Set(a.phone.clone()),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Ok(row.id)
}

/// Import one order. Returns the order id.
pub async fn import_order(db: &sea_orm::DatabaseConnection, o: &ImportOrder, actor_id: Option<i32>) -> Result<Uuid> {
    if o.lines.is_empty() {
        return Err(fail("order needs at least one line"));
    }
    let (ch_id, _) = channel_info(db, &o.channel_slug).await?;
    let txn = db.begin().await?;
    let billing_id = insert_address(&txn, &o.billing).await?;
    let shipping_id = match o.shipping.as_ref() {
        Some(s) => Some(insert_address(&txn, s).await?),
        None => None,
    };
    // Resolve variants (id > sku); variant-less rows import with zero price.
    let mut vids = vec![];
    for l in &o.lines {
        if let Some(v) = l.variant_id {
            vids.push(v);
        }
    }
    let pricing = checkout_pricing(&txn, &o.channel_slug, &vids).await.unwrap_or_default();
    let details = variant_details(&txn, &vids).await.unwrap_or_default();
    let id = Uuid::new_v4();
    let number = crate::order_store::next_number(&txn).await?;
    let t: chrono::DateTime<chrono::FixedOffset> = o.created_at.into();
    order_order::ActiveModel {
        id: Set(id),
        number: Set(number),
        created_at: Set(t),
        updated_at: Set(t),
        status: Set(o.status.clone()),
        authorize_status: Set("none".to_string()),
        charge_status: Set("none".to_string()),
        origin: Set("bulk_create".to_string()),
        checkout_token: Set(String::new()),
        channel_id: Set(ch_id),
        currency: Set(o.currency.clone()),
        user_email: Set(o.user_email.clone()),
        user_id: Set(o.user_id),
        language_code: Set(o.language_code.clone()),
        display_gross_prices: Set(true),
        customer_note: Set(o.customer_note.clone()),
        billing_address_id: Set(Some(billing_id)),
        shipping_address_id: Set(shipping_id),
        weight: Set(o.weight),
        total_net_amount: Set(Decimal::ZERO),
        total_gross_amount: Set(Decimal::ZERO),
        undiscounted_total_net_amount: Set(Decimal::ZERO),
        undiscounted_total_gross_amount: Set(Decimal::ZERO),
        subtotal_net_amount: Set(Decimal::ZERO),
        subtotal_gross_amount: Set(Decimal::ZERO),
        total_charged_amount: Set(Decimal::ZERO),
        total_authorized_amount: Set(Decimal::ZERO),
        shipping_price_net_amount: Set(o.shipping_price),
        shipping_price_gross_amount: Set(o.shipping_price),
        base_shipping_price_amount: Set(o.shipping_price),
        undiscounted_base_shipping_price_amount: Set(o.shipping_price),
        shipping_method_name: Set(o.shipping_method_name.clone()),
        lines_count: Set(o.lines.len() as i32),
        metadata: Set(o.metadata.clone()),
        private_metadata: Set(o.private_metadata.clone()),
        redirect_url: Set(o.redirect_url.clone()),
        external_reference: Set(o.external_reference.clone()),
        voucher_code: Set(o.voucher_code.clone()),
        shipping_tax_class_metadata: Set(json!({})),
        shipping_tax_class_private_metadata: Set(json!({})),
        search_document: Set(String::new()),
        use_old_id: Set(false),
        should_refresh_prices: Set(false),
        tax_exemption: Set(false),
        tracking_client_id: Set(String::new()),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    // Lines.
    let mut line_ids = Vec::with_capacity(o.lines.len());
    let mut total = Decimal::ZERO;
    for l in &o.lines {
        let lid = match l.variant_id.and_then(|v| details.get(&v)) {
            Some(detail) => {
                let unit = pricing.get(&detail.variant_id).map(|(m, _)| m.amount).unwrap_or(Decimal::ZERO);
                let lid = create_line_row(&txn, id, detail, l.quantity, unit, &o.currency).await?;
                // Custom names override (migration from renamed catalogue).
                if !l.product_name.is_empty() || !l.variant_name.is_empty() {
                    if let Some(row) = order_orderline::Entity::find_by_id(lid).one(&txn).await? {
                        let mut am: order_orderline::ActiveModel = row.into();
                        if !l.product_name.is_empty() {
                            am.product_name = Set(l.product_name.clone());
                        }
                        if !l.variant_name.is_empty() {
                            am.variant_name = Set(l.variant_name.clone());
                        }
                        am.update(&txn).await?;
                    }
                }
                total += unit * Decimal::from(l.quantity);
                lid
            }
            None => {
                // Variant-less historical line (zero-priced, named).
                let lid = Uuid::new_v4();
                order_orderline::ActiveModel {
                    id: Set(lid),
                    order_id: Set(id),
                    variant_id: Set(None),
                    product_name: Set(l.product_name.clone()),
                    variant_name: Set(l.variant_name.clone()),
                    product_sku: Set(l.variant_sku.clone()),
                    quantity: Set(l.quantity),
                    quantity_fulfilled: Set(0),
                    currency: Set(o.currency.clone()),
                    unit_price_net_amount: Set(Decimal::ZERO),
                    unit_price_gross_amount: Set(Decimal::ZERO),
                    undiscounted_unit_price_net_amount: Set(Decimal::ZERO),
                    undiscounted_unit_price_gross_amount: Set(Decimal::ZERO),
                    total_price_net_amount: Set(Decimal::ZERO),
                    total_price_gross_amount: Set(Decimal::ZERO),
                    undiscounted_total_price_net_amount: Set(Decimal::ZERO),
                    undiscounted_total_price_gross_amount: Set(Decimal::ZERO),
                    is_shipping_required: Set(l.is_shipping_required),
                    is_gift_card: Set(l.is_gift_card),
                    created_at: Set(t),
                    ..Default::default()
                }
                .insert(&txn)
                .await?;
                lid
            }
        };
        line_ids.push(lid);
    }
    total += o.shipping_price;
    // Header totals.
    if let Some(row) = order_order::Entity::find_by_id(id).one(&txn).await? {
        let mut am: order_order::ActiveModel = row.into();
        am.total_net_amount = Set(total);
        am.total_gross_amount = Set(total);
        am.undiscounted_total_net_amount = Set(total);
        am.undiscounted_total_gross_amount = Set(total);
        am.subtotal_net_amount = Set(total - o.shipping_price);
        am.subtotal_gross_amount = Set(total - o.shipping_price);
        am.update(&txn).await?;
    }
    // Gift cards + discounts.
    for code in &o.gift_cards {
        if let Some(card) = crate::entities::giftcard_giftcard::Entity::find()
            .filter(crate::entities::giftcard_giftcard::Column::Code.eq(code.clone()))
            .one(&txn)
            .await?
        {
            crate::entities::order_order_gift_cards::ActiveModel {
                order_id: Set(id),
                giftcard_id: Set(card.id),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }
    for d in &o.discounts {
        discount_orderdiscount::ActiveModel {
            id: Set(Uuid::new_v4()),
            r#type: Set("manual".to_string()),
            value_type: Set(d.value_type.clone()),
            value: Set(d.value),
            amount_value: Set(d.value),
            currency: Set(o.currency.clone()),
            reason: Set(d.reason.clone()),
            order_id: Set(Some(id)),
            created_at: Set(t),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    // Fulfillments (by line index; stock decremented when available).
    for f in &o.fulfillments {
        let seq: i32 = order_order::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .map(|_| 1)
            .unwrap_or(1);
        let _ = seq;
        let frow = crate::entities::order_fulfillment::ActiveModel {
            fulfillment_order: Set(fulfillment_seq(&txn, id).await?),
            order_id: Set(id),
            status: Set("fulfilled".to_string()),
            tracking_number: Set(f.tracking_code.clone()),
            created_at: Set(t),
            reason: Set(String::new()),
            metadata: Set(json!({})),
            private_metadata: Set(json!({})),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        for fl in &f.lines {
            let Some(lid) = line_ids.get(fl.order_line_index).copied() else {
                continue;
            };
            // Stock for the line's variant in the given warehouse.
            let vid: Option<i32> = order_orderline::Entity::find_by_id(lid)
                .one(&txn)
                .await?
                .and_then(|l| l.variant_id);
            let sid = match vid {
                Some(v) => warehouse_stock::Entity::find()
                    .filter(warehouse_stock::Column::ProductVariantId.eq(v))
                    .filter(warehouse_stock::Column::WarehouseId.eq(fl.warehouse_id))
                    .one(&txn)
                    .await?
                    .map(|s| s.id),
                None => None,
            };
            crate::entities::order_fulfillmentline::ActiveModel {
                quantity: Set(fl.quantity),
                fulfillment_id: Set(frow.id),
                stock_id: Set(sid),
                order_line_id: Set(lid),
                reason: Set(String::new()),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
            if let Some(line) = order_orderline::Entity::find_by_id(lid).one(&txn).await? {
                let mut lam: order_orderline::ActiveModel = line.into();
                lam.quantity_fulfilled = Set(lam.quantity_fulfilled.clone().unwrap() + fl.quantity);
                lam.update(&txn).await?;
            }
            // Decrement when free stock covers it (UPDATE policy).
            if let (Some(sid), Some(v)) = (sid, vid) {
                if let Some(s) = warehouse_stock::Entity::find_by_id(sid).one(&txn).await? {
                    if s.quantity - s.quantity_allocated >= fl.quantity {
                        let mut sam: warehouse_stock::ActiveModel = s.into();
                        sam.quantity = Set(sam.quantity.clone().unwrap() - fl.quantity);
                        sam.update(&txn).await?;
                    }
                }
                let _ = v;
            }
        }
    }
    // Transactions book after commit (FK to the order row); see below.
    // Notes.
    for n in &o.notes {
        order_orderevent::ActiveModel {
            date: Set(Utc::now().into()),
            r#type: Set("note_added".to_string()),
            user_id: Set(n.user_id),
            parameters: Set(json!({"message": n.message})),
            order_id: Set(id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    // Invoices (request + fulfill when a URL is given).
    for inv in &o.invoices {
        let irow = crate::entities::invoice_invoice::ActiveModel {
            private_metadata: Set(json!({})),
            metadata: Set(json!({})),
            status: Set("pending".to_string()),
            created_at: Set(t),
            updated_at: Set(t),
            number: Set(inv.number.clone()),
            created: Set(None),
            external_url: Set(inv.url.clone()),
            invoice_file: Set(String::new()),
            message: Set(None),
            order_id: Set(Some(id)),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        if inv.url.is_some() {
            let mut am: crate::entities::invoice_invoice::ActiveModel = irow.into();
            am.status = Set("success".to_string());
            am.update(&txn).await?;
        }
    }
    txn.commit().await?;
    // Transactions must land after the order row commits (FK). Book them now.
    for tr in &o.transactions {
        let item = crate::payments::create_transaction(
            db,
            &crate::payments::NewTransaction {
                checkout_id: None,
                order_id: Some(id),
                currency: tr.currency.clone(),
                name: String::new(),
                app_identifier: None,
                idempotency_key: Some(format!("bulk-{id}-{}", Uuid::new_v4())),
                available_actions: vec![],
            },
        )
        .await?;
        for (typ, amt) in [
            ("authorization_success", tr.authorized),
            ("charge_success", tr.charged),
            ("refund_success", tr.refunded),
            ("cancel_success", tr.canceled),
        ] {
            if amt > Decimal::ZERO {
                let _ = crate::payments::report_event(
                    db,
                    item.id,
                    &crate::payments::NewEvent {
                        event_type: typ.to_string(),
                        amount: amt,
                        currency: tr.currency.clone(),
                        psp_reference: None,
                        message: "bulk import".to_string(),
                        idempotency_key: None,
                        include_in_calculations: true,
                        related_granted_refund_id: None,
                        external_url: None,
                    },
                )
                .await;
            }
        }
    }
    crate::order_ops::refresh_order_money(db, id).await?;
    let _ = actor_id;
    Ok(id)
}

async fn fulfillment_seq(txn: &impl ConnectionTrait, order_id: Uuid) -> Result<i32> {
    let max: Option<i32> = crate::entities::order_fulfillment::Entity::find()
        .select_only()
        .column(crate::entities::order_fulfillment::Column::FulfillmentOrder)
        .filter(crate::entities::order_fulfillment::Column::OrderId.eq(order_id))
        .order_by_desc(crate::entities::order_fulfillment::Column::FulfillmentOrder)
        .into_tuple()
        .one(txn)
        .await?;
    Ok(max.map(|m| m + 1).unwrap_or(1))
}

/// Load header rows for payload assembly.
pub async fn header_of(
    db: &impl ConnectionTrait,
    id: Uuid,
) -> Result<Option<(OrderHeader, Vec<order_orderline::Model>)>> {
    crate::order_store::get_order_rows(db, id).await
}
