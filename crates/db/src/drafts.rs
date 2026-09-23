//! Draft orders on Django's `order_order` / `order_orderline` tables.
//!
//! Mirrors `saleor/graphql/order/mutations/draft_order_{create,update,
//! complete,delete}.py` (v1 subset, staff flows):
//! - create: `status = "draft"`, `origin = "draft"`, number from the same
//!   `order_order_number_seq` Django uses; lines priced from the channel
//!   listing unless a custom staff price is given;
//! - edit (add / set-quantity / remove) only on drafts — anything else is
//!   "The order is not draft.";
//! - complete: draft + ≥1 line + sufficient free stock, then allocate
//!   (`warehouse_allocation` rows + `quantity_allocated` bumps, Django's
//!   `allocate_stocks` semantics for `track_inventory` variants) and flip to
//!   `unfulfilled` / `unconfirmed` by the channel flag;
//! - events `draft_created` / `added_products` / `removed_products` /
//!   `placed_from_draft` are written to `order_orderevent`.

use chrono::Utc;
use rust_decimal::Decimal;
use saleor_rustify_core::draft as domain;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    catalog::{channel_info, checkout_pricing},
    entities::{
        channel_channel, order_order, order_orderevent, order_orderline,
        product_productvariant, warehouse_allocation, warehouse_stock,
    },
    order_store::{create_line_row, get_order_rows, next_number, variant_details, OrderHeader},
    DbError, Result,
};

/// Event types — mirrors `OrderEvents` in `saleor/order/__init__.py`.
pub mod events {
    pub const DRAFT_CREATED: &str = "draft_created";
    pub const ADDED_PRODUCTS: &str = "added_products";
    pub const REMOVED_PRODUCTS: &str = "removed_products";
    pub const PLACED_FROM_DRAFT: &str = "placed_from_draft";
}

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Draft(msg.into())
}

pub struct DraftLineInput {
    pub variant_id: i32,    pub quantity: i32,
    pub custom_price: Option<Decimal>,
    pub force_new_line: bool,
}

#[derive(Debug)]
pub struct DraftOrderView {
    pub id: Uuid,    pub number: i32,
    pub status: String,
    pub currency: String,
    pub total_gross: Decimal,
    pub lines_count: i32,
}

async fn write_event(
    txn: &impl ConnectionTrait,
    order_id: Uuid,
    event_type: &str,
    parameters: serde_json::Value,
    user_id: Option<i32>,
) -> Result<()> {
    order_orderevent::ActiveModel {
        date: Set(Utc::now().into()),
        r#type: Set(event_type.to_string()),
        user_id: Set(user_id),
        parameters: Set(parameters),
        order_id: Set(order_id),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Ok(())
}

async fn draft_row(
    txn: &impl ConnectionTrait,
    order_id: Uuid,
) -> Result<order_order::Model> {
    let row = order_order::Entity::find_by_id(order_id)
        .one(txn)
        .await?
        .ok_or_else(|| fail(format!("order {order_id} not found")))?;
    domain::require_draft(&row.status).map_err(|e| fail(e.to_string()))?;
    Ok(row)
}

/// Recompute header totals from lines (pre-tax: net == gross).
async fn recalc_totals(txn: &impl ConnectionTrait, order_id: Uuid) -> Result<Decimal> {
    let lines = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .all(txn)
        .await?;
    let total: Decimal = lines.iter().map(|l| l.total_price_gross_amount).sum();
    let count = lines.len() as i32;
    let order = order_order::Entity::find_by_id(order_id)
        .one(txn)
        .await?
        .ok_or_else(|| fail(format!("order {order_id} not found")))?;
    let mut am: order_order::ActiveModel = order.into();
    am.total_net_amount = Set(total);
    am.total_gross_amount = Set(total);
    am.undiscounted_total_net_amount = Set(total);
    am.undiscounted_total_gross_amount = Set(total);
    am.subtotal_net_amount = Set(total);
    am.subtotal_gross_amount = Set(total);
    am.lines_count = Set(count);
    am.updated_at = Set(Utc::now().into());
    am.update(txn).await?;
    Ok(total)
}

/// Insert one draft line, resolving the unit price (custom staff price wins,
/// otherwise the channel listing — Django's `OrderLineData` price logic).
async fn insert_line(
    txn: &impl ConnectionTrait,
    order_id: Uuid,
    currency: &str,
    input: &DraftLineInput,
    pricing: &std::collections::HashMap<i32, (saleor_rustify_core::money::Money, String)>,
    details: &std::collections::HashMap<i32, crate::order_store::VariantDetail>,
) -> Result<Uuid> {
    if input.quantity < 1 {
        return Err(fail("line quantity must be positive"));
    }
    let detail = details
        .get(&input.variant_id)
        .ok_or_else(|| fail(format!("variant {} not found", input.variant_id)))?;
    let (listed, _) = pricing
        .get(&input.variant_id)
        .ok_or_else(|| fail(format!("variant {} is not listed in this channel", input.variant_id)))?;
    let unit = input.custom_price.unwrap_or(listed.amount);
    if unit < Decimal::ZERO {
        return Err(fail("line price cannot be negative"));
    }
    create_line_row(txn, order_id, detail, input.quantity, unit, currency).await
}

/// Create a draft order with lines. Returns the view.
pub async fn create_draft(
    db: &DatabaseConnection,
    channel_slug: &str,
    email: &str,
    user_id: Option<i32>,
    lines: Vec<DraftLineInput>,
    actor_id: Option<i32>,
) -> Result<DraftOrderView> {
    if lines.is_empty() {
        return Err(fail("draft order needs at least one line"));
    }
    let (ch_id, currency) = channel_info(db, channel_slug).await?;
    let vids: Vec<i32> = lines.iter().map(|l| l.variant_id).collect();
    let pricing = checkout_pricing(db, channel_slug, &vids).await?;
    let details = variant_details(db, &vids).await?;

    let txn = db.begin().await?;
    let id = Uuid::new_v4();
    let number = next_number(&txn).await?;
    let t = Utc::now();
    order_order::ActiveModel {
        id: Set(id),
        number: Set(number),
        created_at: Set(t.into()),
        updated_at: Set(t.into()),
        status: Set("draft".to_string()),
        authorize_status: Set("none".to_string()),
        charge_status: Set("none".to_string()),
        origin: Set("draft".to_string()),
        checkout_token: Set(String::new()),
        channel_id: Set(ch_id),
        currency: Set(currency.clone()),
        user_email: Set(email.to_string()),
        user_id: Set(user_id),
        language_code: Set("en".to_string()),
        display_gross_prices: Set(true),
        customer_note: Set(String::new()),
        weight: Set(0.0),
        total_net_amount: Set(Decimal::ZERO),
        total_gross_amount: Set(Decimal::ZERO),
        undiscounted_total_net_amount: Set(Decimal::ZERO),
        undiscounted_total_gross_amount: Set(Decimal::ZERO),
        subtotal_net_amount: Set(Decimal::ZERO),
        subtotal_gross_amount: Set(Decimal::ZERO),
        total_charged_amount: Set(Decimal::ZERO),
        total_authorized_amount: Set(Decimal::ZERO),
        shipping_price_net_amount: Set(Decimal::ZERO),
        shipping_price_gross_amount: Set(Decimal::ZERO),
        base_shipping_price_amount: Set(Decimal::ZERO),
        undiscounted_base_shipping_price_amount: Set(Decimal::ZERO),
        lines_count: Set(0),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        shipping_tax_class_metadata: Set(json!({})),
        shipping_tax_class_private_metadata: Set(json!({})),
        search_document: Set(String::new()),
        use_old_id: Set(false),
        should_refresh_prices: Set(true),
        tax_exemption: Set(false),
        tracking_client_id: Set(String::new()),
        ..Default::default()
    }
    .insert(&txn)
    .await?;

    for line in &lines {
        // Draft create merges same-variant lines (no force flag on create).
        insert_line(&txn, id, &currency, line, &pricing, &details).await?;
    }
    let total = recalc_totals(&txn, id).await?;
    write_event(&txn, id, events::DRAFT_CREATED, json!({}), actor_id).await?;
    txn.commit().await?;
    Ok(DraftOrderView {
        id,
        number,
        status: "draft".to_string(),
        currency,
        total_gross: total,
        lines_count: lines.len() as i32,
    })
}

/// Add lines to a draft (merges same-variant lines unless `force_new_line`).
pub async fn add_lines(
    db: &DatabaseConnection,
    order_id: Uuid,
    channel_slug: &str,
    lines: Vec<DraftLineInput>,
    actor_id: Option<i32>,
) -> Result<DraftOrderView> {
    if lines.is_empty() {
        return Err(fail("no lines to add"));
    }
    let (ch_id, _) = channel_info(db, channel_slug).await?;
    let vids: Vec<i32> = lines.iter().map(|l| l.variant_id).collect();
    let pricing = checkout_pricing(db, channel_slug, &vids).await?;
    let details = variant_details(db, &vids).await?;

    let txn = db.begin().await?;
    let row = draft_row(&txn, order_id).await?;
    if row.channel_id != ch_id {
        return Err(fail("order belongs to a different channel"));
    }
    for input in &lines {
        if input.quantity < 1 {
            return Err(fail("line quantity must be positive"));
        }
        let mut merged = false;
        if !input.force_new_line && input.custom_price.is_none() {
            // Merge into the first same-variant line (Django's default matching).
            let existing = order_orderline::Entity::find()
                .filter(order_orderline::Column::OrderId.eq(order_id))
                .filter(order_orderline::Column::VariantId.eq(input.variant_id))
                .order_by_asc(order_orderline::Column::CreatedAt)
                .one(&txn)
                .await?;
            if let Some(line) = existing {
                let qty = line.quantity + input.quantity;
                let unit = line.unit_price_gross_amount;
                let mut lam: order_orderline::ActiveModel = line.into();
                lam.quantity = Set(qty);
                lam.total_price_net_amount = Set(unit * Decimal::from(qty));
                lam.total_price_gross_amount = Set(unit * Decimal::from(qty));
                lam.undiscounted_total_price_net_amount = Set(unit * Decimal::from(qty));
                lam.undiscounted_total_price_gross_amount = Set(unit * Decimal::from(qty));
                lam.update(&txn).await?;
                merged = true;
            }
        }
        if !merged {
            insert_line(&txn, order_id, &row.currency, input, &pricing, &details).await?;
        }
    }
    let total = recalc_totals(&txn, order_id).await?;
    write_event(
        &txn,
        order_id,
        events::ADDED_PRODUCTS,
        json!({"lines_added": lines.len()}),
        actor_id,
    )
    .await?;
    txn.commit().await?;
    view_of(db, order_id, total).await
}

/// Set a draft line's quantity.
pub async fn set_line_quantity(
    db: &DatabaseConnection,
    order_id: Uuid,
    line_id: Uuid,
    quantity: i32,
) -> Result<DraftOrderView> {
    if quantity < 1 {
        return Err(fail("line quantity must be positive"));
    }
    let txn = db.begin().await?;
    draft_row(&txn, order_id).await?;
    let line = order_orderline::Entity::find_by_id(line_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("order line not found"))?;
    if line.order_id != order_id {
        return Err(fail("order line does not belong to this order"));
    }
    let unit = line.unit_price_gross_amount;
    let mut lam: order_orderline::ActiveModel = line.into();
    lam.quantity = Set(quantity);
    lam.total_price_net_amount = Set(unit * Decimal::from(quantity));
    lam.total_price_gross_amount = Set(unit * Decimal::from(quantity));
    lam.undiscounted_total_price_net_amount = Set(unit * Decimal::from(quantity));
    lam.undiscounted_total_price_gross_amount = Set(unit * Decimal::from(quantity));
    lam.update(&txn).await?;
    let total = recalc_totals(&txn, order_id).await?;
    txn.commit().await?;
    view_of(db, order_id, total).await
}

/// Remove a line from a draft.
pub async fn remove_line(
    db: &DatabaseConnection,
    order_id: Uuid,
    line_id: Uuid,
    actor_id: Option<i32>,
) -> Result<DraftOrderView> {
    let txn = db.begin().await?;
    draft_row(&txn, order_id).await?;
    let line = order_orderline::Entity::find_by_id(line_id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail("order line not found"))?;
    if line.order_id != order_id {
        return Err(fail("order line does not belong to this order"));
    }
    order_orderline::Entity::delete_by_id(line_id).exec(&txn).await?;
    let total = recalc_totals(&txn, order_id).await?;
    write_event(&txn, order_id, events::REMOVED_PRODUCTS, json!({}), actor_id).await?;
    txn.commit().await?;
    view_of(db, order_id, total).await
}

async fn view_of(db: &DatabaseConnection, order_id: Uuid, total: Decimal) -> Result<DraftOrderView> {
    let (header, _) = get_order_rows(db, order_id)
        .await?
        .ok_or_else(|| fail(format!("order {order_id} not found")))?;
    Ok(DraftOrderView {
        id: header.id,
        number: header.number,
        status: header.status,
        currency: header.currency,
        total_gross: total,
        lines_count: header_lines_count(db, order_id).await?,
    })
}

async fn header_lines_count(db: &DatabaseConnection, order_id: Uuid) -> Result<i32> {
    Ok(order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .all(db)
        .await?
        .len() as i32)
}

/// Allocate stock for one line across warehouses, most-free first.
/// Mirrors `allocate_stocks`: writes `warehouse_allocation` rows and bumps
/// `quantity_allocated`; raises when free stock falls short.
async fn allocate_line(
    txn: &impl ConnectionTrait,
    line: &order_orderline::Model,
    track_inventory: bool,
) -> Result<()> {
    if !track_inventory {
        return Ok(());
    }
    let vid = match line.variant_id {
        Some(v) => v,
        None => return Ok(()), // lines without variants carry no stock
    };
    let mut stocks = warehouse_stock::Entity::find()
        .filter(warehouse_stock::Column::ProductVariantId.eq(vid))
        .all(txn)
        .await?;
    // Most-free first, deterministic by stock id on ties.
    stocks.sort_by(|a, b| {
        (b.quantity - b.quantity_allocated)
            .cmp(&(a.quantity - a.quantity_allocated))
            .then(a.id.cmp(&b.id))
    });
    let mut need = line.quantity;
    for s in &stocks {
        if need <= 0 {
            break;
        }
        let free = s.quantity - s.quantity_allocated;
        if free <= 0 {
            continue;
        }
        let take = free.min(need);
        warehouse_allocation::ActiveModel {
            quantity_allocated: Set(take),
            stock_id: Set(s.id),
            order_line_id: Set(line.id),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        let mut sam: warehouse_stock::ActiveModel = s.clone().into();
        sam.quantity_allocated = Set(s.quantity_allocated + take);
        sam.update(txn).await?;
        need -= take;
    }
    if need > 0 {
        return Err(DbError::Draft(format!(
            "insufficient stock for variant {vid}: short by {need}"
        )));
    }
    Ok(())
}

/// Complete a draft: validate, allocate, flip status, clear draft price
/// expiry. Mirrors `DraftOrderComplete.perform_mutation` (v1: no shipping
/// address handling, no webhooks — the fulfillment/shipping milestones own
/// those).
pub async fn complete_draft(
    db: &DatabaseConnection,
    order_id: Uuid,
    actor_id: Option<i32>,
) -> Result<OrderHeader> {
    let txn = db.begin().await?;
    let row = draft_row(&txn, order_id).await?;
    let lines = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .all(&txn)
        .await?;
    if lines.is_empty() {
        return Err(fail("cannot complete a draft order with no lines"));
    }
    let vids: Vec<i32> = lines.iter().filter_map(|l| l.variant_id).collect();
    let track: std::collections::HashMap<i32, bool> = if vids.is_empty() {
        Default::default()
    } else {
        product_productvariant::Entity::find()
            .filter(product_productvariant::Column::Id.is_in(vids))
            .select_only()
            .column(product_productvariant::Column::Id)
            .column(product_productvariant::Column::TrackInventory)
            .into_tuple::<(i32, bool)>()
            .all(&txn)
            .await?
            .into_iter()
            .collect()
    };
    for line in &lines {
        let tracked = line.variant_id.map(|v| track.get(&v).copied().unwrap_or(true)).unwrap_or(false);
        allocate_line(&txn, line, tracked).await?;
        // Clear draft base-price expiry (Django does this per line on complete).
        let mut lam: order_orderline::ActiveModel = line.clone().into();
        lam.draft_base_price_expire_at = Set(None);
        lam.update(&txn).await?;
    }
    let auto_confirm: Option<bool> = channel_channel::Entity::find_by_id(row.channel_id)
        .select_only()
        .column(channel_channel::Column::AutomaticallyConfirmAllNewOrders)
        .into_tuple::<Option<bool>>()
        .one(&txn)
        .await?
        .flatten();
    let status = domain::complete_status(auto_confirm.unwrap_or(true));
    let mut am: order_order::ActiveModel = row.into();
    am.status = Set(status.to_string());
    am.updated_at = Set(Utc::now().into());
    am.update(&txn).await?;
    write_event(&txn, order_id, events::PLACED_FROM_DRAFT, json!({}), actor_id).await?;
    txn.commit().await?;
    Ok(get_order_rows(db, order_id)
        .await?
        .map(|(h, _)| h)
        .ok_or_else(|| fail(format!("order {order_id} not found")))?)
}

/// Delete a draft order with its lines, allocations and events.
/// Anything non-draft is rejected — completed orders are never deletable.
pub async fn delete_draft(db: &DatabaseConnection, order_id: Uuid) -> Result<()> {
    let txn = db.begin().await?;
    draft_row(&txn, order_id).await?;
    let lines = order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .all(&txn)
        .await?;
    for line in &lines {
        // Roll back allocations (Django's delete path releases them).
        let allocs = warehouse_allocation::Entity::find()
            .filter(warehouse_allocation::Column::OrderLineId.eq(line.id))
            .all(&txn)
            .await?;
        for a in &allocs {
            if let Some(s) = warehouse_stock::Entity::find_by_id(a.stock_id).one(&txn).await? {
                let mut sam: warehouse_stock::ActiveModel = s.clone().into();
                sam.quantity_allocated =
                    Set((s.quantity_allocated - a.quantity_allocated).max(0));
                sam.update(&txn).await?;
            }
            warehouse_allocation::Entity::delete_by_id(a.id).exec(&txn).await?;
        }
        order_orderline::Entity::delete_by_id(line.id).exec(&txn).await?;
    }
    order_orderevent::Entity::delete_many()
        .filter(order_orderevent::Column::OrderId.eq(order_id))
        .exec(&txn)
        .await?;
    order_order::Entity::delete_by_id(order_id).exec(&txn).await?;
    txn.commit().await?;
    Ok(())
}

/// Full draft-create options (Django `DraftOrderCreateInput` parity).
/// Voucher codes are validated (active, dated, channel-listed) and stored;
/// percentage/fixed math runs through `orderDiscount*` like Django's
/// manual-discount path. Metadata lands on the order row.
#[derive(Default)]
pub struct DraftCreateOptions {
    pub user_id: Option<i32>,
    pub billing: Option<crate::order_ops::OrderAddressInput>,
    pub shipping: Option<crate::order_ops::OrderAddressInput>,
    pub save_billing: bool,
    pub save_shipping: bool,
    pub customer_note: Option<String>,
    pub shipping_method_id: Option<i32>,
    pub voucher_code: Option<String>,
    pub redirect_url: Option<String>,
    pub external_reference: Option<String>,
    pub metadata: serde_json::Value,
    pub private_metadata: serde_json::Value,
    pub language_code: Option<String>,
}

/// Validate a voucher code for a channel (Django `clean_voucher_code` core:
/// active, dated, usage, channel listing). Returns the voucher id.
pub async fn validate_voucher_code(
    db: &impl sea_orm::ConnectionTrait,
    code: &str,
    channel_id: i32,
) -> Result<i32> {
    use crate::entities::{discount_voucher, discount_vouchercode, discount_voucherchannellisting};
    let code_row = discount_vouchercode::Entity::find()
        .filter(discount_vouchercode::Column::Code.eq(code))
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("voucher code {code} not found")))?;
    if !code_row.is_active {
        return Err(fail("voucher code inactive"));
    }
    let v = discount_voucher::Entity::find_by_id(code_row.voucher_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("voucher code {code} not found")))?;
    let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
    if v.start_date > now {
        return Err(fail("voucher not started"));
    }
    if let Some(end) = v.end_date {
        if end < now {
            return Err(fail("voucher expired"));
        }
    }
    if let Some(limit) = v.usage_limit {
        if code_row.used >= limit {
            return Err(fail("voucher usage limit reached"));
        }
    }
    discount_voucherchannellisting::Entity::find()
        .filter(discount_voucherchannellisting::Column::VoucherId.eq(v.id))
        .filter(discount_voucherchannellisting::Column::ChannelId.eq(channel_id))
        .one(db)
        .await?
        .ok_or_else(|| fail("voucher not available on channel"))?;
    Ok(v.id)
}

/// Draft create with the full input surface. `channel_id` is the numeric
/// channel pk (dashboard sends `Channel` gids).
#[allow(clippy::too_many_arguments)]
pub async fn create_draft_full(
    db: &DatabaseConnection,
    channel_id: i32,
    channel_slug: &str,
    email: &str,
    lines: Vec<DraftLineInput>,
    opts: DraftCreateOptions,
    actor_id: Option<i32>,
) -> Result<DraftOrderView> {
    use sea_orm::TransactionTrait;
    if lines.is_empty() {
        return Err(fail("draft order needs at least one line"));
    }
    let vids: Vec<i32> = lines.iter().map(|l| l.variant_id).collect();
    let pricing = checkout_pricing(db, channel_slug, &vids).await?;
    let details = variant_details(db, &vids).await?;
    let currency = channel_currency(db, channel_id).await?;

    let txn = db.begin().await?;
    let voucher_id = match opts.voucher_code.clone().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        Some(code) => Some(validate_voucher_code(&txn, &code, channel_id).await?),
        None => None,
    };
    let billing_id = match opts.billing.as_ref() {
        Some(b) => Some(crate::order_ops::insert_address(&txn, b).await?),
        None => None,
    };
    let shipping_id = match opts.shipping.as_ref() {
        Some(s) => Some(crate::order_ops::insert_address(&txn, s).await?),
        None => None,
    };
    let shipping_price = match opts.shipping_method_id {
        Some(mid) => shipping_price_for(&txn, mid, channel_id).await?,
        None => Decimal::ZERO,
    };
    let shipping_name = match opts.shipping_method_id {
        Some(mid) => Some(shipping_name_for(&txn, mid).await?),
        None => None,
    };
    let id = Uuid::new_v4();
    let number = next_number(&txn).await?;
    let t = Utc::now();
    order_order::ActiveModel {
        id: Set(id),
        number: Set(number),
        created_at: Set(t.into()),
        updated_at: Set(t.into()),
        status: Set("draft".to_string()),
        authorize_status: Set("none".to_string()),
        charge_status: Set("none".to_string()),
        origin: Set("draft".to_string()),
        checkout_token: Set(String::new()),
        channel_id: Set(channel_id),
        currency: Set(currency.clone()),
        user_email: Set(email.to_string()),
        user_id: Set(opts.user_id),
        language_code: Set(opts.language_code.unwrap_or_else(|| "en".to_string())),
        display_gross_prices: Set(true),
        customer_note: Set(opts.customer_note.unwrap_or_default()),
        billing_address_id: Set(billing_id),
        shipping_address_id: Set(shipping_id),
        draft_save_billing_address: Set(Some(opts.save_billing)),
        draft_save_shipping_address: Set(Some(opts.save_shipping)),
        shipping_method_id: Set(opts.shipping_method_id),
        shipping_method_name: Set(shipping_name),
        base_shipping_price_amount: Set(shipping_price),
        shipping_price_net_amount: Set(shipping_price),
        shipping_price_gross_amount: Set(shipping_price),
        undiscounted_base_shipping_price_amount: Set(shipping_price),
        weight: Set(0.0),
        total_net_amount: Set(Decimal::ZERO),
        total_gross_amount: Set(Decimal::ZERO),
        undiscounted_total_net_amount: Set(Decimal::ZERO),
        undiscounted_total_gross_amount: Set(Decimal::ZERO),
        subtotal_net_amount: Set(Decimal::ZERO),
        subtotal_gross_amount: Set(Decimal::ZERO),
        total_charged_amount: Set(Decimal::ZERO),
        total_authorized_amount: Set(Decimal::ZERO),
        lines_count: Set(0),
        metadata: Set(opts.metadata),
        private_metadata: Set(opts.private_metadata),
        voucher_id: Set(voucher_id),
        voucher_code: Set(opts.voucher_code.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())),
        redirect_url: Set(opts.redirect_url),
        external_reference: Set(opts.external_reference),
        shipping_tax_class_metadata: Set(json!({})),
        shipping_tax_class_private_metadata: Set(json!({})),
        search_document: Set(String::new()),
        use_old_id: Set(false),
        should_refresh_prices: Set(true),
        tax_exemption: Set(false),
        tracking_client_id: Set(String::new()),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    for line in &lines {
        insert_line(&txn, id, &currency, line, &pricing, &details).await?;
    }
    let total = recalc_totals(&txn, id).await?;
    write_event(&txn, id, events::DRAFT_CREATED, json!({}), actor_id).await?;
    txn.commit().await?;
    Ok(DraftOrderView { id, number, status: "draft".to_string(), currency, total_gross: total, lines_count: lines.len() as i32 })
}

async fn channel_currency(db: &impl sea_orm::ConnectionTrait, channel_id: i32) -> Result<String> {
    Ok(channel_channel::Entity::find_by_id(channel_id)
        .select_only()
        .column(channel_channel::Column::CurrencyCode)
        .into_tuple::<String>()
        .one(db)
        .await?
        .unwrap_or_else(|| "USD".to_string()))
}

async fn shipping_price_for(db: &impl sea_orm::ConnectionTrait, method_id: i32, channel_id: i32) -> Result<Decimal> {
    use crate::entities::shipping_shippingmethodchannellisting;
    Ok(shipping_shippingmethodchannellisting::Entity::find()
        .filter(shipping_shippingmethodchannellisting::Column::ShippingMethodId.eq(method_id))
        .filter(shipping_shippingmethodchannellisting::Column::ChannelId.eq(channel_id))
        .one(db)
        .await?
        .ok_or_else(|| fail("shipping method is not listed in this channel"))?
        .price_amount)
}

async fn shipping_name_for(db: &impl sea_orm::ConnectionTrait, method_id: i32) -> Result<String> {
    use crate::entities::shipping_shippingmethod;
    Ok(shipping_shippingmethod::Entity::find_by_id(method_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("shipping method {method_id} not found")))?
        .name)
}

/// Draft update (Django `draftOrderUpdate`): customer, addresses, note,
/// shipping method, voucher, redirect, external ref, metadata. Draft-only.
#[allow(clippy::too_many_arguments)]
pub async fn update_draft(
    db: &DatabaseConnection,
    order_id: Uuid,
    user_id: Option<Option<i32>>,
    user_email: Option<String>,
    customer_note: Option<String>,
    billing: Option<crate::order_ops::OrderAddressInput>,
    shipping: Option<crate::order_ops::OrderAddressInput>,
    shipping_method_id: Option<Option<i32>>,
    voucher_code: Option<Option<String>>,
    redirect_url: Option<String>,
    external_reference: Option<String>,
    metadata: Option<serde_json::Value>,
    private_metadata: Option<serde_json::Value>,
    language_code: Option<String>,
    actor_id: Option<i32>,
) -> Result<DraftOrderView> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let row = draft_row(&txn, order_id).await?;
    let mut am: order_order::ActiveModel = row.into();
    if let Some(u) = user_id {
        am.user_id = Set(u);
    }
    if let Some(e) = user_email.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        am.user_email = Set(e);
    }
    if let Some(n) = customer_note {
        am.customer_note = Set(n);
    }
    if let Some(b) = billing.as_ref() {
        am.billing_address_id = Set(Some(crate::order_ops::insert_address(&txn, b).await?));
    }
    if let Some(s) = shipping.as_ref() {
        am.shipping_address_id = Set(Some(crate::order_ops::insert_address(&txn, s).await?));
    }
    if let Some(m) = shipping_method_id {
        match m {
            None => {
                am.shipping_method_id = Set(None);
                am.shipping_method_name = Set(None);
                am.base_shipping_price_amount = Set(Decimal::ZERO);
                am.shipping_price_net_amount = Set(Decimal::ZERO);
                am.shipping_price_gross_amount = Set(Decimal::ZERO);
                am.undiscounted_base_shipping_price_amount = Set(Decimal::ZERO);
            }
            Some(mid) => {
                let ch = am.channel_id.clone().unwrap();
                let price = shipping_price_for(&txn, mid, ch).await?;
                let name = shipping_name_for(&txn, mid).await?;
                am.shipping_method_id = Set(Some(mid));
                am.shipping_method_name = Set(Some(name));
                am.base_shipping_price_amount = Set(price);
                am.shipping_price_net_amount = Set(price);
                am.shipping_price_gross_amount = Set(price);
                am.undiscounted_base_shipping_price_amount = Set(price);
            }
        }
    }
    if let Some(v) = voucher_code {
        match v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
            None => {
                am.voucher_id = Set(None);
                am.voucher_code = Set(None);
            }
            Some(code) => {
                let ch = am.channel_id.clone().unwrap();
                let vid = validate_voucher_code(&txn, &code, ch).await?;
                am.voucher_id = Set(Some(vid));
                am.voucher_code = Set(Some(code));
            }
        }
    }
    if let Some(r) = redirect_url {
        am.redirect_url = Set(Some(r));
    }
    if let Some(r) = external_reference {
        am.external_reference = Set(Some(r));
    }
    if let Some(m) = metadata {
        am.metadata = Set(m);
    }
    if let Some(m) = private_metadata {
        am.private_metadata = Set(m);
    }
    if let Some(l) = language_code {
        am.language_code = Set(l);
    }
    am.updated_at = Set(Utc::now().into());
    am.update(&txn).await?;
    let total = recalc_totals(&txn, order_id).await?;
    let _ = actor_id;
    txn.commit().await?;
    let (h, _) = get_order_rows(db, order_id).await?.ok_or_else(|| fail("order vanished"))?;
    Ok(DraftOrderView { id: order_id, number: h.number, status: h.status, currency: h.currency, total_gross: total, lines_count: h_lines(db, order_id).await? })
}

async fn h_lines(db: &impl sea_orm::ConnectionTrait, order_id: Uuid) -> Result<i32> {
    Ok(order_orderline::Entity::find()
        .filter(order_orderline::Column::OrderId.eq(order_id))
        .count(db)
        .await? as i32)
}
