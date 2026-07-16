## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Record the borrowed platform patterns relevant to `pack.workflow.approval.v1` and map them to Macaca descriptors, permissions, policy, service calls, and audit records.
- [x] 1.3 Inventory existing service descriptors, SDK clients, optional packages, plugins, and unavailable providers that can back approval service provider.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And Schema

- [x] 2.1 Define provider-neutral command DTOs for: `approval.request_approval`, `approval.record_decision`, `approval.escalate`, `approval.cancel_approval`, `approval.inspect_evidence`.
- [x] 2.2 Define typed success, partial, denied, unavailable, unsupported, conflict, quota, and failure result DTOs.
- [x] 2.3 Define descriptor metadata for pack id, family, lifecycle, stability, command schemas, permissions, policy template, resource budget, data governance, SDK metadata, compatibility, and diagnostics.
- [x] 2.4 Add stable descriptor hashing and version compatibility checks.
- [x] 2.5 Add unit tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema compatibility.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for scopes: `workflow.approval.request`, `workflow.approval.decide`, `workflow.approval.escalate`.
- [x] 3.2 Add policy checks before side effects and resource reservation before provider calls.
- [x] 3.3 Add entitlement checks and explicit unavailable/denied diagnostics for missing provider, missing permission, missing entitlement, disabled host capability, and unsupported command.
- [ ] 3.4 Add approval behavior for sensitive, external, host, identity, financial, irreversible, or long-running side effects.
- [x] 3.5 Add tests proving denied/unavailable paths do not call concrete providers.

## 4. Service Provider Or Unavailable Provider

- [x] 4.1 Implement or bind approval service provider through the service runtime; do not construct providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, and bounded streaming behavior where applicable.
- [x] 4.3 Add structured provider capability reporting so discovery can distinguish available, degraded, preview, unavailable, unsupported, and retired states.
- [x] 4.4 Add mock and unavailable providers for deterministic tests.

## 5. SDK, Admission, Examples, And Documentation

- [x] 5.1 Extend SDK discovery for `pack.workflow.approval.v1` with command schemas, examples, availability, diagnostics, docs metadata, provider class, and compatibility.
- [x] 5.2 Extend application admission so required declarations block when unavailable and optional declarations degrade explicitly.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls for declared callable commands.
- [x] 5.4 Add examples for request approval, record decision, escalate, cancel approval using generic data and without hardcoded application or provider behavior.

## 6. Trace, Audit, Replay, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, policy, entitlement, resource, service-call, health, snapshot, and unavailable events.
- [x] 6.2 Add replay tests proving `pack.workflow.approval.v1` calls are trace-addressable through the canonical service path.
- [ ] 6.3 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete providers.
- [x] 6.4 Add no-direct-provider-call gates and canonical execution-path tests for all commands.
- [ ] 6.5 Run `openspec validate add-pack-workflow-approval --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create or update the detailed developer guide for `pack.workflow.approval.v1` under `docs/developer-packs/`, covering purpose, manifest declaration, permission scopes, command DTOs, result DTOs, examples, unavailable diagnostics, trace/audit behavior, and provider replacement notes.
- [x] 7.2 Add at least one minimal app-facing example and one provider/unavailable diagnostic example that use generic data and do not hardcode application business logic.
- [x] 7.3 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-workflow-approval` complete.

## 8. Industrial Approval Semantics

- [ ] 8.1 Model `ApprovalRequest`, `ApprovalAssignment`, `ApprovalDecision`, `ApprovalEvidenceBundle`, and `ApprovalDecisionGate` with schema-versioned DTOs and redaction profiles.
- [ ] 8.2 Implement the requested/pending/claimed/escalated/decided/expired/cancelled/consumed state machine as provider-neutral contract tests before provider-specific adapters.
- [ ] 8.3 Add idempotency behavior for duplicate requests and duplicate decisions, including conflict results for mismatched duplicate payloads.
- [ ] 8.4 Add approver eligibility re-checks at decision time, including revoked identity, revoked group membership, expired delegation, and tenant mismatch.
- [ ] 8.5 Add approval-decision consumption checks so protected side-effect services can verify subject, policy hash, trace lineage, expiry, and consumption mode.

## 9. Supplier-Grade Edge Cases

- [ ] 9.1 Test wait timer and deadline expiry behavior without relying on wall-clock sleeps.
- [ ] 9.2 Test escalation from an unclaimed request to a new eligible approver set while preserving previous assignment audit evidence.
- [ ] 9.3 Test cancellation racing with decision submission and guarantee exactly one terminal state.
- [ ] 9.4 Test policy-filtered `approval.list_pending` pagination so callers cannot infer hidden approvals through counts, cursors, or timing-sensitive diagnostics.
