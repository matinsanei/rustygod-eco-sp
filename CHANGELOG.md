# Changelog

## Unreleased

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
- Sweeper (`RUSTYGOD_SWEEP_SECS`, default 60): expired reservations +
  due webhook deliveries, with indexes.
- `delete_checkout_row` now releases line reservations (they were orphaned).
- Fulfillment skips stock moves for `track_inventory = false` variants.
- `RUSTYGOD_PUBLIC_POINTS` for storefront-safe extension points.
