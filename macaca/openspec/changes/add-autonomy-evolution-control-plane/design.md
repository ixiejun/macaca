## Context

The existing self-evolution implementation has service-owned Skill proposal
creation, proposal processing, materialization, semantic Skill identity,
registry/load-path projection, usage telemetry, durable telemetry replay, and
API-first audit. Those are necessary building blocks, but they are not a generic
agent self-evolution control plane.

The control plane introduced by this change owns only autonomous lifecycle
orchestration. It delegates target-specific mutation to replaceable target
adapter Strategies and delegates policy, entitlement, package guard, Store/EventLog,
Skill, Task, Context, and Evaluation behavior through service boundaries.

## Goals

- Add the minimal service contract and state machine for generic evolution runs.
- Keep the first implementation provider-neutral and target-agnostic.
- Make trace, policy, audit, and bounded evidence mandatory for side-effecting
  transitions.
- Provide structured unavailable behavior through SDK/SystemFacade.
- Prepare a Skill target adapter path without duplicating existing Skill service
  materialization semantics.

## Non-Goals

- Do not implement normalized paired benchmarking in this change.
- Do not implement canary rollout or production rollback application in this
  change.
- Do not implement a production Store/EventLog backend migration in this change.
- Do not implement source-code mutation. Future OS-code evolution must start as
  OpenSpec/Superpowers/GitNexus-governed proposals.

## Ownership

| Layer | Ownership |
| --- | --- |
| Kernel | Identity, trace/audit primitives, policy facade, service registry, and typed service-call routing only. |
| Autonomy Evolution Control Plane service | Evolution run lifecycle, target adapter dispatch, transition validation, bounded diagnostics, and provider-neutral snapshots. |
| Skill service | Skill proposal processing, materialization, registry/load-path identity, lifecycle, and usage telemetry. |
| Evaluation service | Future metric aggregation, paired benchmark scoring, and pass/fail/inconclusive rules. |
| Store/EventLog service | Durable run records, replay cursors, snapshots, rollback refs, and sanitized audit records. |
| Policy/Entitlement/Package Guard services | Approval, tenant/application scope, budget, resource, package ownership, and mutation admission decisions. |
| Runtime host | Built-in provider composition, provider factories, target adapter registration, and sanitized diagnostics. |
| SDK/SystemFacade | Provider-neutral commands, results, clients, and unavailable/null-object behavior. |
| Web/CLI/frontend | Thin adapters for triggering commands and rendering diagnostics only. |

## Design Patterns

- Command: every cross-boundary operation uses typed commands/results.
- Facade: SDK/SystemFacade hides provider construction from shells and apps.
- Strategy: target adapters, admission policies, benchmark policies, and
  rollback policies remain replaceable.
- Decorator: trace, policy, resource, entitlement, package guard, metering, and
  sanitization wrap side-effecting service calls.
- State: evolution runs move through explicit lifecycle states.
- Observer: task completion, Skill, Context, Evaluation, Store/EventLog, and
  release events are observed as bounded evidence.
- Memento: snapshots, benchmark baselines, rollback refs, and release refs are
  replayable evidence.
- Specification: transition admission and target adapter requirements are
  executable rules.
- Abstract Factory: runtime-host composition roots create built-in, plugin,
  remote, mock, and unavailable providers.

## Lifecycle

The control plane models this lifecycle:

```text
Observed
  -> CandidateQueued
  -> CandidateClassified
  -> ProposalGenerated
  -> AdmissionReview
  -> Quarantined
  -> BenchmarkPrepared
  -> BaselineMeasured
  -> CandidateMeasured
  -> CanaryRunning
  -> Promoted
  -> ActiveMonitoring
  -> Superseded | RolledBack | Rejected | Inconclusive
```

The first code slice may implement transition validation and snapshot storage
without executing every future downstream phase. Unsupported future operations
must return structured unsupported or unavailable results rather than fake
success.

## Transition Safety

Every transition carries app/tenant/session/task scope, trace id, actor id,
target type, bounded evidence refs, and sanitized diagnostics. Transitions that
can mutate target state, expose a candidate to a run, promote a candidate, or
roll back an active state must also carry a policy decision id and audit refs.

## Target Adapter Contract

A target adapter translates generic lifecycle intent into a target-specific
service operation. The first adapter type is Skill. It must call existing Skill
service commands for proposal processing, materialization, registry/load-path,
usage telemetry, and audit evidence. The control plane must not write Skill
files directly or infer Skill lifecycle from filesystem paths.

Future adapter types may include application capability packs, task/context
policy, and OS-code proposals. OS-code adapters must be non-mutating first and
must produce OpenSpec/Superpowers/GitNexus evidence before any source change.

## Risks And Mitigations

- Risk: the control plane becomes a second Skill service.
  - Mitigation: it owns only orchestration state and delegates Skill semantics to
    the Skill service adapter.
- Risk: shells become semantic owners.
  - Mitigation: SDK commands are the only shell-facing boundary, and tests check
    that shell code remains thin.
- Risk: materialization is mistaken for optimization.
  - Mitigation: lifecycle and result names separate proposal, materialization,
    activation/reuse, and normalized benchmark proof.
- Risk: observability leaks sensitive data.
  - Mitigation: DTOs store refs, counts, reasons, and bounded diagnostics only.

## Migration Plan

1. Add this OpenSpec change and validate it.
2. Add provider-neutral DTOs and transition validation tests.
3. Add a minimal runtime-host in-memory provider skeleton with structured logs.
4. Add SDK/SystemFacade unavailable behavior.
5. Add a Skill target adapter interface and a no-mutation adapter skeleton that
   delegates only through service commands when wired in a later slice.
6. Run targeted tests, boundary checks, OpenSpec validation, and GitNexus change
   detection.
