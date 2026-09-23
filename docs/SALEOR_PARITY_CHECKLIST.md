# Saleor parity — master checklist (measured 2026-09-23)

Schema baseline: `saleor/dashboard/schema-main.graphql` = **90 queries / 342 mutations**.
Ours (live SDL): **69 queries / 256 mutations**, of which **~215 hand-written REAL**,
~110 validated stubs. Dashboard ops: **458/458 green** (project goal = done).

Legend: `[x]` real + tested · `[~]` exists but thin/stub · `[ ]` missing.

## 1. Checkout new API (storefront names)
- [ ] `checkoutCreate` (+ lines/email/address inline)
- [ ] `checkoutLinesAdd` / `checkoutLinesUpdate` / `checkoutLinesDelete` / `checkoutLineDelete`
- [ ] `checkoutDelete`
- [ ] `checkoutEmailUpdate`, `checkoutCustomerAttach/Detach`, `checkoutCustomerNoteUpdate`
- [ ] `checkoutShippingAddressUpdate`, `checkoutBillingAddressUpdate`
- [ ] `checkoutShippingMethodUpdate`, `checkoutDeliveryMethodUpdate`
- [ ] `checkoutLanguageCodeUpdate`
- [ ] `checkoutAddPromoCode` / `checkoutRemovePromoCode`
- [ ] `checkoutPaymentCreate`, `checkoutCreateFromOrder`, `deliveryOptionsCalculate`
- [x] Custom old-style equivalents (`createCheckout`, `checkoutAddLines`, `checkoutComplete`…)

## 2. Transactions / payments (real money movement)
- [x] `transactionUpdate` (Django delta/adjustment semantics incl. psp-less cutoff fix), `transactionEventReport` (idempotent, alreadyProcessed), `transactionRequestAction` (CHARGE/REFUND/CANCEL via manual or Stripe PSP)
- [x] `paymentCapture/Refund/Void` (legacy ledger with guards + cumulative refund counter), `paymentCheckBalance` (honest unsupported-gateway error, like Django without plugin)
- [x] `payment(s)`, `transactions` queries
- [x] `transactionAuthorize/Charge` (manual ledger, `handle_payments`-gated)
- [x] `transactionInitialize` (default amount, channel flow strategy, Stripe intent clientSecret / manual {}), `transactionProcess` (outstanding → PSP execute), `paymentInitialize` (manual config / honest gateway errors)
- [~] Stripe skeleton (mock tests only) + live routing in requestAction when app names stripe; Adyen missing

## 3. Legacy sales (`sale*`)
- [ ] `saleCreate/Update/Delete/BulkDelete`, `saleCataloguesAdd/Remove`, `saleChannelListingUpdate`
- [x] New promotions engine (catalogue + order) — covers the dashboard path

## 4. Invoices
- [ ] `invoiceCreate/Request/Delete/Update` + `invoices` (whole subsystem; needs app surface)

## 5. Warehouse / stock
- [x] `stock` / `stocks` queries (quantity/search filter)
- [x] `stockBulkUpdate` (id-or-SKU × id-or-ref, per-row errors), `assign/unassignWarehouseShippingZone`
- [x] Stock reads in catalog, reservations/allocations, sweeper

## 6. Gift cards extras
- [x] `giftCardCurrencies` (distinct), `giftCardTags` (search), `giftCardSettings` (site row)
- [x] `giftCardBalanceAdjust` (adjust_balance + event)
- [ ] `exportGiftCards` (needs export infra — with CSV, intentionally last)
- [x] giftCards list + giftCard details + create/update/deactivate

## 7. Tax details
- [x] `taxClass`, `taxConfiguration`, `taxCountryConfiguration` singles, `taxTypes` (deprecated → classes)
- [x] `taxExemptionManage` (checkout-or-order id, union payload)
- [ ] `shopFetchTaxRates` (needs tax provider), `orderSettingsUpdate`
- [x] `taxConfigurations` list + `taxConfigurationUpdate` (Save works)

## 8. Webhooks / observability
- [x] `webhookEvents` (emitted subset, exact enums), `webhookSamplePayload`, `exportFiles` (honest empty)
- [ ] `webhookTrigger`, `eventDeliveryRetry`
- [x] Outbox fan-out + attempts + sweeper

## 9. Orders extras
- [x] `orderByToken` (public, token-is-secret), `orderSettings`, `reportProductSales` (ranked variants)
- [x] `orderAddNote` (NOTE_ADDED event), `orderBulkCancel`, `orderCreateFromCheckout` (metadata carry), `draftOrderLinesBulkDelete`
- [ ] `orderBulkCreate` (bulk input assembly — rare, deferred)
- [x] order(s) details, cancel/fulfill/returns, granted refunds

## 10. Catalog leftovers
- [x] `menuItem(s)` (tree, depth-bounded), `menuItemDelete` (subtree), `assignNavigation` (top/bottom menu), `appExtension` single
- [ ] `productBulkCreate`, `productReorderAttributeValues`, `productVariantReorderAttributeValues`
- [ ] `productVariantBulkTranslate`, `productBulkTranslate`, `attributeBulk{Create,Update,Translate}`, `attributeValueBulkTranslate`
- [ ] `promotionBulkDelete`, `promotionRuleTranslate`, `promotionTranslate`, `shopSettingsTranslate`
- [ ] `pageReorderAttributeValues`, `menuItemDelete`, `assignNavigation`
- [x] Everything else catalog (writes, listings, attributes, collections, channels, zones)

## 11. Account leftovers
- [x] `accountAddressCreate` (self or staff-for-customer); staff `addressCreate/Update/Delete/SetDefault` already real
- [ ] `accountAddressUpdate/Delete`, `accountSetDefaultAddress` (own variants — thin wrappers, next)
- [ ] `sendConfirmationEmail` (needs SMTP), external auth (`externalVerify`…)
- [ ] `userAvatarUpdate/Delete` (multipart — intentionally last)
- [x] Staff/customer/groups/addresses/password/register/confirm/delete flows (see `ACCOUNT_SECURITY_CHECKLIST.md`)

## 12. Misc queries
- [x] `address` (staff-any or owner), `appExtension` single, `checkoutLines` (admin view), `exportFiles` (empty), `menuItem(s)`
- [x] `payment(s)` (MANAGE_ORDERS), `transactions` (where/sort)
- [ ] `_entities` / `_service` (federation — N/A, single binary)
- [ ] `appTokenVerify`, `appProblemCreate`, `appReenableSyncWebhooks`
- [ ] `externalNotificationTrigger`, `storedPaymentMethodRequestDelete`, `invoiceRequestDelete`

## 13. Non-GraphQL (each a project)
- [ ] SMTP email (password mails, notifications)
- [ ] CSV exports + `exportVoucherCodes` / `delete_old_export_files` beat
- [ ] Multipart upload (media, avatars)
- [ ] 30+ payment gateways (only Stripe skeleton)
- [ ] Tax providers (AvaTax/TaxJar) + `taxAppId` execution
- [ ] Preorders, Click&Collect (`collectionPoint`) depth
- [ ] Saleor Apps SDK surface beyond tokens
- [ ] `update*SearchVector` beats (our search is pg_trgm — equivalent by design)
