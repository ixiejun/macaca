# Commerce Cart Pack

`pack.commerce.cart.v1` describes provider-neutral cart lifecycle, buyer
context, line mutation, discount, estimate, validation, abandonment, handoff,
and export capabilities. The descriptor is discoverable through SDK catalogs,
but commands remain unavailable until a cart provider is installed through the
runtime composition root.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.commerce.cart.v1"]
```

## Permissions

Use the narrowest scope: `commerce.cart.read`, `commerce.cart.write`,
`commerce.cart.estimate`, `commerce.cart.discount`,
`commerce.cart.handoff`, and `commerce.cart.export`.

## Capability Model

Macaca models carts as scoped cart handles, contexts, buyer or anonymous
session references, locale, currency, address references, lines, custom lines,
discount applications, adjustments, totals, estimates, validation issues,
handoff intents, freshness, attribution, redaction policies, version-token
hashes, and artifact handles. Raw buyer PII, payment data, checkout secrets,
provider mutation DSLs, and unbounded cart exports stay behind provider
adapters.

## Commands And Results

`cart.inspect_provider`, `cart.describe_schema`, `cart.create_cart`,
`cart.read_cart`, `cart.search_carts`, `cart.plan_context_update`,
`cart.update_context`, `cart.plan_line_mutation`, `cart.line_request`,
`cart.plan_discount`, `cart.discount_request`, `cart.estimate_cart`,
`cart.validate_cart`, `cart.plan_handoff`, `cart.handoff_request`,
`cart.inspect_abandonment`, `cart.plan_export`, `cart.export_cart`, and
`cart.get_artifact_handle` are descriptor-owned schema names.

Every command uses a `CommerceCommandEnvelope`. Results use
`CartResultEnvelope<T>` with success, paged, partial, denied, unavailable,
unsupported, conflict, quota-exceeded, stale-data, approval-required,
version-conflict, handoff-accepted, and failure states.

## App-Facing Examples

- Create or read a cart only after provider capability inspection.
- Plan line and discount mutations before writing provider state.
- Recalculate estimates and treat stale data as a structured result.
- Use handoff handles as references; they do not place orders or execute
  payments.
- Update context through bounded buyer, locale, currency, address, and channel
  refs rather than raw private payloads.
- Validate stale carts, version conflicts, unsupported discounts, quota
  failures, and handoff-unavailable states as typed results.

## Trace And Audit

Traces should record cart refs, line refs, context hashes, version-token hashes,
provider class, descriptor hash, freshness class, result status, idempotency
hash, handoff refs, artifact ids, and redaction profile. They must not record
buyer PII, payment credentials, secret checkout URLs, raw provider payloads, or
unbounded exports.

## Boundaries

Cart does not place orders, complete checkout, create payment intents, capture
payments, issue receipts, provision entitlements, fulfill shipments, adjust
inventory, or own promotion/tax-engine semantics.
