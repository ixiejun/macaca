## Context

Macaca OS architecture governance defines Macaca as a microkernel Agent OS: the kernel owns invariants, services own replaceable capabilities, applications own business behavior and UI, and shells only adapt input/output. WASM applications already pass host imports through a ServiceRuntime-backed bridge for `service.call` and store GenUI surfaces through `ui.render`. The missing layer is generic orchestration: a WASM guest should be able to create goals, query session task state, and request app-scoped agent work without becoming coupled to Web internals or concrete providers.

YAML applications currently gain multi-agent behavior through manifest-declared agents, app-scoped executor registration, PlanLoop/WorkerLoop startup, task board state, and framework tool composition. WASM applications need the same OS capabilities, but their orchestration can be programmatic and guest-driven rather than YAML-step-driven.

## Design Pattern Selection

- **Facade**: expose one WASM Orchestration Portal as the guest-facing surface for task and agent orchestration, hiding Task Service, Application Service, executor, and ServiceRuntime details.
- **Bridge / Adapter**: translate WASM ABI imports into provider-neutral service commands without exposing Rust runtime handles or Web state.
- **Command**: model every WASM orchestration operation as a bounded typed command carrying trace, app, session, and optional agent/task scope.
- **Strategy**: keep routing and policy replaceable; app manifests and service contracts decide which services/agents are allowed.
- **Observer**: emit trace/log/audit events at admission, dispatch, delegation, completion, denial, and failure.
- **Memento**: persist session/task/trace state through existing stores so refresh and replay keep working.
- **Specification**: validate trace, app/session scope, capability metadata, payload bounds, declared agents, and service-contract admission before execution.

## Goals

- Let L2Wasm apps use app-scoped agents, goals, tasks, skills, MCP, LLM, and other services through generic Macaca OS boundaries.
- Preserve WASM flexibility: the guest can decide when to create a goal, query task state, call MCP/skill services, or render UI.
- Ensure every operation is trace-required, policy-governed, logged, auditable, and structured on failure.
- Preserve existing YAML application behavior and existing agentless WASM fast-path behavior.
- Keep all OS crate changes generic and application-agnostic.

## Non-Goals

- Do not create a second task system for WASM.
- Do not move Web-local Toolkit internals across service boundaries.
- Do not hardcode coordinator, planner, worker, crypto, stock, or any app-specific behavior.
- Do not add architecture exemptions, temporary debt rules, or shell-owned orchestration semantics.

## Architecture

The WASM Orchestration Portal extends the existing host import bridge. The bridge remains the choke point for guest requests and continues to own validation, routing, result bounding, logging, and audit metadata.

WASM task imports are mapped to `service.task` operations through `ServiceRuntime`. The first implementation supports goal creation and session task-board query. Because the Task Service runtime already owns `CreateGoalCommand` and `QueryTaskBoardCommand`, the portal adapts guest payloads into those commands instead of introducing WASM-specific task DTOs.

WASM agent delegation is modeled as an Application Service command because app-scoped executor lookup is application lifecycle/runtime state, not task-domain state. Application Service owns the adapter into `ApplicationExecutor` through an injected orchestration backend. When that backend is unavailable, the command returns structured unavailable. This keeps `macaca-runtime-host` generic and prevents it from depending on `macaca-web`.

WASM apps with declared agents are no longer automatically forced into the framework coordinator path. `/api/chat/v2` can dispatch WASM apps through the WASM runtime while Application Service session startup and Web bootstrap ensure app-scoped executors/loops exist when agents are declared. Agentless WASM apps continue to use the existing deterministic host-dispatch path.

Skill and MCP usage remains via `service.call` to `service.skill` and `service.mcp`. The orchestration portal does not special-case those domains; it ensures WASM service contracts and policy allow them when declared.

## Data Flow

1. The user starts a WASM app session through `/api/chat/v2`.
2. Web persists the session envelope and calls Application Service session start.
3. If the app declares agents, Web registers an app-scoped executor and starts PlanLoop/WorkerLoop for that session.
4. Web dispatches the WASM `app:start` export through Application Service host dispatch.
5. The WASM guest emits orchestration host imports:
   - `macaca:task/create_goal` for autonomous task planning.
   - `macaca:task/query` for session task-board inspection.
   - `macaca:agent/delegate` for direct app-scoped fork-join work.
   - `macaca:service/call` for LLM, MCP, Skill, Driver, Memory, Finance, or other services.
   - `macaca:ui/render` for GenUI output.
6. The host bridge validates trace, app/session scope, payload size, capability, policy, service availability, and declared agent scope.
7. Results are returned as bounded `ApplicationHostCommandResult` values with stable reason codes and sanitized metadata.
8. EventLog, RunTracer, service-call audit, task events, and GenUI surfaces remain queryable after browser refresh.

## Error Handling

- Missing trace returns `missing_trace` and never dispatches.
- Missing app or session scope returns a structured rejected result.
- Missing capability metadata returns `capability_missing`.
- Undeclared service or disallowed service returns policy-denied or unavailable metadata.
- Agent delegation to a non-app-scoped agent returns policy-denied.
- Missing orchestration backend returns structured unavailable.
- All errors log trace id when available, import name, app id, session id, service/operation, reason code, and bounded payload size without raw prompt, raw payload body, secrets, env, provider credentials, or unbounded backend output.

## Testing Strategy

- Add runtime-host tests proving task imports route through ServiceRuntime, reject missing scope, and preserve sanitized metadata.
- Add Application Service tests proving agent delegation uses an injected generic backend and rejects unavailable backend cleanly.
- Add Web tests proving WASM apps with declared agents still use WASM host dispatch while app-scoped executor/loops are prepared.
- Add integration/governance tests proving the portal preserves microkernel, service, application-framework, and shell boundaries.
- Validate OpenSpec strictly before implementation.
