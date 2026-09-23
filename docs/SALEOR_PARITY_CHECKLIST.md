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
- [ ] `transactionInitialize/Process/Update/EventReport`, `transactionRequestAction`
- [ ] `paymentInitialize/Capture/Refund/Void/CheckBalance`, gateway init + tokenization
- [ ] `payment(s)`, `transactions` queries
- [~] `transactionAuthorize/Charge` (manual ledger only, `handle_payments`-gated, no PSP call)
- [~] Stripe skeleton (mock tests only); Adyen missing

## 3. Legacy sales (`sale*`)
- [ ] `saleCreate/Update/Delete/BulkDelete`, `saleCataloguesAdd/Remove`, `saleChannelListingUpdate`
- [x] New promotions engine (catalogue + order) — covers the dashboard path

## 4. Invoices
- [ ] `invoiceCreate/Request/Delete/Update` + `invoices` (whole subsystem; needs app surface)

## 5. Warehouse / stock
- [ ] `stock` / `stocks` queries, `stockBulkUpdate`
- [ ] `assignWarehouseShippingZone` / `unassignWarehouseShippingZone`
- [x] Stock reads in catalog, reservations/allocations, sweeper

## 6. Gift cards extras
- [ ] `giftCardCurrencies`, `giftCardTags`, `giftCardSettings` queries
- [ ] `giftCardBalanceAdjust`, `exportGiftCards`
- [x] giftCards list + giftCard details + create/update/deactivate

## 7. Tax details
- [ ] `taxClass`, `taxConfiguration`, `taxCountryConfiguration`, `taxTypes` single roots
- [ ] `taxExemptionManage`, `shopFetchTaxRates`, `orderSettingsUpdate`
- [x] `taxConfigurations` list + `taxConfigurationUpdate` (Save works)

## 8. Webhooks / observability
- [ ] `webhookEvents`, `webhookSamplePayload`, `webhookTrigger`, `eventDeliveryRetry`
- [x] Outbox fan-out + attempts + sweeper

## 9. Orders extras
- [ ] `orderByToken`, `orderAddNote`, `orderBulkCancel/BulkCreate`, `orderCreateFromCheckout`
- [ ] `draftOrderLinesBulkDelete`, `reportProductSales`, `orderSettings`
- [x] order(s) details, cancel/fulfill/returns, granted refunds

## 10. Catalog leftovers
- [ ] `productBulkCreate`, `productReorderAttributeValues`, `productVariantReorderAttributeValues`
- [ ] `productVariantBulkTranslate`, `productBulkTranslate`, `attributeBulk{Create,Update,Translate}`, `attributeValueBulkTranslate`
- [ ] `promotionBulkDelete`, `promotionRuleTranslate`, `promotionTranslate`, `shopSettingsTranslate`
- [ ] `pageReorderAttributeValues`, `menuItemDelete`, `assignNavigation`
- [x] Everything else catalog (writes, listings, attributes, collections, channels, zones)

## 11. Account leftovers
- [ ] `accountAddressCreate/Update/Delete`, `accountSetDefaultAddress` (own-address variants; staff variants done)
- [ ] `sendConfirmationEmail` (needs SMTP), external auth (`externalVerify`…)
- [ ] `userAvatarUpdate/Delete` (multipart — intentionally last)
- [x] Staff/customer/groups/addresses/password/register/confirm/delete flows (see `ACCOUNT_SECURITY_CHECKLIST.md`)

## 12. Misc queries
- [ ] `address`, `appExtension` (single), `checkoutLines`, `exportFiles`, `menuItem(s)`
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
