//! Invoices on Django's `invoice_invoice` / `invoice_invoiceevent` tables.
//!
//! Mirrors `saleor/graphql/invoice/mutations/` (v1 subset):
//! - request: order must exist, be requestable (not draft/unconfirmed/
//!   expired) and carry a billing address (`NOT_READY` otherwise); the row
//!   starts `pending` with a REQUESTED event;
//! - fulfill: synchronous stand-in for the async plugin renderer — sets
//!   number/url, flips to `success`, writes CREATED;
//! - send: only ready invoices, writes SENT (no email is actually dispatched
//!   in v1 — the notification milestone owns delivery);
//! - request_deletion + delete: `pending` + REQUESTED_DELETION, then
//!   `deleted` + DELETED (Django splits request/execution across the plugin;
//!   v1 executes immediately and says so).

use chrono::Utc;
use rustygod_core::invoice as domain;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{invoice_invoice, invoice_invoiceevent, order_order},
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Invoice(msg.into())
}

async fn write_event(
    txn: &impl ConnectionTrait,
    event_type: &str,
    invoice_id: Option<i32>,
    order_id: Option<Uuid>,
    parameters: serde_json::Value,
    user_id: Option<i32>,
) -> Result<()> {
    invoice_invoiceevent::ActiveModel {
        date: Set(Utc::now().into()),
        r#type: Set(event_type.to_string()),
        parameters: Set(parameters),
        invoice_id: Set(invoice_id),
        user_id: Set(user_id),
        app_id: Set(None),
        order_id: Set(order_id),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Ok(())
}

async fn get_row(
    db: &impl ConnectionTrait,
    invoice_id: i32,
) -> Result<invoice_invoice::Model> {
    invoice_invoice::Entity::find_by_id(invoice_id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("invoice {invoice_id} not found")))
}

/// Request an invoice for an order (`InvoiceRequest`).
/// Returns the pending invoice row.
pub async fn request_invoice(
    db: &DatabaseConnection,
    order_id: Uuid,
    number: Option<String>,
    user_id: Option<i32>,
) -> Result<invoice_invoice::Model> {
    let order = order_order::Entity::find_by_id(order_id)
        .select_only()
        .column(order_order::Column::Status)
        .column(order_order::Column::BillingAddressId)
        .into_tuple::<(String, Option<i32>)>()
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("order {order_id} not found")))?;
    let (status, billing) = order;
    domain::require_requestable(&status).map_err(|e| fail(e.to_string()))?;
    if billing.is_none() {
        return Err(fail("Cannot request an invoice for order without billing address."));
    }

    let txn = db.begin().await?;
    let t = Utc::now();
    let row = invoice_invoice::ActiveModel {
        private_metadata: Set(json!({})),
        metadata: Set(json!({})),
        status: Set(domain::status::PENDING.to_string()),
        created_at: Set(t.into()),
        updated_at: Set(t.into()),
        number: Set(number.clone()),
        created: Set(None),
        external_url: Set(None),
        invoice_file: Set(String::new()),
        message: Set(None),
        order_id: Set(Some(order_id)),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    write_event(
        &txn,
        domain::events::REQUESTED,
        Some(row.id),
        Some(order_id),
        json!({"number": number}),
        user_id,
    )
    .await?;
    txn.commit().await?;
    Ok(row)
}

/// Fulfill a pending invoice: store number/url, mark `success` (CREATED).
/// v1 executes synchronously where Django waits on the invoice plugin.
pub async fn fulfill_invoice(
    db: &DatabaseConnection,
    invoice_id: i32,
    number: &str,
    url: &str,
    user_id: Option<i32>,
) -> Result<invoice_invoice::Model> {
    let txn = db.begin().await?;
    let row = get_row(&txn, invoice_id).await?;
    if row.status != domain::status::PENDING {
        return Err(fail("only pending invoices can be fulfilled"));
    }
    let mut am: invoice_invoice::ActiveModel = row.clone().into();
    am.status = Set(domain::status::SUCCESS.to_string());
    am.number = Set(Some(number.to_string()));
    am.external_url = Set(Some(url.to_string()));
    am.created = Set(Some(Utc::now().into()));
    am.updated_at = Set(Utc::now().into());
    let updated = am.update(&txn).await?;
    write_event(
        &txn,
        domain::events::CREATED,
        Some(invoice_id),
        row.order_id,
        json!({"number": number, "url": url}),
        user_id,
    )
    .await?;
    txn.commit().await?;
    Ok(updated)
}

/// Mark a pending invoice failed (plugin error path).
pub async fn fail_invoice(
    db: &DatabaseConnection,
    invoice_id: i32,
    message: &str,
) -> Result<invoice_invoice::Model> {
    let txn = db.begin().await?;
    let row = get_row(&txn, invoice_id).await?;
    if row.status != domain::status::PENDING {
        return Err(fail("only pending invoices can fail"));
    }
    let mut am: invoice_invoice::ActiveModel = row.into();
    am.status = Set(domain::status::FAILED.to_string());
    am.message = Set(Some(message.to_string()));
    am.updated_at = Set(Utc::now().into());
    let updated = am.update(&txn).await?;
    txn.commit().await?;
    Ok(updated)
}

/// Record sending a ready invoice to the customer (SENT).
/// v1 writes the audit row; email delivery is a later milestone.
pub async fn send_invoice(
    db: &DatabaseConnection,
    invoice_id: i32,
    email: &str,
    user_id: Option<i32>,
) -> Result<invoice_invoice::Model> {
    let txn = db.begin().await?;
    let row = get_row(&txn, invoice_id).await?;
    domain::require_sendable(&row.status).map_err(|e| fail(e.to_string()))?;
    write_event(
        &txn,
        domain::events::SENT,
        Some(invoice_id),
        row.order_id,
        json!({"email": email}),
        user_id,
    )
    .await?;
    txn.commit().await?;
    Ok(row)
}

/// Request deletion (`InvoiceRequestDelete`): back to `pending` with a
/// REQUESTED_DELETION event; the plugin would pick it up from here.
pub async fn request_deletion(
    db: &DatabaseConnection,
    invoice_id: i32,
    user_id: Option<i32>,
) -> Result<invoice_invoice::Model> {
    let txn = db.begin().await?;
    let row = get_row(&txn, invoice_id).await?;
    let mut am: invoice_invoice::ActiveModel = row.clone().into();
    am.status = Set(domain::status::PENDING.to_string());
    am.updated_at = Set(Utc::now().into());
    let updated = am.update(&txn).await?;
    write_event(
        &txn,
        domain::events::REQUESTED_DELETION,
        Some(invoice_id),
        row.order_id,
        json!({}),
        user_id,
    )
    .await?;
    txn.commit().await?;
    Ok(updated)
}

/// Execute deletion: status `deleted` + DELETED event.
/// Django leaves execution to the invoice plugin; v1 executes immediately.
pub async fn delete_invoice(
    db: &DatabaseConnection,
    invoice_id: i32,
    user_id: Option<i32>,
) -> Result<invoice_invoice::Model> {
    let txn = db.begin().await?;
    let row = get_row(&txn, invoice_id).await?;
    let mut am: invoice_invoice::ActiveModel = row.into();
    am.status = Set(domain::status::DELETED.to_string());
    am.updated_at = Set(Utc::now().into());
    let updated = am.update(&txn).await?;
    write_event(
        &txn,
        domain::events::DELETED,
        Some(invoice_id),
        updated.order_id,
        json!({"invoice_id": invoice_id}),
        user_id,
    )
    .await?;
    txn.commit().await?;
    Ok(updated)
}

/// Ready (successful) invoices of an order, oldest first.
pub async fn ready_invoices(
    db: &impl ConnectionTrait,
    order_id: Uuid,
) -> Result<Vec<invoice_invoice::Model>> {
    Ok(invoice_invoice::Entity::find()
        .filter(invoice_invoice::Column::OrderId.eq(order_id))
        .filter(invoice_invoice::Column::Status.eq(domain::status::SUCCESS))
        .order_by_asc(invoice_invoice::Column::Id)
        .all(db)
        .await?)
}

pub async fn events_of(
    db: &impl ConnectionTrait,
    invoice_id: i32,
) -> Result<Vec<invoice_invoiceevent::Model>> {
    Ok(invoice_invoiceevent::Entity::find()
        .filter(invoice_invoiceevent::Column::InvoiceId.eq(invoice_id))
        .order_by_asc(invoice_invoiceevent::Column::Id)
        .all(db)
        .await?)
}
