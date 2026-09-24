# Saleor parity — master checklist (measured 2026-09-24)

Schema baseline: `saleor/dashboard/schema-main.graphql` = **90 queries / 339 mutations**.
Ours (live SDL): **91 queries / 346 mutations** (extra = legacy aliases like
`orderAddNote`, old-style `createCheckout`, `_nodeAnchor`).
Every schema root resolves live — zero "Unknown field/argument" gaps.
Dashboard ops: **458/458 green**. DB contracts: **171/171 green**.

Legend: `[x]` real + tested · `[~]` real but bounded (documented) · `[ ]` missing/stub.

## 1. Checkout (storefront + legacy names)
- [x] New API: `checkoutCreate/LinesAdd/LinesUpdate/LineDelete/Delete/EmailUpdate/CustomerAttach/Detach/NoteUpdate/Shipping/BillingAddress/Shipping/DeliveryMethod/LanguageCode/Add/RemovePromoCode`
- [x] `checkoutPaymentCreate` (legacy gateway payment row), `checkoutCreateFromOrder` (unavailable-variants reported), `checkoutLinesDelete`, `deliveryOptionsCalculate` (shipping methods + pickup warehouses)
- [x] Old-style equivalents (`createCheckout`, `checkoutAddLines`, `checkoutComplete`…)

## 2. Transactions / payments (real money movement)
- [x] `transactionUpdate` (Django delta/adjustment semantics incl. psp-less cutoff fix), `transactionEventReport` (idempotent), `transactionRequestAction` (CHARGE/REFUND/CANCEL via manual or Stripe PSP)
- [x] `paymentCapture/Refund/Void`, `paymentCheckBalance` (honest unsupported-gateway error), `payment(s)`, `transactions` queries
- [x] `transactionAuthorize/Charge`, `transactionInitialize` (Stripe intent clientSecret / manual {}), `transactionProcess`, `paymentInitialize`
- [x] `transactionCreate` (opening buckets as success events), `transactionRequestRefundForGrantedRefund`, `storedPaymentMethodRequestDelete` (sync-webhook fan-out; fails to deliver with no vault app)
- [x] Gateway session/tokenization roots (`paymentGatewayInitialize*`, `paymentMethod*Tokenization`): honest not-configured results (no session apps installed)
- [~] Stripe skeleton (mock tests + live routing when app names stripe); Adyen + other gateways missing

## 3. Orders (staff ops core)
- [x] `orderConfirm/Capture/Refund/Void/MarkAsPaid/Update/UpdateShipping/NoteAdd/NoteUpdate/LineUpdate/LinesCreate/LineDelete/DiscountAdd/Update/Delete/LineDiscountUpdate/Remove`
- [x] Fulfillments: `orderFulfill/Approve/Cancel/UpdateTracking/RefundProducts/ReturnProducts`, granted refunds create/update/execute, money columns + statuses refreshed
- [x] Drafts: `draftOrderCreate/Update/Delete/Complete/BulkDelete`, `draftOrders` query, `orderLinesCreate` on drafts
- [x] `orderBulkCreate` (migration import: lines/addresses/discounts/fulfillments/transactions/invoices/notes, ≤50/call)
- [x] Order view depth: fulfillments, transactions, invoices, addresses, channel, notes, charge/authorize statuses
- [ ] `checkouts` list query (only dashboard-missing query; needs full `Checkout` node assembly)

## 4. Gift cards
- [x] Full CRUD: create (custom codes ≤16)/update/delete/activate/deactivate/notes/resend/settings/bulk/assign, balance adjust, CSV export
- [ ] `exportGiftCards` filtered scope (ALL/IDS work; FILTER refused honestly), XLSX (CSV only)

## 5. Promotions / vouchers / legacy sales
- [x] Promotions + rules CRUD, bulk delete, channel/gift links, predicate JSON the engine evaluates
- [x] Vouchers CRUD, catalogues add/remove, channel listings, code bulk delete, CSV export
- [x] Legacy `saleCreate/Update/Delete/BulkDelete/CataloguesAdd/Remove/ChannelListingUpdate` (promotion-backed, `Sale` node resolves)
- [x] Translates: promotion/rule/shop + bulk product/variant/attribute/value translates

## 6. Catalog
- [x] Products/variants/categories/collections CRUD + listings, attributes + values, `productBulkCreate` (listings/stocks/attributes, REJECT_EVERYTHING pre-validated), bulk deletes, reorder attributes/values, variant reorder/setDefault/stocksDelete, media CRUD/reorder/assign
- [x] Product types CRUD + `productType(s)`, `productVariant(s)` queries
- [x] `attributeBulkCreate/Update`
- [x] Stock: `stock(s)` reads, `stockBulkUpdate`, zone links, reservations/allocations, sweeper beat
- [ ] `fileUpload` / `userAvatarUpdate/Delete` (need multipart; Upload scalar stubbed)

## 7. Menus / pages
- [x] Menus/items CRUD + move (MPTT rebuilt), navigation assign, `menu(s)/menuItem(s)` reads
- [x] Pages CRUD/bulk/publish, page types CRUD/attributes/reorder, `page(s)/pageType(s)` reads

## 8. Shipping / warehouses / channels / tax
- [x] Zones/methods CRUD/bulk, exclusions, postal rules, channel listings with money, `orderSettingsUpdate`
- [x] Warehouses CRUD (id/externalReference), channels CRUD/activate/settings/zones/warehouses/reorder, tax classes + country rates CRUD
- [x] `shopFetchTaxRates` (honest no-provider result — deprecated root), `taxCountryConfigurations` query
- [~] Tax providers (AvaTax/TaxJar) + `taxAppId` execution; Click&Collect depth; preorders (deactivate only)

## 9. Accounts
- [x] Staff/customer CRUD, groups, customer types, staff notification recipients (create/delete/update), addresses (staff + own `accountAddressCreate/Update/Delete/SetDefaultAddress`), password/register/confirm/delete flows, throttle, tokens
- [x] `addressValidationRules` (built-in US/CA dataset + generic fallback; EU refused like Django)
- [x] `sendConfirmationEmail` + real SMTP (plain relay, Mailpit default; log-and-continue so mail never breaks flows)
- [x] External auth roots: honest not-configured errors (no external plugin installed)
- [ ] Avatars (`userAvatarUpdate/Delete` — multipart, see §6)

## 10. Apps / webhooks / plugins
- [x] Webhooks CRUD + dryRun (real object payloads) + `webhookTrigger` + `eventDeliveryRetry`, outbox fan-out/attempts/sweeper, `webhookEvents/samplePayload`
- [x] Apps: `app/appInstallations` queries, install (real manifest fetch)/retry/delete-failed, CRUD/activate/tokens/problems, `appTokenVerify`, `appProblemDismiss`, `appReenableSyncWebhooks`
- [x] `pluginUpdate`, `plugin(s)` queries, `externalNotificationTrigger` (outbox fan-out)
- [ ] App OAuth dance details (token exchange callbacks are app-side)

## 11. Invoices / settings / exports
- [x] Invoices: request/send/create/update/delete/request-delete + `invoices` on orders
- [x] Refund/return reason settings + queries, gift-card settings, shop settings translate
- [x] CSV exports: products/gift-cards/voucher-codes (ALL/IDS) + `exportFiles`/`exportFile` reads, media-served downloads
- [ ] XLSX output, filtered-export algebra, `delete_old_export_files` beat

## 12. Non-GraphQL (each a project)
- [x] SMTP transactional mail (plain relay, no TLS/auth — localhost-relay posture like Django's `EMAIL_HOST`)
- [ ] Multipart upload (blocks §6 `fileUpload` + §9 avatars)
- [ ] 30+ payment gateways (only Stripe skeleton)
- [ ] Tax providers, preorders/C&C depth, Saleor Apps SDK beyond tokens
- [ ] `update*SearchVector` beats (search is pg_trgm — equivalent by design)

## Appendix — audit trail (nothing was deleted, only consolidated)

Every line of the pre-2026-09-24 checklist and where it lives now. The
rewrite merged 90+ lines into 12 sections; this table proves no item was
dropped silently.

| Old item | Status now |
|---|---|
| checkoutCreate/LinesAdd/LinesUpdate/LineDelete/Delete/EmailUpdate/CustomerAttach/Detach/NoteUpdate/Shipping/BillingAddress/Shipping/DeliveryMethod/LanguageCode/Add/RemovePromoCode | §1 [x] |
| checkoutPaymentCreate / checkoutCreateFromOrder / checkoutLinesDelete / deliveryOptionsCalculate | §1 [x] |
| createCheckout / checkoutAddLines / checkoutComplete (old-style) | §1 [x] |
| transactionUpdate/EventReport/RequestAction/Authorize/Charge/Initialize/Process, paymentInitialize/Capture/Refund/Void/CheckBalance, payment(s)/transactions | §2 [x] |
| transactionCreate / transactionRequestRefundForGrantedRefund | §2 [x] (this wave) |
| storedPaymentMethodRequestDelete, gateway/tokenization roots | §2 [x] (this wave; honest no-vault results) |
| Stripe skeleton / Adyen | §2 [~] (unchanged) |
| orderConfirm/Capture/Refund/Void/MarkAsPaid/Update/Shipping/NoteAdd+Update/LineUpdate/LinesCreate/LineDelete/Discounts/LineDiscounts, orderAddNote alias | §3 [x] |
| fulfillments + granted refunds, draft orders + draftOrders, orderBulkCreate | §3 [x] |
| orderByToken/orderSettings/reportProductSales, orderBulkCancel/CreateFromCheckout/draftOrderLinesBulkDelete | §3 [x] |
| order/fulfillment/transaction/payment views (statuses, invoices, addresses, channel) | §3 [x] |
| giftCards list/details/create/update/deactivate/adjust + full CRUD + assign/notes/resend/settings/bulk | §4 [x] |
| giftCardCurrencies/Tags/Settings reads | §4 [x] (pre-existing, still live) |
| exportGiftCards | §4 [x] + §11 [x] (CSV ALL/IDS) |
| stock/stocks reads, stockBulkUpdate, zone links, reservations/allocations/sweeper | §6 [x] (restored — was missing from first rewrite pass) |
| promotions/rules CRUD/bulk, vouchers CRUD/catalogues/listings/codes | §5 [x] |
| saleCreate/Update/Delete/BulkDelete/CataloguesAdd/Remove/ChannelListingUpdate + sale(s) queries | §5 [x] (this wave) |
| product/variant/category/collection CRUD + listings, attributes/values, productBulkCreate, bulk deletes, reorders, variant reorder/setDefault/stocksDelete, media CRUD/reorder/assign, productVariantPreorderDeactivate | §6 [x] |
| attributeBulkCreate/Update | §6 [x] (this wave) |
| productType(s) + productVariant(s) queries | §6 [x] (this wave) |
| menus/items/move/navigation, pages/bulk/publish, page types/attributes/reorder, page(s)/pageType(s) reads | §7 [x] |
| shipping zones/methods CRUD/bulk, exclusions, postal rules, channel listings, shippingPriceBulkDelete | §8 [x] |
| warehouses CRUD, channels CRUD/settings/links, tax classes + country rates, taxConfigurations + update, taxExemptionManage, taxCountryConfigurations query, shopDomainUpdate, orderSettingsUpdate | §8 [x] |
| shopFetchTaxRates | §8 [x] (honest no-provider) |
| staff/customer/groups/addresses CRUD, password/register/confirm/delete, throttle, tokens, customer types, staffNotificationRecipient create/delete/**update** | §9 [x] (recipient update: this wave) |
| accountAddressCreate/Update/Delete/SetDefaultAddress | §9 [x] (this wave) |
| addressValidationRules, sendConfirmationEmail, external auth roots | §9 [x] (this wave; externals honestly error) |
| userAvatarUpdate/Delete | §9 [ ] (multipart — unchanged) |
| webhooks CRUD/dryRun/Trigger, eventDeliveryRetry, outbox/sweeper, webhookEvents/samplePayload/exportFiles | §10 [x] |
| app/appInstallations queries, install/retry/delete-failed, CRUD/activate/tokens/problems, appTokenVerify, appProblemCreate/Dismiss, appReenableSyncWebhooks, externalNotificationTrigger | §10 [x] (this wave) |
| pluginUpdate + plugin(s) queries | §10 [x] (this wave) |
| invoices request/send/create/update/delete/request-delete + order.invoices | §11 [x] |
| refund/return settings + queries, gift-card/shop-translate settings | §11 [x] (this wave) |
| CSV exports (products/gift-cards/voucher-codes) + exportFiles/exportFile | §11 [x] (this wave) |
| translates: single + bulk product/variant/attribute/value + promotion/rule/shop | §5 [x] (this wave) |
| SMTP / multipart / gateways / tax providers / preorders / Apps SDK / search vectors | §12 (unchanged positions) |
| `_entities`/`_service` federation | N/A by design (single binary, no Apollo federation) |
