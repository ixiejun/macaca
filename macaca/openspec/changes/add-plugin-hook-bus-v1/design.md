# Design: Plugin Hook Bus v1

## Context

OpenClaw shows that mature plugin systems need typed lifecycle hooks with failure policy and timeout handling. Hermes shows a simple hook set is useful for tools, LLM calls, gateway dispatch, session lifecycle, and approvals. Macaca needs an OS-grade version that is traceable, auditable, and compatible with Route C service boundaries.

## Goals

- Provide a typed Hook Bus for plugin lifecycle participation.
- Support observer, mutating, blocking, and approval hook semantics.
- Make every hook invocation bounded, logged, and auditable.
- Preserve existing application, chat, trace, task, gateway, driver, skill, MCP, memory, and context behavior unless a hook explicitly and safely modifies a supported result.
- Prepare future WASM/process/remote plugin hosts.

## Non-Goals

- No arbitrary plugin execution implementation in this proposal.
- No direct prompt or secret body exposure to plugins.
- No plugin-owned core state mutation.
- No provider-specific hook branches.

## Architecture

```text
Service / Framework Event
  -> PluginHookInvocation Command
  -> PluginHookRegistry
  -> PluginHookRunner
  -> Host Proxy / Descriptor-safe Handler / Structured Unavailable
  -> Validated Hook Result
  -> Trace / Audit / Service Decision
```

## Design Patterns

- **Observer**: services publish lifecycle/execution events to hook subscribers.
- **Chain of Responsibility**: hooks execute by priority and may stop lower-priority hooks for blocking decisions.
- **Strategy**: timeout and failure policies are replaceable per hook kind.
- **Command**: every hook invocation is a typed command with trace and scope.
- **Specification**: hook descriptors and hook results are schema-validated.
- **Proxy**: future WASM/process/remote hooks execute behind host proxies.
- **Null Object**: missing hook handlers return structured no-op/unavailable results.

## Hook Categories

- `observer`: observes events, cannot mutate decisions, default fail-open.
- `mutating`: returns a bounded, schema-validated contribution or rewrite.
- `blocking`: returns allow/block/require-approval.
- `approval`: participates in approval lifecycle without bypassing approval policy.

## Initial Hook Set

- `before_agent_start`
- `after_agent_end`
- `before_prompt_build`
- `after_context_assemble`
- `before_tool_call`
- `after_tool_call`
- `before_llm_call`
- `after_llm_call`
- `before_memory_ingest`
- `after_memory_ingest`
- `before_gateway_dispatch`
- `after_gateway_send`
- `before_approval_request`
- `after_approval_response`
- `session_start`
- `session_end`
- `task_started`
- `task_completed`
- `application_start`
- `application_stop`

## Safety

- Hook payloads must be minimal and scoped.
- Hook results must be validated before use.
- Hooks must not receive secret values or unbounded raw prompt/memory bodies.
- Blocking hooks must fail closed only when policy says so.
- Observer hooks should default fail-open after timeout or error.

## Trace And Audit

Every hook invocation must emit plugin id, hook name, hook kind, priority, timeout policy, failure policy, duration, decision, trace id, status, and structured error code. Logs must avoid secrets and unbounded payloads.

## Risks

- **Risk: Hooks make execution nondeterministic.** Mitigation: deterministic priority ordering and explicit result semantics.
- **Risk: Hooks block 7x24 execution.** Mitigation: timeouts and failure policies are mandatory.
- **Risk: Hooks become hidden policy bypasses.** Mitigation: all hook effects are interpreted by service code after permission/resource admission.
