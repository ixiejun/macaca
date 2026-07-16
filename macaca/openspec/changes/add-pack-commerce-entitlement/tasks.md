## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for Stripe Entitlements, RevenueCat entitlements, Apple App Store Server API, Google Play Developer/Billing APIs, Microsoft Store ownership/subscription APIs, Paddle subscription/webhook APIs, and similar entitlement providers.
- [x] 1.3 Confirm the pack scope: grants, checks, batch checks, source sync, state transitions, suspension/resume, revocation, transfer, seat assignment, usage metering, proof export, event references, artifacts, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude payment authorization/capture, subscription billing execution, refund execution, invoice generation, receipt issuance, settlement, payouts, disputes, tax filing, application-specific feature gating, pricing rules, upgrade/downgrade business logic, and checkout UI.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, approval gates, resource gates, entitlement gates, trace/audit helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.commerce.entitlement.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `EntitlementScope`, `EntitlementProviderCapability`, `EntitlementFreshness`, `EntitlementAttribution`, and `EntitlementRedactionPolicy`.
- [x] 2.3 Define `EntitlementSubject` for account, profile, organization, tenant, device, installation, agent, service account, and external customer references.
- [x] 2.4 Define `EntitlementResource` for product, SKU, feature, plan, offering, license, subscription, seat pool, usage credit, content item, capability, and external resource references.
- [x] 2.5 Define `EntitlementDimension` for seats, requests, tokens, storage, time windows, durable ownership, subscription periods, consumable credits, and provider custom dimensions.
- [x] 2.6 Define `EntitlementSourceEvidence` for order, payment, receipt, invoice, subscription, app-store transaction, purchase token, license, manual grant, support override, provider event, and migration references.
- [x] 2.7 Define `EntitlementGrant`, grant handle, subject, resource, dimensions, state, validity window, quantity, usage balance, source evidence, grant reason, suspension/revocation reason, transfer history, freshness, attribution, and redaction class.
- [x] 2.8 Define `EntitlementState` with active, trial, grace, pending payment, pending acknowledgement, paused, suspended, expired, revoked, refunded, transferred, consumed, unknown, and provider custom mappings.
- [x] 2.9 Define `EntitlementSeatAssignment`, seat pool, assignee reference, quantity, role, assignment state, release state, and audit evidence.
- [x] 2.10 Define `EntitlementUsageRecord` and `EntitlementUsageBalance` with dimension, quantity, unit, idempotency key, usage window, reset policy, balance, limit, source evidence, freshness, and conflict metadata.
- [x] 2.11 Define `EntitlementEventReference`, provider class, event type, event timestamp, delivery id hash, webhook freshness, replay pointer, and bounded result code.
- [x] 2.12 Define `EntitlementProofExportPlan`, `EntitlementArtifactHandle`, proof type, checksum, expiry, retention, redaction, access policy, and replay pointer.
- [x] 2.13 Define typed `success`, `partial`, `accepted`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.14 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And State Semantics

- [x] 3.1 Implement command schemas for `entitlement.inspect_provider` and `entitlement.describe_schema`.
- [x] 3.2 Implement command schemas for `entitlement.plan_grant` and `entitlement.grant`, including source evidence, validity, quantity, idempotency, and approval checks.
- [x] 3.3 Implement command schemas for `entitlement.check` and `entitlement.batch_check`, including point-in-time evaluation, freshness, and bounded result sets.
- [x] 3.4 Implement command schemas for `entitlement.sync_source` with source authority, event freshness, and stale-data handling.
- [x] 3.5 Implement command schemas for `entitlement.plan_suspend`, `entitlement.suspend`, `entitlement.plan_resume`, and `entitlement.resume`.
- [x] 3.6 Implement command schemas for `entitlement.plan_revoke` and `entitlement.revoke`, including irreversible-effect approval and revocation evidence.
- [x] 3.7 Implement command schemas for `entitlement.plan_transfer` and `entitlement.transfer`, including subject isolation and provider transfer support.
- [x] 3.8 Implement command schemas for `entitlement.assign_seat` and `entitlement.release_seat`, including seat pool and quantity constraints.
- [x] 3.9 Implement command schemas for `entitlement.record_usage` and `entitlement.get_usage_balance`, including idempotency, dimensions, limits, reset windows, and conflicts.
- [x] 3.10 Implement command schemas for `entitlement.record_event_reference` without raw webhook body storage.
- [x] 3.11 Implement command schemas for `entitlement.plan_proof_export`, `entitlement.proof_export_request`, and `entitlement.get_artifact_handle`.
- [x] 3.12 Add validation for subject/resource isolation, source evidence visibility, source authority, state transitions, validity windows, trial/grace handling, acknowledgement state, seat limits, usage limits, transfer eligibility, idempotency, approval, retention, export bounds, and stale-data conditions.

## 4. Permission, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `commerce.entitlement.read`, `commerce.entitlement.grant`, `commerce.entitlement.revoke`, `commerce.entitlement.suspend`, `commerce.entitlement.transfer`, `commerce.entitlement.seat`, `commerce.entitlement.meter`, and `commerce.entitlement.proof_export`.
- [ ] 4.2 Require policy decisions before every command and approval before manual grants, suspensions, revocations, transfers, seat assignment changes, usage corrections, and retained proof exports.
- [ ] 4.3 Require entitlement checks for provider access, subject scope, resource scope, source type support, grant support, check support, state transition support, seat support, usage support, proof export support, and merchant/store/channel access.
- [ ] 4.4 Reserve and meter resources for batch checks, source sync, state transitions, usage recording, proof export size, provider quotas, storage, and snapshots.
- [ ] 4.5 Return typed denied/unavailable/unsupported/conflict/quota/stale-data outcomes before provider calls when preconditions fail.
- [ ] 4.6 Add tests proving denied, unavailable, unsupported, conflict, quota, stale-data, source-token-redaction, and proof-redaction paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [ ] 5.1 Add the entitlement service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, async sync/export support, and command dispatch.
- [ ] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [ ] 5.3 Implement a mock provider with synthetic subjects, resources, grants, states, seats, usage balances, source events, proof artifacts, stale-data states, and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [ ] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, entitlement state, source freshness, idempotency hash, and replay pointer.
- [ ] 5.6 Add provider capability discovery for source types, states, usage dimensions, seat support, transfer support, proof export support, idempotency model, freshness, limits, attribution, and entitlement.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.commerce.entitlement.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [ ] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for planning/granting entitlements, checking access, batch checks, syncing sources, suspending/resuming, revoking, transferring, assigning seats, recording usage, reading balances, exporting proof, and handling conflicts.
- [x] 6.5 Create `docs/developer-packs/commerce/entitlement.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, idempotency, source freshness, state semantics, usage/seat semantics, proof export, and payment/refund/invoice/receipt/application-feature boundaries.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [ ] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, entitlement, approval, resource, service-call, grant-planning, state-transition-planning, usage-recording, proof-export-planning, unavailable, health, snapshot, and result events.
- [ ] 7.2 Add trace schemas for `entitlement_pack_declared`, `entitlement_pack_admission_validated`, `entitlement_pack_policy_decision`, `entitlement_pack_provider_inspected`, `entitlement_pack_service_call_requested`, `entitlement_pack_service_call_succeeded`, `entitlement_pack_service_call_failed`, `entitlement_pack_grant_planned`, `entitlement_pack_state_transition_planned`, `entitlement_pack_usage_recorded`, `entitlement_pack_proof_export_planned`, `entitlement_pack_unavailable`, and `entitlement_pack_snapshot_recorded`.
- [ ] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [ ] 7.4 Add snapshot tests proving descriptor, provider health, command availability, source/state/usage/seat/transfer/proof support, policy-template hash, redaction profile, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [ ] 7.5 Add redaction tests proving raw purchase tokens, app-store signed payloads, payment credentials, provider webhook bodies, license secrets, private keys, raw signatures, raw provider payloads, and unbounded exports never enter logs, traces, snapshots, or SDK diagnostics.
- [ ] 7.6 Add proof-boundary tests proving proof exports are represented as bounded handles, hashes, and redacted metadata in observability surfaces.

## 8. Boundary, Quality, And Validation Gates

- [ ] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete entitlement providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [ ] 8.3 Add canonical execution-path tests covering read-only, grant, check, batch check, source sync, suspend/resume, revoke, transfer, seat assignment, usage, event reference, proof export, denied, unavailable, unsupported, conflict, quota, stale-data, and redaction paths.
- [ ] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [ ] 8.5 Add boundary tests proving entitlement commands do not execute payments, refunds, invoices, receipts, settlements, pricing changes, checkout flows, or application-specific feature gates.
- [x] 8.6 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.7 Run `openspec validate add-pack-commerce-entitlement --strict`.
- [ ] 8.8 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, redaction checks, proof-boundary checks, and entitlement/payment/refund/invoice/receipt/application-feature boundary checks before marking implementation tasks complete.
