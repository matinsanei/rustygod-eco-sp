# ⚡ saleor-rustify

### Saleor, re-engineered in Rust. Same PostgreSQL. Zero migration. Just swap the image.

[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Rust 1.93](https://img.shields.io/badge/rust-1.93-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![gRPC + GraphQL](https://img.shields.io/badge/api-gRPC%20%2B%20GraphQL-blue.svg)](https://github.com/hyperium/tonic)
[![Dashboard ops](https://img.shields.io/badge/dashboard_ops-458%2F458-brightgreen.svg)](#-proof-not-promises)
[![Contract tests](https://img.shields.io/badge/contract_tests-171-brightgreen.svg)](#-proof-not-promises)
[![Memory](https://img.shields.io/badge/RSS-~58MB-brightgreen.svg)](#-proof-not-promises)

> **Trademark note:** saleor-rustify is an independent community project, not
> affiliated with, endorsed by, or sponsored by Saleor Ltd. / Mirumee Software.
> "Saleor" is used solely to describe API and database compatibility
> (nominative fair use). If the Saleor team prefers a different name, we will
> rename — see [For the Saleor team](#-for-the-saleor-team).

[Saleor](https://github.com/saleor/saleor) is a great commerce engine in a slow
body: CPython, GIL-bound requests, ORM-per-row overhead, gigabytes of RAM per
node. **saleor-rustify** is a ground-up Rust rewrite that runs against **the
exact same PostgreSQL schema Django created** — same 147 tables, same
constraints, same rows, same sequences. Migration friction for operators:
**zero**. Point the binary at your database URL and go. Django keeps working
throughout — both read the same rows, and either side can serve at any point
(strangler-fig, Expand–Contract).

---

## 🚀 Run it in 60 seconds

```bash
# 1. A Saleor database (yours, or spin one up + populate):
cd saleor-core && cp .env.example .env && uv run poe migrate && uv run poe populatedb

# 2. The Rust engine against it — no dump, no ETL, no downtime window:
export RUSTIFY_DATABASE_URL="postgres://saleor:saleor@localhost:5432/saleor"
export RSA_PRIVATE_KEY="$(cat crates/db/tests/testdata/test_rsa.pem)"  # dev only
./target/debug/saleor-rustify
# gRPC :50051 · GraphQL http://127.0.0.1:8000/graphql · metrics :9000

# 3. The stock Saleor Dashboard, pointed at Rust:
API_URL=http://localhost:8000/graphql pnpm --filter dashboard dev
# Every page renders. 458/458 dashboard operations validate. Zero schema errors.
```

Or via Docker (images published per push to `master`):

```bash
docker run --rm -p 50051:50051 -p 8000:8000 \
  -e RUSTIFY_DATABASE_URL="postgres://saleor:saleor@host.docker.internal:5432/saleor" \
  ghcr.io/matinsanei/saleor-rustify:latest
# Modes: ...:latest worker|beat|check|migrate  (celery-worker / beat equivalents)
```

---

## 📊 Proof, not promises

Measured September 2026, release binary, local Postgres — methodology and full
history in [BENCHMARK.md](BENCHMARK.md):

```
binary:          ~60.5MB stripped (LTO + codegen-units = 1)
k6 (200 VU, 30s): 3.28k iters/s · gRPC p50 ~41ms · p95 ~96ms · 100% checks · RSS ~58MB
head-to-head:     ≈20× throughput · ≈19× latency vs CPython on catalog reads
```

| Signal | Value | How verified |
|---|---|---|
| GraphQL surface | **91 queries / 346 mutations live** (schema: 90/339 — every root resolves) | introspection diff vs `schema-main.graphql` |
| Dashboard compat | **458/458 operations validate** | `scripts/verify_dashboard_ops.py` |
| Behavioral contracts | **171/171 green** | `cargo test -p saleor-rustify-db` (money/stock/status vs Django rows) |
| Money math | `rust_decimal` everywhere, decimals cross the wire as strings | Django serializers parity |

---

## ✅ What works today

The stock dashboard runs end-to-end: browse → checkout → pay → fulfill →
refund, plus the whole back-office. Full ledger:
[docs/SALEOR_PARITY_CHECKLIST.md](docs/SALEOR_PARITY_CHECKLIST.md).

| Domain | Highlights |
|---|---|
| 🛒 Checkout | Full new API + legacy names, promos, delivery options, payment create, create-from-order |
| 💸 Money | Transactions (authorize/charge/refund/cancel/initialize/process/create), Stripe intents, legacy payments, granted refunds — one ledger, never diverging |
| 📦 Orders | Confirm/capture/refund/void/mark-paid, lines/discounts/notes, fulfillments + returns, drafts, 50-row bulk import |
| 🎁 Gift cards | Full CRUD, custom codes, tags, notes, resend, settings, bulk, CSV export |
| 🏷️ Marketing | Promotions + rules (engine-evaluated predicates), vouchers + codes, legacy sales (promotion-backed) |
| 🗂️ Catalog | Products/variants/types/categories/collections, attributes, media, bulk create/import, translates |
| 📄 Content | Menus (MPTT), pages + types + attributes |
| 🚚 Logistics | Zones/methods/listings, warehouses, channels + settings, tax classes + country rates |
| 👥 Accounts | Staff/customers/groups/addresses, auth + throttle + single-use tokens, validation rules, **real SMTP mail** |
| 🔌 Apps | Webhooks CRUD + outbox worker, app install (real manifest fetch) + tokens, plugins, exports (CSV) |
| 🧾 Ops extras | Invoices, refund/return-reason settings, order/invoice/discount events |

**Honest gaps** (each documented where it bites, never silent): multipart
upload (`fileUpload`, avatars), `checkouts` list query, Adyen + 29 gateways,
tax providers, XLSX/filtered exports, replacement orders, TLS mail auth.

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    saleor-rustify                       │
│                                                          │
│  crates/proto   Protobuf contracts (tonic build)         │
│  crates/core    Pure domain logic — Money · PSP · discounts│
│  crates/db      SeaORM over the REAL Django schema       │
│    147 entities 1:1 · same tables · same sequences       │
│  crates/graphql GraphQL BFF (thin + generated)           │
│    catalog · checkout · order · payment · commerce · apps │
│  crates/server  tonic gRPC (21 services) + Axum GraphQL   │
│    gRPC :50051 · GraphQL :8000 · metrics :9000            │
│    modes: api · worker · beat · check · migrate · seed    │
└──────────────────────┬──────────────────────────────────┘
                       │  RUSTIFY_DATABASE_URL
                       ▼
              PostgreSQL (Django-managed)
```

**Design rules:** GraphQL is a translation layer over the same `db` functions
gRPC uses — no duplicated money/stock math. `gen.rs` is generated from Saleor's
own schema and never hand-edited. Every write is permission-gated
(`manage_*`, superuser bypass, rotated-JWT revocation). Agents start at
[AGENT.md](AGENT.md) — it documents every trap this project has found
(stub-shadowing, dead impl blocks, money-bucket semantics).

---

## 💌 For the Saleor team

Yes — this is a hello. We rebuilt your engine in Rust because we believe in
the data model and the API design enough to bet a rewrite on them. Concretely:

- **What we want:** a conversation. Feedback on fidelity, blessing to keep the
  compat name (or we'll rename on request), and stability expectations for the
  plugin/webhook surface we build against.
- **What we offer:** head-to-head performance data on your own schema,
  a second implementation that pressure-tests API edge cases, and Rust-side
  learnings (predicate evaluation, bucket recalculation) portable back to
  CPython.
- **What we won't do:** pretend affiliation, fork the trademark, or ship a
  "compatible" badge over stubs — every gap is labeled in the checklist.

Saleor core is BSD-3-Clause; this project copies no Saleor code (clean
reimplementation against the published GraphQL schema and SQL tables) and is
licensed AGPL-3.0. Reach us via GitHub issues at
[matinsanei/saleor-rustify](https://github.com/matinsanei/saleor-rustify).

---

## 🗺️ Roadmap

- [x] Workspace + dual-stack skeleton (gRPC + GraphQL via Axum)
- [x] Domain core with decimal math (Money/Product/Checkout/Order)
- [x] SeaORM entities 1:1 from live Saleor PostgreSQL (sequence parity)
- [x] Checkout writes + order persistence on Django tables
- [x] Product relations (MPTT trees, listings, types, attributes, media)
- [x] Commerce services (discount/shipping/giftcard/menu/page/account/channel/tax/warehouse)
- [x] Payments (event-group recalc, PSP dedup/async/3DS, Stripe intents)
- [x] Webhooks (fan-out, HMAC, attempts, backoff, sweeper) + worker/beat modes
- [x] Auth (PBKDF2/bcrypt, RS256, throttling, single-use tokens, app tokens)
- [x] Fulfillment lifecycle + granted refunds + draft orders + bulk import
- [x] AI layer (`saleor-rustify-ai`): pg_trgm search, co-occurrence recommender, grounded chat
- [x] Enterprise wave: apps, CSV exports, translates, invoices, SMTP, 90+ staff mutations
- [ ] Multipart upload (`fileUpload`, avatars) + `checkouts` list
- [ ] Second gateway (Adyen) + tax provider bridge
- [ ] Storefront SDK v2 generated from protos

---

## 🤝 Contributing

PRs welcome. Rules: **no `float` for money · no `SELECT *`** (tsvector/INTERVAL
mistype) **· every domain PR ships contract tests · every behavior cites the
Saleor test it mirrors.** Read [AGENT.md](AGENT.md) first — it will save you
from every trap we already found. Details: [docs/SALEOR_PARITY_CHECKLIST.md](docs/SALEOR_PARITY_CHECKLIST.md).

## ⚖️ License

AGPL-3.0 — free as in freedom, forever. See [LICENSE](LICENSE).
