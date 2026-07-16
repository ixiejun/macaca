# Commerce Order Pack Design

## Context

`pack.commerce.order.v1` is Macaca's provider-neutral order-management
capability. It owns order records, order lifecycle, order status, fulfillment
intent/status references, cancellation, return references, and order audit
export. It does not own payment execution, refunds, receipts, invoices,
entitlement provisioning, inventory adjustment, or carrier execution.

Many provider APIs expose broader surfaces than this pack. Macaca normalizes the
order slice and makes adjacent capabilities explicit through references and
handoff metadata.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| Shopify | Orders, fulfillment orders, cancellations, returns/exchanges/refunds, receipts/invoices/shipping-label related resources | Fulfillment orders can be separate objects; refund/receipt/label behavior is adjacent; API scopes and lifecycle state vary |
| commercetools | Orders from carts/quotes, order states, order edits, shipments, returns, versioned updates | Cart-to-order boundary, state transitions, version tokens, order edits with financial impact, B2B quote conversion |
| BigCommerce | Orders V2/V3, statuses, shipments, shipping addresses, transactions, refunds, fees, taxes | Payments and refunds are separate concerns; optimistic update semantics; order status taxonomy varies |
| Square | Orders itemize purchases, calculate totals, confirm payments, track fulfillment, update inventory | Payment confirmation and inventory update are out of scope; fulfillment state may be tied to payment timing |
| Salesforce Commerce | Basket-to-order conversion, order search, payment prep, fulfillment, returns | Shopper context, order conversion boundary, regional/site constraints, separate authorization surfaces |

## Goals

- Provide provider inspection, schema discovery, order planning, order creation,
  order read/search, status sync, lifecycle transition planning, fulfillment
  intent planning, cancellation planning, return/reference reading, audit export,
  and artifact retrieval.
- Preserve order version tokens, idempotency, lifecycle validation, approval,
  status freshness, line/totals evidence, and provider capability discovery.
- Keep payment, refund, receipt, invoice, entitlement, inventory, and shipment
  execution as separate capability boundaries.
- Route every command through canonical service runtime with trace, policy,
  entitlement, resource, approval where required, health, snapshot, and
  structured errors.

## Non-Goals

- Payment authorization, payment capture, refunds, receipt issuance, invoice
  generation, entitlement provisioning, inventory reservation/adjustment, carrier
  label purchase, shipment tracking provider integration, tax filing, or
  application checkout workflow.
- Provider-specific order-numbering, fulfillment routing, returns policy,
  cancellation policy, or status taxonomy hardcoded into OS layers.
- Raw buyer PII, payment credentials, raw provider payloads, labels, receipts,
  invoices, or unbounded order exports in observability.

## Ownership And Boundaries

- Pack id: `pack.commerce.order.v1`.
- Family: `commerce`.
- Backing service owner: order service provider family.
- SDK surface: `sdk.packs.commerce.order`.
- Command namespace: `order.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, and adapter composition through approved composition roots.
- Service ownership: capability discovery, lifecycle transition validation,
  provider Strategy dispatch, state normalization, redaction, and sanitized audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `order.inspect_provider` | Return provider capability, lifecycle support, fulfillment support, export support, freshness, and attribution | Read-only |
| `order.describe_schema` | Return order, line, status, fulfillment, cancellation, return, and reference schema | Read-only |
| `order.plan_order` | Validate source cart/quote/order draft, lines, totals, parties, addresses, and provider constraints | Planning |
| `order.create_order` | Create order record through approved side-effect path | Mutating |
| `order.read_order` | Read one normalized order record | Read-only |
| `order.search_orders` | Search orders by scope, status, party, date, amount, or cursor | Read-only |
| `order.sync_status` | Refresh provider lifecycle, payment-status reference, and fulfillment-status reference | Read-only or provider sync |
| `order.plan_state_transition` | Validate status/state transition and provider version token | Planning |
| `order.state_transition_request` | Apply approved order state transition | Mutating |
| `order.plan_fulfillment_intent` | Validate non-carrier fulfillment intent/status update | Planning |
| `order.fulfillment_intent_request` | Record approved fulfillment intent/status reference | Mutating metadata |
| `order.plan_cancellation` | Validate cancellation reason, lifecycle state, payment/refund boundary, and provider support | Planning |
| `order.cancel_order` | Apply approved cancellation when provider supports it | Mutating |
| `order.list_return_references` | Read return/exchange references without refund execution | Read-only |
| `order.plan_audit_export` | Plan order audit export scope, format, redaction, and retention | Planning |
| `order.audit_export_request` | Produce order audit artifact handle | Mutating/export |
| `order.get_artifact_handle` | Retrieve artifact metadata without raw payload leakage | Read-only |

Every command must define typed command DTOs, success DTOs, partial/async shapes,
denied/unavailable/unsupported/conflict/quota/stale-data/failure results,
idempotency for side effects, redaction policy, and replay metadata.

## Provider-Neutral DTO Model

- `OrderScope`: application, tenant, session, task, store/channel, order handle,
  customer/session handle, source cart/quote handle, and permission scope.
- `OrderProviderCapability`: creation support, source conversion support,
  lifecycle support, cancellation support, fulfillment-intent support,
  return-reference support, export support, versioning, status freshness,
  limits, attribution, and entitlement.
- `OrderRecord`: order handle, external number reference, source reference,
  lifecycle state, lines, adjustments, totals, parties, addresses, payment-status
  references, invoice/receipt references, fulfillment references, return
  references, version token, freshness, and redaction class.
- `OrderLine`, `OrderAdjustment`, `OrderTotals`: catalog references, quantity,
  price snapshots, taxes, duties, discounts, fees, shipping, currency precision,
  and source evidence.
- `OrderLifecycleState`: created, pending, confirmed, paid-reference,
  fulfillment-pending, partially fulfilled, fulfilled, cancelled, closed,
  returned-reference, and provider custom states with mapping metadata.
- `FulfillmentIntent`, `FulfillmentStatusReference`: location, pickup/shipment
  intent, line allocation, status, tracking reference handle, and carrier-handoff
  boundary marker.
- `OrderCancellationPlan`, `OrderCancellationResult`: reason, refundable status
  reference, cancellation eligibility, provider support, and side-effect
  evidence.
- `OrderAuditExportPlan`, `OrderArtifactHandle`: export scope, format, checksum,
  expiry, retention, redaction, access policy, and replay pointer.
- `OrderFreshness`, `OrderAttribution`, `OrderRedactionPolicy`: freshness source,
  provider attribution, and observability rules.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `commerce.order.read`
- `commerce.order.write`
- `commerce.order.status`
- `commerce.order.fulfillment_intent`
- `commerce.order.cancel`
- `commerce.order.audit_export`

Policy defaults:

- Scope every call to application id, tenant id, session id, task id, trace id,
  store/channel, order handle, customer/session handle, and source handle.
- Require approval for order creation, lifecycle transitions, fulfillment-intent
  mutation, cancellation, and retained audit exports.
- Require idempotency keys for mutating commands and export requests.
- Require provider version tokens where providers expose optimistic concurrency.
- Return typed `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` before provider calls when
  preconditions fail.
- Enforce resource budgets for order search, status sync, audit export size,
  provider quotas, storage, and snapshots.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `order_pack_declared`
- `order_pack_admission_validated`
- `order_pack_policy_decision`
- `order_pack_provider_inspected`
- `order_pack_service_call_requested`
- `order_pack_service_call_succeeded`
- `order_pack_service_call_failed`
- `order_pack_lifecycle_planned`
- `order_pack_fulfillment_intent_planned`
- `order_pack_unavailable`
- `order_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, order/source handles, lifecycle transition, policy decision,
provider class, descriptor hash, latency, freshness, version token hash, bounded
resource counters, result code, and sanitized artifact references. Events must
exclude raw buyer PII, payment credentials, raw provider payloads, labels,
receipts, invoices, refund payloads, and unbounded order exports.

Snapshots include descriptor version, provider health, command availability,
lifecycle/fulfillment/cancellation/export support, policy-template hash,
redaction profile, freshness, resource counters, and replay pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at `docs/developer-packs/commerce/order.md` must
cover:

- Manifest declaration and permission scopes.
- Provider/schema discovery and unavailable diagnostics.
- DTO reference for orders, lines, totals, lifecycle states, fulfillment intents,
  cancellation plans, return references, audit exports, and artifact handles.
- Examples for planning/creating orders, reading/searching orders, syncing
  status, planning lifecycle transitions, recording fulfillment intent,
  cancelling orders, exporting audit evidence, and handling conflicts.
- Provider replacement, mock/unavailable provider behavior, trace/audit
  interpretation, redaction guarantees, and boundaries with cart, payment,
  receipt, invoice, entitlement, inventory, and fulfillment services.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding providers.
- **Command**: every order operation is a typed command/result DTO.
- **Strategy**: Shopify-like, commercetools-like, BigCommerce-like,
  Square-like, and Salesforce-like order providers adapt behind one service
  contract.
- **Decorator**: trace, policy, entitlement, resource, approval, metering, and
  redaction wrap every service call.
- **State**: order lifecycle, cancellation, fulfillment intent, status sync,
  audit export, and provider health are explicit states.
- **Specification**: admission validates declarations, scopes, provider schema,
  lifecycle transitions, version tokens, source conversion, and resource limits.
- **Observer**: trace, audit, provider, lifecycle, fulfillment, and snapshot
  events are subscribable.
- **Memento**: effective capability reports, order plans, lifecycle evidence,
  fulfillment-intent evidence, and artifact handles are replayable bounded
  records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: order pack becomes payment/refund/receipt/inventory logic. Mitigation:
  those surfaces are references only and require separate declared packs.
- Risk: fulfillment intent is confused with carrier execution. Mitigation:
  fulfillment DTOs carry boundary markers and no label purchase/tracking-provider
  execution.
- Risk: provider lifecycle differences cause invalid transitions. Mitigation:
  schema/capability discovery plus state Specification checks run before side
  effects.
- Risk: observability leaks order PII or payment data. Mitigation: traces store
  handles, hashes, bounded codes, freshness, and sanitized metadata only.
