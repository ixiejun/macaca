# Commerce Cart Pack Design

## Context

`pack.commerce.cart.v1` is the Macaca capability for mutable shopping-cart and
basket state. It owns cart creation, line mutation, buyer context, discounts,
estimated costs, shipping/tax estimation, validation, stale-data diagnostics,
abandonment diagnostics, and handoff planning. It does not own order placement,
checkout completion, payment execution, receipt generation, entitlement
provisioning, fulfillment, or inventory adjustment.

Cart providers differ significantly in pricing and update semantics. The pack
therefore makes provider capability discovery and version/stale-data behavior
first-class.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| Shopify Storefront Cart | Cart create/get, merchandise lines, buyer identity, discount/gift-card codes, custom attributes, estimated costs, delivery options, checkout URL | Checkout handoff is not order/payment execution; buyer identity affects pricing; estimated costs can be stale |
| commercetools Carts | Line items, custom line items, price modes, taxed prices, shipping methods, cart discounts, discount codes, states, versioned updates, conversion to orders | Versioned mutation, price recalculation on updates, discount stacking, tax mode, order conversion boundary |
| BigCommerce Cart APIs | Create carts, add/update/remove lines, coupons, currency updates, storefront and management surfaces | Optimistic concurrency, storefront vs management capability split, custom item limitations |
| Salesforce B2C Shopper Baskets | Baskets, product items, customer association, shipping, payment prep, order placement through separate API family | Basket/order/payment family separation, shopper identity and session constraints |
| Square/order-draft style APIs | Order drafts or checkout sessions can act as cart-like state | Provider may not expose a true cart; command support must be explicit and degraded where needed |

## Goals

- Provide cart provider inspection, creation, reading, line mutation, buyer
  context update, discount/gift-card application, address/delivery estimation,
  price/tax recalculation, validation, abandonment diagnostics, handoff planning,
  and cart export/artifacts.
- Preserve cart version tokens, idempotency, stale-price diagnostics, line-level
  validation, and buyer-context redaction.
- Make order/payment/receipt/entitlement boundaries explicit and enforceable.
- Route every operation through canonical service runtime with trace, policy,
  entitlement, resource, approval where required, health, snapshot, and
  structured errors.

## Non-Goals

- Order placement, checkout completion, payment intent creation, payment capture,
  receipt generation, entitlement provisioning, fulfillment, shipment, inventory
  adjustment, promotion engine authoring, tax engine implementation, or storefront
  UI rendering.
- Provider-specific cart mutation DSLs, abandoned-cart marketing workflows,
  shipping carrier business rules, or application-specific checkout flows in
  Macaca OS layers.
- Raw buyer PII, raw payment data, secret checkout URLs, full provider payloads,
  or unbounded cart exports in observability.

## Ownership And Boundaries

- Pack id: `pack.commerce.cart.v1`.
- Family: `commerce`.
- Backing service owner: cart service provider family.
- SDK surface: `sdk.packs.commerce.cart`.
- Command namespace: `cart.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, and adapter composition through approved composition roots.
- Service ownership: capability discovery, cart state normalization, mutation
  planning, provider Strategy dispatch, stale-data handling, redaction, and
  sanitized audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `cart.inspect_provider` | Return provider capability, lifecycle, pricing, discount, tax, delivery, and handoff support | Read-only |
| `cart.describe_schema` | Return cart, line, context, discount, delivery, and total schema | Read-only |
| `cart.create_cart` | Create a cart with optional initial context and lines | Mutating |
| `cart.read_cart` | Read one normalized cart by handle | Read-only |
| `cart.search_carts` | Search carts by scope, state, customer/session, date, or cursor | Read-only |
| `cart.plan_context_update` | Validate buyer identity, locale, currency, country, address, channel, and customer group changes | Planning |
| `cart.update_context` | Apply approved context changes | Mutating |
| `cart.plan_line_mutation` | Validate add/update/remove line operations, product/variant references, quantity, custom data, and provider version | Planning |
| `cart.line_request` | Apply approved line mutation with idempotency and version checks | Mutating |
| `cart.plan_discount` | Validate discount/gift-card/promotion application or removal | Planning |
| `cart.discount_request` | Apply approved discount mutation | Mutating |
| `cart.estimate_cart` | Recalculate price, tax, duty, discount, shipping, and total estimates | Read-only or provider sync |
| `cart.validate_cart` | Return line-level and cart-level validation issues | Read-only |
| `cart.plan_handoff` | Plan checkout/order handoff without placing order or collecting payment | Planning |
| `cart.handoff_request` | Create handoff intent or checkout URL handle when provider supports it | Mutating metadata |
| `cart.inspect_abandonment` | Return abandonment diagnostics and age/freshness metadata without marketing action | Read-only |
| `cart.plan_export` | Plan cart export, redaction, retention, and resource budget | Planning |
| `cart.export_cart` | Produce cart artifact handle through approved path | Mutating/export |
| `cart.get_artifact_handle` | Retrieve artifact metadata without raw payload leakage | Read-only |

Every command must define typed command DTOs, typed success DTOs, typed partial
or async shapes, typed denied/unavailable/unsupported/conflict/quota/stale-data/
failure results, idempotency where side effects exist, redaction policy, and
replay metadata.

## Provider-Neutral DTO Model

- `CartScope`: application, tenant, session, task, store/channel, locale,
  currency, cart handle, buyer/customer/session handles, and permission scope.
- `CartProviderCapability`: line support, custom line support, discount/gift-card
  support, buyer identity support, tax/shipping estimate support, handoff
  support, versioning, stale-price behavior, search/export support, limits,
  freshness, and attribution.
- `Cart`: handle, lifecycle state, context, lines, adjustments, estimates,
  validation issues, version token, freshness, attribution, and redaction class.
- `CartContext`: buyer identity reference, anonymous/session reference, locale,
  currency, country, customer group, channel, shipping/billing address references,
  and consent/redaction metadata.
- `CartLine`: line handle, catalog product/variant references, custom line
  reference, quantity, unit price snapshot, selected options, selling-plan or
  subscription reference, shipping requirements, and validation state.
- `CartAdjustment`, `CartDiscountApplication`: discount code, gift card,
  promotion, coupon, automatic discount, target, amount, eligibility, stacking,
  and provider evidence.
- `CartTotals`, `CartEstimate`: subtotal, line discounts, cart discounts, tax,
  duties, shipping, fees, total, currency precision, price-valid timestamp, and
  stale flags.
- `CartValidationIssue`: line/cart scope, issue code, severity, retriable flag,
  suggested remediation, and bounded provider reason.
- `CartHandoffIntent`: checkout URL handle, order-draft handle, quote handle,
  expiry, access policy, no-payment/no-order boundary marker, and replay pointer.
- `CartArtifactHandle`: export format, checksum, expiry, retention, redaction,
  access policy, and replay pointer.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `commerce.cart.read`
- `commerce.cart.write`
- `commerce.cart.estimate`
- `commerce.cart.discount`
- `commerce.cart.handoff`
- `commerce.cart.export`

Policy defaults:

- Scope every command to application id, tenant id, session id, task id, trace id,
  store/channel, locale, currency, cart handle, and buyer/session handle.
- Require approval for persistent cart creation, handoff intent creation,
  retained exports, and operations that expose external checkout URLs.
- Require idempotency keys for mutating commands and handoff/export requests.
- Require provider version tokens when providers expose optimistic concurrency.
- Return `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` before provider calls when
  preconditions fail.
- Enforce resource budgets for cart search, line count, estimate recalculation,
  discount fan-out, export size, provider quotas, storage, and snapshots.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `cart_pack_declared`
- `cart_pack_admission_validated`
- `cart_pack_policy_decision`
- `cart_pack_provider_inspected`
- `cart_pack_service_call_requested`
- `cart_pack_service_call_succeeded`
- `cart_pack_service_call_failed`
- `cart_pack_mutation_planned`
- `cart_pack_handoff_planned`
- `cart_pack_unavailable`
- `cart_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, cart handle, line handles, context hash, policy decision, provider
class, descriptor hash, latency, freshness, version token hash, bounded resource
counters, result code, and sanitized artifact references. Events must exclude
raw buyer PII, payment data, raw provider payloads, secret checkout URLs,
provider-specific mutation DSLs, and unbounded cart exports.

Snapshots include descriptor version, provider health, command availability,
schema/version support, pricing/discount/handoff support, policy-template hash,
redaction profile, freshness, resource counters, and replay pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at `docs/developer-packs/commerce/cart.md` must
cover:

- Manifest declaration and permission scopes.
- Provider/schema discovery and unsupported/degraded diagnostics.
- DTO reference for carts, context, lines, adjustments, totals, estimates,
  validation issues, handoff intents, and artifacts.
- Examples for creating a cart, adding/removing lines, updating buyer context,
  applying discounts, recalculating estimates, validating stale carts, planning
  handoff, and handling conflicts.
- Provider replacement, mock/unavailable provider behavior, trace/audit
  interpretation, redaction guarantees, and boundaries with catalog, order,
  payment, receipt, fulfillment, and entitlement packs.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding providers.
- **Command**: every cart operation is a typed command/result DTO.
- **Strategy**: Shopify-like, commercetools-like, BigCommerce-like,
  Salesforce-like, and order-draft providers adapt behind one service contract.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  redaction, and buyer-data minimization wrap every service call.
- **State**: cart lifecycle, version conflict, stale estimate, handoff, export,
  and provider health are explicit states.
- **Specification**: admission validates declarations, scopes, provider schema,
  version tokens, lifecycle, handoff boundary, and resource limits.
- **Observer**: trace, audit, provider, mutation, handoff, and snapshot events
  are subscribable.
- **Memento**: effective capability reports, mutation plans, handoff intents, and
  artifact handles are replayable bounded records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: cart pack becomes checkout/order/payment logic. Mitigation: handoff
  intents are non-executing; order/payment behavior requires separate packs.
- Risk: stale prices mislead applications. Mitigation: estimates carry
  price-valid timestamps, stale flags, version tokens, and validation issues.
- Risk: buyer PII leaks through observability. Mitigation: buyer context is
  redacted and traces store references/hashes only.
- Risk: concurrent cart updates overwrite state. Mitigation: mutation planning,
  version tokens, idempotency keys, and conflict results are mandatory.
