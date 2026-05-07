# Design: System service contract

## Context

Phase 01 defined microkernel primitives: service identity, capability identity, trace context, policy request/decision, resource scopes, and facade skeletons. Phase 02 builds on those primitives by defining how replaceable capabilities become system services.

The goal is not to immediately reroute existing production behavior. The goal is to create a contract that every future built-in service, plugin service, optional module, remote service, and WASM-facing service can obey: describe itself, move through lifecycle states, report health, require policy/permission context, accept command-style calls, emit trace/audit events, and clean up resources.

## Goals

- Define open, provider-neutral service descriptor types in `macaca-proto`.
- Define `SystemService` and related lifecycle/call contracts in `macaca-kernel`.
- Require every service call to carry `TraceContext`; missing trace context is a structured error.
- Require service calls to produce trace/audit events and key execution logs.
- Add adapter skeletons for existing service-like crates without changing their current runtime call paths.
- Keep service types extensible through string-backed value objects rather than closed business enums.
- Preserve current YAML applications, `/api/chat/v2`, goal pipeline, live trace, skill/MCP behavior, and no-network baseline behavior.

## Non-Goals

- Do not build the distributed service bus.
- Do not make kernel own concrete provider execution.
- Do not migrate all upper-layer consumers.
- Do not implement Store, Payment, Web3, EVM, GenUI, package manager, or plugin installer behavior.
- Do not create demo-only no-op contracts that cannot enforce trace, lifecycle, health, or structured error invariants.

## Design Patterns

- **Adapter**: Existing LLM, Task, Trace, Driver, Skill, Gateway, and Memory implementations are represented through service adapter skeletons, allowing migration without rewriting business logic.
- **Abstract Factory**: Service descriptors and manifests can later select service adapter implementations without hardcoding provider-specific constructors into callers.
- **Command**: `ServiceCommand` represents one service call with command name, payload, trace, policy, and resource context.
- **Observer**: Service call execution emits trace/audit events through a trace boundary rather than directly coupling to Web SSE or frontend state.
- **Chain of Responsibility**: Service call middleware validates trace context, policy, budget, metering, and logging in a fixed sequence before dispatching to service logic.
- **State**: `ServiceLifecycleState` models install/register/authorize/start/call/stop/cleanup transitions explicitly.
- **Specification**: service permissions, supported scopes, required resources, and trace schemas are data evaluated by policy and compatibility checks.
- **Value Object**: `ServiceType`, `ServiceCapability`, `ServiceCommandName`, and trace schema ids remain string-backed so third-party services can extend the ecosystem.

## Contract Shape

### `macaca-proto/src/service.rs`

The protocol module will define data-only service contracts:

- `ServiceType`: extensible string-backed service classification.
- `ServiceCapability`: capability advertised by a service descriptor.
- `ServiceDescriptor`: service id, type, capabilities, lifecycle, health, permissions, scopes, trace schema, cleanup policy, and metadata.
- `ServiceLifecycleState`: install/register/authorize/start/call/stop/cleanup/error states.
- `ServiceHealth`: healthy/degraded/unavailable/disabled-by-policy style state.
- `ServiceCommand`: command name, JSON payload, trace context, and metadata.
- `ServiceCallResult`: JSON output, trace ids, status metadata, and optional cleanup hints.
- `ServiceError`: structured service contract errors.

These types must not depend on `macaca-web`, concrete provider crates, application manifests, or plugin implementations.

### `macaca-kernel`

The kernel will define service runtime contracts:

- `SystemService`: descriptor export, lifecycle, health check, call, stop, cleanup.
- `ServiceLifecycleController`: lifecycle transition helper that protects state invariants.
- `ServiceCallMiddleware`: Chain of Responsibility trait for trace-required, policy, budget/metering, and logging/audit middleware.
- `ServiceCallExecutor`: facade that validates context, logs key execution boundaries, emits trace/audit events, then calls a `SystemService`.

The kernel can host contracts and generic skeletons, but provider execution remains in service crates or adapters.

### Adapter Skeletons

Adapter skeletons will be introduced for:

- First slice: LLM, Task, Trace.
- Second slice: Driver, Skill, Gateway, Memory.

Each adapter skeleton must export a descriptor, health check, supported scopes, required permissions, and trace schema. Skeletons must not migrate existing production calls yet; they only make services describable and testable through the contract.

## Trace, Audit, And Logging Rules

Every `SystemService::call` invocation must include a `TraceContext`. The generic call executor must reject missing trace context before dispatching to service logic. A successful call must emit a trace event containing service id, command name, lifecycle state, call status, and correlation ids. Failed calls must emit a failure trace/audit event with structured error code and reason.

The implementation must use structured logging around key execution boundaries:

- service registration;
- lifecycle transition;
- call accepted;
- call rejected before dispatch;
- call completed;
- call failed;
- cleanup started/completed.

Logging must not leak secrets or raw provider credentials.

## Permission And Policy Rules

Phase 02 does not enforce a production permission backend, but the contract must carry required permissions and enough policy context for Phase 03+ enforcement. The call middleware chain must make policy insertion explicit so future services cannot bypass policy without changing the contract.

## Error Model

Errors must be structured. Missing trace context, invalid lifecycle transition, unavailable optional service, disabled-by-policy service, unsupported command, health failure, and provider adapter failure must be distinguishable. The service contract must avoid panic/hang/string-only errors for expected service boundary failures.

## Compatibility Plan

1. Add service protocol descriptors and serde tests.
2. Add kernel service traits, lifecycle helper, middleware contract, executor skeleton, and mock tests.
3. Add LLM/Task/Trace adapter skeletons without changing current runtime calls.
4. Add Driver/Skill/Gateway/Memory adapter skeletons without provider hardcode.
5. Add trace-required middleware tests and log/trace assertions.
6. Run Route C baseline and targeted crate checks.

## Risks / Trade-offs

- **Risk: Contract becomes too abstract to use.** Mitigation: mock service tests must prove lifecycle, call, trace, and errors work end-to-end.
- **Risk: Kernel starts owning service behavior.** Mitigation: kernel only owns contracts, lifecycle invariants, middleware sequencing, and trace/audit boundaries.
- **Risk: Adapter skeletons are mistaken for migration completion.** Mitigation: tasks and docs explicitly say runtime calls are not migrated in Phase 02.
- **Risk: Trace/logging adds noise.** Mitigation: trace schema and structured logs focus on service id, command, lifecycle, status, and error code, not raw payload dumps.

## Open Questions

- None for Phase 02. Service bus transport, remote service discovery, package entitlement, and production policy backends belong to later phases.
