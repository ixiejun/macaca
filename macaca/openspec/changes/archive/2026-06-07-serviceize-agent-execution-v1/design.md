# Design: Serviceized Unified Agent Execution

## Context

Macaca OS is a microkernel Agent OS. The kernel must guard invariants, services must own replaceable capabilities, applications must own product behavior, and shells must remain adapters. Agent execution is replaceable, extensible, policy-governed, and variable by application, tenant, provider, and runtime. It therefore belongs behind a service boundary.

The current implementation still has several production entrypoints that can start agent work. Some routes enter the framework context builder, while others can reach an executor or launcher path that does not consistently build persona, skill snapshot, tool policy, workspace context, memory/context recall, trace, or audit evidence.

## Goals

- Provide exactly one production path for starting agent work.
- Make agent execution and agent context construction service-owned capabilities.
- Preserve YAML, WASM, GenUI, headless, chat, task, and goal application behavior through adapters.
- Ensure every agent execution has trace, policy, context snapshot, sanitized logs, audit, and structured errors.
- Remove Web shell semantic ownership of agent construction.

## Non-Goals

- Do not move concrete agent execution into the microkernel.
- Do not make WASM, YAML, or task loops special execution owners.
- Do not redesign LLM provider routing, skill runtime, MCP runtime, memory runtime, or task planning beyond their service calls.
- Do not add app-specific branches or business-domain logic.

## Decisions

### Decision: Agent execution is a system service

Introduce `service.agent_execution` as the only production boundary that starts agent work. All application adapters must dispatch `AgentExecutionCommand` through `ServiceRuntime`.

### Decision: Context construction is a system service

Introduce `service.agent_context` to own persona, manifest semantics, capabilities, skill snapshot, MCP/tool catalog, memory/context recall, workspace guide context, and tool policy context. Applications may supply user prompts and bounded context, but not system prompts.

### Decision: Application adapters produce commands only

YAML workflow steps, WASM `macaca:agent/delegate`, chat main thread, task/goal workers, and future SDK entrypoints are command producers. They do not own agent construction semantics.

### Decision: Existing framework behavior becomes a built-in provider

The current `FrameworkRunner` behavior should be extracted behind built-in service providers so behavior can remain stable while ownership is corrected.

## Architecture

```text
Application input
  -> application runtime adapter
  -> ServiceRuntime.call(service.agent_execution, AgentExecutionCommand)
  -> AgentExecutionService
  -> ServiceRuntime.call(service.agent_context, AgentContextBuildCommand)
  -> Agent runtime provider
  -> EventLog / Audit / RunTrace / live stream
  -> AgentExecutionResult
```

`AgentExecutionCommand` carries app, session, task, source agent, target agent, execution intent, user prompt, delegated context, capability scope, trace context, and policy context.

`AgentContextBuildCommand` carries app, session, task, target agent, execution intent, declared scope, trace context, policy context, context budget, and snapshot policy.

`AgentExecutionResult` carries structured status, bounded output, artifacts, trace ids, context snapshot reference, token/cost metadata when available, and sanitized diagnostics.

## Layer Ownership

- Kernel: identity, service registry primitive, policy facade, trace identity, session/task primitive.
- Runtime host: ServiceRuntime, service decorators, provider bootstrapping, WASM host-import bridge, diagnostics.
- System services: agent execution, agent context, LLM/model routing, skill, MCP, memory/context, task planning/review.
- Application framework: manifests, YAML adapter, WASM ABI adapter, app-scoped permission declarations, app session envelope.
- Shells: input parsing, command submission, rendering, approval, trace/audit diagnostics.

## Migration Plan

1. Define service DTOs and unavailable providers.
2. Extract context building from Web-owned `FrameworkRunner` into the first built-in `service.agent_context` provider.
3. Wrap current framework runtime execution as the first built-in `service.agent_execution` provider.
4. Migrate WASM `agent.delegate` to `service.agent_execution`.
5. Migrate YAML workflow steps to `service.agent_execution`.
6. Migrate chat main thread execution to `service.agent_execution`.
7. Migrate task/goal worker execution to `service.agent_execution`.
8. Deprecate or remove direct production launch paths after all consumers converge.
9. Add dependency-boundary and audit-replay gates.

## Risks And Mitigations

- Risk: serviceization changes prompt behavior.
  - Mitigation: first provider preserves existing context output byte-for-byte where possible; add snapshot comparison tests.
- Risk: Web remains hidden semantic owner.
  - Mitigation: Web may register providers at composition root, but service contracts and context rules live outside shell-only modules.
- Risk: migration breaks live trace or refresh replay.
  - Mitigation: EventLog append remains before streaming; audit replay tests cover service admission, context build, model call, and result.
- Risk: old paths survive indefinitely.
  - Mitigation: add tests and static gates that reject new production direct agent construction.

## Open Questions

- Should `service.agent_context` be public to SDK/app-framework clients immediately, or only callable by `service.agent_execution` in v1?
- Which store should own persistent `AgentContextSnapshot` references?
- Should direct `FrameworkRunner` builders become compile-time deprecated first or hard-disabled as soon as migrations finish?
