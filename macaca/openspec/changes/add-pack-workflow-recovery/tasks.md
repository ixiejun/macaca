## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Record the borrowed platform patterns relevant to `pack.workflow.recovery.v1` and map them to Macaca descriptors, permissions, policy, service calls, and audit records.
- [x] 1.3 Inventory existing service descriptors, SDK clients, optional packages, plugins, and unavailable providers that can back recovery service provider.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And Schema

- [x] 2.1 Define provider-neutral command DTOs for: `recovery.classify_failure`, `recovery.list_recovery_points`, `recovery.retry`, `recovery.repair_state`, `recovery.resume`, `recovery.export_replay`.
- [x] 2.2 Define typed success, partial, denied, unavailable, unsupported, conflict, quota, and failure result DTOs.
- [x] 2.3 Define descriptor metadata for pack id, family, lifecycle, stability, command schemas, permissions, policy template, resource budget, data governance, SDK metadata, compatibility, and diagnostics.
- [x] 2.4 Add stable descriptor hashing and version compatibility checks.
- [x] 2.5 Add unit tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema compatibility.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for scopes: `workflow.recovery.read`, `workflow.recovery.repair`, `workflow.recovery.resume`.
- [x] 3.2 Add policy checks before side effects and resource reservation before provider calls.
- [x] 3.3 Add entitlement checks and explicit unavailable/denied diagnostics for missing provider, missing permission, missing entitlement, disabled host capability, and unsupported command.
- [ ] 3.4 Add approval behavior for sensitive, external, host, identity, financial, irreversible, or long-running side effects.
- [x] 3.5 Add tests proving denied/unavailable paths do not call concrete providers.

## 4. Service Provider Or Unavailable Provider

- [x] 4.1 Implement or bind recovery service provider through the service runtime; do not construct providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, and bounded streaming behavior where applicable.
- [x] 4.3 Add structured provider capability reporting so discovery can distinguish available, degraded, preview, unavailable, unsupported, and retired states.
- [x] 4.4 Add mock and unavailable providers for deterministic tests.

## 5. SDK, Admission, Examples, And Documentation

- [x] 5.1 Extend SDK discovery for `pack.workflow.recovery.v1` with command schemas, examples, availability, diagnostics, docs metadata, provider class, and compatibility.
- [x] 5.2 Extend application admission so required declarations block when unavailable and optional declarations degrade explicitly.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls for declared callable commands.
- [x] 5.4 Add examples for classify failure, list recovery points, retry, repair state using generic data and without hardcoded application or provider behavior.

## 6. Trace, Audit, Replay, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, policy, entitlement, resource, service-call, health, snapshot, and unavailable events.
- [x] 6.2 Add replay tests proving `pack.workflow.recovery.v1` calls are trace-addressable through the canonical service path.
- [ ] 6.3 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete providers.
- [x] 6.4 Add no-direct-provider-call gates and canonical execution-path tests for all commands.
- [ ] 6.5 Run `openspec validate add-pack-workflow-recovery --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create or update the detailed developer guide for `pack.workflow.recovery.v1` under `docs/developer-packs/`, covering purpose, manifest declaration, permission scopes, command DTOs, result DTOs, examples, unavailable diagnostics, trace/audit behavior, and provider replacement notes.
- [x] 7.2 Add at least one minimal app-facing example and one provider/unavailable diagnostic example that use generic data and do not hardcode application business logic.
- [x] 7.3 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-workflow-recovery` complete.

## 8. Industrial Recovery Semantics

- [ ] 8.1 Model `FailureRecord`, `RecoveryPoint`, `RetryPolicy`, `RecoveryPlan`, `RepairAction`, `CompensationRef`, `ResumePlan`, and `ReplayExport` with schema-versioned DTOs and redaction profiles.
- [ ] 8.2 Implement the failed/classified/planned/retrying/repairing/compensating/resumed/terminal state machine as provider-neutral contract tests.
- [ ] 8.3 Add failure classification rules for transient, permanent, policy-denied, quota-exhausted, provider-unavailable, corrupted-checkpoint, and unknown failures.
- [ ] 8.4 Add checkpoint integrity and compatibility checks before retry, repair, resume, or replay export.
- [ ] 8.5 Add retry budget, backoff, terminalization, and compensation-reference behavior with deterministic time control.

## 9. Supplier-Grade Edge Cases

- [ ] 9.1 Test retry budget exhaustion and ensure recovery stops with a typed terminal or escalation result instead of infinite loops.
- [ ] 9.2 Test corrupted checkpoint rejection and preserve diagnostic evidence without exposing raw checkpoint bytes.
- [ ] 9.3 Test resume after service restart using only recovery points, trace lineage, and provider-neutral service state.
- [ ] 9.4 Test replay export redaction for prompts, manifests, credentials, provider payloads, package bytes, and unbounded output.
