# S1 ServiceRuntime v1 Brainstorm

## Context

S1 follows `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`. S0 already added an executable dependency boundary gate and `macaca/docs/route-c-serviceization-allowlist.md` records current migration debt.

The current codebase already contains important Route C building blocks:

- `macaca-proto/src/service.rs` defines provider-neutral `ServiceDescriptor`, `ServiceCommand`, `ServiceCallResult`, `ServiceLifecycleState`, `ServiceHealth`, and `ServiceError`.
- `macaca-ipc/src/service_bus.rs` defines `ServiceBus`, middleware, trace sink, and local/remote transport bridge points.
- `macaca-kernel/src/system_service.rs` defines the `SystemService` trait and `MockSystemService`.
- `macaca-kernel/src/service_call.rs` defines `ServiceCallExecutor` with trace-required middleware and trace event emission.
- `macaca-kernel/src/service_bus_bridge.rs` adapts `ServiceEnvelope` to `SystemService`.
- `macaca-kernel/src/service_registry.rs` records provider-neutral service id/scope, but does not own provider instances.
- `macaca-runtime-host` currently owns plugin/MCP/package/entitlement host glue, making it the right crate for ServiceRuntime orchestration.

S1 should connect these pieces into a real runtime without migrating Task, LLM, Memory, Driver, Skill, MCP, Gateway, Payment, Web3, or EVM providers yet.

## Constraints

- Must strictly follow `macaca/docs/agent-os-microkernel-boundaries.md`.
- Must preserve `macaca/docs/route-c-serviceization-allowlist.md`; S1 must not pretend existing direct provider deps are fixed.
- Must follow `macaca/docs/route-c-architecture-governance.md`: no trace, no call; no permission/policy, no call; Web/CLI remain shells; new provider dependencies in kernel/presentation are forbidden.
- Must be additive-first and preserve YAML application loading, `/api/chat/v2`, trace, task board, resume, driver, skill/MCP, Web UI, and CLI behavior.
- Must not hardcode app names, workflow names, provider names, model names, driver names, gateway names, chain names, or business logic.
- All new code must have detailed English comments and structured logs at lifecycle/call/rejection nodes.
- Rust files must stay below 500 lines.

## Design Pattern Candidates

### Option A: Runtime Facade + Decorator Chain + Provider Factory

Build `ServiceRuntime` in `macaca-runtime-host` as a facade over provider registration, lifecycle, bus registration, traced calls, stop, cleanup, and health snapshots. Use decorators for trace, policy, resource, entitlement, and metering checks. Use an abstract factory trait for built-in and plugin-backed providers.

Patterns:

- Facade: `ServiceRuntime` hides lifecycle, registry, local transport, and decorator sequencing.
- Decorator / Chain of Responsibility: trace, policy, resource, entitlement, and metering checks compose around calls.
- Bridge: runtime dispatches through `macaca-ipc` local service transport first, leaving remote/plugin transport open.
- State: service lifecycle transitions are explicit and auditable.
- Abstract Factory: provider factories return descriptors and service instances without hardcoding provider categories.
- Observer: runtime emits structured lifecycle and call events.
- Memento: runtime snapshot records service health/lifecycle for diagnostics.

Pros:

- Aligns directly with S1 goals.
- Reuses existing protocol, bus, and kernel service contracts.
- Keeps provider migration for S2-S12.
- Gives future providers one pluggable registration path.

Cons:

- Requires careful dependency choices because `macaca-runtime-host` currently does not depend on `macaca-kernel` or `macaca-ipc`.
- If decorators are too broad in S1, it can become over-engineered before real provider migration.

Risk:

- Introducing `macaca-runtime-host -> macaca-kernel` is already allowed by current workspace dependencies, but `macaca-runtime-host -> macaca-ipc` would be a new direct dependency. This should be acceptable if classified as runtime-host to ipc-service-bus, but S0 gate must verify it.

### Option B: Kernel-Owned ServiceRuntime

Put `ServiceRuntime` in `macaca-kernel` next to `SystemService`, `ServiceCallExecutor`, and `ServiceRegistry`.

Pros:

- Fewer crate dependency changes.
- Direct access to kernel service primitives.

Cons:

- Violates the microkernel boundary direction: kernel would start owning provider runtime orchestration.
- Makes future provider migration harder because provider factories would trend toward kernel.
- Conflicts with `agent-os-microkernel-boundaries.md`, where kernel owns registry/facades but services own replaceable capabilities.

Risk:

- High architecture risk. This option should be rejected.

### Option C: IPC-Only Runtime

Treat `macaca-ipc::ServiceBus` as the runtime and only add helper registration functions around local transport.

Pros:

- Minimal code.
- Very low immediate integration cost.

Cons:

- Does not model lifecycle, provider factory, health snapshots, entitlement/resource/metering hooks, or runtime ownership.
- Pushes lifecycle and policy decisions into callers, recreating macro-kernel coordination elsewhere.
- Fails S1 milestone: mock service complete start/call/stop with snapshot health.

Risk:

- Medium to high long-term risk because it leaves system semantics scattered.

### Option D: Generic Runtime Kernel With Trait-Only Dependencies

Define a very small trait-only runtime in `macaca-runtime-host`, avoiding direct `macaca-kernel` dependency by duplicating minimal service traits or wrapping only `macaca-proto`.

Pros:

- Keeps runtime-host independent from kernel.
- Could reduce dependency cycle risk.

Cons:

- Duplicates `SystemService`, `ServiceCallExecutor`, and bridge semantics already present.
- Creates parallel abstractions and likely inconsistent trace/policy behavior.
- Harder to keep governance docs and tests aligned.

Risk:

- Medium design drift risk. This is only useful if a real dependency cycle blocks Option A.

## Recommended Approach

Choose Option A with strict S1 limits:

- Add `ServiceRuntime` to `macaca-runtime-host`.
- Reuse `macaca-proto` service contracts, `macaca-ipc` service bus/local transport, and `macaca-kernel` `SystemServiceBusHandler`.
- Add only minimal runtime-host abstractions needed for orchestration:
  - `ServiceProviderFactory`
  - `ServiceRuntime`
  - `ServiceRuntimeDecorator`
  - `ServiceRuntimeSnapshot`
  - `ServiceRuntimeEventSink`
- Implement trace-required and policy-required decorators as runtime-level guards. S1 can ship permissive policy strategy for tests, but calls must still pass through policy data and decorator nodes.
- Keep resource, entitlement, and metering decorators as optional extension points with no provider-specific behavior in S1.
- Do not migrate existing providers or remove allowlist rows.

## Key Risks and Mitigations

- Risk: `ServiceRuntime` becomes a provider construction hub.
  - Mitigation: factories receive provider-neutral descriptors and return `Arc<dyn SystemService>`; no LLM/driver/skill/gateway-specific branches.

- Risk: duplicate trace/policy checks across bus, kernel executor, and runtime.
  - Mitigation: keep runtime decorators as outer admission control, then rely on `ServiceBus` and `ServiceCallExecutor` for lower-level trace enforcement. Tests should prove missing trace/policy is rejected before service dispatch.

- Risk: lifecycle state is split between descriptor and runtime state.
  - Mitigation: S1 runtime owns runtime lifecycle snapshot; descriptors remain advertised contract data. Later phases can reconcile provider-reported descriptor state.

- Risk: adding direct dependencies triggers S0 gate.
  - Mitigation: run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`. If a new edge violates the gate, stop and update OpenSpec/allowlist only if architecturally justified.

- Risk: over-design before provider migration.
  - Mitigation: S1 exposes only mock-service tested lifecycle and call paths; concrete Task/LLM/Memory/Driver/Skill/MCP migration remains S4-S8.

- Risk: policy-required rule is implemented as a no-op.
  - Mitigation: model policy as a Strategy with an explicit `PolicyDecision` and test both allow and deny strategies. The default test strategy can allow, but the runtime must call it and log the decision.

## Decision

Proceed with Option A. It is the most aligned with Route C boundaries because it gives replaceable capabilities a real host-owned runtime while keeping kernel focused on primitives, registry, and service call facades.
