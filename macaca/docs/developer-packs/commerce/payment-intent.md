# Commerce Payment Intent Pack

`pack.commerce.payment.intent.v1` describes provider-neutral payment-intent
planning, creation, confirmation, action inspection, authorization, capture,
cancel or void, status sync, idempotency, event references, audit export, and
artifact handles. The descriptor is discoverable through SDK catalogs, but
commands remain unavailable until a payment-intent provider is installed
through the runtime composition root.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.commerce.payment.intent.v1"]
```

## Permissions

Use the narrowest scope: `commerce.payment.intent.read`,
`commerce.payment.intent.create`, `commerce.payment.intent.confirm`,
`commerce.payment.intent.capture`, `commerce.payment.intent.cancel`, and
`commerce.payment.intent.audit_export`.

## Capability Model

Macaca models payment intents as merchant-scoped plans, tokenized payment-method
references, amount and currency precision, capture modes, action requirements,
authorization evidence, capture evidence, cancellation evidence, event
references, idempotency hashes, freshness, attribution, redaction policies, and
audit artifact handles. Raw PAN, CVV, bank credentials, wallet cryptograms,
client secrets, SCA payloads, provider webhook bodies, signatures, and raw
provider payloads stay behind provider adapters or are rejected before calls.

## Commands And Results

`payment_intent.inspect_provider`, `payment_intent.describe_schema`,
`payment_intent.plan_intent`, `payment_intent.create_intent`,
`payment_intent.plan_confirmation`, `payment_intent.confirm`,
`payment_intent.inspect_action`, `payment_intent.plan_capture`,
`payment_intent.capture`, `payment_intent.plan_cancellation`,
`payment_intent.cancel`, `payment_intent.get_status`,
`payment_intent.inspect_idempotency`,
`payment_intent.record_event_reference`,
`payment_intent.plan_audit_export`,
`payment_intent.audit_export_request`, and
`payment_intent.get_artifact_handle` are descriptor-owned schema names.

Every command uses a `CommerceCommandEnvelope`. Results use
`PaymentIntentResultEnvelope<T>` with success, partial, action-required,
denied, unavailable, unsupported, conflict, quota-exceeded, stale-data,
approval-required, raw-credential-rejected, state-invalid, export-accepted, and
failure states.

## App-Facing Examples

- Plan a payment intent with amount, currency, merchant account, and tokenized
  method reference.
- Confirm, inspect actions, capture, cancel, and inspect idempotency through
  state-aware commands.
- Treat raw credential detection as a denied result before any provider call.
- Authorize or capture only through explicit state-transition plans with
  idempotency hashes and freshness metadata.
- Sync status and event references without storing webhook bodies or client
  secrets.
- Handle action-required, state-invalid, version conflict, quota, stale-data,
  unsupported capture, cancel/void denied, and unavailable-provider diagnostics
  as typed results.

## Trace And Audit

Traces should record payment intent refs, order or cart refs, state transition,
amount class, idempotency hash, event ref, provider class, descriptor hash,
freshness class, result status, artifact id, and redaction profile. They must
not record raw payment credentials, client secrets, SCA payloads, webhook
bodies, signatures, or raw provider payloads.

## Boundaries

Payment intent does not execute refunds, issue receipts, handle disputes,
perform settlement or payouts, provision entitlements, decide fraud risk, own
tax handling, or render checkout UI.
