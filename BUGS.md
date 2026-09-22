# BUGS — Dashboard runtime issues (validation is at FAILED=0; these are behavioral)

## B1. (fixed 2026-09-22) Sidebar collapsed + "unexpected error" after page refresh
- **Root cause:** the Dashboard NEVER sends `Authorization: Bearer`. Its
  fetch link (`dashboard/src/graphql/authFetch.ts`) sends
  **`authorization-bearer: <token>`** — Saleor honors it first
  (`saleor/core/auth.py`: `SALEOR_AUTH_HEADER = "HTTP_AUTHORIZATION_BEARER"`,
  fallback `DEFAULT_AUTH_HEADER`). Our server only read `Authorization`,
  so after reload (cold Apollo cache) `UserDetails.me` went out with no
  recognized credential → anonymous `me: null` → menu collapsed. Right
  after login it worked because `tokenCreate` seeds `me` into the cache
  (`seedUserQuery`), so `UserDetails` never hit the network.
- **Fix:** `crates/server/src/main.rs` reads `authorization-bearer` first,
  then `Authorization: Bearer` — Saleor's exact priority. Browser-verified:
  reload → full menu, `me` + 25 perms, zero console/network errors.
- Hypotheses 1–3 in the original report are superseded (token chain was
  always fine; it was the header name).

## B2. (fixed 2026-09-20) Sidebar showed only Home + Extensions even right after login
- **Cause:** `Permission.code` returned raw DB codenames (`handle_checkouts`);
  dashboard gates on enum names (`HANDLE_CHECKOUTS`). Fixed via
  `common::permission_enum_code` (exact 25-map from
  `saleor/permission/enums.py`). Commit `5b6f414`.

## B3. (fixed 2026-09-20) `channels` crashed on INTERVAL columns
- `channel_channel.delete_expired_orders_after` decoded as String → whole
  `BaseChannels` boot query failed → sidebar partly rendered. Fixed with
  slim `select_only` in `commerce.rs::channels`.
