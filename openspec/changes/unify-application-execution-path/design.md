# Design: Unified Application Execution Path

## Context

Macaca OS is a microkernel Agent OS. The kernel owns invariants, services own replaceable capabilities, applications own product behavior, and shells render state. The current execution stack violates that shape by allowing multiple production paths to create task graphs and terminal state for the same session.

The observed failure is representative: a WASM app-owned `agent_delegate` execution completed, generated files, and emitted agent events, while a legacy fallback decomposition graph failed and blocked unrelated Task Board entries. The system then projected `ExecutionFailed` even though the authoritative work path had completed.

## Goals

- One execution ingress for all application types.
- One task graph owner for execution work.
- One production agent execution boundary.
- One terminal state projection per application execution run.
- Replayable trace/audit evidence after browser refresh or subscriber loss.
- Application-neutral, provider-neutral, workflow-neutral, and shell-neutral architecture.

## Non-Goals

- No Workbench-specific fix.
- No YAML-specific or WASM-specific terminal rule.
- No rewrite of LLM providers, driver providers, MCP, skills, memory, or context services.
- No new concrete provider dependency in the kernel or SDK.
- No frontend-owned authoritative execution state.

## Ownership Model

```text
Application intent
  -> Application adapter (WASM/YAML/GenUI/headless/app-owned UI)
  -> SDK/SystemFacade
  -> service.application_execution
  -> service.task
  -> service.agent_execution
  -> EventLog / Audit / current-state projection
  -> Shell and app UI renderers
```

- Microkernel: identity, service call invariants, policy facade, trace/audit primitives, session/task identity primitives.
- `service.application_execution`: run ingress, provider assignment, run control, replay, current-state projection, run terminal state.
- `service.task`: execution task graph admission, task lifecycle, review, retry/recovery, task terminal aggregation.
- `service.agent_execution`: agent work execution and agent event production.
- Application framework: manifest, application ABI, YAML adapter, WASM adapter, app-scoped capability declaration.
- Web/CLI/frontend/app-owned UI: command adapters and projection renderers only.

## Decisions

### Decision 1: Application execution is the only ingress

Every application execution request, regardless of application type, enters through `service.application_execution`. WASM host imports, YAML workflow steps, GenUI actions, headless app triggers, and shell routes may produce commands, but they do not own run lifecycle.

Patterns: Facade, Command, Adapter.

### Decision 2: Task Service owns the execution task graph

Task graph creation, dependency evaluation, review, retry, and task terminal aggregation belong to `service.task`. Compatibility fallback tasks must be explicitly marked as compatibility or diagnostic graph entries and must not become authoritative application-execution terminal facts.

Patterns: State, Mediator, Observer.

### Decision 3: Agent Execution Service owns agent work

Application adapters and Task Service call `service.agent_execution` for actual agent work. They must not construct runtime agents directly. Agent work emits sanitized events that application execution can mirror and replay.

Patterns: Strategy, Adapter, Decorator.

### Decision 4: Current state is a projection, not a second source of truth

Current state is derived from EventLog and Task Service state. Web, app-owned UI, and local browser caches must render projections only. Local event arrays are mementos for display continuity, not authoritative state.

Patterns: Observer, Memento, State.

### Decision 5: Compatibility paths are contained

Legacy `loop_manager` fallback decomposition can remain during migration only as a compatibility adapter. It must carry a non-authoritative graph owner and structured reason code. It must not create a second authoritative task graph for the same execution run.

Patterns: Adapter, Specification.

## Terminal State Rules

- `Completed`: all required authoritative execution tasks are terminal completed and no required recovery remains.
- `Failed`: a required authoritative execution task failed and bounded recovery is exhausted.
- `Blocked`: the authoritative execution graph waits on approval, resource, entitlement, policy, or external dependency.
- `Running`: any authoritative execution task is pending, assigned, in progress, pending review, or recoverable.
- `Cancelled`: a traced control command cancelled the authoritative execution run.
- `Degraded`: optional or compatibility work failed but authoritative required execution completed.

Non-authoritative compatibility failures may emit diagnostics but cannot by themselves mark the application execution run failed.

## Logging, Trace, And Audit

Every key node must log bounded fields:

- application id
- session id
- run id
- service id
- command name
- graph owner
- task id when available
- trace id
- lifecycle state
- reason code

Logs and events must not include raw prompts, secrets, package bytes, WASM bytes, private keys, raw provider payloads, or unbounded output. Full artifacts must be referenced through payload refs or source artifact refs.

## Migration Plan

1. Add OpenSpec and tests that encode the unified path.
2. Tighten or annotate application execution DTOs so run ownership and terminal projection ownership are explicit.
3. Add Task Service task graph ownership and admission rules.
4. Move Web fallback decomposition behind Task Service compatibility strategy seams.
5. Update hosted execution aggregation to count only authoritative execution rows as terminal facts.
6. Force WASM and YAML adapters through the same application execution and agent execution command chain.
7. Update shell/app UI projections so they render current state and replay only.
8. Add end-to-end tests with one WASM app-owned task and one YAML app task.

## Risks / Trade-Offs

- Risk: Migration changes existing YAML behavior.
  - Mitigation: Add cross-adapter equivalence tests and keep compatibility adapters until new path is proven.
- Risk: Task Service becomes a God Service.
  - Mitigation: Task Service owns task graph state only; provider strategies execute planning/review/worker behavior through replaceable service calls.
- Risk: Terminal projection hides real failures.
  - Mitigation: Non-authoritative failures are still logged and replayed as diagnostics; only terminal authority is separated.
- Risk: Web shell remains hidden semantic owner.
  - Mitigation: Add boundary tests that reject direct provider construction or task semantic creation from Web routes.

## Open Questions

- Should compatibility graph entries be visible in the default Task Board, or shown in a separate diagnostics rail?
- Should the first implementation use an explicit `graph_owner` enum or derive ownership from existing service command scope?
- Should `Degraded` be added to current-state lifecycle now, or represented as `Completed` with diagnostics in the first slice?
