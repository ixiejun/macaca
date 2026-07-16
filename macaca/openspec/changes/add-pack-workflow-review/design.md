# Workflow Review Pack Design

## Context

`pack.workflow.review.v1` is a child proposal of the developer-pack industrial capability catalog. It makes review request, finding capture, fix loop, re-review, approval, and terminal-state closure available through Macaca's microkernel-compatible service model. The pack must behave like an operating-system capability: declarative in application manifests, policy checked before use, provider-neutral at the SDK boundary, observable through trace/audit evidence, and replaceable by built-in, plugin, remote, mock, or unavailable providers.

## Research Synthesis

Mature platforms converge on the same pattern:

- GitHub PR reviews/checks: review requests, comments, requested changes,
  approvals, dismissed reviews, check conclusions, and merge protection are
  separate durable resources with audit evidence.
- Gerrit labels/submit rules: review outcomes are typed votes and blocking
  requirements tied to a patch-set revision and reviewer identity.
- GitLab merge approvals: review rules can require specific approver classes and
  re-approval after changes.
- Camunda user tasks: review can be represented as claimable work with evidence,
  completion, reassignment, and due dates.

Macaca should adapt those ideas as pack descriptors, service commands, policy decorators, and replayable audit records. The design does not copy platform API names; it preserves Macaca's own microkernel boundary and service runtime execution path.

## Supplier Capability Matrix

| Supplier pattern | Macaca contract element |
| --- | --- |
| PR review request | `ReviewRequest`, `ReviewSubjectRef`, reviewer constraints |
| Review comment/finding | `ReviewFinding`, severity, location/evidence reference |
| Requested changes | `FixRequest`, blocking state, required remediation evidence |
| Patch-set/revision re-review | `ReviewRound`, subject revision hash, stale finding marker |
| Submit/merge gate | `ReviewClosureGate`, unresolved blocking finding policy |

## Domain Model

- `ReviewRequest`: durable request containing subject reference, subject
  revision hash, reviewer constraints, review rubric reference, blocking policy,
  deadline, idempotency key, and redaction profile.
- `ReviewRound`: one complete pass over a subject revision with reviewer
  identity, start/end timestamps, finding set hash, and outcome.
- `ReviewFinding`: immutable finding with severity, category, optional location
  reference, bounded summary, remediation requirement, and status.
- `FixRequest`: provider-neutral request for remediation tied to one or more
  findings and expected fix evidence.
- `ReviewClosureGate`: normalized gate consumed by task or workflow services
  before marking reviewed work as terminal.

## State Machine

```text
requested -> in_review -> changes_requested -> fix_submitted -> in_review
requested -> in_review -> approved -> closed
requested -> in_review -> dismissed -> closed
requested|in_review|changes_requested -> cancelled
approved -> stale_after_revision_change -> in_review
```

The pack owns review lifecycle semantics only. It does not own the business
meaning of the reviewed artifact, patch, document, generated UI, or workflow.

## Goals

- Provide review request, finding capture, fix loop, re-review, approval, and terminal-state closure.
- Expose stable pack id `pack.workflow.review.v1`, command namespace `review.*`, permission scopes, SDK metadata, health, snapshot, and unavailable diagnostics.
- Route every operation through `SystemFacade` or focused SDK clients into the canonical service runtime path.
- Return structured `unavailable`, `unsupported`, `denied`, `conflict`, `quota_exceeded`, and `failure` results.
- Emit sanitized trace/audit events for declaration, admission, policy, entitlement/resource decisions, service calls, provider health, snapshots, and replay.

## Non-Goals

- Do not implement an application-specific feature, workflow, UI, or business rule.
- Do not put concrete provider construction in the microkernel, SDK, shells, or generic application framework.
- Do not expose raw secrets, prompts, manifests, package bytes, credentials, raw provider payloads, private keys, signatures, or unbounded output in logs, traces, snapshots, SDK diagnostics, or examples.
- Do not silently degrade to a different provider, fake success, or bypass policy when the declared provider is absent.

## Ownership And Boundaries

- Pack id: `pack.workflow.review.v1`.
- Family: `workflow`.
- Backing service owner: review service provider.
- SDK surface: `sdk.packs.workflow.review`.
- Command namespace: `review.*`.
- Microkernel ownership: identity, service-call evidence, policy facade, trace/audit primitives only.
- Application framework ownership: manifest declaration, app-scoped permission declarations, lifecycle/effective-capability projection.
- Runtime-host ownership: provider registration and decorators only through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `review.request_review` | Typed command/result DTO for request review | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `review.record_finding` | Typed command/result DTO for record finding | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `review.request_fix` | Typed command/result DTO for request fix | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `review.request_rereview` | Typed command/result DTO for request rereview | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `review.approve` | Typed command/result DTO for approve | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `review.close_review` | Typed command/result DTO for close review | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `review.dismiss` | Dismiss a review outcome under policy | Requires dismissal reason, eligibility check, and immutable audit evidence |
| `review.list_findings` | Query policy-visible findings | Requires severity/status filters and stable pagination |
| `review.evaluate_gate` | Produce a review closure gate for downstream services | Requires unresolved blocking finding checks and subject revision verification |

Every command must define a typed command DTO, typed success result, typed partial-result shape when streaming or asynchronous, typed error result, redaction policy, idempotency semantics where side effects exist, and replay metadata.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `workflow.review.request`
- `workflow.review.write`
- `workflow.review.approve`
- `workflow.review.dismiss`
- `workflow.review.finding.read`

Policy defaults:

- Scope every command to application id, tenant id, session id, task id, and trace id when available.
- Apply state-machine ownership, resumable checkpoints, approval gates, bounded retries, delegation evidence, and review recovery.
- Require explicit approval for commands that cross user-sensitive, financial, identity, device, host, network, external-recipient, or irreversible side-effect boundaries.
- Enforce resource budgets for time, memory, storage, network, provider quota, streaming output, and retained snapshots.
- Return `denied` for policy rejection, `unavailable` for absent providers or entitlements, `unsupported` for unknown commands, and `quota_exceeded` for bounded-resource rejection.

## SDK Discovery And Examples

SDK discovery must return pack metadata, lifecycle, service mappings, command schemas, permission scopes, policy templates, examples, availability, health, provider class, version compatibility, and sanitized diagnostics.

- SDK helper example: `sdk.packs.workflow.review.request_review(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.workflow.review.record_finding(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.workflow.review.request_fix(command)` builds a canonical traced service call; it never constructs providers.

Examples must use generic handles and synthetic data. They must not bake in application names, provider names, credentials, business workflows, or domain-specific routing.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `review_pack_declared`
- `review_pack_admission_validated`
- `review_pack_policy_decision`
- `review_pack_service_call_requested`
- `review_pack_service_call_succeeded`
- `review_pack_service_call_failed`
- `review_pack_unavailable`
- `review_pack_snapshot_recorded`

Events include pack id, service id, command name, trace id, application/session/task/tenant identifiers when available, policy decision, provider class, latency, bounded resource counters, stable capability hash, and bounded error code. Snapshots include descriptor version, provider health, command availability, policy template hash, and sanitized replay pointers.

## Implementation Slices

1. Descriptor and contract slice: pack descriptor, command schemas, permissions, policy template, health/snapshot DTOs, unavailable diagnostics.
2. Admission and resolver slice: required/optional declaration handling, lifecycle checks, service mapping checks, permission validation, effective capability memento.
3. Service slice: review service provider command handlers or unavailable provider, lifecycle, health, snapshot, shutdown, and structured error behavior.
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
- Risk: review logic becomes code-review-specific. Mitigation: model findings,
  revisions, gates, and remediation generically; code review is one possible
  provider/application use, not an OS branch.
- Risk: stale approvals close changed work. Mitigation: closure gates must check
  subject revision hash and mark approvals stale after revision changes unless
  policy permits carry-forward.
