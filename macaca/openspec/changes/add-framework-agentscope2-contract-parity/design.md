## Context

AgentScope Java 2.0 is an agent framework with event-stream execution, middleware, tool execution state, model formatting, protocol adapters, and a Harness layer for workspace-like capabilities. Macaca OS is not a single framework clone; it is a microkernel Agent OS where replaceable capabilities are serviceized and the framework layer exposes generic contracts consumed by applications, SDKs, shells, services, and runtime hosts.

This change therefore closes AgentScope 2.0 parity by adding framework-owned contracts and verification gates, not by moving concrete provider logic into `macaca-framework`.

The implementation baseline is latest `origin/main`, where `refactor-unified-call-path-microkernel` has deeply reworked execution ownership toward the 2026-06-07 protocol microkernel target. That target design takes precedence over the previous AgentScope2 WIP branch. During this re-port, any stale WIP code that reintroduces direct web execution, kernel provider construction, compatibility fallbacks, or multi-path coordination is discarded rather than merged.

## Goals

- Make all 25 audit gaps explicit, numbered, and verifiable.
- Preserve framework replaceability through provider-neutral command/result/event contracts.
- Preserve Macaca OS boundaries from:
  - `macaca-os-architecture-governance.md`
  - `macaca-os-microkernel-boundaries.md`
  - `macaca-os-serviceization-allowlist.md`
- Ensure future AgentScope or non-AgentScope framework upgrades can replace internals without changing consumer-facing Macaca contracts.
- Ensure every delegated capability carries trace, policy, structured errors, health/snapshot evidence, and sanitized audit logs.
- Preserve the single `service.call` protocol path: framework contracts may describe a capability, but side effects must execute through SystemFacade/focused service clients, ServiceRuntime, ServiceBus, and runtime-host providers.

## Non-Goals

- Implement concrete LLM providers, model HTTP/WebSocket clients, memory/vector stores, filesystem backends, sandbox backends, skill package loaders, MCP runtime transports, task planners, gateway adapters, payment, Web3, EVM, or application business workflows inside `macaca-framework`.
- Preserve AgentScope 1.0 implementation code through `legacy`, `compat`, `deprecated`, or version-suffixed canonical modules.
- Restore any execution path removed by `refactor-unified-call-path-microkernel`, including web-owned agent execution, kernel-owned providers, direct runtime/toolkit access, or multi-path lifecycle coordination.
- Add application-specific routing, provider-name branching, model-name branching, workflow hardcoding, or business-domain behavior.

## Design Patterns

- **Facade**: `macaca-framework` exposes stable framework execution, capability matrix, health, and snapshot facades for consumers.
- **Command**: cross-boundary model/tool/harness/plan/skill/MCP/filesystem/sandbox operations use typed command/result DTOs.
- **Adapter / Bridge**: service-backed ports adapt LLM, memory, context, skills, MCP, filesystem, sandbox, Agent Protocol, and runtime-host providers without owning concrete implementations.
- **Strategy**: formatter, transport, tool execution, context assembly, tracing export, and structured output behavior are selected by provider-neutral strategy contracts.
- **Decorator**: trace, policy, resource, entitlement, metering, and audit checks wrap every side-effect port.
- **Observer**: `AgentEvent` is the canonical execution stream; protocol adapters and logs observe the same stream.
- **Memento**: agent state, session tree state, suspended tool state, and provider snapshots are replayable checkpoint records.
- **State**: HITL input, tool suspend/resume, plan mode, sandbox lifecycle, and external execution are explicit state machines.
- **Specification**: capability matrix entries, provider admission, workspace filesystem specs, sandbox specs, and evidence refs are executable checks.
- **Abstract Factory / Builder**: framework builders create neutral config and contract objects; concrete provider factories remain in runtime-host composition roots.

## Architecture Decisions

### Decision 1: Framework owns contracts, services own side effects

`macaca-framework` SHALL define the canonical contracts needed for AgentScope 2.0 parity: events, middleware, tool specs, model formatter specs, transport DTOs, protocol projections, harness specs, mementos, capability evidence, and unavailable/null-object behavior.

Concrete service calls SHALL be delegated through service-backed ports with trace context, policy checks, structured errors, health, and snapshots.

On the unified-call-path baseline, service-backed ports SHALL be implemented only at approved runtime-host composition roots or through focused SDK clients. `macaca-web` may adapt HTTP/SSE DTOs and render diagnostics, but it SHALL NOT own the semantic execution loop or construct framework providers.

### Decision 2: Event stream is the canonical runtime surface

`stream_events`-style execution SHALL be the primary framework surface. `reply` or final-message helpers MAY exist only as projections over the event stream and must not own a separate execution path.

### Decision 3: Capability matrix must be evidence-based

Framework availability SHALL not use broad `Available` claims. Every AgentScope Java 2.0 capability SHALL carry status, evidence refs, delegation target, test coverage refs, and known limitations.

Allowed statuses are:

- `equivalent`
- `contract-only`
- `delegated-verified`
- `delegated-unverified`
- `missing`
- `unsupported-by-policy`

### Decision 4: No AgentScope 1.0 leftovers

Canonical framework code SHALL remove AgentScope 1.0 implementation paths outright. There SHALL be no `ReActAgent2`, `AgentScope2RuntimeProvider`, `legacy`, `compat`, or `deprecated` module used to make AgentScope 2.0 give way to AgentScope 1.0 internals. Consumer-facing APIs that must change SHALL be annotated with clear migration notes.

### Decision 5: Observability is part of the contract

Every framework-owned state transition and delegated side-effect request SHALL emit bounded structured logs and replayable trace/audit events. Logs and snapshots SHALL not include raw secrets, raw prompts, manifests, package bytes, WASM bytes, private keys, credentials, raw signatures, raw provider payloads, or unbounded output.

## Risks And Mitigations

- **Risk**: Framework contracts drift from service implementation behavior.
  - **Mitigation**: add contract tests with mock/unavailable/service-backed providers and evidence-backed capability snapshots.
- **Risk**: The framework grows into a concrete service owner.
  - **Mitigation**: boundary tests reject concrete provider imports and require side effects through ports.
- **Risk**: Consumers depend on AgentScope-specific names.
  - **Mitigation**: maintain provider-neutral public types and add naming gates against version-suffixed canonical names.
- **Risk**: Optional provider absence becomes silent fallback.
  - **Mitigation**: every optional port must return structured unavailable/unsupported/denied results with trace evidence.
- **Risk**: Detailed trace leaks sensitive data.
  - **Mitigation**: sanitize payloads, bound output, and test redaction for logs, snapshots, and audit events.

## Migration Plan

1. Add or update framework contract modules in small groups matching the task sections.
2. Wire service-backed adapters only through approved runtime-host composition roots.
3. Replace AgentScope 1.0-era paths instead of preserving internal compatibility modules.
4. Add tests and executable gates before claiming parity for each capability group.
5. Update capability matrix evidence after each group passes contract and boundary tests.

## Open Questions

- Which shared DTO crate should own cross-service model/tool/harness contract types if `macaca-framework` needs them beyond runtime-only usage?
- Which existing trace/audit event schema should be extended for AgentScope-style stream projection evidence?
- Which service-backed capabilities already have stable clients that can be reused directly during implementation?
