# Changelog

## Unreleased

### ⚠️ Breaking: checkout totals now include order promotions (T3)
- Before: checkout total = lines − catalogue − voucher.
- Now: + `orderPromotionTotal` (subtotal_discount or gift). Gifts are
  `is_gift` lines: qty 1, totals 0, listing price as audit, allocated
  stock. Orders carry a `discount_orderdiscount` (type ORDER_PROMOTION)
  and gift order lines (zero-priced, discount audit). Checkout `total`
  on the wire is the discounted value. Migration: no client change
  (total already discounted); read `is_gift` to render freebies.
- `CheckoutLine.is_gift` added (proto). Old clients ignore the unknown
  field; display logic should hide gift `unit_price` as free.

### Added
- Order promotions (T3, Django `order_promotions`): predicate-gated
  (`baseSubtotalPrice`/`baseTotalPrice` range/eq), best-winner by saving,
  gift vs discount competition, voucher precedence (voucher clears them),
  single gift slot. `RefreshOrderPromotion` RPC on CheckoutService.

### ⚠️ Breaking: `CancelOrder` now refunds paid orders (was REQUIRES_REFUND)
- Before: any captured money → `REQUIRES_REFUND` error, staff refunded manually.
- Now: paid cancel auto-refunds every unrefunded remainder (one granted
  decision per charged transaction) + releases allocations + `canceled`,
  all atomically. `refunded_amount` + `granted_refund_ids` added to the
  response. Migration: drop the manual-refund-before-cancel step; treat a
  `canceled` response as money-settled.
- Eligibility now ports `Order.can_cancel()`: deny-list
  {canceled, draft, expired} + refusal on ACTIVE fulfillments (return the
  goods instead). Cancel-after-partial-return is refused while a
  `fulfilled` row exists — Saleor parity, return the rest instead.

### ⚠️ Fixed: refund guard double-counted past refunds
- `charged-minus-refunded` on the net `charged_value` bucket wrongly
  blocked sequential partial refunds (charge 100, refund 20, refund 30
  → second rejected). Guard is now `amount > charged_value` (the bucket
  is already net). Single refunds behaved identically; only sequences change.

### Added
- `ReturnOrderLines` RPC: return lines + line-based granted refund
  executed on one transaction (optional restock for damaged/lost goods),
  over-return rejected with nothing persisted.

### ⚠️ Breaking: second `AUTHORIZATION_SUCCESS` is now rejected
- Before: authorizing twice on one transaction silently double-counted.
- Now: `ALREADY_EXISTS` + a math-excluded failure record (exact Django
  behavior — `deduplicate_event` port). Use `AdjustAuthorization`
  (overwrites the bucket, Django's `AUTHORIZATION_ADJUSTMENT`) to change
  the amount. Migration: replace sequential authorize calls with one
  authorize + adjustments.

### ⚠️ Breaking: PaymentService now requires `manage_payments`
- All money RPCs were unauthenticated; they now demand `MANAGE_PAYMENTS`
  (staff JWT or app token), like every other mutating service.

### ⚠️ New guard: one in-flight transaction per checkout (R3)
- `CreateTransaction` with a `checkout_id` fails with
  `ALREADY_IN_PROGRESS` while another transaction for that checkout has
  pending buckets > 0 (double-Pay-button fails fast instead of
  double-charging). Retry after failure/settle is unaffected.

### Added
- PSP abstraction (`core::psp`): `ManualPsp` (sync), `ChallengePsp`
  (always-3DS rehearsal), `ScriptedPsp` (deterministic test scripts).
- Async money: `execute_via` (Pending → request-only event),
  `PspCallback` RPC (terminal settle with replay + TERMINAL protection),
  `action_required` challenge events with redirect in `external_url`.
- PSP-level dedup `(transaction, psp_reference, type)`: same amount →
  already-processed replay; different amount → math-excluded failure
  record + error. All writes serialize on the transaction row.
- Recalculation now matches Django exactly: chronological order,
  adjustment cutoff (`_get_authorize_events`), timestamp success/failure
  race (`_should_increse_amount`), psp-less app-managed handling.
- `PspCallback` + `AdjustAuthorization` RPCs; `gateway`/`return_url`
  selector and `action_required`/`redirect_url` answers on every action.

### ⚠️ Breaking: `CompleteCheckout` retry is now idempotent replay
- Before: completing an already-completed checkout returned `NOT_FOUND`.
- Now: it returns the **same `order_id`** with no errors (token-stamped
  completion record, `replayed: true` in `db::complete::CompleteOutcome`).
- Migration: clients that treated `NOT_FOUND`-after-complete as success keep
  working; clients that treated any error as failure should accept an
  `order_id` response on retry. Safe to retry unconditionally — mint happens
  at most once per checkout token.

### Added
- Atomic checkout completion (`db::complete`): mint + voucher + gift-card
  debit + stock allocation + `placed` event + checkout delete in one
  transaction; `INSUFFICIENT_STOCK` rolls everything back.
- Webhook outbox: deliveries are created inside the business transaction;
  post-commit fast send + sweeper pickup (`sending` claim, single-flight).
- `OrderService.CancelOrder` (unpaid only) and `ReconcileOrder` (9 checks).
- Sweeper (`RUSTIFY_SWEEP_SECS`, default 60): expired reservations +
  due webhook deliveries, with indexes.
- `delete_checkout_row` now releases line reservations (they were orphaned).
- Fulfillment skips stock moves for `track_inventory = false` variants.
- `RUSTIFY_PUBLIC_POINTS` for storefront-safe extension points.
