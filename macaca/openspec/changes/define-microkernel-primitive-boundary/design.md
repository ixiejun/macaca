# Design: Microkernel primitive boundary

## Context

Route C positions Macaca as a microkernel Agent OS. Phase 0 defined the governance baseline; Phase 01 converts that baseline into concrete additive contracts. The kernel must become the place where system invariants are named and mediated, not a container for provider implementations or application workflows.

The Phase 01 implementation must keep current YAML applications, `/api/chat/v2`, task execution, trace, and no-network pipeline behavior unchanged. It only introduces primitive boundaries that later phases can migrate toward incrementally.

## Goals

- Define stable primitive value objects in `macaca-proto` without depending on upper-layer crates.
- Expose a focused `KernelFacade` in `macaca-kernel` for capability discovery, service discovery, policy decisions, resource scopes, and trace emission.
- Keep policy, scheduling, resource allocation, and trace delivery replaceable through traits.
- Provide an SDK-level access path so upper crates can depend on the facade instead of direct kernel internals.
- Make all new code self-documenting through detailed English comments that explain each primitive's role and operating model.

## Non-Goals

- Do not make the kernel execute provider-specific service calls.
- Do not move `TodoBoard`, `Planner`, `Review`, LLM routing, driver execution, skill runtime, MCP runtime, gateway adapters, memory, or persistence implementations into kernel.
- Do not introduce a full service bus transport; that belongs to later phases.
- Do not change live Web UI behavior or application runtime behavior.

## Design Patterns

- **Facade**: `KernelFacade` is the stable entry point that hides internal registries, policy engines, resource management, and trace publication behind a minimal API.
- **Registry**: `CapabilityRegistry` and `SystemServiceRegistry` store descriptors and service identities without implementing those services.
- **Strategy**: `PolicyEngine` and future scheduling/resource policies can be swapped without changing callers.
- **Observer**: `TraceEventBus` is the trace/audit event outlet for primitive operations; later implementations can forward to EventLog, SSE, or service bus.
- **Specification**: `PolicyRequest` describes facts that a policy implementation evaluates, avoiding hardcoded if/else by provider or application name.
- **Value Object**: `KernelServiceId`, `CapabilityId`, `TraceContext`, and `ResourceScope` are explicit immutable-ish identifiers/data carriers with validation-oriented constructors.
- **Mediator**: `ResourceManager` coordinates resource scope ownership so callers do not coordinate driver/browser/workspace locks directly.

## Contract Shape

### `macaca-proto`

`macaca-proto/src/kernel.rs` will define shared data contracts:

- `KernelServiceId`
- `CapabilityId`
- `CapabilityDescriptor`
- `ServiceScope`
- `TraceContext`
- `PolicyRequest`
- `PolicyDecision`
- `ResourceScope`
- `KernelPrimitiveError`

These types must derive serde traits where appropriate and must not reference `macaca-web`, `macaca-app`, `macaca-framework`, or provider-specific crates. They are ABI-facing protocol/value types, not runtime service implementations.

### `macaca-kernel`

`macaca-kernel` will expose traits and additive skeletons:

- `CapabilityRegistry`
- `SystemServiceRegistry`
- `PolicyEngine`
- `TraceEventBus`
- `ResourceManager`
- `KernelFacade`

The default implementation will be intentionally small: in-memory descriptors, default allow policy for compatibility, explicit deny path for tests, duplicate resource detection, and structured errors. This is not a toy path because each skeleton must preserve invariants and error semantics used by future production implementations.

### `macaca-sdk`

`macaca-sdk` will re-export or wrap the facade entry point so applications and future tooling can discover capabilities and services through an OS-level API. This must remain additive and must not require a `macaca-web` dependency.

## Policy And Trace Rules

Every primitive API that models capability or service use must carry enough context for future policy and trace integration. Phase 01 does not enforce all production permissions, but the API shape must make bypassing policy/trace harder in later phases.

The default policy exists for compatibility only. It must be clearly documented as a temporary permissive strategy and must return structured `PolicyDecision::Allow`. Tests must also cover a deny strategy so callers learn to handle denial as data, not as panic/hang behavior.

## Error Model

All primitive failures must return `KernelPrimitiveError` or a crate-local wrapper that preserves structured causes. Duplicate resource registration, missing capability/service, invalid identifiers, and denied policy decisions must be distinguishable. Optional module absence must be represented as structured unavailable/disabled states in later phases; Phase 01 must not use panic or string-only errors for these primitive paths.

## Comment Standard

All new Rust code in this phase must include detailed English comments explaining:

- what the primitive represents;
- why the primitive belongs in kernel or proto;
- how callers are expected to use it;
- what invariant the implementation protects;
- why default permissive behavior is compatibility-only when applicable.

Comments must clarify operating principles without restating obvious assignments.

## Migration Plan

1. Add `macaca-proto` primitive types and serialization tests.
2. Add `macaca-kernel` facade and registry/policy/resource/trace skeletons.
3. Add `macaca-kernel` tests around registration, lookup, policy, duplicate resources, and structured errors.
4. Add `macaca-sdk` additive facade access.
5. Mark direct internals deprecated only where a tested facade alternative exists.
6. Run Route C baseline checks and targeted crate tests.

## Risks / Trade-offs

- **Risk: Kernel grows too broad.** Mitigation: this proposal limits kernel to invariant-bearing contracts and explicitly rejects provider/application logic.
- **Risk: Skeletons become fake implementations.** Mitigation: tests must prove real invariants: serde round trip, lookup, duplicate rejection, allow/deny policy, and structured errors.
- **Risk: Upper crates bypass facade.** Mitigation: SDK exposes the facade path now; later phases migrate consumers incrementally.
- **Risk: Compatibility break.** Mitigation: additive-only implementation and Route C regression checks keep current YAML app and no-network goal pipeline behavior intact.

## Open Questions

- None for Phase 01. Later phases will decide service bus transport, package runtime guard integration, and production policy backends.
