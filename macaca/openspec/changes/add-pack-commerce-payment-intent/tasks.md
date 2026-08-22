## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for Stripe PaymentIntents, Adyen Payments/Checkout, PayPal Orders/Payments, Braintree Transactions, Square Payments, and similar gateway providers.
- [x] 1.3 Confirm the pack scope: intent planning, intent creation, confirmation/authorization, next-action inspection, capture, cancel/void, status sync, idempotency, event references, audit export, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude raw credential collection, PCI vaulting, refunds, disputes, payouts, settlement reconciliation, receipts, entitlement provisioning, fraud decisions, tax handling, and application checkout UI/workflows.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, approval gates, resource gates, entitlement gates, trace/audit helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.commerce.payment.intent.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `PaymentIntentScope`, `PaymentIntentProviderCapability`, `PaymentIntentFreshness`, `PaymentIntentAttribution`, and `PaymentIntentRedactionPolicy`.
- [x] 2.3 Define `PaymentIntentPlan`, `PaymentIntentRecord`, amount, currency, capture mode, merchant account, order/cart reference, customer/session reference, state, action requirements, idempotency key, freshness, and redaction class.
- [x] 2.4 Define `PaymentMethodReference` as tokenized-only with type class, region support, reusable flag, risk/eligibility metadata, and raw-credential rejection diagnostics.
- [x] 2.5 Define `PaymentActionRequirement` with redirect/action handle, action type, expiry, return/callback reference, and redaction.
- [x] 2.6 Define `PaymentAuthorization`, `PaymentCapture`, `PaymentCancellation`, partial capture metadata, expiry, provider reference, and side-effect evidence.
- [x] 2.7 Define `PaymentIntentEventReference`, event type, provider class, event timestamp, delivery id hash, freshness, replay pointer, and bounded result code.
- [x] 2.8 Define `PaymentIntentAuditExportPlan`, `PaymentIntentArtifactHandle`, export format, checksum, expiry, retention, redaction, and access policy.
- [x] 2.9 Define typed `success`, `partial`, `action_required`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.10 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And State Machine

- [x] 3.1 Implement command schemas for `payment_intent.inspect_provider` and `payment_intent.describe_schema`.
- [x] 3.2 Implement command schemas for `payment_intent.plan_intent` and `payment_intent.create_intent`.
- [x] 3.3 Implement command schemas for `payment_intent.plan_confirmation`, `payment_intent.confirm`, and action-required responses.
- [x] 3.4 Implement command schemas for `payment_intent.inspect_action` without leaking client secrets or SCA payloads.
- [x] 3.5 Implement command schemas for `payment_intent.plan_capture` and `payment_intent.capture`, including partial capture constraints.
- [x] 3.6 Implement command schemas for `payment_intent.plan_cancellation` and `payment_intent.cancel`.
- [x] 3.7 Implement command schemas for `payment_intent.get_status`, `payment_intent.inspect_idempotency`, and `payment_intent.record_event_reference`.
- [x] 3.8 Implement command schemas for `payment_intent.plan_audit_export`, `payment_intent.audit_export_request`, and `payment_intent.get_artifact_handle`.
- [x] 3.9 Add validation for tokenized payment method references, amount/currency precision, capture mode, capture amount, state transitions, authorization expiry, cancellation eligibility, idempotency, event freshness, pagination, async jobs, export bounds, and stale-data conditions.

## 4. Permission, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `commerce.payment.intent.read`, `commerce.payment.intent.create`, `commerce.payment.intent.confirm`, `commerce.payment.intent.capture`, `commerce.payment.intent.cancel`, and `commerce.payment.intent.audit_export`.
- [x] 4.2 Require policy decisions before every command and approval before create, confirm/authorize, capture, cancel/void, and retained audit exports.
- [x] 4.3 Require entitlement checks for provider access, payment method support, create support, confirm/authorize support, capture support, cancel/void support, event support, and merchant account access.
- [x] 4.4 Reserve and meter resources for provider calls, status sync, event reference ingestion, audit export size, storage, provider quotas, and snapshots.
- [x] 4.5 Return typed denied/unavailable/unsupported/conflict/quota/stale-data outcomes before provider calls when preconditions fail.
- [x] 4.6 Add tests proving denied, raw-credential, unavailable, unsupported, conflict, quota, and stale-data paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [x] 5.1 Add the payment-intent service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, async event/export support, and command dispatch.
- [x] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [x] 5.3 Implement a mock provider with synthetic intent states, action-required states, captures, cancellations, event references, idempotency outcomes, stale-data states, and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [x] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, state, freshness, idempotency hash, and replay pointer.
- [x] 5.6 Add provider capability discovery for payment methods, capture modes, action/redirect support, async event support, cancel/void support, partial capture support, idempotency model, status freshness, limits, attribution, and entitlement.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.commerce.payment.intent.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [x] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for planning/creating intents, confirming/authorizing, handling action-required state, capturing, canceling/voiding, syncing status, inspecting idempotency, and handling conflicts.
- [x] 6.5 Create `docs/developer-packs/commerce/payment-intent.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, state-machine semantics, idempotency, action-required handling, and refund/receipt/settlement boundaries.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [x] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, entitlement, approval, resource, service-call, state-transition-planning, sensitive-input-rejection, unavailable, health, snapshot, and result events.
- [x] 7.2 Add trace schemas for `payment_intent_pack_declared`, `payment_intent_pack_admission_validated`, `payment_intent_pack_policy_decision`, `payment_intent_pack_provider_inspected`, `payment_intent_pack_service_call_requested`, `payment_intent_pack_service_call_succeeded`, `payment_intent_pack_service_call_failed`, `payment_intent_pack_state_transition_planned`, `payment_intent_pack_sensitive_input_rejected`, `payment_intent_pack_unavailable`, and `payment_intent_pack_snapshot_recorded`.
- [x] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [x] 7.4 Add snapshot tests proving descriptor, provider health, command availability, state/capture/cancel/event support, policy-template hash, redaction profile, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [x] 7.5 Add redaction tests proving raw payment credentials, client secrets, raw provider payloads, SCA payloads, wallet cryptograms, webhook bodies, private keys, signatures, and unbounded output never enter logs, traces, snapshots, or SDK diagnostics.

## 8. Boundary, Quality, And Validation Gates

- [x] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete payment-intent providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [x] 8.3 Add canonical execution-path tests covering read-only, creation, confirmation, action-required, capture, cancellation, event reference, audit export, denied, unavailable, unsupported, conflict, quota, stale-data, and raw-credential rejection paths.
- [x] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [x] 8.5 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.6 Run `openspec validate add-pack-commerce-payment-intent --strict`.
- [x] 8.7 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, redaction checks, and payment/refund/receipt/settlement boundary checks before marking implementation tasks complete.
