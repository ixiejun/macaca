# Workflow Recovery Pack Design

## Context

`pack.workflow.recovery.v1` is a child proposal of the developer-pack industrial capability catalog. It makes checkpoint discovery, failure classification, retry, repair, resume, and replay diagnostics available through Macaca's microkernel-compatible service model. The pack must behave like an operating-system capability: declarative in application manifests, policy checked before use, provider-neutral at the SDK boundary, observable through trace/audit evidence, and replaceable by built-in, plugin, remote, mock, or unavailable providers.

## Research Synthesis

Mature platforms converge on the same pattern:

- Temporal history/replay/reset: recovery is derived from durable event history,
  retry policy, reset points, and deterministic workflow continuation.
- Airflow task instance states: recovery needs explicit state, retry counters,
  clear/backfill operations, and scheduler-visible repair.
- Kubernetes controllers/jobs: recovery is reconciliation against desired state
  with restart policy, backoff limits, terminal status, and status conditions.
- Saga compensation: partial side effects require compensating action references
  and ordered recovery plans.

Macaca should adapt those ideas as pack descriptors, service commands, policy decorators, and replayable audit records. The design does not copy platform API names; it preserves Macaca's own microkernel boundary and service runtime execution path.

## Supplier Capability Matrix

| Supplier pattern | Macaca contract element |
| --- | --- |
| Workflow event history and reset | `RecoveryPoint`, `ReplayCursor`, `ResumePlan` |
| Task instance retry/backfill | `RetryPolicy`, `RetryAttempt`, `RepairAction` |
| Controller reconciliation | `RecoveryPlan`, desired-state reference, status condition |
| Backoff limit and terminal failure | failure class, retry budget, terminal reason |
| Saga compensation | `CompensationRef`, ordered repair/rollback references |

## Domain Model

- `FailureRecord`: normalized failure with class, origin service, bounded
  reason code, retryability, trace link, resource counters, and redaction
  profile.
- `RecoveryPoint`: checkpoint or event-history cursor with integrity hash,
  compatibility version, owner service, and replay metadata.
- `RecoveryPlan`: ordered provider-neutral plan containing retry, repair,
  resume, skip, compensate, or terminalize actions with policy requirements.
- `RepairAction`: bounded state repair request guarded by policy and explicit
  evidence references.
- `ReplayExport`: sanitized replay bundle for diagnostics, excluding raw
  prompts, provider payloads, secrets, manifests, package bytes, and unbounded
  output.

## State Machine

```text
healthy -> failed -> classified -> planned -> retrying -> resumed
classified -> planned -> repairing -> resumed
classified -> planned -> compensating -> terminal
classified -> terminal
retrying -> retry_budget_exhausted -> planned|terminal
repairing -> repair_failed -> planned|terminal
```

Recovery state must be separate from the failed workload's business state. The
pack can propose or execute generic recovery actions only through declared
service boundaries and must preserve the original trace lineage.

## Goals

- Provide checkpoint discovery, failure classification, retry, repair, resume, and replay diagnostics.
- Expose stable pack id `pack.workflow.recovery.v1`, command namespace `recovery.*`, permission scopes, SDK metadata, health, snapshot, and unavailable diagnostics.
- Route every operation through `SystemFacade` or focused SDK clients into the canonical service runtime path.
- Return structured `unavailable`, `unsupported`, `denied`, `conflict`, `quota_exceeded`, and `failure` results.
- Emit sanitized trace/audit events for declaration, admission, policy, entitlement/resource decisions, service calls, provider health, snapshots, and replay.

## Non-Goals

- Do not implement an application-specific feature, workflow, UI, or business rule.
- Do not put concrete provider construction in the microkernel, SDK, shells, or generic application framework.
- Do not expose raw secrets, prompts, manifests, package bytes, credentials, raw provider payloads, private keys, signatures, or unbounded output in logs, traces, snapshots, SDK diagnostics, or examples.
- Do not silently degrade to a different provider, fake success, or bypass policy when the declared provider is absent.

## Ownership And Boundaries

- Pack id: `pack.workflow.recovery.v1`.
- Family: `workflow`.
- Backing service owner: recovery service provider.
- SDK surface: `sdk.packs.workflow.recovery`.
- Command namespace: `recovery.*`.
- Microkernel ownership: identity, service-call evidence, policy facade, trace/audit primitives only.
- Application framework ownership: manifest declaration, app-scoped permission declarations, lifecycle/effective-capability projection.
- Runtime-host ownership: provider registration and decorators only through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `recovery.classify_failure` | Typed command/result DTO for classify failure | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `recovery.list_recovery_points` | Typed command/result DTO for list recovery points | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `recovery.retry` | Typed command/result DTO for retry | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `recovery.repair_state` | Typed command/result DTO for repair state | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `recovery.resume` | Typed command/result DTO for resume | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `recovery.export_replay` | Typed command/result DTO for export replay | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `recovery.build_plan` | Build a provider-neutral recovery plan | Requires classified failure, recovery points, policy, and resource budget |
| `recovery.apply_compensation` | Apply or record compensation reference | Requires explicit compensation permission and side-effect gate |
| `recovery.terminalize` | Mark unrecoverable work terminal | Requires policy proof, bounded reason, and audit evidence |

Every command must define a typed command DTO, typed success result, typed partial-result shape when streaming or asynchronous, typed error result, redaction policy, idempotency semantics where side effects exist, and replay metadata.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `workflow.recovery.read`
- `workflow.recovery.repair`
- `workflow.recovery.resume`
- `workflow.recovery.retry`
- `workflow.recovery.compensate`
- `workflow.recovery.export`

Policy defaults:

- Scope every command to application id, tenant id, session id, task id, and trace id when available.
- Apply state-machine ownership, resumable checkpoints, approval gates, bounded retries, delegation evidence, and review recovery.
- Require explicit approval for commands that cross user-sensitive, financial, identity, device, host, network, external-recipient, or irreversible side-effect boundaries.
- Enforce resource budgets for time, memory, storage, network, provider quota, streaming output, and retained snapshots.
- Return `denied` for policy rejection, `unavailable` for absent providers or entitlements, `unsupported` for unknown commands, and `quota_exceeded` for bounded-resource rejection.

## SDK Discovery And Examples

SDK discovery must return pack metadata, lifecycle, service mappings, command schemas, permission scopes, policy templates, examples, availability, health, provider class, version compatibility, and sanitized diagnostics.

- SDK helper example: `sdk.packs.workflow.recovery.classify_failure(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.workflow.recovery.list_recovery_points(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.workflow.recovery.retry(command)` builds a canonical traced service call; it never constructs providers.

Examples must use generic handles and synthetic data. They must not bake in application names, provider names, credentials, business workflows, or domain-specific routing.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `recovery_pack_declared`
- `recovery_pack_admission_validated`
- `recovery_pack_policy_decision`
- `recovery_pack_service_call_requested`
- `recovery_pack_service_call_succeeded`
- `recovery_pack_service_call_failed`
- `recovery_pack_unavailable`
- `recovery_pack_snapshot_recorded`

Events include pack id, service id, command name, trace id, application/session/task/tenant identifiers when available, policy decision, provider class, latency, bounded resource counters, stable capability hash, and bounded error code. Snapshots include descriptor version, provider health, command availability, policy template hash, and sanitized replay pointers.

## Implementation Slices

1. Descriptor and contract slice: pack descriptor, command schemas, permissions, policy template, health/snapshot DTOs, unavailable diagnostics.
2. Admission and resolver slice: required/optional declaration handling, lifecycle checks, service mapping checks, permission validation, effective capability memento.
3. Service slice: recovery service provider command handlers or unavailable provider, lifecycle, health, snapshot, shutdown, and structured error behavior.
4. SDK slice: discovery APIs, typed command helper builders, examples, diagnostics, and Null Object behavior.
5. Observability slice: trace/audit events, replay tests, snapshot sanitization, and metrics.
6. Gates slice: OpenSpec validation, DTO compatibility, dependency-boundary tests, no-direct-provider-call tests, canonical execution-path tests, file-size gates.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders; it does not construct providers.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider selection, unavailable behavior, policy routing, and version compatibility are replaceable.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **Specification**: admission validates pack id, lifecycle, commands, permissions, policy, and service mapping.
- **Observer**: trace, audit, health, and service events are subscribable and replayable.
- **Memento**: effective capability reports and snapshots preserve bounded recovery state.
- **Abstract Factory**: optional providers register only through approved composition roots.

## Risks And Mitigations

- Risk: broad capability becomes an OS-layer business workflow. Mitigation: keep the pack contract generic and place domain/provider semantics in replaceable services.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only build canonical service-call commands and are covered by no-direct-provider-call gates.
- Risk: preview or unavailable providers look callable. Mitigation: availability validators require descriptor, service registration, command schema, permission, entitlement, and health evidence before callable state.
- Risk: observability leaks sensitive data. Mitigation: event schema permits identifiers, hashes, counters, bounded codes, and sanitized snippets only.
- Risk: recovery repairs application-specific state incorrectly. Mitigation:
  generic recovery can only use declared recovery points and service-owned
  repair commands; app/business repair semantics remain in the owning service
  or application.
- Risk: replay export leaks sensitive payloads. Mitigation: replay export is a
  sanitized diagnostic bundle with hashes, references, counters, and bounded
  reason codes only.
