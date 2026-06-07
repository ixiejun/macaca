# Skill Autonomous Materialization Operator Design

## Context

Run 38 rechecked the live self-evolution loop after the service-owned proposal
materialization lane was added. The real `/api/chat/v2` task failed before
terminal success because the configured LLM provider returned a `524`, so that
run correctly produced no new proposal. The surrounding operations snapshots
still exposed the next platform gap: proposals continue to accumulate as
`Draft`/`Queued`, while no autonomous operator invokes processing, marks any
record `ReadyForMaterialization`, calls the materialization command, registers
load-path evidence, or emits usage telemetry.

The previous materialization lane intentionally solved only the privileged
single-proposal command. The missing capability is a governed operator that can
drive the backlog through processing and materialization without moving policy
or semantics into Web, CLI, applications, or the kernel.

## Requirements

- The operator must live behind `service.skill` as a provider-neutral command
  and result contract.
- The operator must be trace-required, policy-gated, entitlement-aware,
  rollback-backed, and observable through sanitized logs and snapshots.
- The operator must not branch on application names, workflow names, model
  names, provider names, driver names, gateway names, chain names, payment
  names, or business-domain identifiers.
- The operator must never return raw prompts, raw provider payloads, full
  generated `SKILL.md` bodies, package bytes, manifests, credentials, secrets,
  or unbounded task output.
- The operator must not count materialization as later activation, reuse, or
  optimization without separate telemetry evidence.

## Recommended Approach

Implement a service-owned orchestration command inside the Skill service
provider. The command composes existing Strategies instead of duplicating them:

1. Run deterministic proposal processing with caller-provided policy and audit
   refs.
2. Select only records in `ReadyForMaterialization`.
3. Resolve a package root through a provider-neutral target resolver Strategy.
4. Invoke the existing proposal materialization command for each eligible
   record, respecting dry-run/apply mode and bounded batch limits.
5. Return an aggregate result containing processed counts, selected proposal
   refs, materialization result refs, denied reasons, rollback refs, and
   telemetry handoff requirements.

This keeps the operator small and auditable. It acts as a Director over existing
Command, Strategy, State, Builder, Specification, Memento, and Observer
components rather than becoming a second implementation of processing or
materialization.

## Alternatives Considered

- Web observer calls processing and materialization directly after proposal
  creation. This would be simple to trigger but would move self-evolution
  semantics into a presentation shell, violating the shell boundary.
- A background kernel scheduler owns proposal materialization. This would make
  materialization feel automatic, but Skill processing and package writes are
  replaceable capability behavior, not kernel invariants.
- Manual operations-only materialization. This is useful for debugging but does
  not satisfy the autonomous closed-loop proof requirement.

## Data Flow

```text
service.agent_execution completion
  -> SkillExperienceProposalCommand
  -> Skill proposal backlog
  -> SkillAutonomousMaterializationRunCommand
  -> processing Strategy
  -> ReadyForMaterialization records
  -> package target resolver Strategy
  -> materialization command
  -> content mutation Strategy
  -> governance promotion
  -> operations snapshot evidence
  -> later activation/reuse telemetry gates
```

## Acceptance Criteria

- Focused tests prove the operator denies apply mode without policy refs,
  package readiness, entitlement readiness, or package target resolution.
- Focused tests prove dry-run mode processes/selects/materializes previews
  without writing files or promoting governance.
- Focused tests prove apply mode can process a high-quality proposal,
  materialize `SKILL.md`, promote governance, and return body-free audit refs.
- Operations snapshots expose operator run counts and materialization result
  counts without leaking generated bodies.
- A live `/api/chat/v2` proof can distinguish external LLM failure from a clean
  successful run that reaches proposal capture, operator invocation,
  materialization, registry/load-path evidence, and later usage telemetry.

## Scope Control

This design does not add activation/reuse optimization scoring. It only creates
the missing autonomous bridge from captured proposal backlog to governed
materialization. Activation, registry load-path consumption, usage telemetry,
and measurable optimization remain separate gates after materialization evidence
exists.
