# STATUS: Saleor Dashboard parity via enterprise codegen

> **For the next AI:** this file is the complete handoff. Read it fully before
> touching anything. The working tree is **mid-surgery and may not compile** —
> start with `cargo check -p saleor-rustify-graphql` and work from the harness.

Date: 2026-09-20. Goal: stock Saleor Dashboard works against the Rust backend
with **zero schema-validation errors on every page** — no hand-porting of
Saleor's Python, no backend slowdown.

## 1. Environment (do not guess — use exactly this)

| Item | Value |
|---|---|
| Our repo | `/home/matin/Desktop/dev/saleor-rustify` (binary crate `saleor-rustify-server`) |
| Saleor core (reference) | `/home/matin/Desktop/dev/saleor/saleor-core` (`__version__ = "3.24.0-a.0"`, schema 3.24) |
| Dashboard src (reference) | `/home/matin/Desktop/dev/saleor/dashboard` (v3.23.33 sources) |
| Dashboard running | docker `sad_williamson`, `ghcr.io/saleor/saleor-dashboard:3.23.33`, `http://localhost/` (= port 80), `API_URL=http://localhost:8000/graphql` |
| Postgres | `saleor-db` on `localhost:5434`, db/user/pass `saleor`; env `RUSTIFY_DATABASE_URL=postgres://saleor:saleor@localhost:5434/saleor` |
| Ports | gRPC `127.0.0.1:50051`, GraphQL `127.0.0.1:8000/graphql`, metrics `:9000` |
| Auth | staff JWT RS256, `RSA_PRIVATE_KEY` = `crates/db/tests/testdata/test_rsa.pem`; login `admin@example.com` / `admin` |
| Permissions | `Permission.code` is the **enum NAME** (`MANAGE_PRODUCTS`), never the DB codename — `common::permission_enum_code` (exact 25-map from `saleor/permission/enums.py`). Lowercase codes hide the whole Dashboard sidebar (full menu verified in headless browser: Catalog/Fulfillment/Customers/Discounts/Modeling/Translations/Configuration all render) |
| Server management | kill by PID file (`kill -9 $(cat /tmp/rust.pid)`); **NEVER `pkill -f`** (kills your own shell). Start: `RUST_LOG=info RUSTIFY_DATABASE_URL=... RSA_PRIVATE_KEY="$(cat ...)" setsid ./target/debug/saleor-rustify-server </dev/null >/tmp/rust.log 2>&1 < /dev/null & disown` (the `&` + setsid + stdin redirect is required or the tool hangs) |
| Query logging | `server/src/main.rs` logs first 500 chars of every GraphQL query at info level (`RUST_LOG=info` required; without it `/tmp/rust.log` stays empty) |
| Seed data | DB wiped 2026-09-20 and rebuilt purely from Saleor (`DROP SCHEMA public CASCADE` → `migrate` → `seed --createsuperuser`; backup `/tmp/saleor_backup_full.dump`): 145 tables, 29 products, 20 orders, 45 users, 2 channels. **Never re-run populatedb** (would duplicate). Empty admin first/last name is **upstream Saleor behavior** (`dangerously_get_or_create_superuser` sets no names) — Django shows the same blank profile, not our bug |
| Dashboard schema mode | Release build defaults to **main-schema mode** (`FF_USE_STAGING_SCHEMA` unset). Our backend mimics 3.24, so we support the **superset** (see `LEGACY_FIELDS`) |

## 2. Architecture (unchanged)

- gRPC core (`saleor-rustify-db` + `saleor-rustify-core`) is source of truth; GraphQL
  (`crates/graphql`) is a thin BFF for the Dashboard (Apollo): same Postgres,
  same domain functions, shape-only translation, `Authorization: Bearer` auth.
- SeaORM rules (hard): **never `SELECT *`** on tables with `tsvector` /
  `INTERVAL` (`search_vector`, `delete_expired_orders_after`,
  `checkout_ttl_before_releasing_funds`); slim `select_only` + per-row fetch.
  Exception: `app_app` full-model select is safe (no exotic columns).
- Money on the wire is **strings** (`rust_decimal` → string, like Saleor's
  Decimal scalar). Lock order: checkout → stocks asc id → voucher → giftcard.
- Runtime modes (`crates/server/src/modes.rs`, no CLI dep) mirror Saleor's
  processes: `api` (default, uvicorn equivalent: gRPC + GraphQL + metrics +
  embedded sweeper), `worker` (celery-worker equivalent: foreground webhook
  outbox loop, same `sweeper::tick_once`, `RUSTIFY_WORKER_SECS` default 10),
  `beat` (celery-beat equivalent: all 20 `CELERY_BEAT_SCHEDULE` entries with
  Saleor task names/intervals; only `delete-expired-reservations` is real,
  rest are deferred stubs; `RUSTIFY_BEAT_SCALE` multiplies intervals for
  dev),   `check` (readiness probe: `SELECT 1` + row counts, exit 0/1).
  compose has matching `server`/`worker`/`beat` services.
- DDL + seed data are **never re-guessed**: `migrate` / `seed` /
  `createsuperuser` delegate to saleor-core's own `manage.py` (venv python,
  `RUSTIFY_DATABASE_URL` mapped to `DATABASE_URL`, stdio inherited, exit
  code propagated; `SALEOR_CORE_DIR` override). Verified: `migrate --check`
  exit 0 on the live DB.

## 3. What was done this session

### 3a. Boot fixes from live traffic (all verified working)

- `products.totalCount`, `orders` tsvector crash fix, `Shop` expansion
  (name/version/domain/countries/languages/permissions/limits/metadata/
  announcements from shared Postgres rows), `Address` full shape,
  `apps`/`appExtensions` from real `app_app` rows (empty here — correct),
  `accountUpdate` + `shopSettingsUpdate` persisting metadata JSON,
  `AppFilterInput`/`AppExtensionFilterInput` exact Saleor shapes
  (`mountName`, not `mount` — this was breaking sidebar nav).

### 3b. Full-schema examination (`scripts/schema_examine.py`)

Saleor `schema.graphql` (38,757 lines) is uniform codegen output → exact
text parsing (regex + brace matching). Results: **89 query roots, 331
mutation roots, 902 types, 312 inputs, 208 enums, 13 unions**. Dashboard:
**431 operations, 293 unique roots, 494 touched types** (684 docs,
257 fragments). We had 47 resolvers (~19 roots). Artifacts: `/tmp/schema_ir.json`,
`/tmp/dash_ops.txt`, `/tmp/mutation_roots.txt`. Parser bug history (fixed):
`@doc(...)` parens skipping types, `"""` description lines parsed as enum
values, brace counting inside description strings, union single-line bodies.

### 3c. Codegen (`scripts/schema_codegen.py` → `crates/graphql/src/gen.rs`)

Generates **only dashboard-selected fields** (usage ∩ schema; ~500 structs,
4 real interfaces, 8 unions, 272 roots, ~250 inputs, ~90 enums, ~500 KiB,
0 warnings). Key design (all learned the hard way, in `STATUS.md` history):

- Outputs relaxed-nullable; stubs return `None`/`[]`/empty connections =
  **zero DB cost**. Inputs/args exact (variable coercion).
- Hand types that were incomplete got **REPLACED** (constructors preserved):
  `Product`, `ProductVariant`, `Order`, `OrderLine`, `User`, `Channel`,
  `App` (+`AppProblem/Token/Webhook/EventDelivery*`), `Address` kept but
  extended. Kept types extended cheaply: `Money.fractionDigits`,
  `TaxedMoney.currency`, `Image.alt`, `UserPermission.sourcePermissionGroups`,
  `Address.metadata/privateMetadata`.
- `#[graphql(complex)]` is **mandatory** or `ComplexObject` methods silently
  vanish (one missing flag hid 94 fields: `avatar`, `thumbnail`, …).
- Interfaces via `method="gen_iface_*"` + `pub` backers (direct field access
  needs `From<&T>` bounds std lacks). `Node`, `ObjectWithMetadata`,
  `AssignedAttribute`, `PaymentMethodDetails`.
- `Box` cycle-breaking via DFS (struct cycles AND union/interface member
  edges; box the back edge itself). Recursive inputs would need the same
  (not yet needed after slimming `AddressInput`).
- `argid()` (`_arg_where`) — `_` + raw-ident (`_r#where`) is illegal Rust.
- `split_args` must accept leading `[` (one-char bug dropped every
  `[ID!]!` arg: `ids/moves/products/variants`).
- Enum `pascal()` must dedupe; struct with zero plain fields →
  unit-struct + `#[Object]` (e.g. `AppBrandLogo`); zero-selected structs get
  `id/cursor` fallback (else "must define one or more fields").
- `payload` ctors skip methods; mutation roots return `Option<payload>`;
  connection ctors adapt to kept (by-value `PageInfo`) vs gen (`Option`).
- Custom scalars declared wire-compatible: `Decimal`, `PositiveDecimal`,
  `JSONString` (strings), lenient `WeightScalar`, `Upload` stub (multipart
  deferred — documented gap, not wired).
- `LEGACY_FIELDS`: main-schema (3.23) fields removed in 3.24 the released
  dashboard still selects: `Attribute.filterableInStorefront/availableInGrid/
  storefrontSearchPosition`.
- `op_roots` registers **all** roots per operation (multi-root ops like
  `UpdateShopSettings{shopSettingsUpdate+shopAddressUpdate}` and
  `updateMetadata{update+delete}` — first-root-only silently skipped 8 roots).
- `KEPT` (never redefine — GraphQL name clash panics at runtime):
  `AccountError, Address, Announcement, AppBrand(Logo), App*Connection,
  CountryDisplay, CreateToken, LanguageDisplay, Limit*, MetadataItem,
  Order/ProductCountableConnection, PageInfo, Permission, ShopSettingsUpdate,
  StockSettings, UserPermission, EventDelivery*Connection, Money, TaxedMoney,
  Image, Query, Mutation` (+ paths in `KEPT_PATHS`).
- `KEPT_ENUMS` → hand paths (`AppTypeEnum`, `IconThumbnailFormatEnum`);
  `KEPT_INPUTS` → `MetadataInput`; hand `ProductCreateInput` **deleted**,
  catalog uses `gen::ProductCreateInput`.
- `@lockSchema` is **client-side**: dashboard strips the directive pre-flight
  (verified `dashboard/src/graphql/lockSchema.ts` + release bundle default
  main). Harness mirrors stripping; only `main`-locked fields exist and are
  kept (superset strategy above).

### 3d. Verification harness (`scripts/verify_dashboard_ops.py`)

Runs **all 431 dashboard docs** (fragment closures, `{}` vars) against live
backend. Ignores missing-variable/auth errors; fails on `Unknown
field/type/argument/directive`, bad spreads. Progress:
**272 → 123 → 43 failed (of 457 tested)**. One-line wins en route: the
`complex` flag (94 fields), the `[` arg fix (all list args).

## 4. Current state (2026-09-20, end of session)

- `cargo check -p saleor-rustify-graphql`: **clean, 0 errors, 0 warnings** (last verified).
- Server binary builds; last running pid `764265` serves an **older** binary
  (rebuild + restart required to pick up latest gen + surgery).
- Harness: **457 tested, 18 failed** (`/tmp/harness3.txt` has the full list).
- Working tree is **MID-SURGERY (may not compile right now)** — commerce
  `shop()` was just switched to `gen::Shop`; see §6.
- Existing tests: `crates/graphql/tests/schema_sdl.rs` is behavioral
  (needle checks + no-panic), not a snapshot — keep it green.

## 5. Remaining work (the 43 — all root-caused)

### 5a. Finish shop surgery (in flight, highest value)

`commerce.rs::shop()` now returns `gen::Shop`, but:
1. **OPEN QUESTION — `countries` missing from `gen::Shop`.** Codegen's own
   usage includes `countries`, yet the emitted struct lacks it. Prime
   suspects: (a) `struct_fields()` filter vs `types['Shop']['fields']` key
   mismatch, (b) stale `gen.rs` vs fixed codegen (regen and diff!),
   (c) `M['structs']` vs emission divergence. Debug: re-run
   `schema_codegen.py --emit`, grep `countries` in `Shop` struct; if still
   absent, print `struct_fields('Shop')` vs `g.usage['Shop']`. Until fixed,
   `ShopInfo` (countries) FAILS — worse than before. Fallback: add
   `countries: Vec<GqlCountryDisplay>` via `LEGACY_FIELDS`-style injection.
2. `shopSettingsUpdate` payload still hand `{GqlShop, GqlError}` → change to
   `{ shop: Option<gen::Shop>, errors: Vec<gen::ShopError> }` (covers both
   nav-pins `UpdateShopNavigationPins` and `OrderSettingsUpdate`, which also
   needs `...ShopOrderSettings` = fields already in `gen::Shop`).
3. Delete now-dead hand `GqlShop`/`GqlDomain`/etc. once nothing references
   them (compiler will list).

### 5b. Hand-root signatures still mismatched (~15 failures)

- `products`: needs `where: ProductWhereInput` — **already added** (verify compiles).
- `orders`: `where/sortBy/search` — **already added** (filter dropped; Saleor
  keeps deprecated `filter: OrderFilterInput` which dashboard never sends —
  do NOT re-add unless harness demands).
- `pages`/`warehouses`/`taxClasses`/`promotions`/`menus`: converted to gen
  connections + gen filter/sort args — **already rewritten** (verify compiles).
- `orderFulfill` (hand, `order.rs`): dashboard sends `(order: ID,
  input: OrderFulfillInput!)`, ours takes `(order_id, lines)` → rewrite to
  `order: Option<ID>, input: gen::OrderFulfillInput`, adapt body
  (`input.lines: [OrderFulfillLineInput] {orderLineId, stocks}`,
  `notifyCustomer`, `allowStockToBeExceeded`).
- Confirm `checkouts`/`transaction`/`menu`/`channel`/`user`/`staffUsers`
  roots match dashboard args (harness will confirm).

### 5c. Unreferenced-type pruning (~12 failures)

async-graphql drops types unreachable from roots. Inputs like
`PageFilterInput`, `OrderFulfillInput`, `WarehouseFilterInput`,
`MenuSortingInput`, `BulkStockError`, and interface `Node` exist in `gen.rs`
but are "Unknown type" because **hand roots don't reference them**.
Fix = §5b (hand roots take gen input types → they register). For `Node`
(`... on Node` under `updateMetadata.item`): ensure `Node` is reachable —
options: type `item` as `Node`, or reference `Node` from any reachable
position. Verify overlap(`ObjectWithMetadata`, `Node`) holds via shared
implementors (see `type_overlap` semantics in §3c).

### 5d. Shop gaps already covered by `gen::Shop` (verify after §5a)

`staffNotificationRecipients`, `availablePaymentGateways`,
`channelCurrencies`, `ShopError`/`Shop` fragment spreads — all generated;
they fail today only because `shop()` still served hand `GqlShop` at harness
time... (note: harness2 ran against binary predating the shop() switch —
re-verify after rebuild).

### 5e. After the above: rerun gate, then wire real data (priority order)

```bash
cargo check -p saleor-rustify-graphql          # must be 0/0
cargo build -p saleor-rustify-server && restart # §1 for exact restart recipe
python3 scripts/verify_dashboard_ops.py   # must print FAILED=0
cargo test -j2 -p saleor-rustify-graphql --test schema_sdl
```

Then, page by page (highest traffic first: products → orders → customers):
replace stub bodies with slim real queries reusing `saleor-rustify-db::*`
(check `crates/db/src/` for existing fetchers before writing SQL).
Candidates already fetched but unexposed: variant `price` (lives in
`pricing`/channelListings stubs), order `payments`/`fulfillments`,
user `addresses`/`orders`, category/product `products` connections
(resolvers exist in gen as `None` — fill them).

### 5f. Explicit non-goals (documented gaps, do not chase blindly)

- `Upload` multipart handling (`fileUpload` validates, execution deferred).
- Server-side filtering/sorting — DONE 2026-09-22 for products/orders/
  customers (see §8): search/ids/slugs/name/type/category(subtree)/
  collection/isPublished/hasCategory + AND/OR (products); status/ids/number/
  email/created/channels + sort (orders); search/ids/email/name/isActive/
  dateJoined + sort (customers). Still ignored: price/attribute/stock/date/
  metadata sub-filters, order AND/OR nesting, payment-state pseudo-filters.
- `problems`/`webhooks.eventDeliveries` (always empty = healthy).
- Announcements feed, brand logos, avatar binaries (empty/None).
- Payment-gateway mutations beyond stubs (need PSP decisions per gateway).

## 6. Files inventory

Added: `crates/graphql/src/gen.rs` (~500 KiB, machine-written),
`crates/graphql/src/apps.rs`, `scripts/schema_examine.py`,
`scripts/schema_codegen.py`, `scripts/verify_dashboard_ops.py`, this file.
Modified: `catalog.rs` (gen Product/Variant + `ProductCreate` payload),
`order.rs` (gen Order/Line/Channel, `to_gen_order`, `OrderCancel` payload),
`account.rs` (gen User via `to_gen_user`, `Image.alt`,
`UserPermission.sourcePermissionGroups`),
`commerce.rs` (gen Channel/ connections/pages/menus/taxClasses/promotions,
`to_gen_shop` in flight), `apps.rs` (gen App nodes, gen filter inputs),
`common.rs` (`Money.fractionDigits`, `MetadataItem/Input`, merge helpers),
`order.rs` `GqlAddress` (+metadata, `complex`, iface backers),
`schema.rs` (+`GenQuery`/`GenMutation`), `lib.rs` (+`gen` mod),
`server/src/main.rs` (query logging only).

## 8. Server-side filtering (2026-09-22)

- DB layer (`crates/db/src/catalog.rs`): `parse_gid` (plain int or Saleor
  global ID), `ProductListFilter`, `list_products_filtered` /
  `product_ids_filtered` (fast ids-only path for AND/OR branches), MPTT
  category subtree (self + descendants, like Saleor), collection m2m,
  channel-listing publication flag, ILIKE + tsvector search.
- GraphQL: `catalog.rs` translates ProductFilterInput/Where (+AND/OR via
  branch id-set intersect/union) and ProductOrder NAME; `order.rs` builds
  SeaORM Conditions (status/ids/number/email/created/channels/search/sort);
  `account.rs` hand `customers` root (non-staff + search/filter/sort;
  gen stub dropped by regen via `ours`); `User.orders` is a REAL_METHODS
  codegen entry (COUNT by user_id).
- tsvector/INTERVAL rule still holds: `account_user.search_vector` crashed
  the customers list — slim selects only.
- Infra note: `/tmp/*.json` (schema_ir, our_fields) does NOT survive
  reboots — regen via `schema_examine.py --json-out`; `our_fields.json`
  now lives in-repo (`scripts/`) with `dump_our_fields.py`; codegen
  PROBLEMS back to 0.

## 9. Detail roots + media (2026-09-22)

- Hand roots (gen stubs dropped via `ours`): `product(id,slug,channel)`,
  `category(id,slug)`, `collection(id,slug)`, `user(id,email,
  externalReference)` — detail pages open with real rows.
- `Promotion.type` uppercased (DB stores lowercase; dashboard switches on
  CATALOGUE/ORDER and throws otherwise — was the discounts crash).
- Thumbnails real: `REAL_METHODS` codegen entries for `Product.thumbnail`
  + `OrderLine.thumbnail` (first product image → `/media/<path>`);
  `/media/` served by Axum from `SALEOR_MEDIA_DIR`
  (default `../saleor/saleor-core/media`); `SALEOR_MEDIA_URL` prefix.
- tsvector mistype killed at the root: removed `search_vector` from all 6
  entity Models (account_user, order_order, page_page, giftcard_giftcard,
  product_product, checkout_checkout) — full-model selects can no longer
  crash on it; nothing read the column (WHERE-only raw SQL unaffected).
- ID format decision: plain-int IDs stay (dashboard treats IDs opaque and
  round-trips via `parse_gid`, which accepts Saleor global IDs too).
  Migrating all output to base64 global IDs is mechanical but touches
  ~25 resolvers + tests for zero functional gain today — deferred,
  overrulable.

## 7. Regeneration contract (or gen.rs rots)

`gen.rs` is **never hand-edited**. All shape fixes go into
`scripts/schema_codegen.py` (`LEGACY_FIELDS`, `KEPT*`, `REPLACE`-era logic,
scalar maps) + re-run `--emit`. If dashboard upgrades (3.24+), re-run
`schema_examine.py` (refresh `/tmp/schema_ir.json`), then codegen, then the
harness — new gaps appear as harness failures by construction.

## 10. Codegen input = dashboard's own schema (2026-09-22)

- Both scripts now read `SALEOR_SCHEMA` env, defaulting to
  `saleor/dashboard/schema-main.graphql` (main-schema mode = what the
  released container runs) — NOT saleor-core's 3.24 `schema.graphql`.
  Proven skew: `filterableInStorefront/availableInGrid/
  storefrontSearchPosition` exist in main but were already dropped in 3.24
  (the entire reason LEGACY_FIELDS existed). With the dashboard schema as
  input the legacy branch is a no-op safety net, not load-bearing.
- New schema stats: 90 query / 339 mutation roots, 911 types, 318 inputs,
  209 enums. Regen is clean (PROBLEMS=0, harness 457/457).
- Note: the dashboard clone is a grafted shallow checkout (main tip,
  package.json 3.23.33) — schema-main is stable-branch content, safe.
- Scalar fix (same session): `Decimal/PositiveDecimal/JSONString/
  WeightScalar` now CARRY their strings (tuple structs) — the old unit
  structs silently dropped every price/description input, which made
  variant price writes impossible.

## 11. Translations wired (2026-09-22)

- `translations(kind)` list (10 table-backed kinds, paginated) +
  `translation(kind, id)` single, returning `TranslatableContent` unions
  with real nested entities.
- `translation(languageCode:)` resolves for real on all 10 entity types +
  all 10 content types + `ShippingMethodType` (REAL_METHODS → shared
  lookups in `translations.rs`).
- 10 `*Translate` mutations (product/variant/category/collection/page/
  voucher/shipping/menuItem/attribute/attributeValue): language-scoped
  upserts, provided-fields-only. `saleTranslate` returns an honest error
  (legacy sales don't exist in 3.24). PROMOTION kinds list empty (same).
- `LanguageCodeEnum` values recovered deterministically
  (`language_code_value()`: AF_NA → "af-na", Saleor's own enums.py rule);
  `language { code language }` served from Saleor's frozen
  `core/languages.py` map (779 entries).
- Verified live: DE upsert on product 137 → read-back + list + details.

## 12. Product assembly + promotion details (2026-09-22)

- Populatedb rows were never the problem — our assembly returned `vec![]`
  for media/listings/stocks/collections/attributes. `assemble_list_products`
  now batches everything: product extras (description/seo/rating/tax),
  media, product+variant channel listings (with price-range pricing),
  stocks+warehouses, variant media m2m, collections, product+variant
  attributes. Shared `load_variant_batches` backs the variants grid
  (`product.productVariants` REAL_METHODS, search+paginate),
  `media_by_id`, and variant `attributes(variantSelection:)`.
- `ProductMedia.url` resolves real `/media/` paths (REAL_METHODS).
- Global-ID migration regressions fixed: `Product.thumbnail` (was
  `parse::<i32>`, always None → no images anywhere) and
  `OrderLine.thumbnail` (bare-Uuid parse) now use `parse_gid` /
  `parse_uuid_gid`.
- `promotion(id:)` hand root (dashboard routes promotion rows to
  `/discounts/sales/:id` but fires `PromotionDetails`): full assembly
  with rules (channels, gifts, predicates, rewards).
- Browser-verified: promotion details renders with data (rules, 20% off),
  product 137 shows real `/media/` images.
