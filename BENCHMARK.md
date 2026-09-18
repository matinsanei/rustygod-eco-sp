# 📊 rustygod-saleor Benchmarks

Numbers or it didn't happen. All figures below are reproducible with the
`bench` example against a local Saleor PostgreSQL (`populatedb` data).

## Setup

- **Binary:** release profile, 20MB static-ish binary (`target/release/rustygod-server`)
- **DB:** PostgreSQL 15 (Alpine, Docker `--network host`), Saleor `populatedb` dataset
- **Machine:** dev laptop, 14GB RAM (see `free -h` at runtime)
- **Client:** `crates/server/examples/bench.rs` — native tonic clients,
  mixed workload per connection: 70% `ListProducts`, 20% `SemanticSearch`,
  10% `GetProduct`
- **Command:** `bench load [conns] [secs]` · `bench spike [conns] [secs]`
  (base 20/10s → spike → cool 20/10s)

## Results (2026-09-18)

```
idle:               RSS ~8MB
load  (20 conn):    ~5,400 rps · p50 3.8ms · p99 6.7ms · max 38ms · errors=0 · RSS →21MB
spike (1000 conn):  ~5,400 rps · p50 194ms · p99 275ms · errors=2/108630 · RSS →101MB
cool  (20 conn):    ~5,000 rps · p50 4.1ms · instant recovery
```

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
| spike errors | 0.002% | 0 |
| Django head-to-head | — | published, win or lose |

## Reproduce

```bash
cargo build --release -p rustygod-server --bin rustygod-server --example bench
./target/release/rustygod-server &          # needs RUSTYGOD_DATABASE_URL
./target/release/examples/bench load 200 20
./target/release/examples/bench spike 1000 20
```
