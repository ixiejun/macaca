# Change: Add Commerce Cart Pack

## Why

Macaca applications need `pack.commerce.cart.v1` as an industrial shopping-cart
capability for session-scoped carts, line items, buyer context, discounts,
shipping/tax estimates, price recalculation, validation, and conversion
handoff. Cart APIs sit between catalog discovery and order/payment execution, so
the boundary must be precise: the cart pack may calculate and mutate cart state,
but it must not place orders, capture payments, issue receipts, or provision
entitlements.

This proposal defines cart as a provider-neutral, serviceized pack. Applications
declare the pack; admission validates permissions and provider availability; SDK
helpers build typed canonical commands; cart providers implement the service
contract behind policy, resource, entitlement, approval, trace, and redaction
decorators.

## Supplier And API Baseline

The design is based on mature cart and basket APIs:

- Shopify Storefront Cart APIs create/retrieve carts, update merchandise lines,
  buyer identity, discount codes, gift cards, custom attributes, estimated costs,
  delivery options, and checkout URL handoff.
- commercetools Carts model line items, custom line items, price modes, taxed
  prices, shipping methods, cart discounts, discount codes, states, versioned
  updates, stale price behavior, and conversion to orders.
- BigCommerce Storefront/Management Cart APIs create carts, add/update/remove
  line items, apply coupons, change currency, and recommend optimistic
  concurrency to avoid lost updates.
- Salesforce B2C Commerce Shopper Baskets APIs model baskets, items, customers,
  shipping, payment method preparation, and order placement as separate API
  families.
- Square and similar providers often model cart-like behavior through order
  drafts or checkout sessions; Macaca maps only pre-order cart semantics here.

The common denominator is a mutable, versioned, session/customer-scoped cart with
line items, pricing snapshots, discounts, buyer context, shipping/tax estimates,
validation messages, stale-data handling, and an explicit handoff to order or
checkout capabilities.

## Macaca Provider-Neutral Mapping

`pack.commerce.cart.v1` maps supplier concepts into stable Macaca contracts:

- Provider carts/baskets become `Cart`.
- Merchandise lines, custom lines, bundles, subscriptions, and service lines
  become `CartLine`.
- Buyer identity, locale, currency, country, customer group, channel, and
  shipping address hints become `CartContext`.
- Discount codes, gift cards, promotions, and automatic discounts become
  `CartAdjustment` and `CartDiscountApplication`.
- Estimated prices, taxes, duties, shipping, discounts, subtotal, and total
  become `CartTotals` and `CartEstimate`.
- Cart warnings, invalid lines, stale prices, unavailable items, and stock
  conflicts become `CartValidationIssue`.
- Checkout URLs, order conversion tokens, and quote handoff become
  `CartHandoffIntent`, not order placement or payment execution.

## What Changes

- Add provider-neutral `pack.commerce.cart.v1` under the commerce family.
- Define commands for provider inspection, cart creation/read/search, buyer
  context updates, line add/update/remove, discount apply/remove, address and
  delivery estimate updates, tax/price recalculation, validation, abandonment
  diagnostics, handoff planning, and export/artifact retrieval.
- Define DTOs for cart scope, provider capability, cart context, lines,
  adjustments, totals, estimates, validation issues, handoff intents, freshness,
  attribution, redaction, version tokens, and idempotency.
- Require policy, entitlement, resource bounds, buyer-data redaction, version
  conflict detection, stale-price diagnostics, no-order/no-payment boundaries,
  and sanitized trace/audit evidence.
- Require detailed developer documentation at
  `docs/developer-packs/commerce/cart.md`.

## Impact

- Affected specs: `pack-commerce-cart`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, cart service providers, mock/unavailable
  providers, trace/audit schemas, replay tests, redaction tests, and
  dependency-boundary gates.

## Non-Goals

- No order placement, checkout completion, payment intent creation, payment
  capture, receipt generation, entitlement provisioning, fulfillment, shipment,
  inventory adjustment, or storefront UI.
- No provider-specific promotion engine, tax calculation policy, shipping
  carrier selection, abandoned-cart marketing workflow, or business rule
  hardcoded into Macaca OS layers.
- No raw payment data, full buyer PII, provider payloads, checkout URLs with
  secrets, unbounded cart exports, or provider-specific cart mutation DSLs in
  logs, traces, snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
