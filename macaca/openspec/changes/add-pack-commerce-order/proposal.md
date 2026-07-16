# Change: Add Commerce Order Pack

## Why

Macaca applications need `pack.commerce.order.v1` as an industrial order
management capability for creating order records, reading order state, managing
order lifecycle transitions, tracking fulfillment intent/status, cancellation,
returns references, and audit export. Provider APIs frequently combine order,
payment, refund, fulfillment, inventory, invoice, and receipt operations; Macaca
must expose a stricter provider-neutral boundary so applications can compose
commerce safely through declared packs.

This proposal defines the order pack as a serviceized capability. It supports
order state and order-management evidence, but it does not capture payments,
issue receipts, provision entitlements, adjust inventory, or own fulfillment
carrier workflows. Those capabilities must be declared and invoked separately.

## Supplier And API Baseline

The design is based on mature order APIs:

- Shopify Admin APIs expose order objects, fulfillment orders, cancellation,
  returns/exchanges/refund-related objects, invoices/receipts/shipping labels,
  and fulfillment state, with order and fulfillment objects separated.
- commercetools Orders are usually created from carts after checkout; order
  state transitions, order edits, shipments, returns, custom states, and versioned
  update actions are modeled explicitly.
- BigCommerce Orders APIs expose order create/read/update/delete, statuses,
  shipments, shipping addresses, transactions, taxes, fees, and refunds, with
  payment processing documented separately.
- Square Orders API can itemize purchases, calculate totals, confirm payments,
  track fulfillment, and update catalog inventory; Macaca maps only order record
  and fulfillment-state concepts here and keeps payment/inventory side effects
  separate.
- Salesforce Commerce Shopper/Order APIs model basket-to-order conversion,
  order search, payment preparation, fulfillment, and returns with distinct
  authorization and shopper context concerns.

The common denominator is a versioned order record with line items, customer and
shipping references, totals, payment-status references, fulfillment intent/status
references, cancellation/return references, lifecycle states, provider version
tokens, and auditable state transitions.

## Macaca Provider-Neutral Mapping

`pack.commerce.order.v1` maps supplier concepts into stable Macaca contracts:

- Provider orders become `OrderRecord`.
- Order line items, custom line items, fees, taxes, duties, discounts, shipping
  lines, and service charges become `OrderLine`, `OrderAdjustment`, and
  `OrderTotals`.
- Order statuses, payment status references, fulfillment status references,
  cancellation, return, and closed states become `OrderLifecycleState`.
- Fulfillment orders, shipment requests, pickup tasks, and delivery tasks become
  `FulfillmentIntent` and `FulfillmentStatusReference`; carrier execution and
  label purchase remain out of scope.
- Provider payments, refunds, receipts, and invoices become references only,
  never execution behavior inside this pack.
- Mutating operations use plan-before-side-effect commands:
  `order.plan_order`, `order.create_order`, `order.plan_state_transition`,
  `order.state_transition_request`, `order.plan_fulfillment_intent`,
  `order.fulfillment_intent_request`, `order.plan_cancellation`,
  `order.cancel_order`, `order.plan_audit_export`, and
  `order.audit_export_request`.

## What Changes

- Add provider-neutral `pack.commerce.order.v1` under the commerce family.
- Define commands for provider inspection, schema discovery, order creation
  planning, order create/read/search, status sync, lifecycle transition planning,
  fulfillment intent planning, cancellation planning, return/reference reading,
  audit export, and artifact retrieval.
- Define DTOs for order scope, provider capability, order records, lines, totals,
  addresses/parties, lifecycle states, payment/receipt/invoice references,
  fulfillment references, cancellation reasons, return references, freshness,
  attribution, redaction, version tokens, and idempotency.
- Require policy, entitlement, resource bounds, approval, lifecycle transition
  validation, provider version checks, no-payment/no-receipt boundaries, and
  sanitized trace/audit evidence.
- Require detailed developer documentation at
  `docs/developer-packs/commerce/order.md`.

## Impact

- Affected specs: `pack-commerce-order`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, order service providers, mock/unavailable
  providers, trace/audit schemas, replay tests, redaction tests, and boundary
  gates.

## Non-Goals

- No payment authorization/capture/refund, receipt issuance, invoice generation,
  entitlement provisioning, inventory adjustment/reservation, carrier label
  purchase, shipment tracking provider integration, tax filing, or application
  checkout workflow.
- No provider-specific order-numbering policy, fulfillment routing, return
  policy, cancellation rules, or business workflow hardcoded into Macaca OS
  layers.
- No raw payment credentials, full buyer PII, raw provider payloads, shipping
  labels, receipts, invoices, or unbounded order exports in logs, traces,
  snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
