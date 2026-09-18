# Changelog

## Unreleased

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
