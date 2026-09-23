# Account / Permissions / Auth — coverage checklist vs Saleor 3.23

Source of truth: `saleor-core/saleor/{permission/enums.py,graphql/account,account,core/jwt.py}`.
Legend: `[x]` real + tested · `[~]` real but partial · `[ ]` stub/missing.

## Permission codes (25, `permission/enums.py:16-92`)
- [x] `account.manage_users`, `account.manage_staff`, `account.impersonate_user`
- [x] `app.manage_apps`, `app.manage_observability`
- [x] `channel.manage_channels`
- [x] `account.manage_customer_types_and_attributes`
- [x] `discount.manage_discounts`
- [x] `plugins.manage_plugins`
- [x] `giftcard.manage_gift_card`
- [x] `menu.manage_menus`
- [x] `checkout.manage_checkouts`, `checkout.handle_checkouts`, `checkout.handle_taxes`, `checkout.manage_taxes`
- [x] `order.manage_orders`, `order.manage_orders_import`
- [x] `payment.handle_payments`
- [x] `page.manage_pages`, `page.manage_page_types_and_attributes`
- [x] `product.manage_products`, `product.manage_product_types_and_attributes`
- [x] `shipping.manage_shipping`
- [x] `site.manage_settings`, `site.manage_translations`
- Codes above = permission **enum mapping** (`permission_enum_code`) + `has_permission`
  resolution (direct + groups + superuser bypass). Enforcement per mutation below.

## Queries
- [x] `me` (JWT → user + permissions)
- [x] `customers` (filter/where/sort)
- [x] `user(id,email,externalReference)` (scope rules)
- [x] `staffUsers` (MANAGE_STAFF, filter/sort)
- [x] `permissionGroups` / `permissionGroup` (MANAGE_STAFF)
- [ ] `customerType(s)` (rare in dashboard; deferred)
- [ ] `addressValidationRules` (i18n data; deferred)

## Staff / customer management
- [x] `staffCreate` (forces `is_staff`, MANAGE_STAFF, email lower+unique, out-of-scope guard)
- [x] `staffUpdate` (self/superuser deactivate guards, last-manageable guard)
- [x] `staffDelete` (+ `staffBulkDelete`) (self/superuser/non-staff guards, gift-card PROTECT surfaces)
- [x] `customerCreate` / `customerUpdate` / `customerDelete` (+ `customerBulkDelete`) (MANAGE_USERS, staff-account guard)
- [x] `userBulkSetActive` (own/superuser guards)
- [x] `addressCreate/Update/Delete/SetDefault` (staff-managed, MAX 100, dedupe, default handling)
- [x] `permissionGroupCreate/Update/Delete` (MANAGE_STAFF, out-of-scope + last-manageable + cannot-remove-self-from-last-group)
- [ ] `userAvatarUpdate/Delete` (multipart — intentionally last, like CSV)
- [ ] `customerType*` attribute assignment (deferred with customerType roots)
- [ ] `customerBulkUpdate` (deferred — dashboard rarely uses)

## Auth flows
- [x] `tokenCreate` (throttled, confirmation/active/login-disabled checks, last_login bump)
- [x] `tokenRefresh` (token-key match, EXPIRED)
- [x] `tokenVerify` (honest: decode + active + key match)
- [x] `tokensDeactivateAll` (rotates `jwt_token_key`, kills access+refresh+reset family)
- [x] `passwordChange` (old-password verify + event)
- [x] `requestPasswordReset` + `setPassword` (single-use token table, key-bound, 1h TTL; dev token via log like Django console backend)
- [x] `accountRegister` + `confirmAccount` (inactive until confirm; token table)
- [x] `accountUpdate` (own profile + metadata)
- [x] `accountRequestDeletion` + `accountDelete` (own non-staff only; gift-card PROTECT; webhook-grade event row)
- [x] `requestEmailChange` + `confirmEmailChange` (1h TTL token, collision guard)
- [ ] external auth (OAuth plugin surface — out of scope, no provider configured)
- [ ] `sendConfirmationEmail` (needs SMTP; covered by register+confirm flow; deferred with email infra)

## Hardening / edge cases (Django parity)
- [x] Throttling: per-IP + per-IP+user exponential backoff, dummy-user timing equalization, 2h window, success clears (DB table, pod-safe)
- [x] Email lowercasing + `iexact` collision handling
- [x] Self-delete / superuser-delete / non-staff-delete guards
- [x] Deactivate-own / deactivate-superuser / activate-own guards
- [x] Last-manageable-staff guard (deactivate, delete, group edit, group delete)
- [x] Out-of-scope users/permissions/groups guard for non-superusers
- [x] Cannot-remove-self-from-last-group
- [x] Customer events written (`password_changed/reset`, `account_created`, `customer_deleted`, `email_changed`, …)
- [x] `MAX_USER_ADDRESSES=100` evict-oldest-non-default + dedupe
- [x] Fine-grained `require_perm` on all account mutations; domain codenames on catalog/commerce/translation writes
- [ ] `user.search_vector` refresh (Django FTS; our search is pg_trgm — equivalent, no-op by design)
- [ ] Staff notifications recipients (entity exists; no dashboard flow needs it yet)
