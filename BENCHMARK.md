# 📊 rustygod-saleor Benchmarks

Numbers or it didn't happen. All figures below are reproducible with the
`bench` example or the k6 suite (`bench/k6-grpc.js`) against a local Saleor
PostgreSQL (`populatedb` data, 52 orders).

## Setup

- **Binary:** release profile, ~95MB stripped (Sep 2026; the old 20MB figure
  predates the wasmtime/aws-lc/codegen deps). `[profile.release]` now sets
  `strip + lto + codegen-units = 1` — rebuild in background, it takes a while.
- **DB:** PostgreSQL on `127.0.0.1:5434`, Saleor `populatedb` dataset
- **Machine:** dev laptop, 15GB RAM — server, k6 and Postgres share it
  (single-machine numbers; a split setup would read better)
- **Workload:** 70% `ListProducts`, 20% `SemanticSearch`, 10% `GetProduct`
- **Commands:** `k6 run bench/k6-grpc.js` · `K6_SCENARIO=spike k6 run bench/k6-grpc.js`

## Results (2026-09-18)

### Native bench (release, persistent conns)

```
idle:               RSS ~8MB
load  (20 conn):    ~5,400 rps · p50 3.8ms · p99 6.7ms · max 38ms · errors=0 · RSS →21MB
spike (1000 conn):  ~5,400 rps · p50 194ms · p99 275ms · errors=2/108k · RSS →101MB
cool  (20 conn):    ~5,000 rps · p50 4.1ms · instant recovery
```

### k6 suite (independent harness, persistent conns per VU)

```
load  (200 VU, 30s):  93,886 iters · 100% checks ·
                      grpc p50 ~48ms · p95 ~99ms · 3.1k iters/s
spike (1000 VU, 40s): 115,808 checks · 100% pass · 0 failed ·
                      grpc p50 ~239ms · server RSS 109MB
docker (10 VU smoke): p50 2.8ms · p95 8.7ms · 100% (23.6MB image)
```

### Re-run 2026-09-22 (LTO release, 60.5MB binary, same laptop + DB)

```
load  (200 VU, 30s):  98,575 iters · 100% checks · 3.28k iters/s ·
                      grpc p50 ~41ms · p95 ~96ms · server RSS ~58MB
```

Notes: binary 119MB → **60.5MB** (`strip + lto + codegen-units = 1`); throughput
p50/p95 slightly better than 09-18 at half the RSS. The `p(99)<50ms` roadmap
threshold still trips on catalog joins (DB-bound, unchanged) — read replicas /
caching remain the lever, not codegen.

### Target scorecard (Phase 2 exit criteria)

| Metric | Target | Measured | Verdict |
|---|---|---|---|
| catalog p99 (20 conn) | < 50ms | 6.7ms | ✅ |
| sustained rps | 20k+ | ~5.4k (DB-bound) | ❌ — Postgres joins cap it; read replicas / caching next |
| RSS @ 1k conns | < 256MB | 101–109MB | ✅ |
| spike errors | 0 | 2/108k (0.002%) | ~ pool timeouts at 1000 conn |
| Docker image | < 100MB | 23.6MB | ✅ |
| k6 suite published | yes | `bench/k6-grpc.js` | ✅ |

Debug-build reference (same machine, pre-fix): ~400 rps @ p50 ~150ms.

## What moved the needle (6x throughput, 8x latency)

1. **Batched catalog reads** — `ListProducts` went from N+1 queries
   (1 + products × (variants + listing + stock)) to exactly **3 round-trips**
   (variants + listings + stocks), assembled in memory. Same pattern as
   Django's dataloaders, but explicit and auditable.
2. **Chat grounding without full scans** — `products_by_ids` instead of
   `list_products(10_000)`.
3. **DB pool 16 → 64** (+22% throughput before the batching fix).
4. **Release profile** (~5x vs debug).

## Honest limits

- These are **catalog/search reads**; write-heavy flows (checkout complete,
  promotions) are covered by contract tests, not yet by load tests.
- No head-to-head Django benchmark is claimed here — running one fairly
  (same data, same machine, warmed caches) is a Phase-2 milestone, and the
  report will be published here either way.
- The 2 spike errors (0.002%) are pool timeouts at 1000 concurrent
  connections — next: per-query shaping + pool autosizing.
- rps is currently DB-bound (~5.4k ceiling = Postgres doing the joins),
  not Rust-bound. That is exactly where you want the bottleneck.

## Targets (Phase 2)

| Metric | Now | Target |
|---|---|---|
| catalog p99 (20 conn) | 6.7ms | < 5ms |
| sustained rps | ~5.4k | 20k+ (batching + read replicas) |
| RSS @ 1k conns | 101MB | < 256MB |
| spike errors | 0.002% (pool timeouts @1000 conn) | pool autosizing |
| Django head-to-head | — | published, win or lose |

## Observability (Phase 2)

- Every RPC emits a trace span (`TraceLayer::new_for_grpc`) and Prometheus
  metrics: `grpc_requests_total{service,method}`,
  `grpc_request_duration_seconds{service,method}` (with quantiles).
- Scrape: `RUSTYGOD_METRICS_ADDR` (default `127.0.0.1:9000`).
- Honest limit: tonic puts the RPC status in trailers, so the tower layer
  labels transport-level signals only — business errors stay explicit in
  each response's `errors` field.

## Docker (Phase 2)

- `docker build -t rustygod-saleor .` → **23.6MB** (busybox:glibc + stripped binary)
- `docker compose up --build` → Postgres + server (:50051 gRPC, :9000 metrics)
- The DB starts empty; restore Saleor data with `./scripts/db.sh restore <dump>`
  or run `migrate` + `populatedb` from saleor-core.

## Backup / restore

- `./scripts/db.sh backup [name]` → `backups/*.dump` (custom format, verified 780K roundtrip)
- `./scripts/db.sh restore <dump>` → drop + recreate + `pg_restore` (validated on a scratch DB: 52 orders back)
- The live DB is reached over TCP (`PGHOST`/`PGPORT`, default `127.0.0.1:5434`).

## Reproduce

```bash
cargo build --release -p rustygod-server --bin rustygod-server --example bench
./target/release/rustygod-server &          # needs RUSTYGOD_DATABASE_URL
./target/release/examples/bench load 200 20
./target/release/examples/bench spike 1000 20
k6 run bench/k6-grpc.js                     # 200 VU load, p99<50ms threshold
K6_SCENARIO=spike k6 run bench/k6-grpc.js   # 1000 VU spike
```
