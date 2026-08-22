## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for Shopify Storefront Cart, commercetools Carts, BigCommerce Cart APIs, Salesforce B2C Shopper Baskets, Square/order-draft style APIs, and similar cart providers.
- [x] 1.3 Confirm the pack scope: cart lifecycle, buyer context, line items, custom lines, discounts, gift cards, estimates, tax/shipping/duty estimates, validation issues, stale diagnostics, abandonment diagnostics, handoff intents, export, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude order placement, checkout completion, payment intent creation, payment capture, receipt generation, entitlement provisioning, fulfillment, shipment, inventory adjustment, promotion authoring, tax-engine implementation, and application-specific checkout workflows.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, approval gates, resource gates, entitlement gates, trace/audit helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.commerce.cart.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `CartScope`, `CartProviderCapability`, `CartFreshness`, `CartAttribution`, and `CartRedactionPolicy`.
- [x] 2.3 Define `Cart`, lifecycle state, context, lines, adjustments, estimates, validation issues, version token, freshness, attribution, and redaction class.
- [x] 2.4 Define `CartContext`, buyer identity reference, anonymous/session reference, locale, currency, country, customer group, channel, address references, and consent/redaction metadata.
- [x] 2.5 Define `CartLine`, catalog product/variant references, custom line reference, quantity, unit price snapshot, selected options, selling-plan/subscription reference, shipping requirements, and validation state.
- [x] 2.6 Define `CartAdjustment`, `CartDiscountApplication`, discount code, gift card, promotion, coupon, automatic discount, target, amount, eligibility, stacking, and provider evidence.
- [x] 2.7 Define `CartTotals`, `CartEstimate`, subtotal, line/cart discounts, tax, duties, shipping, fees, total, currency precision, price-valid timestamp, and stale flags.
- [x] 2.8 Define `CartValidationIssue`, issue code, severity, retriable flag, remediation, bounded provider reason, and line/cart scope.
- [x] 2.9 Define `CartHandoffIntent`, checkout URL handle, order-draft handle, quote handle, expiry, access policy, no-payment/no-order marker, and replay pointer.
- [x] 2.10 Define `CartArtifactHandle`, export format, checksum, expiry, retention, redaction, and access policy.
- [x] 2.11 Define typed `success`, `partial`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.12 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And Cart Semantics

- [x] 3.1 Implement command schemas for `cart.inspect_provider` and `cart.describe_schema`.
- [x] 3.2 Implement command schemas for `cart.create_cart`, `cart.read_cart`, and `cart.search_carts`.
- [x] 3.3 Implement command schemas for `cart.plan_context_update` and `cart.update_context`.
- [x] 3.4 Implement command schemas for `cart.plan_line_mutation` and `cart.line_request` covering add, update quantity, update attributes, and remove.
- [x] 3.5 Implement command schemas for `cart.plan_discount` and `cart.discount_request`.
- [x] 3.6 Implement command schemas for `cart.estimate_cart`, `cart.validate_cart`, and stale-data diagnostics.
- [x] 3.7 Implement command schemas for `cart.plan_handoff` and `cart.handoff_request` without order placement or payment execution.
- [x] 3.8 Implement command schemas for `cart.inspect_abandonment`, `cart.plan_export`, `cart.export_cart`, and `cart.get_artifact_handle`.
- [x] 3.9 Add validation for provider schema support, cart lifecycle, line count, item availability, price/tax estimate freshness, version tokens, idempotency, unsupported discounts, pagination, async jobs, export bounds, and stale-data conditions.

## 4. Permission, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `commerce.cart.read`, `commerce.cart.write`, `commerce.cart.estimate`, `commerce.cart.discount`, `commerce.cart.handoff`, and `commerce.cart.export`.
- [x] 4.2 Require policy decisions before every command and approval before persistent cart creation, handoff intent creation, retained exports, and operations exposing external checkout URLs.
- [x] 4.3 Require entitlement checks for provider access, line mutation, discount support, estimate support, handoff support, export support, and store/channel access.
- [x] 4.4 Reserve and meter resources for cart search, line count, estimate recalculation, discount fan-out, export size, provider quotas, storage, and snapshots.
- [x] 4.5 Return typed denied/unavailable/unsupported/conflict/quota/stale-data outcomes before provider calls when preconditions fail.
- [x] 4.6 Add tests proving denied, unavailable, unsupported, conflict, quota, and stale-data paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [x] 5.1 Add the cart service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, async export support, and command dispatch.
- [x] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [x] 5.3 Implement a mock provider with synthetic carts, lines, discounts, estimates, validation issues, handoff intents, stale-data states, and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [x] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, freshness, version conflict, and replay pointer.
- [x] 5.6 Add provider capability discovery for line support, custom line support, discount/gift-card support, buyer identity support, tax/shipping estimate support, handoff support, versioning, stale-price behavior, search/export support, limits, freshness, and attribution.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.commerce.cart.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [x] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for creating a cart, adding/removing lines, updating context, applying discounts, estimating prices, validating stale carts, planning handoff, and handling version conflicts.
- [x] 6.5 Create `docs/developer-packs/commerce/cart.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, stale-data semantics, version conflicts, handoff boundaries, and cart/order/payment separation.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [x] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, entitlement, approval, resource, service-call, mutation-planning, handoff-planning, unavailable, health, snapshot, and result events.
- [x] 7.2 Add trace schemas for `cart_pack_declared`, `cart_pack_admission_validated`, `cart_pack_policy_decision`, `cart_pack_provider_inspected`, `cart_pack_service_call_requested`, `cart_pack_service_call_succeeded`, `cart_pack_service_call_failed`, `cart_pack_mutation_planned`, `cart_pack_handoff_planned`, `cart_pack_unavailable`, and `cart_pack_snapshot_recorded`.
- [x] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [x] 7.4 Add snapshot tests proving descriptor, provider health, command availability, schema/version support, pricing/discount/handoff support, policy-template hash, redaction profile, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [x] 7.5 Add redaction tests proving raw buyer PII, payment data, raw provider payloads, secret checkout URLs, provider-specific mutation DSLs, and unbounded cart exports never enter logs, traces, snapshots, or SDK diagnostics.

## 8. Boundary, Quality, And Validation Gates

- [x] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete cart providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [x] 8.3 Add canonical execution-path tests covering read-only, context update, line mutation, discount, estimate, validation, handoff, export, denied, unavailable, unsupported, conflict, quota, and stale-data paths.
- [x] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [x] 8.5 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.6 Run `openspec validate add-pack-commerce-cart --strict`.
- [x] 8.7 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, redaction checks, and cart/order/payment boundary checks before marking implementation tasks complete.
