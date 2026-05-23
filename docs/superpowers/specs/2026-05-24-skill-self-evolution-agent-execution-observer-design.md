# Skill Self-Evolution Agent Execution Observer Design

## Context

Live verification showed that real delegated tasks completed and emitted
`delegated_task_complete`, but the Skill operations snapshot still contained no
experience proposals and the session EventLog had no
`skill_self_evolution_observer` event. The current repair attempt attached the
observer to several shell-adjacent points, which made coverage difficult to
prove and left the production completion boundary ambiguous.

## Selected Approach

Use a Decorator plus Observer at the `service.agent_execution` backend boundary.
`macaca-web` will wrap `WebAgentExecutionBackend` with a focused
`SkillSelfEvolutionObservedAgentExecutionBackend` before registering
`AgentExecutionSystemServiceProvider`. The wrapper calls the inner backend,
observes the returned `AgentExecutionResult`, records a bounded EventLog
checkpoint, and forwards a typed `SkillExperienceProposalCommand` through the
existing SDK Skill facade.

This keeps the provider-neutral Agent Execution trait unchanged. Runtime Host
continues to own the service boundary, Web remains a composition adapter, and
the Skill service remains the only owner of proposal validation, storage,
curation, promotion, rejection, and rollback semantics.

## Design Pattern Mapping

- Decorator: the wrapper adds observation around an existing
  `AgentExecutionBackend` without changing the backend trait or runtime-host
  provider.
- Observer: successful execution results become sanitized proposal candidates
  through the Skill service facade.
- Command: proposal creation still crosses the Skill service as
  `skill.evolution.propose_from_task`.
- Facade: Web uses `SystemSkillClient`; it does not construct or inspect Skill
  provider internals.
- Memento: observer EventLog entries contain bounded status, task id,
  proposal id, and failure reason so live verification can replay what happened.

## Ownership Boundaries

The wrapper may read only `AgentExecutionResult` identifiers, status, metadata,
and bounded output shape. It must not parse prompts, branch on application names,
inspect business output, mutate skill files, or make promotion decisions.

The wrapper records unavailable or rejected proposal outcomes as explicit
observer events but must never fail the original agent execution because Skill
self-evolution is unavailable.

## Trace And Audit

The observer must emit structured logs for:

- service result seen.
- proposal forwarding attempted.
- proposal created.
- proposal skipped or failed.

The EventLog checkpoint must be written after the service result is available and
before the wrapper returns to callers when possible. This makes live black-box
verification deterministic: a completed service call can be checked for a nearby
`skill_self_evolution_observer` event and for proposal count growth through
`GET /api/apps/{app_id}/skills/operations`.

## Implementation Notes

The current scattered observer hooks in `chat_orchestrator`,
`agent_execution_backend`, and `event_persistence` should be removed or reduced
to plain EventLog persistence. Otherwise the same task can produce duplicate
proposals and the live chain remains hard to reason about.

The existing `skill_self_evolution_observer.rs` command builder should remain
the proposal conversion owner. The new decorator should call that module instead
of duplicating proposal construction.

## Verification

- Add a unit/source guard proving `macaca-web` registers Agent Execution through
  the observed backend wrapper.
- Add a unit/source guard proving the previous scattered observer hooks are not
  still present in chat orchestration or event persistence.
- Run targeted `macaca-web` tests for the observer and agent execution backend.
- Run `cargo check -p macaca-web`.
- Run `openspec validate add-self-evolution-evaluation-harness --strict`.
- Run `git diff --check` and GitNexus change detection.
