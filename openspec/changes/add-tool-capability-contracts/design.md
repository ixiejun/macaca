## Context

`docs/macaca-industrial-tools-system-design.md` defines a complete service-owned Tool Capability Plane. This first change creates the stable provider-neutral contracts only. It deliberately does not migrate runtime invocation or add new rich providers.

Existing Macaca primitives already point in the right direction: `CapabilityToolDescriptor`, `CapabilityToolInvocation`, `CapabilityToolInvocationResult`, service-backed Driver/Skill/MCP catalog paths, `macaca-tools` command/schema primitives, and context capability catalogs. The missing piece is a comprehensive industrial descriptor and command surface that all later slices can share.

## Goals

- Define stable DTOs for industrial tool descriptors, plans, visible entries, hidden diagnostics, conflicts, availability expressions, policy refs, result classes, artifact refs, provider status, and audit refs.
- Define `service.tool` command names and command/result DTOs.
- Add SDK `SystemToolClient` as the focused Facade over `service.tool`.
- Add unavailable client behavior that returns explicit unavailable results instead of fake success.
- Keep ownership data explicit so later routers can dispatch to the correct owning service without parsing visible tool names.

## Non-Goals

- Do not implement production invocation routing in this change.
- Do not implement the planning service in this change.
- Do not add runtime environment providers or managed gateways in this change.
- Do not move Driver, Skill, MCP, Memory, Task, Scheduler, Gateway, Store, or any provider lifecycle into `service.tool`.
- Do not introduce application-specific tools or business logic.

## Decisions

### Command

Every `service.tool` operation is a typed command/result DTO. This keeps service calls auditable, versionable, and portable across SDK, Web, CLI, WASM, and future remote service transports.

### Facade

`SystemToolClient` is the SDK-facing Facade. Shells and application adapters call this client; they do not construct runtime-host providers or evaluate tool policy themselves.

### Memento

Tool plans, provider snapshots, result refs, and audit refs are mementos. They are replayable evidence, not live provider handles. They must be bounded and sanitized.

### Specification

Availability and policy inputs use declarative expression DTOs. Later proposals will implement evaluators, but the contract must represent config, secret, auth, environment, binary, service health, platform, resource, entitlement, plugin, manifest, agent policy, and session context signals.

### Null Object

The unavailable `SystemToolClient` returns structured unavailable results for every command. Absence is a valid state; crash, hang, silent fallback, and fake success are not valid states.

## Ownership

- Kernel owns only service identity, typed service-call invariants, trace/audit primitives, and policy facade hooks.
- `macaca-proto` owns provider-neutral DTOs and command names.
- `macaca-sdk` owns focused client traits and unavailable behavior.
- `macaca-runtime-host` will own provider implementations in later proposals.
- Provider services retain lifecycle and concrete invocation ownership.
- Shells only render command results and diagnostics.

## Trace, Audit, And Logging Requirements

Implementation must include English comments explaining security and lifecycle semantics for non-obvious DTOs. Later runtime code must log key command handling with trace ids, service ids, counts, hashes, refs, and reason codes rather than raw payloads.
