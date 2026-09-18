//! Atomic, idempotent checkout completion (audit items R1–R7, E1, E13).
//!
//! Django's `complete_checkout` in one place: validate → mint → voucher →
//! gift cards → allocate → events → delete checkout, all or nothing.
//!
//! Lock ordering (deadlock prevention, R6) — always this sequence:
//!  1. checkout row (`FOR UPDATE`)
//!  2. stock rows, ascending id (`warehouses::allocate_order_lines`)
//!  3. voucher code row (`promotions::increase_usage` locks it)
//!  4. gift-card rows (`FOR UPDATE` in `redeem_for_order_tx`)
//!
//! SHARED LOCK CONTRACT: `cancel_order` (cancel.rs) takes the SAME order —
//! order row first, then stock rows ascending by id. Any future writer that
//! touches (orders × stocks) must lock in this order or concurrent
//! complete/cancel on one variant deadlocks. No exceptions.
//!
//! Idempotency (R3/R7): an order stamped with the checkout token is the
//! completion record — replaying complete with the same token returns the
//! existing order instead of minting a second one. Webhooks fire AFTER
//! commit (R8): a fan-out failure never rolls back money, deliveries stay
//! `pending` with backoff retry instead.

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, SelectorTrait, Set, TransactionTrait,
    sea_query::LockType,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    catalog, checkout_store,
    entities::{checkout_checkout, order_order, order_orderevent, warehouse_reservation},
    giftcards, order_store, promotions, warehouses, webhooks,
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::Complete(msg.into())
}

#[derive(Debug)]
pub struct CompleteOutcome {
    pub order_id: Uuid,
    pub number: i32,
    pub status: String,
    pub replayed: bool,
    /// Outbox delivery ids created atomically with the order. The service
    /// sends them best-effort right after commit; anything left `pending`
    /// (crash, 5xx) is picked up by the sweeper — R8.
    pub delivery_ids: Vec<i32>,
}

/// Existing completion for a checkout token (idempotency record).
async fn existing_order(
    db: &impl ConnectionTrait,
    token: Uuid,
) -> Result<Option<CompleteOutcome>> {
    let row: Option<(Uuid, i32, String)> = order_order::Entity::find()
        .select_only()
        .column(order_order::Column::Id)
        .column(order_order::Column::Number)
        .column(order_order::Column::Status)
        .filter(order_order::Column::CheckoutToken.eq(token.to_string()))
        .into_tuple()
        .one(db)
        .await?;
    Ok(row.map(|(order_id, number, status)| CompleteOutcome {
        order_id,
        number,
        status,
        replayed: true,
        delivery_ids: vec![],
    }))
}

async fn write_order_event(
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

/// Complete a checkout atomically. See module docs for the protocol.
pub async fn complete_checkout(
    db: &DatabaseConnection,
    token: Uuid,
) -> Result<CompleteOutcome> {
    // Idempotent replay first (no locks needed for the read).
    if let Some(done) = existing_order(db, token).await? {
        return Ok(done);
    }

    let txn = db.begin().await?;
    // 1. Lock the checkout row.
    let co_opt = checkout_checkout::Entity::find_by_id(token)
        .lock(LockType::Update)
        .one(&txn)
        .await?;
    let Some(co) = co_opt else {
        // The row is gone: it never existed, or a concurrent completion
        // committed between our replay read and our lock (R7 race window —
        // in READ COMMITTED the locked read skips the deleted row).
        // Re-check the completion record before 404ing.
        if let Some(done) = existing_order(&txn, token).await? {
            txn.commit().await?;
            return Ok(done);
        }
        return Err(fail("checkout not found"));
    };
    let lines = crate::entities::checkout_checkoutline::Entity::find()
        .filter(crate::entities::checkout_checkoutline::Column::CheckoutId.eq(token))
        .order_by_asc(crate::entities::checkout_checkoutline::Column::CreatedAt)
        .order_by_asc(crate::entities::checkout_checkoutline::Column::Id)
        .lock(LockType::Update)
        .all(&txn)
        .await?;
    if lines.is_empty() {
        return Err(fail("checkout has no lines"));
    }
    // Re-check idempotency inside the lock (a concurrent completion may
    // have committed between our read and our lock — R7 race window).
    if let Some(done) = existing_order(&txn, token).await? {
        txn.commit().await?;
        return Ok(done);
    }
    let ch_slug = catalog::channel_slug_for_id(&txn, co.channel_id).await?;
    let domain = checkout_store::to_domain(&co, &lines, &ch_slug);
    let vids: Vec<i32> = lines.iter().map(|l| l.variant_id).collect();
    let pricing = catalog::checkout_pricing(&txn, &ch_slug, &vids).await?;

    // 2. Mint the order (header + lines, pre-tax — base-calculation parity).
    let order = order_store::mint_from_checkout(&txn, &domain, co.channel_id, &ch_slug, &pricing)
        .await?;
    let order_id: Uuid = order.id.parse().map_err(|_| fail("minted bad order id"))?;

    // 3. Voucher usage (locked increment, R4).
    if let Some(code) = co.voucher_code.clone() {
        let email = co.email.clone().filter(|e| !e.is_empty());
        promotions::increase_usage(&txn, &code, email.as_deref()).await?;
    }

    // 4. Gift cards, greedy over active attached balances (R5).
    let cards = giftcards::checkout_cards(&txn, token).await?;
    if !cards.is_empty() {
        let balances: Vec<Decimal> = cards
            .iter()
            .filter(|c| {
                c.currency == co.currency
                    && rustygod_core::giftcard::is_active_card(
                        c.is_active,
                        c.expiry_date,
                        Utc::now().date_naive(),
                    )
            })
            .map(|c| c.current_balance_amount)
            .collect();
        let total: Decimal = cards.first().map(|_| domain.total().amount).unwrap_or(Decimal::ZERO);
        let mut remaining = rustygod_core::giftcard::cover_total(total, &balances).0;
        for card in cards.iter().filter(|c| c.currency == co.currency) {
            if remaining <= Decimal::ZERO {
                break;
            }
            let take = card.current_balance_amount.min(remaining);
            if take > Decimal::ZERO {
                giftcards::redeem_for_order_tx(&txn, &card.code, Some(order_id), take, co.user_id)
                    .await?;
                remaining -= take;
            }
        }
    }

    // 5. Allocate stocks (locked, ascending id — R1/R6; E1/E13 surface here
    // as INSUFFICIENT_STOCK and roll everything back).
    let order_lines = order_store::get_order_rows(&txn, order_id)
        .await?
        .map(|(_, ls)| ls)
        .unwrap_or_default();
    let alloc_items: Vec<(Uuid, i32, i32)> = order_lines
        .iter()
        .filter_map(|l| l.variant_id.map(|v| (l.id, v, l.quantity)))
        .collect();
    let own_lines: Vec<Uuid> = lines.iter().map(|l| l.id).collect();
    warehouses::allocate_order_lines(&txn, &alloc_items, &own_lines).await?;

    // 6. Consume this checkout's reservations (now allocations).
    if !own_lines.is_empty() {
        warehouse_reservation::Entity::delete_many()
            .filter(warehouse_reservation::Column::CheckoutLineId.is_in(own_lines))
            .exec(&txn)
            .await?;
    }

    // 7. Order event (R8/RC8 audit trail).
    write_order_event(
        &txn,
        order_id,
        "placed",
        json!({"checkout_token": token.to_string()}),
        co.user_id,
    )
    .await?;

    // 8. Delete the checkout (same end state as Django).
    checkout_store::delete_checkout_row(&txn, token).await?;

    // 9. OUTBOX writes inside the same transaction (R8): deliveries are
    // atomic with money. HTTP sending happens after commit (service,
    // best-effort) or via the sweeper — never inside this transaction.
    let payload = json!({
        "id": order_id.to_string(),
        "number": order.number,
        "status": order.status.as_str(),
        "checkout_token": token.to_string(),
    })
    .to_string();
    let mut delivery_ids =
        webhooks::trigger_event_tx(&txn, "order_created", Some(&ch_slug), &payload).await?;
    delivery_ids.extend(
        webhooks::trigger_event_tx(&txn, "checkout_completed", Some(&ch_slug), &payload).await?,
    );

    txn.commit().await?;

    Ok(CompleteOutcome {
        order_id,
        number: order.number.parse().unwrap_or(0),
        status: order.status.as_str().to_string(),
        replayed: false,
        delivery_ids,
    })
}
