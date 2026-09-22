//! Real metadata mutations (Saleor `updateMetadata` / `deleteMetadata` /
//! `updatePrivateMetadata` / `deletePrivateMetadata`).
//!
//! The generated stubs validate but persist nothing; these write through to
//! the Django tables' `metadata` / `private_metadata` JSONB columns (raw SQL
//! keyed by a static type->table map — Saleor's `ModelWithMetadata` set) and
//! return a minimal item carrying the merged metadata + Saleor global id.
//! Only global IDs are accepted: a bare int carries no type and would risk
//! writing the wrong table (product 137 vs category 137).

use async_graphql::{Context, Error, ID, Object, Result};
use sea_orm::{ConnectionTrait, Statement};

use crate::{
    common::{self, MetadataInput},
    context::GqlContext,
    gen,
};

#[derive(Default)]
pub struct MetadataMutation;

/// GraphQL type -> (django table, pk column, is-uuid).
fn target(ty: &str) -> Option<(&'static str, &'static str, bool)> {
    Some(match ty {
        "Product" => ("product_product", "id", false),
        "ProductVariant" => ("product_productvariant", "id", false),
        "Category" => ("product_category", "id", false),
        "Collection" => ("product_collection", "id", false),
        "ProductType" => ("product_producttype", "id", false),
        "ProductMedia" => ("product_productmedia", "id", false),
        "Attribute" => ("attribute_attribute", "id", false),
        "User" => ("account_user", "id", false),
        "Page" => ("page_page", "id", false),
        "PageType" => ("page_pagetype", "id", false),
        "Menu" => ("menu_menu", "id", false),
        "MenuItem" => ("menu_menuitem", "id", false),
        "Voucher" => ("discount_voucher", "id", false),
        "ShippingMethod" => ("shipping_shippingmethod", "id", false),
        "ShippingZone" => ("shipping_shippingzone", "id", false),
        "TaxClass" => ("tax_taxclass", "id", false),
        "GiftCard" => ("giftcard_giftcard", "id", false),
        "Channel" => ("channel_channel", "id", false),
        "Invoice" => ("invoice_invoice", "id", false),
        "Fulfillment" => ("order_fulfillment", "id", false),
        "App" => ("app_app", "id", false),
        "OrderLine" => ("order_orderline", "id", false),
        "Order" => ("order_order", "id", true),
        "Checkout" => ("checkout_checkout", "token", true),
        "Promotion" => ("discount_promotion", "id", true),
        "Warehouse" => ("warehouse_warehouse", "id", true),
        _ => return None,
    })
}

fn err(code: &str, message: String) -> gen::MetadataError {
    gen::MetadataError { field: None, message: Some(message), code: Some(code.into()) }
}

fn require_auth(ctx: &Context<'_>) -> Result<(), Error> {
    let bearer = ctx
        .data_opt::<crate::context::Bearer>()
        .map(|b| b.0.as_str())
        .or_else(|| ctx.data_opt::<GqlContext>().and_then(|g| g.bearer.as_deref()));
    if bearer.is_none() {
        return Err(Error::new("authentication required"));
    }
    Ok(())
}

/// Shared write path. `input` merges (update), `drop_keys` deletes;
/// `private` selects the column. Returns (errors, item).
async fn apply(
    ctx: &Context<'_>,
    id: &ID,
    input: &[MetadataInput],
    drop_keys: &[String],
    private: bool,
) -> Result<(Vec<gen::MetadataError>, Option<gen::ObjectWithMetadata>), Error> {
    require_auth(ctx)?;
    let g = ctx.data::<GqlContext>()?;
    let db = g.db().map_err(Error::new)?;
    let (ty, raw) = common::split_gid(&id.0).ok_or_else(|| {
        Error::new("metadata mutations require a global ID (Type:pk), got a bare id")
    })?;
    let (table, pk_col, is_uuid) = target(&ty).ok_or_else(|| {
        Error::new(format!("cannot mutate metadata on {ty}"))
    })?;
    let cast = if is_uuid { "uuid" } else { "int" };
    if is_uuid && raw.parse::<uuid::Uuid>().is_err() {
        return Ok((vec![err("INVALID", format!("bad uuid in id {raw}"))], None));
    }
    if !is_uuid && raw.parse::<i32>().is_err() {
        return Ok((vec![err("INVALID", format!("bad int id {raw}"))], None));
    }
    let col = if private { "private_metadata" } else { "metadata" };
    let sel = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        format!("SELECT {col} FROM {table} WHERE {pk_col} = $1::{cast}"),
        [raw.clone().into()],
    );
    let row = db.query_one(sel).await.map_err(|e| Error::new(e.to_string()))?;
    let Some(row) = row else {
        return Ok((vec![err("NOT_FOUND", format!("{ty} {raw} not found"))], None));
    };
    let cur: serde_json::Value = row.try_get::<serde_json::Value>("", col).unwrap_or(serde_json::Value::Null);
    let mut map = cur.as_object().cloned().unwrap_or_default();
    for i in input {
        map.insert(i.key.clone(), serde_json::Value::String(i.value.clone()));
    }
    for k in drop_keys {
        map.remove(k);
    }
    let merged = serde_json::Value::Object(map);
    let upd = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        format!("UPDATE {table} SET {col} = $1::jsonb WHERE {pk_col} = $2::{cast}"),
        [merged.to_string().into(), raw.clone().into()],
    );
    db.execute(upd).await.map_err(|e| Error::new(e.to_string()))?;
    let items = common::json_to_metadata_items(&merged);
    // Re-read the untouched sibling column so item carries both.
    let sib_col = if private { "metadata" } else { "private_metadata" };
    let sib = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        format!("SELECT {sib_col} FROM {table} WHERE {pk_col} = $1::{cast}"),
        [raw.clone().into()],
    );
    let sib_val: serde_json::Value = db
        .query_one(sib)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .and_then(|r| r.try_get::<serde_json::Value>("", sib_col).ok())
        .unwrap_or(serde_json::Value::Null);
    let sib_items = common::json_to_metadata_items(&sib_val);
    let (md, pmd) = if private { (sib_items, items) } else { (items, sib_items) };
    Ok((vec![], build_item(&ty, id.0.clone(), md, pmd)))
}

/// Minimal item: global id + both metadata lists (all dashboard selects).
#[allow(clippy::too_many_lines)]
fn build_item(
    ty: &str,
    gid: String,
    metadata: Vec<crate::common::MetadataItem>,
    private_metadata: Vec<crate::common::MetadataItem>,
) -> Option<gen::ObjectWithMetadata> {
    Some(match ty {
        "Product" => gen::ObjectWithMetadata::Product(lit_product(gid, metadata, private_metadata)),
        "ProductVariant" => gen::ObjectWithMetadata::ProductVariant(lit_product_variant(gid, metadata, private_metadata)),
        "Category" => gen::ObjectWithMetadata::Category(lit_category(gid, metadata, private_metadata)),
        "Collection" => gen::ObjectWithMetadata::Collection(lit_collection(gid, metadata, private_metadata)),
        "ProductType" => gen::ObjectWithMetadata::ProductType(lit_product_type(gid, metadata, private_metadata)),
        "ProductMedia" => gen::ObjectWithMetadata::ProductMedia(lit_product_media(gid, metadata, private_metadata)),
        "Attribute" => gen::ObjectWithMetadata::Attribute(lit_attribute(gid, metadata, private_metadata)),
        "User" => gen::ObjectWithMetadata::User(lit_user(gid, metadata, private_metadata)),
        "Page" => gen::ObjectWithMetadata::Page(lit_page(gid, metadata, private_metadata)),
        "PageType" => gen::ObjectWithMetadata::PageType(lit_page_type(gid, metadata, private_metadata)),
        "Menu" => gen::ObjectWithMetadata::Menu(lit_menu(gid, metadata, private_metadata)),
        "MenuItem" => gen::ObjectWithMetadata::MenuItem(lit_menu_item(gid, metadata, private_metadata)),
        "Voucher" => gen::ObjectWithMetadata::Voucher(lit_voucher(gid, metadata, private_metadata)),
        "ShippingMethod" => gen::ObjectWithMetadata::ShippingMethod(lit_shipping_method(gid, metadata, private_metadata)),
        "ShippingZone" => gen::ObjectWithMetadata::ShippingZone(lit_shipping_zone(gid, metadata, private_metadata)),
        "TaxClass" => gen::ObjectWithMetadata::TaxClass(lit_tax_class(gid, metadata, private_metadata)),
        "GiftCard" => gen::ObjectWithMetadata::GiftCard(lit_gift_card(gid, metadata, private_metadata)),
        "Channel" => gen::ObjectWithMetadata::Channel(lit_channel(gid, metadata, private_metadata)),
        "Invoice" => gen::ObjectWithMetadata::Invoice(lit_invoice(gid, metadata, private_metadata)),
        "Fulfillment" => gen::ObjectWithMetadata::Fulfillment(lit_fulfillment(gid, metadata, private_metadata)),
        "App" => gen::ObjectWithMetadata::App(lit_app(gid, metadata, private_metadata)),
        "OrderLine" => gen::ObjectWithMetadata::OrderLine(lit_order_line(gid, metadata, private_metadata)),
        "Order" => gen::ObjectWithMetadata::Order(lit_order(gid, metadata, private_metadata)),
        "Promotion" => gen::ObjectWithMetadata::Promotion(lit_promotion(gid, metadata, private_metadata)),
        "Warehouse" => gen::ObjectWithMetadata::Warehouse(lit_warehouse(gid, metadata, private_metadata)),
        _ => return None,
    })
}

#[Object]
impl MetadataMutation {
    async fn update_metadata(&self, ctx: &Context<'_>, id: ID, input: Vec<MetadataInput>) -> Result<gen::UpdateMetadata> {
        let (errors, item) = apply(ctx, &id, &input, &[], false).await?;
        Ok(gen::UpdateMetadata { errors, item })
    }
    async fn delete_metadata(&self, ctx: &Context<'_>, id: ID, keys: Vec<String>) -> Result<gen::DeleteMetadata> {
        let (errors, item) = apply(ctx, &id, &[], &keys, false).await?;
        Ok(gen::DeleteMetadata { errors, item })
    }
    async fn update_private_metadata(&self, ctx: &Context<'_>, id: ID, input: Vec<MetadataInput>) -> Result<gen::UpdatePrivateMetadata> {
        let (errors, item) = apply(ctx, &id, &input, &[], true).await?;
        Ok(gen::UpdatePrivateMetadata { errors, item })
    }
    async fn delete_private_metadata(&self, ctx: &Context<'_>, id: ID, keys: Vec<String>) -> Result<gen::DeletePrivateMetadata> {
        let (errors, item) = apply(ctx, &id, &[], &keys, true).await?;
        Ok(gen::DeletePrivateMetadata { errors, item })
    }
}

pub(crate) fn lit_product(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Product {
    gen::Product {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        seo_title: None,
        seo_description: None,
        name: None,
        description: None,
        product_type: None,
        slug: None,
        category: None,
        created: None,
        updated_at: None,
        weight: None,
        default_variant: None,
        rating: None,
        is_available: None,
        attributes: vec![],
        channel_listings: vec![],
        media: vec![],
        collections: vec![],
        is_available_for_purchase: None,
        tax_class: None,
    }
}

pub(crate) fn lit_product_variant(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::ProductVariant {
    gen::ProductVariant {
        id: Some(ID(id)),
        private_metadata,
        metadata,
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

pub(crate) fn lit_category(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Category {
    gen::Category {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        seo_title: None,
        seo_description: None,
        name: None,
        description: None,
        slug: None,
        parent: None,
        level: None,
        updated_at: None,
    }
}

pub(crate) fn lit_collection(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Collection {
    gen::Collection {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        seo_title: None,
        seo_description: None,
        name: None,
        description: None,
        slug: None,
        channel_listings: vec![],
    }
}

pub(crate) fn lit_product_type(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::ProductType {
    gen::ProductType {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        name: None,
        slug: None,
        has_variants: None,
        is_shipping_required: None,
        weight: None,
        kind: None,
        tax_class: None,
        assigned_variant_attributes: vec![],
        product_attributes: vec![],
    }
}

pub(crate) fn lit_product_media(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::ProductMedia {
    gen::ProductMedia {
        id: Some(ID(id)),
        r#type: None,
        private_metadata,
        metadata,
        sort_order: None,
        alt: None,
        oembed_data: None,
    }
}

pub(crate) fn lit_attribute(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Attribute {
    gen::Attribute {
        id: Some(ID(id)),
        r#type: None,
        private_metadata,
        metadata,
        input_type: None,
        entity_type: None,
        reference_types: vec![],
        name: None,
        slug: None,
        unit: None,
        value_required: None,
        visible_in_storefront: None,
        with_choices: None,
        filterable_in_storefront: None,
        available_in_grid: None,
        storefront_search_position: None,
    }
}

pub(crate) fn lit_user(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::User {
    gen::User {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        email: None,
        first_name: None,
        last_name: None,
        is_staff: None,
        is_active: None,
        is_confirmed: None,
        addresses: vec![],
        note: None,
        user_permissions: vec![],
        permission_groups: vec![],
        editable_groups: vec![],
        accessible_channels: vec![],
        restricted_access_to_channels: None,
        default_shipping_address: None,
        default_billing_address: None,
        external_reference: None,
        customer_type: None,
        last_login: None,
        date_joined: None,
    }
}

pub(crate) fn lit_page(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Page {
    gen::Page {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        seo_title: None,
        seo_description: None,
        title: None,
        content: None,
        published_at: None,
        is_published: None,
        slug: None,
        page_type: None,
        attributes: vec![],
    }
}

pub(crate) fn lit_page_type(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::PageType {
    gen::PageType {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        name: None,
        slug: None,
        attributes: vec![],
        has_pages: None,
    }
}

pub(crate) fn lit_menu(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Menu {
    gen::Menu {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        name: None,
        items: vec![],
    }
}

pub(crate) fn lit_menu_item(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::MenuItem {
    gen::MenuItem {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        name: None,
        menu: None,
        category: None,
        collection: None,
        page: None,
        level: None,
        children: vec![],
        url: None,
    }
}

pub(crate) fn lit_voucher(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Voucher {
    gen::Voucher {
        id: Some(ID(id)),
        r#type: None,
        private_metadata,
        metadata,
        name: None,
        code: None,
        usage_limit: None,
        used: None,
        start_date: None,
        end_date: None,
        apply_once_per_order: None,
        apply_once_per_customer: None,
        single_use: None,
        only_for_staff: None,
        min_checkout_items_quantity: None,
        countries: vec![],
        discount_value_type: None,
        channel_listings: vec![],
    }
}

pub(crate) fn lit_warehouse(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Warehouse {
    gen::Warehouse {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        name: None,
        slug: None,
        email: None,
        is_private: None,
        address: None,
        click_and_collect_option: None,
    }
}

pub(crate) fn lit_shipping_method(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::ShippingMethod {
    gen::ShippingMethod {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        name: None,
        price: None,
        active: None,
        message: None,
    }
}

pub(crate) fn lit_shipping_zone(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::ShippingZone {
    gen::ShippingZone {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        name: None,
        default: None,
        price_range: None,
        countries: vec![],
        shipping_methods: vec![],
        warehouses: vec![],
        channels: vec![],
        description: None,
    }
}

pub(crate) fn lit_tax_class(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::TaxClass {
    gen::TaxClass {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        name: None,
        countries: vec![],
    }
}

pub(crate) fn lit_gift_card(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::GiftCard {
    gen::GiftCard {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        display_code: None,
        last4_code_chars: None,
        code: None,
        created: None,
        created_by: None,
        created_by_email: None,
        assigned_to: None,
        assigned_to_email: None,
        last_used_on: None,
        expiry_date: None,
        app: None,
        product: None,
        events: vec![],
        tags: vec![],
        bought_in_channel: None,
        is_active: None,
        initial_balance: None,
        current_balance: None,
    }
}

pub(crate) fn lit_channel(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Channel {
    gen::Channel {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        slug: None,
        name: None,
        is_active: None,
        currency_code: None,
        has_orders: None,
        default_country: None,
        warehouses: vec![],
        stock_settings: None,
        order_settings: None,
        checkout_settings: None,
        payment_settings: None,
        tax_configuration: None,
    }
}

pub(crate) fn lit_invoice(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Invoice {
    gen::Invoice {
        private_metadata,
        metadata,
        status: None,
        created_at: None,
        id: Some(ID(id)),
        number: None,
        url: None,
    }
}

pub(crate) fn lit_fulfillment(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Fulfillment {
    gen::Fulfillment {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        fulfillment_order: None,
        status: None,
        tracking_number: None,
        created: None,
        lines: vec![],
        warehouse: None,
        shipping_refunded_amount: None,
        total_refunded_amount: None,
        reason: None,
        reason_reference: None,
    }
}

pub(crate) fn lit_app(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::App {
    gen::App {
        id: Some(ID(id)),
        r#type: None,
        private_metadata,
        metadata,
        identifier: None,
        permissions: vec![],
        created: None,
        is_active: None,
        name: None,
        tokens: vec![],
        webhooks: vec![],
        about_app: None,
        data_privacy_url: None,
        homepage_url: None,
        support_url: None,
        app_url: None,
        manifest_url: None,
        version: None,
        access_token: None,
        author: None,
        brand: None,
    }
}

pub(crate) fn lit_order(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Order {
    gen::Order {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        created: None,
        updated_at: None,
        status: None,
        user: None,
        billing_address: None,
        shipping_address: None,
        shipping_method_name: None,
        collection_point_name: None,
        channel: None,
        fulfillments: vec![],
        lines: vec![],
        actions: vec![],
        shipping_methods: vec![],
        invoices: vec![],
        number: None,
        is_paid: None,
        payment_status: None,
        authorize_status: None,
        charge_status: None,
        transactions: vec![],
        payments: vec![],
        total: None,
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
        user_email: None,
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


pub(crate) fn lit_promotion(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::Promotion {
    gen::Promotion {
        id: Some(ID(id)),
        r#type: None,
        private_metadata,
        metadata,
        name: None,
        description: None,
        start_date: None,
        end_date: None,
        rules: vec![],
    }
}

pub(crate) fn lit_order_line(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::OrderLine {
    gen::OrderLine {
        id: Some(ID(id)),
        private_metadata,
        metadata,
        product_name: None,
        variant_name: None,
        product_sku: None,
        is_shipping_required: None,
        quantity: None,
        quantity_fulfilled: None,
        tax_rate: None,
        unit_price: None,
        undiscounted_unit_price: None,
        unit_discount: None,
        unit_discount_reason: None,
        unit_discount_value: None,
        unit_discount_type: None,
        total_price: None,
        undiscounted_total_price: None,
        is_price_overridden: None,
        price_override_reason: None,
        variant: None,
        allocations: vec![],
        quantity_to_fulfill: None,
        tax_class: None,
        voucher_code: None,
        is_gift: None,
        discounts: vec![],
    }
}


pub(crate) fn lit_attribute_value(id: String, _metadata: Vec<crate::common::MetadataItem>, _private_metadata: Vec<crate::common::MetadataItem>) -> gen::AttributeValue {
    gen::AttributeValue {
        id: Some(ID(id)),
        name: None,
        slug: None,
        value: None,
        input_type: None,
        reference: None,
        file: None,
        rich_text: None,
        plain_text: None,
        boolean: None,
        date: None,
        date_time: None,
    }
}

pub(crate) fn lit_shipping_method_type(id: String, metadata: Vec<crate::common::MetadataItem>, private_metadata: Vec<crate::common::MetadataItem>) -> gen::ShippingMethodType {
    gen::ShippingMethodType {
        id: Some(ID(id)),
        r#type: None,
        private_metadata,
        metadata,
        name: None,
        description: None,
        channel_listings: vec![],
        postal_code_rules: vec![],
        minimum_order_weight: None,
        maximum_order_weight: None,
        maximum_delivery_days: None,
        minimum_delivery_days: None,
        tax_class: None,
    }
}
