# S1 ServiceRuntime v1 Implementation Plan

## Scope

Implement S1 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`: `ServiceRuntime v1`.

S1 creates the runtime layer that registers provider-neutral system services, starts them, routes calls through the existing service bus, stops/cleans them up, and exposes health/lifecycle snapshots. It does not migrate concrete providers and does not remove existing direct dependencies recorded in `macaca/docs/route-c-serviceization-allowlist.md`.

## Required Governance Inputs

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- Current `macaca-proto`, `macaca-ipc`, `macaca-kernel`, and `macaca-runtime-host` service-related modules

## Architecture Decision

Use a host-owned `ServiceRuntime` facade in `macaca-runtime-host`.

Design patterns:

- Facade: `ServiceRuntime` coordinates registration, lifecycle, bus dispatch, stop, cleanup, and snapshots.
- Abstract Factory: `ServiceProviderFactory` creates provider-neutral `SystemService` instances from descriptors and runtime context.
- Bridge: runtime calls services through `macaca-ipc::ServiceBus` and local service transport, not by directly invoking providers from callers.
- Adapter: `macaca-kernel::SystemServiceBusHandler` adapts bus envelopes to `SystemService`.
- Decorator / Chain of Responsibility: runtime decorators enforce trace, policy, and future resource/entitlement/metering checks.
- Strategy: policy evaluation is a replaceable strategy with allow/deny test implementations.
- State: runtime-owned lifecycle records track Registered, Starting, Running, Calling, Stopping, Stopped, CleaningUp, CleanedUp, and Failed transitions.
- Observer: runtime emits structured service lifecycle/call events to an event sink.
- Memento: runtime snapshots expose deterministic service health/lifecycle state for diagnostics and tests.

Rejected alternatives:

- Kernel-owned runtime: rejected because kernel must not own provider orchestration.
- IPC-only runtime: rejected because bus alone does not model lifecycle, provider factories, policy, or snapshots.
- Parallel trait-only runtime: rejected unless dependency cycles block reuse of existing `SystemService` and bus contracts.

## Proposed OpenSpec Change

Expected change id:

- `add-service-runtime-v1`

Expected artifacts:

- `openspec/changes/add-service-runtime-v1/proposal.md`
- `openspec/changes/add-service-runtime-v1/design.md`
- `openspec/changes/add-service-runtime-v1/tasks.md`
- `openspec/changes/add-service-runtime-v1/specs/service-runtime/spec.md`

The proposal should explicitly state:

- S1 is additive-first.
- S1 does not migrate providers.
- S1 does not remove allowlist debt.
- S1 must not introduce provider/app/workflow hardcoding.
- S1 must preserve current user-visible flows.

## Implementation Slices

### Slice S1.1: Impact and Dependency Check

Files to inspect before editing:

- `macaca/crates/macaca-runtime-host/Cargo.toml`
- `macaca/crates/macaca-runtime-host/src/lib.rs`
- `macaca/crates/macaca-kernel/src/system_service.rs`
- `macaca/crates/macaca-kernel/src/service_bus_bridge.rs`
- `macaca/crates/macaca-ipc/src/service_bus.rs`
- `macaca/crates/macaca-ipc/src/local_service.rs`
- `macaca/crates/macaca-proto/src/service.rs`

Required actions:

1. Run GitNexus impact before modifying any existing symbol.
2. If impact is HIGH or CRITICAL, stop and report blast radius before editing.
3. Confirm new crate dependencies do not violate S0 gate.

### Slice S1.2: Service Provider Factory

Files:

- New: `macaca/crates/macaca-runtime-host/src/service_provider.rs`

Define:

- `ServiceProviderFactory`
- `ServiceProviderFactoryContext`
- `ServiceProviderFactoryError` or reuse service/runtime error types if cleaner
- a test/mock factory used by runtime tests

Rules:

- Factory must return provider-neutral `Arc<dyn SystemService>`.
- Factory must not branch on LLM, driver, gateway, skill, memory, app, model, chain, or business names.
- Factory metadata must be descriptor/capability driven.
- Comments must explain how Abstract Factory avoids hardcoded provider construction.

### Slice S1.3: Runtime Event and Snapshot Model

Files:

- New: `macaca/crates/macaca-runtime-host/src/service_runtime.rs`

Define:

- `ServiceRuntime`
- `ServiceRuntimeConfig`
- `ServiceRuntimeState`
- `ServiceRuntimeSnapshot`
- `ServiceRuntimeServiceSnapshot`
- `ServiceRuntimeEvent`
- `ServiceRuntimeEventSink`
- `InMemoryServiceRuntimeEventSink`

Rules:

- Snapshot ordering must be deterministic by service id.
- Events must include service id, operation, lifecycle state, health, timestamp, trace id when available, and structured payload.
- Runtime logs must cover register/start/call/stop/cleanup/health/failure.
- Runtime must not store provider-specific state outside `Arc<dyn SystemService>`.

### Slice S1.4: Runtime Decorator Chain

Files:

- New: `macaca/crates/macaca-runtime-host/src/service_decorator.rs`

Define:

- `ServiceRuntimeDecorator`
- `ServiceRuntimeCallContext`
- `TraceRequiredRuntimeDecorator`
- `PolicyRuntimeDecorator`
- optional extension traits/placeholders for resource, entitlement, and metering decorators
- `ServiceRuntimePolicy`
- `ServiceRuntimePolicyDecision`
- allow/deny policy implementations for tests

Rules:

- Missing trace must be rejected before bus dispatch.
- Policy must be evaluated before bus dispatch.
- Policy denial must be structured and logged.
- The decorator chain must be ordered and explicit.
- S1 can include no-op/pass-through resource/entitlement/metering extension points, but they must not claim real enforcement until later phases.

### Slice S1.5: Bus and Lifecycle Integration

Files:

- Modify: `macaca/crates/macaca-runtime-host/src/service_runtime.rs`
- Modify: `macaca/crates/macaca-runtime-host/src/lib.rs`
- Modify: `macaca/crates/macaca-runtime-host/Cargo.toml`

Behavior:

1. `register_provider(factory)` creates service instance and descriptor.
2. Runtime records Registered state and registers local handler in `LocalServiceTransport`.
3. `start(service_id, trace)` transitions Starting -> Running and calls `SystemService::start`.
4. `call(service_id, source, command)` builds `ServiceEnvelope`, evaluates decorators, dispatches through `ServiceBus`, and records Calling -> Running.
5. `stop(service_id, trace)` transitions Stopping -> Stopped and calls `SystemService::stop`.
6. `cleanup(service_id, trace)` transitions CleaningUp -> CleanedUp and calls `SystemService::cleanup`.
7. `snapshot()` returns deterministic lifecycle/health state.

Rules:

- Runtime must use `ServiceBus` rather than exposing direct provider calls to external callers.
- Runtime must attach bus trace sink and runtime event sink.
- Runtime must preserve existing service bus trace-required enforcement.
- Runtime must fail with structured errors for duplicate registration, unknown service, missing trace, policy denial, lifecycle invalidity, bus dispatch failure, and provider failure.

### Slice S1.6: Tests

Files:

- New: `macaca/crates/macaca-runtime-host/tests/service_runtime.rs`

Test cases:

1. Mock service can register, start, call, stop, cleanup.
2. Runtime snapshot reports deterministic lifecycle and health.
3. Missing trace is rejected before service dispatch.
4. Deny policy rejects before service dispatch.
5. Service bus trace events and runtime events are emitted at key nodes.
6. Duplicate service registration fails.
7. Unknown service call fails with structured error.
8. Provider failure transitions service into Failed and emits event/log.

Constraints:

- Tests must not require network, frontend, browser, real LLM provider, Web3 node, EVM node, MCP server, or external services.
- Tests should use `MockSystemService` or a runtime-host-local test service.

### Slice S1.7: Documentation

Files:

- Update: `macaca/docs/route-c-architecture-governance.md`
- Optional update: `macaca/docs/agent-os-microkernel-boundaries.md` only if wording needs to clarify runtime-host ownership
- Do not update allowlist unless S1 introduces a new forbidden edge

Documentation must state:

- `ServiceRuntime` is host-owned orchestration, not kernel provider ownership.
- Runtime calls must go through trace and policy decorators.
- New system services should register through provider factories and descriptors.
- Provider migrations happen in later S phases.

## Dependency Boundary Expectations

Potential new direct dependencies:

- `macaca-runtime-host -> macaca-ipc`
- `macaca-runtime-host -> macaca-kernel` already exists in current workspace metadata

Expected S0 gate outcome:

- `macaca-runtime-host -> macaca-ipc` should be allowed because runtime-host may bridge to IPC/service bus.
- No new `kernel -> provider` edge.
- No new `presentation -> provider` edge.
- No new provider -> presentation edge.
- If the gate fails, stop and update OpenSpec/allowlist only if the dependency is architecturally justified.

## Verification

Run after implementation:

```bash
openspec validate add-service-runtime-v1 --strict
cargo fmt --check
cargo test -p macaca-runtime-host service_runtime
cargo test -p macaca-ipc service_bus
cargo test -p macaca-kernel system_service
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
cargo check --workspace
npx gitnexus detect-changes --repo agent
```

Expected result:

- All commands pass.
- Existing warnings may remain, but S1 must not introduce new failures.
- S0 dependency gate must remain green.

## Completion Criteria

- Superpowers brainstorm and plan exist.
- OpenSpec proposal/design/tasks/spec exists and validates before implementation.
- `ServiceRuntime` can register/start/call/stop/cleanup a mock service through the bus.
- Runtime rejects missing trace and denied policy before service dispatch.
- Runtime emits structured logs and trace/audit-friendly events at key nodes.
- Runtime snapshot is deterministic and includes lifecycle/health.
- No concrete provider migration is performed in S1.
- No Route C allowlist row is removed unless a later implementation genuinely eliminates that debt.
