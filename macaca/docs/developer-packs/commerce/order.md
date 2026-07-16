# Commerce Order Pack

`pack.commerce.order.v1` describes provider-neutral order records, lifecycle
states, status sync, fulfillment-intent references, cancellation, return
references, audit export, and artifact handles. The descriptor is discoverable
through SDK catalogs, but commands remain unavailable until an order provider is
installed through the runtime composition root.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.commerce.order.v1"]
```

## Permissions

Use the narrowest scope: `commerce.order.read`, `commerce.order.write`,
`commerce.order.status`, `commerce.order.fulfillment_intent`,
`commerce.order.cancel`, and `commerce.order.audit_export`.

## Capability Model

Macaca models orders as source references, lifecycle states, order lines,
adjustments, totals, party references, payment-status references, receipt and
invoice references, fulfillment intent references, return references,
cancellation plans, status freshness, attribution, redaction policies,
version-token hashes, and audit artifact handles. Raw buyer PII, payment
credentials, labels, receipts, invoices, refund payloads, provider payloads,
and unbounded exports stay behind provider adapters.

## Commands And Results

`order.inspect_provider`, `order.describe_schema`, `order.plan_order`,
`order.create_order`, `order.read_order`, `order.search_orders`,
`order.sync_status`, `order.plan_state_transition`,
`order.state_transition_request`, `order.plan_fulfillment_intent`,
`order.fulfillment_intent_request`, `order.plan_cancellation`,
`order.cancel_order`, `order.list_return_references`,
`order.plan_audit_export`, `order.audit_export_request`, and
`order.get_artifact_handle` are descriptor-owned schema names.

Every command uses a `CommerceCommandEnvelope`. Results use
`OrderResultEnvelope<T>` with success, paged, partial, denied, unavailable,
unsupported, conflict, quota-exceeded, stale-data, approval-required,
version-conflict, lifecycle-invalid, export-accepted, and failure states.

## App-Facing Examples

- Plan an order from a cart or quote reference before requesting creation.
- Sync order status and lifecycle references without executing payments or
  carrier actions.
- Plan state transitions, fulfillment intents, cancellation, and audit exports
  before side effects.
- Read or search orders with bounded pages and redacted party references.
- Handle lifecycle conflicts, version conflicts, unsupported transitions,
  stale status, quota failures, and unavailable providers as typed results.

## Trace And Audit

Traces should record order refs, source refs, lifecycle state, fulfillment
intent refs, cancellation plan refs, descriptor hash, provider class, freshness
class, result status, idempotency hash, artifact id, and redaction profile.
They must not record buyer PII, payment credentials, raw labels, receipts,
invoices, refund payloads, or unbounded order exports.

## Boundaries

Order does not authorize or capture payments, execute refunds, issue receipts
or invoices, provision entitlements, adjust inventory, buy carrier labels, or
perform shipment tracking provider integration.
