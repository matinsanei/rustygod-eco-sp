# 📋 Audit Report — rustygod-saleor vs the E-Commerce Engine Checklist

Date: 2026-09-18 · Commit scope: atomic complete + allocation + reconcile.
Method: every item verified against code + green tests (no vibes).
Score: ✅ = 1, ⚠️ = 0.5, ❌ = 0.

## Overall: 65% (47 / 72)

| Section | Score | Before this session |
|---|---|---|
| §1 Features (37) | **64%** (20 ✅ · 7 ⚠️ · 10 ❌) | ~49% |
| §2 Risks (10) | **75%** (5 ✅ · 5 ⚠️) | ~25% |
| §3 Edge cases (15) | **50%** (6 ✅ · 3 ⚠️ · 6 ❌) | ~13% |
| §4 Reconciliation (10) | **85%** (7 ✅ · 3 ⚠️) | ~10% |

## §1 Features — what changed today

| # | Item | Status | Evidence |
|---|---|---|---|
| 3 | Atomic complete | ✅ was ❌ | `db/src/complete.rs`, `contract_complete.rs` (3) |
| 4 | Allocation | ✅ was ❌ | `warehouses::allocate_order_lines` |
| 6 | Oversell prevention | ✅ was ❌ | rollback test green |
| 14 | Voucher usage persistence | ✅ was ❌ | `increase_usage` inside TXN |
| 16 | Gift redeem in checkout | ✅ was ❌ | `redeem_for_order_tx` inside TXN |
| 18 | Events | ⚠️ was ❌ | PLACED + draft/invoice/giftcard events; not every transition |

Still ❌ (10): GraphQL gateway (by design), order cancel, payment orchestration,
granted refunds, translations, CSV, thumbnails, site settings, preorder, schedulers.
Still ⚠️ (7): full fulfillment (no approve/replace), tax (flat-rate only),
attributes/product-writes/customer-accounts (reads > writes), click-and-collect (flag only).

## §2 Risks

| ID | Risk | Status | Evidence |
|---|---|---|---|
| R1 | Stock race → oversell | ✅ | FOR UPDATE + rollback test |
| R2 | TOCTOU check→reserve | ⚠️ | reservations + allocate account for them; no reserve-at-add-line flow yet |
| R3 | Double payment | ⚠️ | complete idempotent; double-Pay-button (two checkouts) still possible |
| R4 | Voucher double-spend | ✅ | locked atomic increment |
| R5 | Gift double-redeem | ✅ | locked debit inside completion TXN |
| R6 | Deadlock | ✅ | documented lock order 1→4 |
| R7 | Complete idempotency | ✅ | token-stamped replay test |
| R8 | Webhook after commit | ⚠️ | after-commit + pending/backoff; at-least-once duplicates possible |
| R9 | Expired reservations | ⚠️ | `reserved_until` honored; no background sweeper |
| R10 | Zero-total orders | ⚠️ | totals floor at zero; no dedicated status test |

## §3 Edge cases

✅ E1 (zero stock → INSUFFICIENT_STOCK), E5 (partial fulfill), E8 (overcharge
flagged by reconcile), E12 (reservation expiry honored), E13 (second buyer
loses cleanly), E15 (fulfillment cancel restores stock).
⚠️ E4 (unlisted variant errors, code is generic), E6 (multi-stock allocate,
no split-shipment flow), E7 (refund exists, no return flow).
❌ E2 (zone validation), E3 (guest→user link), E9 (3DS), E10 (auto-complete
task), E11 (guest conversion), E14 (cancel-after-payment).

## §4 Reconciliation — `ReconcileOrder` RPC, 9 checks, all green on fresh orders

✅ charged_vs_total · fulfilled_vs_lines · allocated_vs_lines ·
voucher_usage · giftcard_balances · payments_linked · single_completion.
⚠️ refunded_within_charged (no granted-refund entity yet) ·
event_trail (PLACED only) · stale-sweep (no sweeper).

## Top 5 remaining financial risks

1. **No order cancel** (E14) — paid orders can't be voided + restocked.
2. **No granted refunds** (RC2) — decision vs money not separated.
3. **Payment orchestration** — single manual gateway; no PSP, no 3DS.
4. **No background sweeper** (R9/RC10) — expired reservations/checkouts linger.
5. **Guest→user + address linking** (E3/E11) — completion doesn't attach identity/addresses.

## How to re-run this audit

```bash
cargo test --workspace 2>&1 | grep -cE "test .* ok$"   # must equal checks below
# R1/R7: contract_complete (3) · RC*: reconcile asserts inside it (9/9)
# R4: contract_promotions · R5: contract_giftcards · events: contract_drafts/invoices
```
