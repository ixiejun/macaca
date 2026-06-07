## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-03-ipc-service-bus.md`.
- [x] 1.2 Review Phase 01 and Phase 02 contracts in `macaca-proto`, `macaca-kernel`, and `macaca-ipc`.
- [x] 1.3 Run GitNexus impact before modifying each symbol selected during implementation; warn before editing any HIGH or CRITICAL impact symbol.

## 2. Protocol Contracts

- [x] 2.1 Add additive service bus protocol contracts for envelope id, service envelope, service reply, transport kind, deadline, idempotency key, and structured service bus errors.
- [x] 2.2 Add serde and validation tests proving envelope/reply round trips preserve trace, source identity, target service id, session/task context, deadline, idempotency key, transport kind, and metadata.
- [x] 2.3 Add tests proving missing trace and expired deadline are representable as structured errors without panic or string parsing.

## 3. `macaca-ipc` Service Bus

- [x] 3.1 Add focused `macaca-ipc` modules for envelope helpers, service bus facade, service transport bridge, local typed transport, and middleware/decorator boundaries.
- [x] 3.2 Implement a local typed transport that dispatches a `ServiceEnvelope` to a mock Phase 02 `SystemService` without forcing JSON serialization for local dispatch.
- [x] 3.3 Implement route selection by `KernelServiceId`, not by application, provider, driver, gateway, workflow, or model name.
- [x] 3.4 Add structured logs for accepted, rejected, routed, dispatched, completed, failed, timed-out, and unavailable service bus calls.

## 4. Trace, Audit, Policy, And Deadline Middleware

- [x] 4.1 Add trace-required middleware/decorator that rejects envelopes lacking trace context before dispatch.
- [x] 4.2 Add audit/trace observer hooks that emit presentation-neutral bus-level events for accepted, completed, failed, rejected, and timed-out calls.
- [x] 4.3 Add a policy middleware insertion point and a denial test proving denied calls do not dispatch to the target service.
- [x] 4.4 Add deadline enforcement for local transport and tests proving expired calls return structured timeout/deadline errors.

## 5. Kernel Bridge

- [x] 5.1 Add an additive bridge between the service bus and Phase 02 `ServiceCallExecutor` / `SystemService` contracts without changing existing production call paths.
- [x] 5.2 Verify the bridge preserves trace context by returning replies with the same trace id and by emitting bus-level and service-level trace records without duplicate same-source entries.
- [x] 5.3 Keep dependency direction acyclic; if a direct kernel dependency would cycle, place the bridge in `macaca-ipc` or a dedicated integration module.

## 6. Future Transport Extension Points

- [x] 6.1 Define extension-point-only transport kinds for local, child process, MCP, HTTP, signed remote A2A, and custom transports.
- [x] 6.2 Add a mock remote transport test proving a non-local transport can satisfy the `ServiceTransport` trait without implementing production remote execution.
- [x] 6.3 Document that production child process, MCP, HTTP, and signed remote A2A transports are non-goals for Phase 03.

## 7. Regression And Verification

- [x] 7.1 Run `openspec validate add-ipc-service-bus --strict`.
- [x] 7.2 Run `cargo test -p macaca-proto service` and the new service bus serde tests.
- [x] 7.3 Run `cargo test -p macaca-ipc`.
- [x] 7.4 Run `cargo test -p macaca-kernel service_call` and any new bridge tests.
- [x] 7.5 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 7.6 Run `cargo check --workspace`.
- [x] 7.7 Run a hardcode scan over new bus files for application names, workflow names, provider names, driver names, gateway names, model names, and chain names.
- [x] 7.8 Run `gitnexus_detect_changes(scope: "all")` before committing and verify affected flows match the expected Phase 03 scope.
