# ⚡ rustygod-saleor

**[Saleor](https://github.com/saleor/saleor), rewritten in Rust. Same PostgreSQL. Zero data migration. Just swap the image.**

[![Rust](https://img.shields.io/badge/rust-1.93-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![gRPC](https://img.shields.io/badge/api-gRPC%20%2B%20GraphQL-blue.svg)](https://github.com/hyperium/tonic)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-215-brightgreen.svg)](#behavioral-contract)
[![Contract Tests](https://img.shields.io/badge/contract_tests-119-brightgreen.svg)](#behavioral-contract)
[![Dashboard](https://img.shields.io/badge/dashboard_ops-457%2F457-brightgreen.svg)](#graphql-bff-enterprise)

[Saleor](https://github.com/saleor/saleor) is a great commerce engine trapped in a slow body: Python/Django, gigabytes of RAM,
GraphQL overhead on every hot path. **rustygod-saleor** is a ground-up Rust rewrite that runs
against **the exact same PostgreSQL schema Django created** — 145 tables, same constraints,
same rows. Migration friction for operators: **zero**. Point the binary at your database URL
and go.

> Forecasts, not promises: the design target is **<1 GB RAM** per node and **multi-x throughput**
> vs CPython on catalog/checkout reads. Benchmarks will be published per milestone — numbers
> or it didn't happen.

---

## Why not just optimize Django?

Because the ceiling is structural: GIL-bound request handling, ORM-per-row overhead, and a
38,000-line GraphQL schema served on every hot path. Rust gives us:

- **One static binary** — no interpreter, no venv, no 2 GB container layers.
- **Decimal money math at compile-time discipline** — `rust_decimal`, never `float`.
- **gRPC + tonic** for internal/API traffic — binary protobuf, HTTP/2 multiplexing, strict contracts.
- **Fearless concurrency** — checkout, inventory and promotion logic that the type system
  keeps honest under load.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    rustygod-saleor                       │
│                                                          │
│  crates/proto   Protobuf contracts (tonic build)         │
│    common · product · checkout · order                   │
│                                                          │
│  crates/core    Pure domain logic (no I/O)               │
│    Money · Product · Checkout · Order                    │
│    mirrors saleor/{product,checkout,order}/models.py     │
│                                                          │
│  crates/db      SeaORM over the REAL Django schema       │
│    147 entities generated 1:1 from live PostgreSQL       │
│    same tables · same constraints · same rows            │
│                                                          │
│  crates/graphql GraphQL BFF (enterprise, thin + generated)         │
│    Saleor-compatible /graphql/ — same DB, same logic     │
│    69 query roots · 244 mutation roots · 457/457 Dashboard ops green │
│    catalog · checkout · order · payment · commerce · apps             │
│                                                          │
│  crates/server  tonic gRPC (21 services) + Axum GraphQL   │
│    gRPC :50051 · GraphQL :8000 · metrics :9000            │
│    modes: api · worker · beat · check · migrate · seed    │
└──────────────────────┬──────────────────────────────────┘
                       │  RUSTYGOD_DATABASE_URL
                       ▼
              PostgreSQL (Django-managed)
              145 tables · populatedb-compatible
```

**Strangler-Fig strategy:** Rust and Django share one database under the Expand-Contract
pattern. Traffic moves service by service. At any point, either side can serve.

**ORM call:** SeaORM, not Diesel — a deliberate senior decision. This environment has no
`libpq` headers and no root, and `pq-sys` won't compile here; SeaORM is pure Rust and
`sea-orm-cli` generates entities from the live database with zero system dependencies.
The contract (same schema, same constraints) is identical either way.

## Benchmarks (release binary, local Postgres, 2026-09)

```
idle:             RSS ~8MB (binary ~95MB stripped, Sep 2026)
load  (20 conn):  ~5,400 rps · p50 3.8ms · p99 6.7ms · errors=0
spike (1000 conn): ~5,400 rps · p50 194ms · errors=2/108k · RSS →101MB
cool:             instant recovery to p50 ~4ms
```

Fixes that moved the needle 6x: batched catalog reads (3 queries no
matter the page size — the dataloader fix), DB pool 16→64, release
profile. Next: per-query shaping + k6 suite (Phase 2).

## Zero-friction migration

```bash
# 1. You already have this (Saleor's database, migrated + populated):
psql $DATABASE_URL -c "select count(*) from product_product;"
# 2. Point Rust at it — no dump, no ETL, no downtime window:
export RUSTYGOD_DATABASE_URL="postgres://saleor:saleor@db:5432/saleor"
./rustygod-server   # gRPC on 127.0.0.1:50051 + GraphQL on http://127.0.0.1:8000/graphql
```

Unset `RUSTYGOD_DATABASE_URL` and the server runs the offline in-memory demo instead.
Django keeps working throughout — both read the same rows.

## Quickstart

```bash
# Prerequisites: Rust 1.93+, a Saleor PostgreSQL (see below for the dev one)
cargo build
# Run tests per package with -j2 (full-workspace parallel builds can OOM small machines):
cargo test -j2 -p rustygod-graphql --test schema_sdl   # 3/3 green, no DB needed
cargo test -j2 -p rustygod-db --test contract_guest    # DB contract smoke vs live rows
# Totals: 215 tests (119 DB contracts vs Django rows + unit + flow + SDL)

# Against the real database:
export RUSTYGOD_DATABASE_URL="postgres://saleor:saleor@localhost:5432/saleor"
./target/debug/rustygod-server          # api: gRPC :50051 + GraphQL :8000 + metrics :9000
./target/debug/rustygod-server worker   # webhook outbox delivery (celery-worker equivalent)
./target/debug/rustygod-server beat     # periodic scheduler (celery-beat equivalent, 20 entries)
./target/debug/rustygod-server check    # readiness probe (DB + key tables)
./target/debug/rustygod-server migrate  # Saleor's own Django DDL (manage.py migrate)
./target/debug/rustygod-server seed --createsuperuser  # Saleor's own populatedb mock data
# Dashboard (existing Saleor Dashboard): API_URL=http://localhost:8000/graphql pnpm --filter dashboard dev
```

Spin up a Saleor database locally (from the [Saleor](https://github.com/saleor/saleor) repo):

```bash
cd saleor-core/.devcontainer && docker compose up -d db
cd saleor-core && cp .env.example .env && uv run poe migrate && uv run poe populatedb
```

## Behavioral contract

Saleor's **1,219 test files** are the spec. We don't reimplement vibes — we port the
*rules* as Rust tests, and comparative tests assert byte-level parity against rows Django wrote.

| Rust test | Saleor origin | Rule under contract |
|---|---|---|
| `core/tests/checkout_math.rs` (7) | `checkout/tests/test_base_calculations.py` | line price = variant channel price; `price_override` wins; decimal-exact totals; currency match; positive qty |
| `db/tests/contract_catalog.rs` (4) | `product/tests/test_product_availability.py`, channel-listing tests | published count == Django listings; variant price == `price_amount`; unpublished hidden; stock ≥ 0 |
| `db/tests/contract_checkout.rs` (5) | `checkout/tests/test_checkout.py`, `test_base_calculations.py` | Django defaults (US/en/none); line totals net==gross pre-tax; override precedence; qty validator; complete deletes rows |
| `db/tests/contract_orders.rs` (3) | `order/tests/test_order.py`, `test_order_from_checkout.py` | Django sequence numbers; persisted header+lines parity; exact status strings |
| `db/tests/contract_relations.rs` (6) | `product/tests/test_category.py`, `test_collections_availability.py`, attribute tests | MPTT tree order + parent/child consistency; collection visibility; type/attribute/media parity |
| `db/tests/contract_commerce.rs` (8) | discount/shipping/giftcard/menu/page/account/channel/tax/warehouse tests | voucher/promotion rules; shipping listings; giftcard rows; menu tree; address round-trip; stock reserve/release |
| `server/tests/flow.rs` (4) | `checkout/tests/test_checkout_complete.py`, `test_order_from_checkout.py` | complete mints matching order; checkout consumed exactly once; bad variant rejected; order numbering |
| `core/tests/payment_calc.rs` (12) + `db/tests/contract_payments.rs` (3) + `contract_psp.rs` (11) | `payment/transaction_item_calculations.py`, `payment/utils.py`, manual gateway | event-group recalc + PSP dedup, single `AUTHORIZATION_SUCCESS`, async Pending + `action_required` (3DS), per-checkout R3 guard, `adjust_authorization` cutoff, psp-less amounts |
| `core giftcard/draft/invoice` (7+3+3) + `db/tests/contract_giftcards.rs` (9) + `contract_drafts.rs` (7) + `contract_invoices.rs` (4) | `giftcard/tests/`, `graphql/order/mutations/draft_order_*`, `graphql/invoice/tests/` | dashed codes + active(date) + restrictions; draft merge/split/allocate-on-complete; invoice pending→success→sent + deletion flow |
| `core/tests/discount_math.rs` (8) + `db/tests/contract_promotions.rs` (3) + `contract_order_promotions.rs` (6) | `discount/utils/promotion.py`, `discount/utils/order.py`, `prices/discount.py` | HALF_UP percentage, floor-zero fixed, best-rule, predicate AND/OR + base64 GIDs; order promotions (gift XOR discount, max saving, voucher precedence), gift line `is_gift` + channel stock, complete carry → order discount + allocation |
| `db/tests/contract_guest.rs` (2) | `checkout/complete_checkout.py::_process_user_data_for_order` | guest→user link by email + authenticated checkout user_id, billing/shipping address carry (E3/E11) |
| `crates/graphql/tests/schema_sdl.rs` (3) + `catalog` productCreate | `saleor/dashboard` UserDetails + ProductCreate | GraphQL BFF thin: `products/checkout/order/me` + `productCreate` (manage_products), SDL parity, no-panic without DB |
| `ai` unit + `ai/tests/ai_contract.rs` (4+4) | `product/tests/test_product_search.py` | trigram ranking + empty-gibberish; co-purchase scores sorted; chat grounded; embedder determinism + vector-store top-k |

Porting order follows blast radius: **checkout → payment → discount → inventory → webhooks**.
Each domain lands with its contract tests before the next begins. The full 1,219-file suite
is not ported line-by-line — it is *distilled* into contracts per domain (see Roadmap).

## API (gRPC — 21 services)

```proto
service ProductService  { GetProduct · ListProducts · CreateProduct · GetCategory ·
                          ListCategories · GetCollection · ListCollections ·
                          GetProductType · GetProductAttributes · ListProductMedia }
service CheckoutService { CreateCheckout · GetCheckout · AddLines · CompleteCheckout }
service OrderService    { GetOrder · ListOrders · CreateFulfillment · CancelFulfillment · RefundFulfillment · ListFulfillments }
service DiscountService { ValidateVoucher · ListPromotions }
service ShippingService { ListShippingMethods }
service GiftCardService { Issue · GetByCode · AttachToCheckout · DetachFromCheckout · CheckoutBalance · Redeem · AdjustBalance · Refund · SetActive }
service DraftOrderService { CreateDraftOrder · AddDraftLines · SetDraftLineQuantity · RemoveDraftLine · CompleteDraftOrder · DeleteDraftOrder }
service InvoiceService { RequestInvoice · FulfillInvoice · SendInvoice · RequestDeletion · DeleteInvoice · ListReady }
service PluginService { RegisterPlugin · UnregisterPlugin · CallPlugin · ListPlugins · CalculateTax }
service MenuService     { GetMenu }
service PageService     { GetPage · ListPages }
service AccountService  { GetCustomer · CreateAddress · CreateGroup · ListGroups · RenameGroup · DeleteGroup · Add/RemoveGroupMembers · Grant/RevokeGroupPermissions }
service ChannelService  { ListChannels · GetChannel · CreateChannel · UpdateChannel · DeleteChannel · SetProductListing · SetVariantPrice }
service TaxService      { ListTaxClasses · GetTaxRate · CalculateTaxes }
service WarehouseService{ ListWarehouses · ListStocks · ReserveStock · ReleaseReservation · CreateWarehouse · UpdateWarehouse · DeleteWarehouse · UpsertStock · ListZones · Assign/UnassignZone }
service SemanticSearch  { SearchProducts }   # pg_trgm today, vectors next — same RPC
service Recommender     { RecommendProducts } # co-occurrence from Django order lines
service ChatAgent       { Chat (server-streaming) } # retrieval-grounded assistant
service PaymentService  { CreateTransaction · GetTransaction · Authorize · Charge · Refund · Cancel }
  service WebhookService  { TriggerEvent · GetDelivery · ListAttempts · SendDelivery · DueDeliveries }
  service AuthService     { Login · RefreshToken · VerifyToken · CheckPermission · CreateAppToken · VerifyAppToken · RevokeAppToken }
  ```

  GraphQL BFF (same binary, `/graphql`): **69 query roots + 244 mutation roots**
  (live SDL counts) generated from Saleor's own `schema.graphql`
  (`scripts/schema_codegen.py` → `crates/graphql/src/gen.rs`, never hand-edited;
  `scripts/check_input_nullability.py` pins all 3,243 input fields to Saleor
  nullability). All **457/457 Dashboard operations validate**
  (`scripts/verify_dashboard_ops.py`) — the stock Saleor Dashboard runs against
  it with `API_URL=http://localhost:8000/graphql`.

Money is `{ currency, amount<string> }` — decimals cross the wire as strings, exactly like
Saleor's serializers. IDs are strings carrying Django's integer PKs, so existing tooling
keeps working during the transition.

## Access control

Staff-only RPCs demand a permission codename, carried by a staff JWT or an
app token (`Authorization: Bearer …`):

- GiftCard `Issue`/`AdjustBalance`/`SetActive` → `manage_gift_card`
- All `DraftOrderService` / `InvoiceService` RPCs → `manage_orders`
- `CreateAppToken`/`RevokeAppToken` → `manage_apps`

Checkout-flow verbs (attach/detach/redeem, checkout complete) stay open —
exactly the mutations Django leaves permission-free. App tokens are stored
hashed (PBKDF2, shown once) and verified `last-4 + check_password`, byte for
byte like `AppTokenVerify`. Superusers bypass checks; rotated JWTs and
inactive apps never authorize.

## Plugins (WASM extensions)

Saleor plugins were Python-only and in-process. Ours are sandboxed WASM
with declared capabilities (`log`, `events`) and extension points — three
hand-written, toolchain-free reference plugins (integer math, auditable to
the byte): **flat-rate-tax** (`calculate_tax`), **min-order-validator**
(`checkout.validate`), **order-notifier** (`order.paid`, emits through the
webhook path). `PluginService` registers/calls/lists/unregisters modules
(staff: `manage_apps`; builtins pinned); the registry persists in the
`rustygod_plugin` table (ours — Django ignores it). Extension points named
in `RUSTYGOD_PUBLIC_POINTS` are callable without auth (the storefront path).
`sdk/plugin.mjs` is the JS client sketch.

## TypeScript SDK (generated, single source of truth)

`sdk/ts` holds ts-proto clients generated from `crates/proto` — the same
modules a storefront ships. Regenerate: `cd sdk/ts && npm run gen`.
`npm run demo` runs the full buy flow over real gRPC (search → checkout →
order #522 minted live during development). Known edge, documented in
`generate.sh`: `getChannel` collides with grpc-js's built-in method, so
`commerce.ts` carries `@ts-nocheck` (the wire contract is untouched).

## Roadmap

- [x] Workspace + tonic skeleton (gRPC + GraphQL dual-stack via Axum)
- [x] Domain core: Money/Product/Checkout/Order with decimal math (guest→user link, address carry)
- [x] SeaORM entities 1:1 from live Saleor PostgreSQL (147 files, Saleor `order_order_number_seq` parity)
- [x] DB-backed catalog reads (channel-aware pricing, published filtering, warehouse stock) + `productCreate` (GraphQL `manage_products`)
- [x] Checkout writes on Django tables (`checkout_checkout`/`checkoutline`: token PK, US/en/none defaults, net==gross pre-tax, complete deletes rows, `is_gift` gifts)
- [x] Order persistence on Django tables (`order_order`/`orderline`: same sequence, exact status strings, variant-detail lines, user + addresses from checkout)
- [x] Product relations (MPTT category tree, channel-aware collections, types, attributes, media)
- [x] Commerce services (discount/shipping/giftcard/menu/page/account/channel/tax/warehouse) — now also via GraphQL `channels/warehouses/taxClasses/shippingMethods/pages/promotions/menu`
- [x] AI layer v1 (`rustygod-ai`): `Embedder`/`VectorStore` traits with deterministic local backends, pg_trgm search, co-occurrence recommender, streaming grounded chat
- [x] 119 contract tests green (unit + comparative vs Django rows + service flow + audit + GraphQL SDL)
- [x] Promotion engine (catalogue + **order promotions**: gift XOR discount, max-saving winner, voucher precedence, `is_gift` carry + allocation)
- [x] Payments (TransactionItem event-group recalc + PSP dedup/async/3DS per-checkout R3 guard, `adjust_authorization`)
- [x] Webhooks (fan-out with channel filter, HMAC-SHA256, attempt log, success/failed, backoff + sweeper)
- [x] Auth (Django PBKDF2/bcrypt, RS256 `RSA_PRIVATE_KEY`, `me { User }` via GraphQL, staff `manage_*` + app tokens)
- [x] Fulfillment (auto-increment, remainder guards, stock decrease/restore, cancel, refunds, `determine_order_status`) + `return_and_refund` + per-transaction `granted_refunds`
- [x] GraphQL BFF (enterprise, thin): `crates/graphql` + `server` dual-stack `gRPC :50051` / `GraphQL :8000` / `metrics :9000`, same DB/logic (`me`, `products`, `checkout`, `order`, `channels`, `transaction`), Dashboard `API_URL=http://localhost:8000/graphql`
- [x] Audit-hardened: zero `unwrap` in production, transactional checkout-complete (user/address + promotions + gift cards + stock), idempotent add-lines, `FOR UPDATE` lock ordering (R6), outbox + sweeper (R8/R9)
- [ ] Checkout `auto-complete` expired + TTL sweeper final (E10)
- [ ] Product attribute writes / collection writes full parity
- [ ] CSV / thumbnails / site settings (intentionally last)
- [ ] AI layer: pgvector, recommender v2, agentic buying (natural-language checkout)
- [ ] Storefront/ Dashboard v2 generated from protos (Phase 4, <100ms TTFB target)

## Contributing

PRs welcome. Rules: no `float` for money, no `SELECT *` on generated entities (codegen
mistypes `tsvector`/`INTERVAL` — project required columns), every domain PR ships its
contract tests, every behavioral change cites the Saleor test it mirrors.

## License

AGPL-3.0 — free as in freedom, forever. See [LICENSE](LICENSE).
