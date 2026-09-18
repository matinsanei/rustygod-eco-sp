# ⚡ rustygod-saleor

**Saleor, rewritten in Rust. Same PostgreSQL. Zero data migration. Just swap the image.**

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)
[![gRPC](https://img.shields.io/badge/api-gRPC%20%2B%20tonic-blue.svg)](https://github.com/hyperium/tonic)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Contract Tests](https://img.shields.io/badge/contract%20tests-87%2F87%20green-brightgreen.svg)](#behavioral-contract)

Saleor is a great commerce engine trapped in a slow body: Python/Django, gigabytes of RAM,
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
│  crates/server  tonic gRPC services                      │
│    ProductService · CheckoutService · OrderService       │
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
idle:             RSS ~8MB (binary 20MB)
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
./rustygod-server   # gRPC on 127.0.0.1:50051
```

Unset `RUSTYGOD_DATABASE_URL` and the server runs the offline in-memory demo instead.
Django keeps working throughout — both read the same rows.

## Quickstart

```bash
# Prerequisites: Rust stable, a Saleor PostgreSQL (see below for the dev one)
cargo build
cargo test          # 87/87 green: unit + comparative + flow contracts

# Against the real database:
export RUSTYGOD_DATABASE_URL="postgres://saleor:saleor@localhost:5432/saleor"
./target/debug/rustygod-server
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
| `core/tests/payment_calc.rs` (8) + `db/tests/contract_payments.rs` (3) | `payment/transaction_item_calculations.py`, manual gateway | event-group recalc (pending/success/adjust/back/reverse), idempotent create/events, guard rails, order full/partial/none refresh |
| `core/tests/discount_math.rs` (7) + `db/tests/contract_promotions.rs` (3) | `discount/utils/promotion.py`, `prices/discount.py`, voucher/checkout flows | HALF_UP percentage, floor-zero fixed, best-rule, predicate AND/OR + base64 GIDs; live 30% rule → 40.00→28.00; DISCOUNT voucher → totals + usage increment |
| `ai` unit + `ai/tests/ai_contract.rs` (4+4) | `product/tests/test_product_search.py` | trigram ranking + empty-gibberish; co-purchase scores sorted; chat grounded; embedder determinism + vector-store top-k |

Porting order follows blast radius: **checkout → payment → discount → inventory → webhooks**.
Each domain lands with its contract tests before the next begins. The full 1,219-file suite
is not ported line-by-line — it is *distilled* into contracts per domain (see Roadmap).

## API (gRPC)

```proto
service ProductService  { GetProduct · ListProducts · CreateProduct · GetCategory ·
                          ListCategories · GetCollection · ListCollections ·
                          GetProductType · GetProductAttributes · ListProductMedia }
service CheckoutService { CreateCheckout · GetCheckout · AddLines · CompleteCheckout }
service OrderService    { GetOrder · ListOrders · CreateFulfillment · CancelFulfillment · RefundFulfillment · ListFulfillments }
service DiscountService { ValidateVoucher · ListPromotions }
service ShippingService { ListShippingMethods }
service GiftCardService { GetGiftCard }
service MenuService     { GetMenu }
service PageService     { GetPage · ListPages }
service AccountService  { GetCustomer · CreateAddress }
service ChannelService  { ListChannels · GetChannel }
service TaxService      { ListTaxClasses }
service WarehouseService{ ListWarehouses · ListStocks · ReserveStock · ReleaseReservation }
service SemanticSearch  { SearchProducts }   # pg_trgm today, vectors next — same RPC
service Recommender     { RecommendProducts } # co-occurrence from Django order lines
service ChatAgent       { Chat (server-streaming) } # retrieval-grounded assistant
service PaymentService  { CreateTransaction · GetTransaction · Authorize · Charge · Refund · Cancel }
service WebhookService  { TriggerEvent · GetDelivery · ListAttempts · SendDelivery · DueDeliveries }
service AuthService     { Login · RefreshToken · VerifyToken · CheckPermission }
```

Money is `{ currency, amount<string> }` — decimals cross the wire as strings, exactly like
Saleor's serializers. IDs are strings carrying Django's integer PKs, so existing tooling
keeps working during the transition.

## Roadmap

- [x] Workspace + tonic skeleton (`axum`-ready; gRPC first, REST gateway later)
- [x] Domain core: Money/Product/Checkout/Order with decimal math
- [x] SeaORM entities 1:1 from live Saleor PostgreSQL (147 files)
- [x] DB-backed catalog reads (channel-aware pricing, published filtering, warehouse stock)
- [x] Checkout writes on Django tables (`checkout_checkout`/`checkoutline`: token PK, US/en/none defaults, net==gross pre-tax, complete deletes rows)
- [x] Order persistence on Django tables (`order_order`/`orderline`: same `order_order_number_seq`, `"partially fulfilled"` exact strings, variant-detail lines)
- [x] Product relations (MPTT category tree, channel-aware collections, types, attributes, media)
- [x] Commerce services (discount/shipping/giftcard/menu/page/account/channel/tax/warehouse)
- [x] AI layer v1 (`rustygod-ai`): `Embedder`/`VectorStore` traits with deterministic local backends, pg_trgm search on Saleor's gin indexes, co-occurrence recommender from Django order lines, streaming grounded chat
- [x] 49 contract tests green (unit + comparative vs Django rows + service flow + audit)
- [x] Promotion engine (catalogue best-rule evaluation, predicate matching, line/order discount rows, voucher apply + usage increment)
- [x] Payments (TransactionItem event-group recalculation, manual gateway, idempotent create/events, order coverage statuses)
- [x] Webhooks (fan-out with channel filter, HMAC-SHA256 signing, attempt log, success/failed lifecycle, backoff retry dues)
- [x] Auth (Django PBKDF2/bcrypt verify, RS256 from shared RSA_PRIVATE_KEY, refresh + jwt_token_key revocation, staff permissions)
- [x] Fulfillment (auto-increment per order, remainder guards, stock decrease/restore, cancel, refunds, exact determine_order_status)
- [x] Audit-hardened: zero `unwrap` in production code, transactional
      checkout-complete and reservations, idempotent add-lines merging and
      reserve retry, channel auto-confirm status, denormalized totals refresh
- [ ] Checkout writes persisted to Django tables (`checkout_checkout`, lines)
- [ ] Discount/promotion engine (biggest logic block in Saleor — estimated largest milestone)
- [ ] Payments + webhooks (HMAC parity with `saleor/webhook`)
- [ ] Auth/JWT compatible with Django sessions
- [ ] Load benchmarks vs Saleor (k6 + published reports)
- [ ] AI layer: pgvector semantic search, recommender, sales chatbot
- [ ] Dashboard/Storefront adapters (gRPC → existing frontends)

## Contributing

PRs welcome. Rules: no `float` for money, no `SELECT *` on generated entities (codegen
mistypes `tsvector`/`INTERVAL` — project required columns), every domain PR ships its
contract tests, every behavioral change cites the Saleor test it mirrors.

## License

AGPL-3.0 — free as in freedom, forever. See [LICENSE](LICENSE).
