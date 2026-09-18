# 🗺️ rustygod-saleor Roadmap: what Saleor couldn't build

**Goal:** a real Rust commerce engine that beats Saleor where it failed —
performance, size, frontend experience — without regrowing its 116k-line
GraphQL monster.

## What Saleor got wrong (and our answers)

| Saleor's failure | Root cause | Our answer |
|---|---|---|
| Slow API (Django resolvers, N+1 everywhere) | Per-field Python resolvers + ORM | gRPC core + batched SQL (SeaORM loaders); thin gateway only at the edge |
| 2GB+ RAM per node | CPython + Django footprint | Single static binary, target <256MB RSS per node |
| Frontend chained to a 39k-line schema | Dashboard/storefront co-designed with GraphQL internals | Stable versioned API (gRPC + generated TS clients); storefront talks to contracts, not internals |
| Plugins nobody uses | Python-only, in-process, no isolation, painful upgrades | **Out-of-process plugins**: WASM (any language) for hot paths + webhooks for async — JS/TS community welcome |
| Permission sprawl (per-field checks) | Object-level checks scattered in resolvers | Central policy engine: one `CheckPermission` path, capabilities not spaghetti |
| Upgrade fear (monolith) | Extensions coupled to core releases | Strangler-compatible DB + versioned protos: old clients keep working |

## Architecture target

```
┌──────────────┐   ┌──────────────────┐   ┌───────────────────┐
│  Storefront  │   │   Dashboard v2   │   │  JS/TS Apps       │
│  (edge-first │   │   (generated     │   │  (webhooks +      │
│   TS client) │   │    from protos)  │   │   app bridge)     │
└──────┬───────┘   └────────┬─────────┘   └─────────┬─────────┘
       │ gRPC-Web / BFF     │                       │ webhooks
       ▼                    ▼                       ▼
┌──────────────────────────────────────────────────────────────┐
│                    API GATEWAY (thin)                        │
│        auth · rate-limit · policy check · proto routing      │
└──────────────────────────────┬───────────────────────────────┘
                               │ tonic gRPC
┌──────────────────────────────▼───────────────────────────────┐
│                      RUST CORE (this repo)                   │
│  catalog · checkout · orders · promotions · payments · auth  │
│  fulfillment · webhooks · AI (search/reco/chat)              │
│                     ▲                  ▲                     │
│              WASM plugins        webhook plugins             │
│         (tax/pricing/validate)   (async, any language)       │
└──────────────────────────────┬───────────────────────────────┘
                               │ SeaORM
                               ▼
                    PostgreSQL (Saleor schema)
```

## Plugin strategy (where the big community lives)

1. **WASM first (extism):** hot-path extensions (tax calc, price rules,
   checkout validation, fraud checks) run in-process sandboxed modules.
   Authors write **JavaScript/TypeScript, Python, Go** — compiled to WASM.
   No Rust knowledge required, no core redeploys, crash-isolated.
2. **Webhooks second:** async events (order created, payment authorized…)
   with HMAC, retries, dues — already shipped. Any stack can consume.
3. **Never:** in-process Rust plugins (that's how you get a monolith
   with extra steps and zero community).

## Permissions & API done right

- Permissions = **capabilities on verbs** (`order.fulfill`, `product.edit`),
  checked once at the gateway + service edge — never per-field resolver soup.
- Mutations = gRPC verbs with explicit error codes (what Saleor's
  `error_codes.py` wanted to be, without the GraphQL tax).
- Keep a **GraphQL compatibility gateway LAST** (only if dashboard-compat
  demands it) — generated from protos, never hand-written resolvers.

## Phases & exit criteria

### Phase 1 — Core parity (NOW, ~50% → 80%)
- [x] Catalog, checkout, orders, promotions, payments, webhooks, auth, fulfillment, AI-v1
- [x] Giftcard issue/redeem, draft orders, invoice records (123 tests green)
- [x] Allocation-table stock model (draft complete writes `warehouse_allocation`)
- [ ] Promotion gifts + order promotions on checkout totals
- [ ] **Exit:** 120+ contract tests green ✓, smoke covers full buy flow

### Phase 2 — Production hardening (DONE 2026-09-18)
- [x] Tracing spans + Prometheus metrics on every RPC (`telemetry.rs`, `:9000`)
- [x] k6 load suite (`bench/k6-grpc.js`): 200VU/100% + 1000VU spike/100%, numbers in BENCHMARK.md
- [x] Docker image **23.6MB** (<100MB ✅) + compose one-command demo
- [x] Backup/restore story (`scripts/db.sh`, scratch-DB validated)
- [x] Reproducible benchmark report in BENCHMARK.md (incl. missed targets, honestly)
- [ ] Follow-ups → Phase 3: OTLP export, 20k rps (read replicas/caching), pool autosizing

### Phase 3 — Plugin platform (IN PROGRESS)
- [x] extism WASM runtime (`crates/plugins`: closed-world capabilities + extension points)
- [x] Reference plugin #1: flat-rate tax in hand-written WAT (no toolchain, HALF_UP cents)
- [x] `PluginService` gRPC (register/call/list/tax, `manage_apps`) + `sdk/plugin.mjs` sketch
- [x] Capability-based permission checks on every mutating RPC (Phase 2.5 access layer)
- [ ] Reference plugins #2–3 (min-order validator, Slack notifier via webhooks)
- [ ] Plugin persistence (registry is in-memory today) + per-point storefront access
- [ ] **Exit:** a JS dev ships a plugin without touching Rust

### Phase 4 — Frontend that beats Saleor
- [ ] Generated TS clients from protos (single source of truth)
- [ ] Edge-first storefront (ISR product pages, <100ms TTFB target)
- [ ] Dashboard v2: generated CRUD from protos, custom views only where valuable
- [ ] **Exit:** demo shop faster than storefront.saleor.io on every page

### Phase 5 — AI that sells (the moat Saleor lacks)
- [ ] pgvector embeddings (replace trigram tier, same RPC)
- [ ] Recommender v2 (session + collaborative signals)
- [ ] Agentic buying: natural-language checkout over the same verbs
- [ ] **Exit:** A/B shows conversion lift, or it gets cut

## What we will NOT build
- A hand-written GraphQL layer (the 116k-line trap)
- In-process Rust plugins
- Feature parity for parity's sake (CSV export #47 can wait; checkout speed cannot)

## Metrics that matter
- p99 latency per RPC (k6, published)
- RSS per node at fixed rps
- Contract tests green (behavioral lock vs Django)
- Time for a JS dev to ship first plugin (target: <1 day)
- Demo-shop Lighthouse + conversion in A/B
