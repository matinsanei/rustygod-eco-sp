# AGENT.md — saleor-rustify contributor guide

Rust rewrite of Saleor on the **same PostgreSQL** (zero migration).
Measured 2026-09-24: live SDL **91 queries / 346 mutations** (schema: 90/339 —
every schema root resolves), dashboard harness **458/458**, DB contracts **171/171**.

## Iron rules (learned the hard way)

1. **Never hand-edit `crates/graphql/src/gen.rs`.** It is generated from
   `saleor/dashboard/schema-main.graphql`. Fix `scripts/schema_codegen.py` and re-run.
2. **Root-name law.** A hand resolver MUST satisfy exactly one of:
   - Rust name camel-matches the schema root (`order_confirm` → `orderConfirm`), or
   - explicit `#[graphql(name = "orderNoteAdd")]`.
   Otherwise `schema_examine.py` can't see it (`our_roots()` scans `async fn`
   names), codegen **keeps the stub**, and `GenMutation` — last in the
   `MergedObject` — **silently wins at runtime**. Real code, dead endpoint,
   green harness. This exact bug hid `orderNoteAdd` and friends for weeks.
3. **One `#[Object]` impl per struct.** A second plain `impl` compiles fine but
   serves nothing (32 account methods — register/password/groups — were dead
   this way). If you see two `impl X` blocks, merge them.
4. **New-root ritual** (in this order, no shortcuts):
   ```bash
   python3 scripts/schema_examine.py --json-out /tmp/schema_ir.json
   python3 scripts/schema_codegen.py --emit
   cargo check -j2 -p saleor-rustify-graphql
   cargo build -j2 -p saleor-rustify-server   # then restart backend, then harness
   ```
   Skipping `--json-out` regenerates `gen.rs` from a **stale IR** and your new
   root loses to its own stub at runtime with zero errors anywhere.
5. **Money buckets** (`payments::execute_via` guards are law):
   `authorized` = *remaining* authorized (charge/cancel already decrement it),
   `charged` = *net* of refunds. Never compute `authorized-charged` or
   `charged-refunded` — both double-count. Capture/void take from
   `authorized`, refunds take from `charged`.
6. **Slim selects only.** `product_product.search_vector` (tsvector) and
   `channel_channel` INTERVAL columns crash full-model decode. `select_only()`
   what you need. SeaORM tuples cap at **12 columns** — split wider selects.
7. **Honest errors over silent lies.** No gateway → `gateway X is not
   configured`. No plugin → `... is not configured`. Unsupported input →
   explicit error naming the gap (precedents: `removeCheckout=false`,
   `replace=true`, XLSX export, FILTER scope). Stubs that return `[]` with no
   error are tech debt — track them in the checklist, don't create new ones.

## Commands

```bash
# DB (host network, port 5434 — never 5432)
./scripts/db.sh psql -t -c "SELECT 1;"
export RSA_PRIVATE_KEY="$(cat crates/db/tests/testdata/test_rsa.pem)"
export RUSTIFY_GRAPHQL_ADDR=127.0.0.1:8000

# Build / check (always -j2; full parallel starves rust-analyzer + pg)
cargo check -j2 -p saleor-rustify-graphql
cargo check -j2 --workspace --all-targets
cargo build -j2 -p saleor-rustify-server

# Backend (kill → start → wait ~50s cold start)
pkill -x saleor-rustify; sleep 1
setsid ./target/debug/saleor-rustify > /tmp/opencode/backend.log 2>&1 < /dev/null &
# token (expires ~1h; refresh when you see ExpiredSignature)
curl -s http://127.0.0.1:8000/graphql -H 'Content-Type: application/json' \
  -d '{"query":"mutation { tokenCreate(email: \"admin@example.com\", password: \"admin\") { token } }"}'

# Tests (per-suite only; full runs take minutes)
cargo test -j2 -p saleor-rustify-db --test contract_order_ops
python3 scripts/verify_dashboard_ops.py   # expect TESTED=458 FAILED=0
```

## Map

- `crates/proto`, `crates/core` — pure domain logic (money, PSP trait, discounts).
  `core::psp::ManualPsp` settles synchronously (pending buckets stay 0).
- `crates/db` — SeaORM over Django tables. Money: `payments.rs`. Orders:
  `order_store` (reads), `order_ops` (staff ops), `order_import` (bulk),
  `drafts`, `fulfillment`, `granted_refunds`, `complete`, `cancel`.
  Catalog: `catalog` (reads), `catalog_writes`, `attribute_writes`,
  `content_writes` (menus/pages). Marketing: `promotions` (engine),
  `promo_writes`, `giftcards`. Infra: `webhooks` (outbox), `exports` (CSV),
  `channels`, `warehouses`, `ship_tax_writes`, `taxes`, `apps`, `invoices`.
  `DbError` per-domain variants (`Order`, `Draft`, `GiftCardNotApplicable`…).
- `crates/graphql` — translation layer only. Sections: `order`, `payment`,
  `checkout`, `catalog`, `commerce`, `account`, `apps`, `translations`,
  `metadata` + `common` (gid/parse, money, metadata merge) + `mail` (SMTP).
  Hand payloads are `Gql*` with explicit `#[graphql(name = "...")]`
  (e.g. `GqlOrderCancel` → `OrderCancel`); dashboard-selected shapes are
  reused from `gen::` directly.
- `crates/server` — binary + gRPC services + beats
  (`checkout-automatic-completion`, `refresh-product-embeddings`,
  `prune-security-tables`).

## Auth

- Staff JWT (`RSA_PRIVATE_KEY`) via `authorization-bearer` header.
- `require_perm(ctx, "manage_orders")` (superuser bypass) for mutations;
  `requester(ctx, db)` → `(user_id, _)` for actor ids (never `?` in payload
  resolvers — `unwrap_or((0, String::new()))` and let the DB guard speak).
- App calls: `app_caller(ctx, db)` (app-token bearer only; staff JWT rejected).
- App tokens stored hashed (`token_last_4` lookup + check); raw shown once.

## Docs

- `docs/SALEOR_PARITY_CHECKLIST.md` — the honest ledger (measured header +
  per-area status + appendix mapping every old item). Update it when you land
  a batch; never silently drop a line — move it with its reason.
- `docs/ACCOUNT_SECURITY_CHECKLIST.md`, `STATUS.md`, `BENCHMARK.md` (the ~20×
  claim still needs a published Django baseline — do not cite it externally).
- `scripts/our_fields.json` is regenerated by `dump_our_fields.py` (KEPT
  coverage check input, not hand-edited).

## Known gaps (don't re-discover)

- Multipart upload missing → `fileUpload`, `userAvatar*` are stubs; product
  media create takes `mediaUrl`. axum needs the `multipart` feature (multer
  3.1.0 is in the cargo cache).
- `checkouts` list needs full `gen::Checkout` assembly (biggest single gap).
- Only Stripe skeleton + manual PSP; Adyen/30+ gateways missing.
- No tax providers; `shopFetchTaxRates` honestly reports none configured.
- SMTP is plain relay, no TLS/auth (`RUSTIFY_SMTP_HOST/PORT`, default
  `127.0.0.1:1025` → Mailpit). Mails log-and-continue, never break flows.
- CSV only (no XLSX), export scope ALL/IDS (FILTER refused), no replacement
  orders, external-auth plugins absent (honest errors), `saleor-db` container
  is unstable (`Exit 255` idle — `restart: unless-stopped` set; watch it).
- Seed was wiped 2026-09-20, never re-run `populatedb`. `/tmp/*.json`,
  `/tmp/opencode/tok.txt` are volatile (IR lives at `/tmp/schema_ir.json`).
