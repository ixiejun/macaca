# Workflow Approval Pack Design

## Context

`pack.workflow.approval.v1` is a child proposal of the developer-pack industrial capability catalog. It makes approval request, decision capture, policy binding, escalation, and evidence replay available through Macaca's microkernel-compatible service model. The pack must behave like an operating-system capability: declarative in application manifests, policy checked before use, provider-neutral at the SDK boundary, observable through trace/audit evidence, and replaceable by built-in, plugin, remote, mock, or unavailable providers.

## Research Synthesis

Mature platforms converge on the same pattern:

- GitHub protected environments: approval is attached to a protected resource,
  has reviewer eligibility rules, can include wait timers, and must block the
  protected action until a recorded decision satisfies policy.
- ServiceNow approvals: approval records carry approver assignment, delegation,
  escalation, state history, comments, and audit evidence while provider
  business rules remain behind the platform boundary.
- Camunda user tasks: approval-like human work is a claimable, assignable,
  completable task with candidate users/groups, due dates, forms, variables,
  and process linkage.
- Temporal workflow signals/queries: human decisions are durable workflow
  events with deterministic replay, timeout, cancellation, and inspection
  semantics.

Macaca should adapt those ideas as pack descriptors, service commands, policy decorators, and replayable audit records. The design does not copy platform API names; it preserves Macaca's own microkernel boundary and service runtime execution path.

## Supplier Capability Matrix

| Supplier pattern | Macaca contract element |
| --- | --- |
| Protected deployment/environment approval | `ApprovalSubjectRef`, `ApprovalPolicyRef`, `ApprovalDecisionGate` |
| User task claim/complete | `ApprovalAssignment`, `ApprovalDecisionCommand`, approver eligibility policy |
| Delegated approver and escalation | `ApprovalEscalationRule`, `ApprovalDelegationRef`, bounded escalation event |
| Workflow signal/query | durable `approval_id`, idempotency key, replay pointer, queryable evidence view |
| Audit comment and evidence | sanitized `ApprovalEvidenceRef` and bounded decision metadata |

## Domain Model

The pack introduces generic records, not application-specific approval forms:

- `ApprovalRequest`: durable resource containing `approval_id`, subject
  reference, requester identity, required decision policy, allowed decision
  kinds, approver constraints, deadline, escalation policy, idempotency key,
  and redaction profile.
- `ApprovalAssignment`: normalized eligible actor/group/provider-independent
  role descriptor plus claim state and revocation evidence.
- `ApprovalDecision`: immutable decision record with approver identity,
  decision kind, reason code, bounded comment reference, decision timestamp,
  policy hash, and trace link.
- `ApprovalEvidenceBundle`: sanitized evidence references that can be rendered
  by shells or developer tools without raw provider payloads.

## State Machine

Approval state is explicit and replayable:

```text
requested -> pending -> claimed -> decided
requested -> pending -> escalated -> claimed -> decided
requested -> pending -> expired
requested|pending|claimed -> cancelled
decided -> consumed
```

Transitions must be monotonic except provider-neutral repair commands owned by
the recovery service. Duplicate `record_decision` calls with the same
idempotency key return the existing decision; conflicting duplicate calls return
`conflict` and emit sanitized audit evidence.

## Goals

- Provide approval request, decision capture, policy binding, escalation, and evidence replay.
- Expose stable pack id `pack.workflow.approval.v1`, command namespace `approval.*`, permission scopes, SDK metadata, health, snapshot, and unavailable diagnostics.
- Route every operation through `SystemFacade` or focused SDK clients into the canonical service runtime path.
- Return structured `unavailable`, `unsupported`, `denied`, `conflict`, `quota_exceeded`, and `failure` results.
- Emit sanitized trace/audit events for declaration, admission, policy, entitlement/resource decisions, service calls, provider health, snapshots, and replay.

## Non-Goals

- Do not implement an application-specific feature, workflow, UI, or business rule.
- Do not put concrete provider construction in the microkernel, SDK, shells, or generic application framework.
- Do not expose raw secrets, prompts, manifests, package bytes, credentials, raw provider payloads, private keys, signatures, or unbounded output in logs, traces, snapshots, SDK diagnostics, or examples.
- Do not silently degrade to a different provider, fake success, or bypass policy when the declared provider is absent.

## Ownership And Boundaries

- Pack id: `pack.workflow.approval.v1`.
- Family: `workflow`.
- Backing service owner: approval service provider.
- SDK surface: `sdk.packs.workflow.approval`.
- Command namespace: `approval.*`.
- Microkernel ownership: identity, service-call evidence, policy facade, trace/audit primitives only.
- Application framework ownership: manifest declaration, app-scoped permission declarations, lifecycle/effective-capability projection.
- Runtime-host ownership: provider registration and decorators only through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `approval.request_approval` | Typed command/result DTO for request approval | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `approval.record_decision` | Typed command/result DTO for record decision | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `approval.escalate` | Typed command/result DTO for escalate | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `approval.cancel_approval` | Typed command/result DTO for cancel approval | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `approval.inspect_evidence` | Typed command/result DTO for inspect evidence | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `approval.list_pending` | Page through pending approvals visible to the caller | Requires policy-filtered query, stable pagination cursor, and no raw provider payloads |
| `approval.consume_decision` | Bind a valid approval decision to a protected side effect | Requires same policy template hash, subject reference, and trace lineage before success |

Every command must define a typed command DTO, typed success result, typed partial-result shape when streaming or asynchronous, typed error result, redaction policy, idempotency semantics where side effects exist, and replay metadata.

## Protected Side-Effect Binding

The pack does not perform the protected business action. It emits a
provider-neutral `ApprovalDecisionGate` that another service can require before
executing sensitive side effects. Gate validation must compare:

- approval id and subject reference;
- tenant, application, session, task, and trace lineage;
- policy template hash and required permission scopes;
- decision kind, approver eligibility, expiry, and revocation status;
- one-time or multi-use consumption policy.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `workflow.approval.request`
- `workflow.approval.decide`
- `workflow.approval.escalate`

Policy defaults:

- Scope every command to application id, tenant id, session id, task id, and trace id when available.
- Apply state-machine ownership, resumable checkpoints, approval gates, bounded retries, delegation evidence, and review recovery.
- Require explicit approval for commands that cross user-sensitive, financial, identity, device, host, network, external-recipient, or irreversible side-effect boundaries.
- Enforce resource budgets for time, memory, storage, network, provider quota, streaming output, and retained snapshots.
- Return `denied` for policy rejection, `unavailable` for absent providers or entitlements, `unsupported` for unknown commands, and `quota_exceeded` for bounded-resource rejection.

## SDK Discovery And Examples

SDK discovery must return pack metadata, lifecycle, service mappings, command schemas, permission scopes, policy templates, examples, availability, health, provider class, version compatibility, and sanitized diagnostics.

- SDK helper example: `sdk.packs.workflow.approval.request_approval(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.workflow.approval.record_decision(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.workflow.approval.escalate(command)` builds a canonical traced service call; it never constructs providers.

Examples must use generic handles and synthetic data. They must not bake in application names, provider names, credentials, business workflows, or domain-specific routing.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `approval_pack_declared`
- `approval_pack_admission_validated`
- `approval_pack_policy_decision`
- `approval_pack_service_call_requested`
- `approval_pack_service_call_succeeded`
- `approval_pack_service_call_failed`
- `approval_pack_unavailable`
- `approval_pack_snapshot_recorded`

Events include pack id, service id, command name, trace id, application/session/task/tenant identifiers when available, policy decision, provider class, latency, bounded resource counters, stable capability hash, and bounded error code. Snapshots include descriptor version, provider health, command availability, policy template hash, and sanitized replay pointers.

## Implementation Slices

1. Descriptor and contract slice: pack descriptor, command schemas, permissions, policy template, health/snapshot DTOs, unavailable diagnostics.
2. Admission and resolver slice: required/optional declaration handling, lifecycle checks, service mapping checks, permission validation, effective capability memento.
3. Service slice: approval service provider command handlers or unavailable provider, lifecycle, health, snapshot, shutdown, and structured error behavior.
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
- Risk: approver revocation races with decision submission. Mitigation:
  `record_decision` must re-check eligibility at decision time and include the
  eligibility evidence hash in the immutable decision record.
- Risk: approval evidence becomes a generic data exfiltration path. Mitigation:
  evidence commands return references, summaries, hashes, and bounded snippets
  only; raw artifacts remain in the owning service.
