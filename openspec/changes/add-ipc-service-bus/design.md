# Design: IPC service bus

## Context

Route C Phase 01 established microkernel primitives such as `KernelServiceId`, `TraceContext`, policy requests, resource scopes, and a kernel facade. Phase 02 added provider-neutral `SystemService`, `ServiceCommand`, `ServiceCallResult`, lifecycle, health, trace-required middleware, and adapter skeletons.

Phase 03 adds the service call plane between those contracts and future transports. The bus must be local-first because current Macaca execution is in-process and performance-sensitive. It must also be transport-neutral because later Route C phases will introduce plugin processes, MCP runtimes, HTTP services, signed remote A2A, Store-mediated entitlements, and optional Web3/EVM modules.

## Goals

- Define `ServiceEnvelope`, `ServiceReply`, `ServiceTransport`, `ServiceBus`, and transport routing contracts.
- Keep local service calls typed-first and avoid mandatory serialization overhead.
- Require `TraceContext` before dispatch; no trace means no call.
- Provide trace/audit/logging boundaries independent of Web SSE or frontend state.
- Support policy and permission insertion points without a production policy backend in this phase.
- Bridge to Phase 02 `SystemService` through an additive local adapter.
- Preserve all existing YAML application, `/api/chat/v2`, task, trace, resume, driver, skill/MCP, and no-network baseline behavior.

## Non-Goals

- Do not implement a distributed runtime.
- Do not replace current production call paths wholesale.
- Do not make `macaca-ipc` own service lifecycle, provider execution, planner behavior, or application workflow.
- Do not make NATS the service bus semantics layer; existing NATS message transport remains a lower-level IPC option.
- Do not leak Web UI, CLI, gateway, or application-specific semantics into the bus contract.

## Superpowers Brainstorm Summary

### Current Problem

Macaca has service descriptors and service call execution, but service invocation is not yet mediated by a transport-neutral bus. This leaves future child-process services, MCP-backed services, remote A2A services, and plugin-provided services without one consistent trace/policy/audit path.

### Options Considered

1. **Local typed bus first, remote extension points only.**
   - Pros: preserves current performance, lowest regression risk, makes trace/policy invariants explicit now.
   - Cons: remote transports still need later implementation.
   - Verdict: recommended for Phase 03.

2. **Serialize every bus call into JSON immediately.**
   - Pros: simpler cross-process story.
   - Cons: adds unnecessary overhead to local hot paths, weakens typed contracts, risks regressions.
   - Verdict: reject for Phase 03.

3. **Build real child-process/MCP/HTTP/A2A transports now.**
   - Pros: demonstrates future architecture quickly.
   - Cons: combines too many phases, raises security/resource/entitlement complexity, likely breaks existing runtime.
   - Verdict: reject as over-scoped.

### Recommended Plan

Implement an additive local typed bus with explicit bridge, command, decorator, strategy, observer, and specification boundaries. Later phases can add real remote transports behind the same `ServiceTransport` contract without changing caller semantics.

## Design Patterns

- **Bridge**: `ServiceTransport` separates service invocation semantics from local, process, MCP, HTTP, or remote transport implementation.
- **Command**: `ServiceEnvelope` carries one service command, its metadata, trace context, permission scope, deadline, and idempotency key.
- **Facade**: `ServiceBus` becomes the stable caller-facing entry point for route, dispatch, trace, and structured reply handling.
- **Proxy**: future remote services can present the same local service interface through proxy transports.
- **Decorator**: trace, audit, policy, timeout, logging, and metering wrappers can decorate a transport without modifying the transport implementation.
- **Chain of Responsibility**: middleware validates trace, policy, deadline, idempotency, and logging in deterministic order before dispatch.
- **Strategy**: route and transport selection remain replaceable; local transport is only the first strategy.
- **Observer**: bus-level trace/audit events are emitted through presentation-neutral observers, not directly through Web SSE.
- **Specification**: permission scope, service capability, deadline, optional module availability, and transport support are declarative facts evaluated by policy middleware.

## Contract Shape

### Protocol Contracts

Phase 03 may add service bus protocol data either under `macaca-proto/src/service.rs` or a focused `macaca-proto/src/service_bus.rs` module, depending on implementation size. The contracts should include:

- `ServiceEnvelopeId`: value object for call correlation.
- `ServiceEnvelope`: source identity, target `KernelServiceId`, Phase 02 `ServiceCommand`, optional session id, optional task id, permission scope, deadline, idempotency key, trace context, and metadata.
- `ServiceReply`: structured success, structured failure, trace context, transport kind, latency, and metadata.
- `TransportKind`: extensible local, child process, MCP, HTTP, signed remote A2A, and custom identifiers.
- `ServiceBusError`: structured missing trace, no route, unsupported transport, deadline exceeded, timeout, policy denied, transport unavailable, dispatch failed, and reply decode errors.

`ServiceEnvelope` carries trace at envelope level and command level for compatibility with Phase 02. The bus must reject the envelope if neither level has trace. When only envelope trace is present, the local adapter can populate the command trace before invoking `SystemService`.

### `macaca-ipc`

The crate should add focused modules:

- `envelope.rs`: envelope/reply helpers and validation.
- `service_bus.rs`: `ServiceBus` facade, router, middleware chain, and public builder.
- `transport.rs`: extend or add service-call transport traits without breaking the existing message sender/receiver bridge.
- `local.rs`: add local typed service transport or a sibling module if keeping existing pub/sub code smaller is cleaner.

The existing pub/sub `MessageSender` and `MessageReceiver` APIs remain compatible. The new service bus is additive and must not remove or redefine existing IPC message semantics.

### Kernel Bridge

`macaca-kernel/src/service_call.rs` should gain an additive bridge adapter that lets the bus invoke Phase 02 `SystemService` through `ServiceCallExecutor`. This keeps service call middleware, trace emission, and structured errors consistent.

The bridge must not make `macaca-kernel` depend on concrete service provider crates. If dependency direction would become wrong, keep the adapter in `macaca-ipc` or a small integration module that depends on both abstractions without creating a cycle.

## Trace, Audit, And Logging

Every service bus call must emit structured logs at key boundaries:

- envelope accepted;
- trace validation rejected;
- route selected;
- transport selected;
- call dispatched;
- call completed;
- call failed;
- deadline exceeded;
- transport unavailable.

The logs must include envelope id, source identity, target service id, command name, transport kind, status, duration, and error code when present. Logs must not include raw credentials, secrets, or unredacted payloads.

Trace/audit events must be presentation-neutral and include enough correlation data for Web UI, EventLog, CLI inspectors, and future audit services to reconstruct the service call lifecycle.

## Policy And Permission

Phase 03 does not implement a production policy backend. It must still carry permission scope and expose a policy middleware insertion point so future policy, budget, region, entitlement, optional module availability, and payment checks cannot be bypassed without changing the bus contract.

The default compatibility policy may allow calls, but tests must include a denial middleware path that proves policy denial returns structured data before dispatch.

## Deadline And Idempotency

The envelope includes deadline and idempotency data because bus calls will later cross process and network boundaries. Phase 03 local transport must enforce deadline expiration with a structured timeout/deadline error. It does not need to implement durable idempotency storage, but it must preserve the idempotency key in replies and trace metadata for later phases.

## Compatibility Plan

1. Add protocol-level envelope/reply/error data and serde tests.
2. Add `ServiceTransport` bridge and `ServiceBus` facade in `macaca-ipc`.
3. Add in-process typed local service transport with mock `SystemService` dispatch.
4. Add trace-required decorator/middleware and structured logging.
5. Add extension-point-only transport kinds for child process, MCP, HTTP, and signed remote A2A.
6. Add a kernel service call bridge without migrating production consumers.
7. Run targeted tests and Route C regression checks.

## Risks / Trade-offs

- **Risk: Service bus becomes a second service lifecycle manager.** Mitigation: lifecycle remains in Phase 02 `SystemService`; bus only routes calls.
- **Risk: Mandatory serialization slows local runtime.** Mitigation: local transport remains typed-first and only serializes at true transport boundaries later.
- **Risk: Trace events duplicate existing service executor events.** Mitigation: bus emits bus-level events with envelope ids; service executor emits service-level events with command ids. Tests should verify no duplicate same-source trace entries for one middleware layer.
- **Risk: Dependency cycle between `macaca-ipc` and `macaca-kernel`.** Mitigation: keep shared data in `macaca-proto`; place concrete bridge in whichever crate can depend one-way without cycles.
- **Risk: Proposal is mistaken for full migration.** Mitigation: tasks explicitly stop at one mock/local bridge and leave upper consumer migration to later OpenSpec.

## Open Questions

- None for Phase 03. Real child process, MCP, HTTP, signed remote A2A, entitlement, and remote identity verification belong to later phases.
