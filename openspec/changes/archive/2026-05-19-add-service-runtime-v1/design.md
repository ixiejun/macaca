# Design: ServiceRuntime v1

## Context

Existing Route C foundations already provide protocol contracts, service call enforcement, and service bus routing:

- `macaca-proto` owns provider-neutral service descriptors, commands, lifecycle states, health, trace context, and service errors.
- `macaca-ipc` owns service bus facade, middleware, trace sink, local service transport, and future transport bridge points.
- `macaca-kernel` owns microkernel primitives such as service registry, `SystemService`, `ServiceCallExecutor`, and the `SystemServiceBusHandler` adapter.
- `macaca-runtime-host` owns host-level runtime glue for plugin/MCP/package/entitlement and is the correct owner for provider runtime orchestration.

S1 must connect these pieces without turning the kernel, Web, or CLI into provider construction hubs.

## Goals

- Provide a host-owned `ServiceRuntime` facade.
- Register provider-neutral system services through factories.
- Start, call, stop, clean up, and health-check services.
- Dispatch calls through the existing service bus path.
- Enforce trace-required and policy-required runtime admission before dispatch.
- Emit structured logs and audit-friendly runtime events at lifecycle and call nodes.
- Return deterministic snapshots for diagnostics.
- Keep S1 additive and provider-migration-free.

## Non-Goals

- No concrete provider migration.
- No provider dependency removal.
- No ServiceRuntime ownership inside `macaca-kernel`.
- No presentation shell changes.
- No real remote transport implementation.
- No real entitlement, resource lock, payment, or metering enforcement beyond pluggable extension points.

## Design Patterns

### Facade

`ServiceRuntime` hides the implementation details of factory invocation, runtime registry state, local transport handler registration, service bus dispatch, lifecycle transitions, events, and snapshots.

### Abstract Factory

`ServiceProviderFactory` creates `Arc<dyn SystemService>` from provider-neutral descriptors and runtime context. This prevents S1 from hardcoding LLM, memory, driver, gateway, skill, MCP, payment, Web3, EVM, application, or workflow names.

### Bridge

Runtime calls use `macaca-ipc::ServiceBus` and local service transport. The bus boundary keeps runtime dispatch independent from current local-only execution and leaves remote/plugin transports pluggable later.

### Adapter

`macaca-kernel::SystemServiceBusHandler` adapts a bus `ServiceEnvelope` into the existing `SystemService` call executor path. Runtime does not duplicate kernel service-call semantics.

### Decorator and Chain of Responsibility

Runtime admission control is an ordered decorator chain:

- trace-required decorator,
- policy decorator,
- future resource decorator,
- future entitlement decorator,
- future metering decorator.

Decorators validate before bus dispatch. This keeps cross-cutting checks composable and auditable.

### Strategy

Policy evaluation is a replaceable strategy. S1 should include deterministic allow/deny strategies for tests while preserving a runtime extension point for real permission, budget, region, entitlement, and module-availability policy engines.

### State

Runtime service records own lifecycle state transitions for registered services. S1 should record Registered, Starting, Running, Calling, Stopping, Stopped, CleaningUp, CleanedUp, and Failed.

### Observer

`ServiceRuntimeEventSink` receives structured lifecycle/call/rejection/failure events. Runtime also uses `tracing` logs at key execution nodes.

### Memento

`ServiceRuntimeSnapshot` captures deterministic runtime health and lifecycle state for diagnostics, tests, and future service inspector surfaces.

## Runtime Model

### Provider Factory

The factory returns:

- a stable `ServiceDescriptor`,
- an `Arc<dyn SystemService>`,
- optional factory metadata for diagnostics.

The runtime validates descriptor identity and registers a local bus handler for the service id.

### Lifecycle

Lifecycle operations must be explicit:

1. Register provider factory.
2. Record Registered.
3. Start service: Starting -> Running or Failed.
4. Call service: Running -> Calling -> Running, or Failed on provider/bus failure.
5. Stop service: Stopping -> Stopped or Failed.
6. Cleanup service: CleaningUp -> CleanedUp or Failed.
7. Snapshot service state and health.

### Call Path

The runtime call path is:

1. Caller submits service id, source, and `ServiceCommand`.
2. Runtime checks service exists.
3. Runtime decorators enforce trace and policy.
4. Runtime builds `ServiceEnvelope`.
5. Runtime dispatches through `ServiceBus`.
6. Local transport routes to `SystemServiceBusHandler`.
7. Kernel service executor applies existing trace-required middleware and emits service call trace events.
8. Runtime records completion, failure, and events.

### Trace and Audit

S1 must produce audit-friendly evidence at these nodes:

- provider registration requested/succeeded/failed,
- service start requested/succeeded/failed,
- call accepted/rejected/dispatched/completed/failed,
- policy allowed/denied,
- stop requested/succeeded/failed,
- cleanup requested/succeeded/failed,
- snapshot requested.

Runtime code must use English comments to explain function and runtime principles. Logs should use structured fields such as service id, command name, trace id, lifecycle state, health, and error.

## Dependency Boundary

S1 may add `macaca-runtime-host -> macaca-ipc` to bridge runtime-host orchestration to service bus transport. This is consistent with runtime-host service orchestration and should pass the S0 dependency gate.

S1 must not add:

- `macaca-kernel -> provider` direct dependencies,
- `macaca-web` or `macaca-cli` new provider dependencies,
- provider-to-presentation dependencies,
- optional module dependencies required by base OS crates.

## Risks and Mitigations

- Risk: ServiceRuntime becomes a provider construction hub.
  - Mitigation: use descriptor-driven factories and no provider/category-specific branches.

- Risk: Policy-required is implemented as a no-op.
  - Mitigation: policy is a Strategy with explicit allow/deny decisions; tests must prove denial happens before dispatch.

- Risk: Trace checks are duplicated across runtime, bus, and kernel service executor.
  - Mitigation: runtime performs admission control; bus/executor remain defense-in-depth. Tests verify missing trace is rejected.

- Risk: Lifecycle state diverges from descriptor state.
  - Mitigation: S1 snapshots report runtime-owned state; descriptors remain service-advertised contract data.

- Risk: Dependency gate fails after adding runtime-host to ipc dependency.
  - Mitigation: run S0 dependency gate and stop if a forbidden edge appears.

## Verification

- `openspec validate add-service-runtime-v1 --strict`
- `cargo fmt --check`
- `cargo test -p macaca-runtime-host service_runtime`
- `cargo test -p macaca-ipc service_bus`
- `cargo test -p macaca-kernel system_service`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cargo check --workspace`
- `npx gitnexus detect-changes --repo agent`
