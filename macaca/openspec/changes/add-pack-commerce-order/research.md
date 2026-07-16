# Commerce Order Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.commerce.order.v1`. The order pack must expose order records, source
conversion, lifecycle states, line items, totals, payment-status references,
fulfillment-intent references, cancellation, return references, status sync,
audit export, artifacts, freshness, attribution, and redaction through typed
service commands. It must not execute payments, refunds, receipts, invoices,
entitlements, inventory adjustment, carrier workflows, tax filing, or
application-specific checkout behavior.

## Source Baseline

- Shopify Orders and Fulfillment Orders:
  <https://shopify.dev/docs/api/admin-graphql/latest/objects/Order> and
  <https://shopify.dev/docs/api/admin-graphql/latest/objects/FulfillmentOrder>
- commercetools Orders and Order Edits:
  <https://docs.commercetools.com/api/projects/orders> and
  <https://docs.commercetools.com/api/projects/order-edits>
- BigCommerce Orders:
  <https://developer.bigcommerce.com/docs/rest-management/orders>
- Square Orders:
  <https://developer.squareup.com/reference/square/orders-api>
- Salesforce Commerce Shopper Orders:
  <https://developer.salesforce.com/docs/commerce/commerce-api/references/shopper-orders>

## Supplier API Notes

- Shopify contributes order objects, order transactions, fulfillments,
  fulfillment orders, cancellation, returns/exchanges, and POS/admin concerns.
  Macaca should represent payment, receipt, invoice, refund, and fulfillment
  execution as references or adjacent packs, not order-pack side effects.
- commercetools contributes cart-to-order conversion, versioned order updates,
  order state, shipment state, custom states, returns, and order edits. Macaca
  should preserve source references, version tokens, and lifecycle transition
  validation.
- BigCommerce contributes order create/read/update, statuses, shipping
  addresses, shipments, transactions, taxes, fees, and refunds. Macaca should
  expose only order record and lifecycle semantics while isolating payment and
  refund execution.
- Square Orders contributes line items, taxes, discounts, service charges,
  fulfillments, tenders/payments linkage, and catalog inventory integration.
  Macaca should normalize order totals and fulfillment intent while keeping
  payment and inventory effects outside this pack.
- Salesforce Commerce contributes shopper order surfaces derived from baskets,
  payment preparation, order search, and fulfillment/return context. Macaca
  should model conversion/source references and shopper redaction boundaries.

## Macaca-Owned Abstractions

`pack.commerce.order.v1` should define `OrderScope`,
`OrderProviderCapability`, `OrderRecord`, `OrderLine`, `OrderAdjustment`,
`OrderTotals`, `OrderLifecycleState`, `FulfillmentIntent`,
`FulfillmentStatusReference`, `OrderCancellationPlan`,
`OrderCancellationResult`, return/exchange reference DTOs,
`OrderAuditExportPlan`, `OrderArtifactHandle`, `OrderFreshness`,
`OrderAttribution`, and `OrderRedactionPolicy`.

The DTOs must carry source cart/quote references, external number references,
payment/receipt/invoice references, fulfillment references, return references,
parties/address references, line/totals snapshots, lifecycle state mappings,
version tokens, idempotency, capability hashes, redaction classes, bounded
provider reason codes, and replay pointers. Raw payment credentials, full buyer
PII, raw provider payloads, shipping labels, receipts, invoices, carrier
payloads, and unbounded order exports are rejected.

## Explicit Non-Goals

- Do not implement concrete Shopify, commercetools, BigCommerce, Square,
  Salesforce, fulfillment, carrier, payment, refund, tax, invoice, or receipt
  adapters in this research phase.
- Do not define payment authorization/capture/refund, receipt issuance, invoice
  generation, entitlement provisioning, inventory reservation/adjustment,
  carrier label purchase, shipment tracking provider integration, tax filing,
  or application checkout workflows inside this pack.
- Do not expose provider-specific order-numbering policies, fulfillment routing,
  return policies, cancellation rules, raw provider payloads, or business
  workflow branches as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` already provides
  descriptor metadata, lifecycle/availability, policy templates, SDK metadata,
  diagnostics, provider snapshots, unavailable diagnostics, and effective
  capability expansion concepts that order descriptors can reuse.
- `crates/facade/macaca-sdk/src/system_facade.rs` and focused SDK clients
  provide the Facade pattern expected for app-facing discovery and command
  construction; order SDK helpers should only build canonical traced service
  calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics for optional domain-pack
  providers.
- `crates/kernel/macaca-kernel/src/policy.rs`,
  `crates/runtime/macaca-runtime-host/src/service_policy_engine.rs`,
  `crates/kernel/macaca-kernel/src/audit.rs`,
  `crates/foundation/macaca-proto/src/audit_redaction.rs`, and
  `crates/runtime/macaca-runtime-host/src/service_call_audit.rs` provide
  reusable policy, redaction, trace, and audit substrate.
- Current evidence does not prove order-specific DTOs, descriptors, command
  schemas, providers, SDK helpers, WASM ABI metadata, trace schemas, replay
  tests, redaction tests, dependency gates, or developer documentation.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
