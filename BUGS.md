# BUGS — Dashboard runtime issues (validation is at FAILED=0; these are behavioral)

## B1. Sidebar collapses + "We've encountered an unexpected error" after page refresh
- **Status:** open, reproduced 2026-09-20 (headless Chromium, `dashreload.mjs`).
- **Repro:** login as admin → full menu (Catalog/Fulfillment/…/Configuration) →
  `page.reload()` → sidebar shows only Home + Extensions; error banner
  "We've encountered an unexpected error. Try to refresh the page…".
- **What it is NOT (verified):**
  - No console errors, no failed network requests after reload.
  - `tokenCreate → tokenRefresh → me` chain returns user + all 25
    `MANAGE_*` permissions (curl-verified with fresh tokens).
  - `RefreshTokenWithUser` + `UserDetails` both fire on reload and validate.
  - So this is NOT a schema/validation bug and NOT an empty-permissions bug
    at the API level — the dashboard ends up with empty `userPermissions`
    in client state despite the API returning them.
- **Hypotheses (ranked):**
  1. **Race on reload:** `UserDetails.me` fires with the pre-refresh access
     token in parallel with `RefreshTokenWithUser`; if `me` errors first
     (expired/invalid token → `data: null` + ExpiredSignature), AuthProvider
     nulls the user and never recovers. After fresh login the token is new
     so the race is harmless. Check: our access-token TTL vs Saleor's
     (Saleor: 5 min access / 30 day refresh?); check what `me` does with an
     expired token vs Saleor (error shape `data:null` may trip the boundary
     while Saleor returns something softer).
  2. **RefreshToken payload gap:** compare our `tokenRefresh` payload field by
     field with Saleor's (`token refreshToken csrfToken user errors`?) —
     dashboard SDK may need a rotated `refreshToken` echo for the NEXT cycle.
  3. **Apollo cache cold-start:** after reload a different query order hits an
     execution (not validation) error in one boot query → ErrorBoundary.
     Next step: run `dashreload.mjs` capturing GraphQL `errors[]` per
     response (browser network interception), not just console/HTTP status.
- **Next steps:** (a) network-level capture of every GraphQL response body on
  reload; (b) dump localStorage tokens before/after; (c) compare `me` +
  expired-token behavior against real Saleor; (d) check access/refresh TTLs
  in `crates/server/src/service_auth.rs` vs Saleor settings.

## B2. (fixed 2026-09-20) Sidebar showed only Home + Extensions even right after login
- **Cause:** `Permission.code` returned raw DB codenames (`handle_checkouts`);
  dashboard gates on enum names (`HANDLE_CHECKOUTS`). Fixed via
  `common::permission_enum_code` (exact 25-map from
  `saleor/permission/enums.py`). Commit `5b6f414`.

## B3. (fixed 2026-09-20) `channels` crashed on INTERVAL columns
- `channel_channel.delete_expired_orders_after` decoded as String → whole
  `BaseChannels` boot query failed → sidebar partly rendered. Fixed with
  slim `select_only` in `commerce.rs::channels`.
