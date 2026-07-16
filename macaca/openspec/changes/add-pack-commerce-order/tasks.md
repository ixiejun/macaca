## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for Shopify Orders/Fulfillment Orders, commercetools Orders/Order Edits, BigCommerce Orders, Square Orders, Salesforce Commerce Orders, and similar order providers.
- [x] 1.3 Confirm the pack scope: order records, source conversion, lifecycle states, line items, totals, payment-status references, fulfillment-intent references, cancellation, return references, status sync, audit export, artifacts, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude payment authorization/capture/refund, receipt issuance, invoice generation, entitlement provisioning, inventory reservation/adjustment, carrier label purchase, shipment tracking provider integration, tax filing, and application checkout workflows.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, approval gates, resource gates, entitlement gates, trace/audit helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.commerce.order.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `OrderScope`, `OrderProviderCapability`, `OrderFreshness`, `OrderAttribution`, and `OrderRedactionPolicy`.
- [x] 2.3 Define `OrderRecord`, source reference, external number reference, lifecycle state, payment-status references, invoice/receipt references, fulfillment references, return references, version token, freshness, and redaction class.
- [x] 2.4 Define `OrderLine`, catalog references, custom lines, quantity, price snapshots, tax, duties, discounts, fees, shipping, currency precision, and source evidence.
- [x] 2.5 Define `OrderAdjustment`, `OrderTotals`, party references, address references, and redacted customer/session references.
- [x] 2.6 Define `OrderLifecycleState`, provider state mapping, custom state metadata, lifecycle transition request/result, and transition validation diagnostics.
- [x] 2.7 Define `FulfillmentIntent`, `FulfillmentStatusReference`, location/pickup/shipment intent, line allocation, tracking reference handle, and carrier-handoff boundary marker.
- [x] 2.8 Define `OrderCancellationPlan`, `OrderCancellationResult`, cancellation reason, refundable status reference, provider support, and side-effect evidence.
- [x] 2.9 Define return/exchange reference DTOs without refund execution semantics.
- [x] 2.10 Define `OrderAuditExportPlan`, `OrderArtifactHandle`, export format, checksum, expiry, retention, redaction, and access policy.
- [x] 2.11 Define typed `success`, `partial`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.12 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And Lifecycle Semantics

- [x] 3.1 Implement command schemas for `order.inspect_provider` and `order.describe_schema`.
- [x] 3.2 Implement command schemas for `order.plan_order`, `order.create_order`, `order.read_order`, and `order.search_orders`.
- [x] 3.3 Implement command schemas for `order.sync_status` and provider status freshness handling.
- [x] 3.4 Implement command schemas for `order.plan_state_transition` and `order.state_transition_request`.
- [x] 3.5 Implement command schemas for `order.plan_fulfillment_intent` and `order.fulfillment_intent_request` without carrier execution.
- [x] 3.6 Implement command schemas for `order.plan_cancellation` and `order.cancel_order`.
- [x] 3.7 Implement command schemas for `order.list_return_references` without refund execution.
- [x] 3.8 Implement command schemas for `order.plan_audit_export`, `order.audit_export_request`, and `order.get_artifact_handle`.
- [x] 3.9 Add validation for provider schema support, source cart/quote state, order lifecycle, line totals, version tokens, idempotency, cancellation eligibility, fulfillment-intent support, pagination, async jobs, export bounds, and stale-data conditions.

## 4. Permission, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `commerce.order.read`, `commerce.order.write`, `commerce.order.status`, `commerce.order.fulfillment_intent`, `commerce.order.cancel`, and `commerce.order.audit_export`.
- [ ] 4.2 Require policy decisions before every command and approval before order creation, lifecycle transitions, fulfillment-intent mutation, cancellation, and retained audit exports.
- [ ] 4.3 Require entitlement checks for provider access, creation support, status sync, fulfillment-intent support, cancellation support, return-reference support, audit export support, and store/channel access.
- [ ] 4.4 Reserve and meter resources for order search, status sync, audit export size, provider quotas, storage, and snapshots.
- [ ] 4.5 Return typed denied/unavailable/unsupported/conflict/quota/stale-data outcomes before provider calls when preconditions fail.
- [ ] 4.6 Add tests proving denied, unavailable, unsupported, conflict, quota, and stale-data paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [ ] 5.1 Add the order service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, async export support, and command dispatch.
- [ ] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [ ] 5.3 Implement a mock provider with synthetic orders, lifecycle transitions, fulfillment intents, cancellations, return references, audit exports, stale-data states, and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [ ] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, lifecycle state, freshness, version conflict, and replay pointer.
- [ ] 5.6 Add provider capability discovery for source conversion, lifecycle support, cancellation support, fulfillment-intent support, return-reference support, export support, versioning, status freshness, limits, attribution, and entitlement.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.commerce.order.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [ ] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for planning/creating orders, reading/searching orders, syncing status, planning lifecycle transitions, recording fulfillment intent, cancelling, exporting audit evidence, and handling conflicts.
- [x] 6.5 Create `docs/developer-packs/commerce/order.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, lifecycle semantics, fulfillment-intent boundaries, and payment/receipt/inventory boundaries.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [ ] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, entitlement, approval, resource, service-call, lifecycle-planning, fulfillment-intent-planning, unavailable, health, snapshot, and result events.
- [ ] 7.2 Add trace schemas for `order_pack_declared`, `order_pack_admission_validated`, `order_pack_policy_decision`, `order_pack_provider_inspected`, `order_pack_service_call_requested`, `order_pack_service_call_succeeded`, `order_pack_service_call_failed`, `order_pack_lifecycle_planned`, `order_pack_fulfillment_intent_planned`, `order_pack_unavailable`, and `order_pack_snapshot_recorded`.
- [ ] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [ ] 7.4 Add snapshot tests proving descriptor, provider health, command availability, lifecycle/fulfillment/cancellation/export support, policy-template hash, redaction profile, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [ ] 7.5 Add redaction tests proving raw buyer PII, payment credentials, raw provider payloads, labels, receipts, invoices, refund payloads, and unbounded order exports never enter logs, traces, snapshots, or SDK diagnostics.

## 8. Boundary, Quality, And Validation Gates

- [ ] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete order providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [ ] 8.3 Add canonical execution-path tests covering read-only, creation, status sync, lifecycle transition, fulfillment intent, cancellation, return references, audit export, denied, unavailable, unsupported, conflict, quota, and stale-data paths.
- [ ] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [x] 8.5 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.6 Run `openspec validate add-pack-commerce-order --strict`.
- [ ] 8.7 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, redaction checks, and order/payment/receipt/inventory boundary checks before marking implementation tasks complete.
