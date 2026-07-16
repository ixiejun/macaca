# Workflow Delegation Pack Design

## Context

`pack.workflow.delegation.v1` is a child proposal of the developer-pack industrial capability catalog. It makes agent delegation, role assignment, handoff, capacity, and result collection available through Macaca's microkernel-compatible service model. The pack must behave like an operating-system capability: declarative in application manifests, policy checked before use, provider-neutral at the SDK boundary, observable through trace/audit evidence, and replaceable by built-in, plugin, remote, mock, or unavailable providers.

## Research Synthesis

Mature platforms converge on the same pattern:

- Temporal task queues: delegation is durable placement into a queue with worker
  leases, heartbeats, retries, cancellation, and deterministic history.
- Camunda assignments: human or automated work has assignees, candidates,
  claim/unclaim behavior, due dates, and process history.
- Kubernetes scheduling/leases: placement is constraint-based; active ownership
  is lease-backed and must expire or be renewed deterministically.
- GitHub Actions runners: execution is matched by labels/capabilities,
  concurrency groups, cancellation, and result artifacts.

Macaca should adapt those ideas as pack descriptors, service commands, policy decorators, and replayable audit records. The design does not copy platform API names; it preserves Macaca's own microkernel boundary and service runtime execution path.

## Supplier Capability Matrix

| Supplier pattern | Macaca contract element |
| --- | --- |
| Task queue routing | `DelegationRequest`, `DelegationQueueRef`, placement constraints |
| Worker lease/heartbeat | `DelegationLease`, renewal deadline, expiry event |
| Candidate group/assignee | `AssigneeConstraint`, `DelegationClaim`, eligibility evidence |
| Runner labels/capacity | `CapacitySnapshot`, capability tags, quota counters |
| Result artifact collection | `DelegationResultRef`, bounded output references, terminal evidence |

## Domain Model

- `DelegationRequest`: work placement request with delegated work reference,
  constraints, required capability tags, priority, deadline, idempotency key,
  cancellation policy, and result contract.
- `DelegationClaim`: active ownership record proving a specific assignee accepted
  work under eligibility and capacity policy.
- `DelegationLease`: renewable ownership lease with heartbeat timestamp, expiry,
  renewal policy, and revocation evidence.
- `DelegationHandoff`: explicit transfer from one owner to another with reason,
  preserved partial result references, and bounded handoff metadata.
- `DelegationResult`: terminal result envelope containing outcome, artifact
  references, checkpoints, error class, and replay pointers.

## State Machine

```text
requested -> queued -> claimed -> in_progress -> completed
requested -> queued -> claimed -> handoff_requested -> claimed
queued|claimed|in_progress -> cancelled
claimed|in_progress -> lease_expired -> queued
in_progress -> failed -> queued|completed
```

The provider-neutral contract must guarantee at most one active claim for a
delegation at any instant. Lease expiry, handoff, cancellation, and completion
are terminal or ownership-changing transitions with replayable evidence.

## Goals

- Provide agent delegation, role assignment, handoff, capacity, and result collection.
- Expose stable pack id `pack.workflow.delegation.v1`, command namespace `delegation.*`, permission scopes, SDK metadata, health, snapshot, and unavailable diagnostics.
- Route every operation through `SystemFacade` or focused SDK clients into the canonical service runtime path.
- Return structured `unavailable`, `unsupported`, `denied`, `conflict`, `quota_exceeded`, and `failure` results.
- Emit sanitized trace/audit events for declaration, admission, policy, entitlement/resource decisions, service calls, provider health, snapshots, and replay.

## Non-Goals

- Do not implement an application-specific feature, workflow, UI, or business rule.
- Do not put concrete provider construction in the microkernel, SDK, shells, or generic application framework.
- Do not expose raw secrets, prompts, manifests, package bytes, credentials, raw provider payloads, private keys, signatures, or unbounded output in logs, traces, snapshots, SDK diagnostics, or examples.
- Do not silently degrade to a different provider, fake success, or bypass policy when the declared provider is absent.

## Ownership And Boundaries

- Pack id: `pack.workflow.delegation.v1`.
- Family: `workflow`.
- Backing service owner: delegation service provider.
- SDK surface: `sdk.packs.workflow.delegation`.
- Command namespace: `delegation.*`.
- Microkernel ownership: identity, service-call evidence, policy facade, trace/audit primitives only.
- Application framework ownership: manifest declaration, app-scoped permission declarations, lifecycle/effective-capability projection.
- Runtime-host ownership: provider registration and decorators only through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `delegation.delegate` | Typed command/result DTO for delegate | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `delegation.accept_delegation` | Typed command/result DTO for accept delegation | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `delegation.handoff` | Typed command/result DTO for handoff | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `delegation.inspect_capacity` | Typed command/result DTO for inspect capacity | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `delegation.collect_result` | Typed command/result DTO for collect result | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `delegation.cancel_delegation` | Typed command/result DTO for cancel delegation | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `delegation.renew_lease` | Renew active ownership for delegated work | Requires current owner, bounded heartbeat interval, and capacity re-check |
| `delegation.release` | Release a claim back to the queue | Requires owner proof and preserves partial evidence |
| `delegation.list_assignments` | Query policy-visible assignments | Requires policy-filtered pagination and no hidden-count leakage |

Every command must define a typed command DTO, typed success result, typed partial-result shape when streaming or asynchronous, typed error result, redaction policy, idempotency semantics where side effects exist, and replay metadata.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `workflow.delegation.create`
- `workflow.delegation.accept`
- `workflow.delegation.cancel`
- `workflow.delegation.capacity.read`
- `workflow.delegation.result.read`

Policy defaults:

- Scope every command to application id, tenant id, session id, task id, and trace id when available.
- Apply state-machine ownership, resumable checkpoints, approval gates, bounded retries, delegation evidence, and review recovery.
- Require explicit approval for commands that cross user-sensitive, financial, identity, device, host, network, external-recipient, or irreversible side-effect boundaries.
- Enforce resource budgets for time, memory, storage, network, provider quota, streaming output, and retained snapshots.
- Return `denied` for policy rejection, `unavailable` for absent providers or entitlements, `unsupported` for unknown commands, and `quota_exceeded` for bounded-resource rejection.

## SDK Discovery And Examples

SDK discovery must return pack metadata, lifecycle, service mappings, command schemas, permission scopes, policy templates, examples, availability, health, provider class, version compatibility, and sanitized diagnostics.

- SDK helper example: `sdk.packs.workflow.delegation.delegate(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.workflow.delegation.accept_delegation(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.workflow.delegation.handoff(command)` builds a canonical traced service call; it never constructs providers.

Examples must use generic handles and synthetic data. They must not bake in application names, provider names, credentials, business workflows, or domain-specific routing.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `delegation_pack_declared`
- `delegation_pack_admission_validated`
- `delegation_pack_policy_decision`
- `delegation_pack_service_call_requested`
- `delegation_pack_service_call_succeeded`
- `delegation_pack_service_call_failed`
- `delegation_pack_unavailable`
- `delegation_pack_snapshot_recorded`

Events include pack id, service id, command name, trace id, application/session/task/tenant identifiers when available, policy decision, provider class, latency, bounded resource counters, stable capability hash, and bounded error code. Snapshots include descriptor version, provider health, command availability, policy template hash, and sanitized replay pointers.

## Implementation Slices

1. Descriptor and contract slice: pack descriptor, command schemas, permissions, policy template, health/snapshot DTOs, unavailable diagnostics.
2. Admission and resolver slice: required/optional declaration handling, lifecycle checks, service mapping checks, permission validation, effective capability memento.
3. Service slice: delegation service provider command handlers or unavailable provider, lifecycle, health, snapshot, shutdown, and structured error behavior.
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
- Risk: two assignees claim the same delegated work. Mitigation: claim and lease
  transitions must be atomic under the provider-neutral state contract and must
  return conflict to losing callers.
- Risk: delegation becomes hidden planner logic in shells. Mitigation: shells may
  render assignments but all placement, capacity, handoff, and lease semantics
  remain service commands.
