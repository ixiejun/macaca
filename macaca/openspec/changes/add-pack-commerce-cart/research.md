# Commerce Cart Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.commerce.cart.v1`. The cart pack must expose mutable cart state, buyer
context, line mutation, discounts, estimates, validation, freshness, and handoff
planning through typed service commands. It must not place orders, collect or
capture payments, issue receipts, provision entitlements, or encode
application-specific checkout workflows.

## Source Baseline

- Shopify Storefront Cart API:
  <https://shopify.dev/docs/api/storefront/latest/mutations/cartCreate>,
  <https://shopify.dev/docs/api/storefront/latest/mutations/cartLinesAdd>, and
  <https://shopify.dev/docs/api/storefront/latest/objects/Cart>
- commercetools Carts:
  <https://docs.commercetools.com/api/projects/carts>
- BigCommerce Storefront Cart APIs:
  <https://developer.bigcommerce.com/docs/rest-storefront/carts>
- Salesforce B2C Commerce Shopper Baskets:
  <https://developer.salesforce.com/docs/commerce/commerce-api/references/shopper-baskets>
- Square Orders as order-draft/cart-like substrate:
  <https://developer.squareup.com/reference/square/orders-api>

## Supplier API Notes

- Shopify contributes cart creation, buyer identity, merchandise lines,
  discount/gift card codes, delivery options, estimated cost, custom attributes,
  and checkout URL handoff. Macaca should model checkout URLs only as redacted
  handoff handles, never as payment or order execution.
- commercetools contributes versioned cart updates, line items, custom line
  items, taxed prices, shipping information, discount codes, cart states, stale
  price behavior, and order conversion boundaries. Macaca should preserve
  version tokens and stale diagnostics as first-class DTO fields.
- BigCommerce contributes storefront cart creation, line item mutation, coupon
  handling, currency behavior, and concurrency-sensitive cart mutation. Macaca
  should normalize conflicts and unsupported capabilities without provider-name
  branches.
- Salesforce B2C contributes shopper baskets, product items, customer/session
  association, shipping data, and adjacent payment/order APIs. Macaca should
  keep basket preparation separate from order placement and payment execution.
- Square-style providers may expose order drafts rather than a native cart.
  Macaca should represent this as provider capability variance and degrade
  unsupported cart commands explicitly.

## Macaca-Owned Abstractions

`pack.commerce.cart.v1` should define `CartScope`, `CartProviderCapability`,
`Cart`, `CartContext`, `CartLine`, `CartAdjustment`,
`CartDiscountApplication`, `CartTotals`, `CartEstimate`,
`CartValidationIssue`, `CartHandoffIntent`, `CartArtifactHandle`,
`CartFreshness`, `CartAttribution`, and `CartRedactionPolicy`.

The DTOs must carry app/tenant/session/task scope, buyer references, locale,
currency, country, channel, line references, discount evidence, price/tax/duty/
shipping estimates, version tokens, stale flags, idempotency, capability hashes,
redaction classes, bounded provider reason codes, and replay pointers. Raw buyer
PII, raw payment data, secret checkout URLs, provider mutation DSLs, provider
payloads, and unbounded cart exports are rejected.

## Explicit Non-Goals

- Do not implement concrete Shopify, commercetools, BigCommerce, Salesforce,
  Square, tax-engine, promotion-engine, checkout, or payment adapters in this
  research phase.
- Do not define order placement, checkout completion, payment intent creation,
  payment capture, receipt issuance, entitlement provisioning, fulfillment,
  shipment, inventory adjustment, or application-specific checkout workflow
  semantics inside this pack.
- Do not expose provider-native cart payloads, raw checkout URLs, payment
  credentials, promotion DSLs, shipping carrier rules, or abandoned-cart
  marketing workflows as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` already provides
  provider-neutral pack metadata, lifecycle, availability, policy template,
  diagnostics, compatibility, provider snapshots, unavailable diagnostics, and
  effective capability expansion concepts that cart descriptors can reuse.
- `crates/facade/macaca-sdk/src/system_facade.rs` and focused SDK clients
  provide the Facade pattern expected for upper layers; cart SDK helpers should
  only build canonical traced service commands and must not construct providers.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  runtime-host registration/composition hooks for domain-pack providers.
- `crates/kernel/macaca-kernel/src/policy.rs`,
  `crates/runtime/macaca-runtime-host/src/service_policy_engine.rs`,
  `crates/kernel/macaca-kernel/src/audit.rs`,
  `crates/foundation/macaca-proto/src/audit_redaction.rs`, and
  `crates/runtime/macaca-runtime-host/src/service_call_audit.rs` provide
  reusable policy, redaction, trace, and audit substrate.
- Current evidence does not prove cart-specific DTOs, descriptors, command
  schemas, providers, SDK helpers, WASM ABI metadata, trace schemas, replay
  tests, redaction tests, dependency gates, or developer documentation.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
