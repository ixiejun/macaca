# Serviceized Agent Execution Design

Date: 2026-05-15

## Context

Macaca OS currently has several ways to start agent work: chat/YAML framework paths, executor delegation, WASM `agent.delegate`, and task/goal worker loops. These paths grew from different migration phases. They now violate the stable architecture direction defined by:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

The most visible failure is WASM delegation: it can reach an executor fast path that treats delegated work as a raw prompt and does not always pass through the same persona, skill snapshot, tool policy, context, trace, and audit construction used by YAML applications.

## Decision

Macaca OS shall have exactly one production agent execution path:

```text
Application input
  -> application runtime adapter
  -> ServiceRuntime
  -> service.agent_execution
  -> service.agent_context
  -> agent runtime provider
  -> trace, audit, event log, result
```

YAML, WASM, chat, task, goal, worker, SDK, and future app adapters may differ only in how they produce typed commands. They must not own separate agent execution semantics.

## Ownership

The kernel owns identity, policy facade, trace identity, session/task primitives, service registry, and service-call dispatch invariants. It does not own concrete agent execution.

`macaca-runtime-host` owns `ServiceRuntime`, service decorators, host-side service providers, WASM host-import routing, and sanitized diagnostics.

`service.agent_execution` owns the agent execution command surface, lifecycle admission, event emission, result shape, structured errors, and provider replacement.

`service.agent_context` owns persona loading, manifest semantics, capability context, workspace guide context, skill snapshot resolution, MCP/tool catalog context, memory/context recall, and tool policy context.

The application framework owns manifest interpretation, YAML adapter behavior, WASM ABI adaptation, application lifecycle, app-scoped permissions, and app-owned UI surfaces.

Web, CLI, and frontends submit commands, render state, show trace/audit diagnostics, and subscribe to events. They must not be semantic owners of agent construction.

## Service Contracts

`AgentExecutionCommand` is the only production command that starts agent work. It must include:

- `application_id`
- `session_id`
- `task_id` when task-scoped
- `source_agent`
- `target_agent`
- `execution_intent`
- `user_prompt`
- `delegated_context`
- `capability_scope`
- `trace_context`
- `policy_context`

Applications and upstream agents may provide only `user_prompt` and bounded `delegated_context`. They may not provide system prompts.

`AgentContextBuildCommand` builds the trusted system context for a target agent. It must include:

- app, session, task, and target agent identity
- execution intent
- declared capability scope
- trace and policy context
- context budget and snapshot policy

`AgentContextSnapshot` must expose sanitized evidence of what was used: persona source, manifest prompt semantics, visible skills, filtered skills, tool policy, MCP/tool catalog summary, memory/context summary, and workspace guide sources.

## Unified Flow

1. An application runtime adapter receives input from chat, YAML workflow, WASM ABI, task loop, goal loop, SDK, or future gateway.
2. The adapter validates app/session scope and produces `AgentExecutionCommand`.
3. The adapter calls `ServiceRuntime` for `service.agent_execution`.
4. `service.agent_execution` applies policy, resource, budget, entitlement, and trace admission before side effects.
5. `service.agent_execution` calls `service.agent_context` to build trusted system context.
6. `service.agent_context` emits `agent_context_built`, `skill_catalog_built`, and `skill_snapshot_created` when applicable.
7. `service.agent_execution` invokes the selected agent runtime provider with system context as system input and `user_prompt` as user input.
8. Agent runtime events are emitted to EventLog, audit, trace, and live subscribers before being streamed to shells.
9. The service returns `AgentExecutionResult` with bounded output, structured status, trace references, and sanitized diagnostics.

## Migration

The migration should be staged but must converge on the single service path.

1. Add OpenSpec contracts and DTOs for `service.agent_execution` and `service.agent_context`.
2. Register unavailable/null providers first so absence is explicit.
3. Extract current `FrameworkRunner::build_context_system_prompt` behavior into an internal `service.agent_context` provider without changing prompt content.
4. Wrap current framework agent execution as the first built-in `service.agent_execution` provider.
5. Move WASM `macaca:agent/delegate` from executor fast path to `service.agent_execution`.
6. Move YAML workflow agent steps to `service.agent_execution`.
7. Move chat main-thread agent execution to `service.agent_execution`.
8. Move task/goal worker execution to `service.agent_execution`.
9. Deprecate direct production use of `AgentExecutionLauncher::launch`, direct `FrameworkRunner` agent construction, and executor-owned agent runtime creation.
10. Add boundary tests that fail if new production code starts an agent outside `service.agent_execution`.

## Rejections

The following designs are rejected:

- A thin facade that leaves four production execution paths alive.
- A WASM-specific delegate path that hand-builds prompts or bypasses context construction.
- A Web-owned agent context builder as the long-term semantic owner.
- A kernel-owned agent execution implementation.
- A task executor that directly constructs framework agents.
- Any app-specific branch based on app name, workflow name, provider name, model name, symbol, or business domain.

## Acceptance Gates

- YAML, WASM, chat, task, goal, and future SDK invocation all dispatch through `service.agent_execution`.
- WASM `agent.delegate` produces persona, skill snapshot, tool policy, context, trace, and audit evidence equivalent to YAML delegation.
- No production path treats delegated prompt as system prompt.
- EventLog persists agent context and execution events before live streaming.
- Missing skill, MCP, memory, model, or context providers return structured unavailable/denied states.
- Audit replay can reconstruct: application input, service admission, context build, agent runtime, result, and UI/event output.
- Web and CLI do not define system semantics for agent execution or context construction.

## Open Questions

- Whether `service.agent_context` should be a separate service id from day one or an internal dependency of `service.agent_execution` with a public command surface.
- Whether the initial provider should persist full `AgentContextSnapshot` in EventLog, a dedicated store, or both.
- Whether old framework construction methods should be hard-disabled immediately after migration or first moved behind compile-time deprecation gates.
