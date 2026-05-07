# Change: Add IPC service bus

## Why

Route C Phase 03 must turn service invocation into a transport-neutral call plane. Phase 01 introduced microkernel primitives and Phase 02 introduced `SystemService` contracts, but service calls still need an IPC/service bus boundary that can start local-first while remaining extensible to child process, MCP, HTTP, and signed remote A2A transports.

Without this boundary, upper layers will keep coupling service calls to concrete in-process code paths, making trace enforcement, policy insertion, transport selection, auditability, and future remote service integration harder to guarantee.

## What Changes

- Add a local-first, typed-first `ServiceEnvelope` contract that wraps a Phase 02 `ServiceCommand` with source identity, target service id, optional session/task context, permission scope, deadline, idempotency key, and trace context.
- Add a transport-neutral `ServiceTransport` bridge and `ServiceBus` facade in `macaca-ipc` so service calls can be routed without binding callers to local, NATS, child process, MCP, HTTP, or remote A2A details.
- Add an in-process local service transport that can dispatch to mock `SystemService` implementations without forcing JSON serialization on hot local paths.
- Add trace and audit middleware/decorator boundaries that reject untraceable calls before dispatch and record accepted, routed, completed, failed, rejected, and timed-out calls.
- Add future transport extension points for child process, MCP, HTTP, and signed remote A2A without implementing those transports in Phase 03.
- Bridge `macaca-kernel` service call execution to the service bus through an additive adapter while preserving existing direct calls until consumers are migrated in later phases.
- Require detailed English comments and structured logs around key execution nodes.

## Impact

- Affected specs: `ipc-service-bus`
- Affected crates: `macaca-ipc`, `macaca-proto`, `macaca-kernel`, `macaca-runtime-host`, future adapters in `macaca-framework`
- Affected tests: `macaca-ipc/tests/service_bus.rs`, `macaca-kernel` service call bridge tests, protocol serde tests, Route C baseline checks
- Regression matrix references: `RC-GOAL-001`, `RC-TRACE-001`, `RC-PIPE-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: IPC / Service Call Facade belongs to the microkernel boundary as a provider-neutral bridge, while concrete replaceable capabilities remain system services.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 03 must preserve goal execution, live trace push, and no-network pipeline behavior.
- Follows `macaca/docs/route-c-phase-template.md`: OpenSpec first, additive-first implementation, GitNexus impact before symbol edits, targeted tests, integration smoke, detect_changes before commit.
- Follows `macaca/docs/route-c-architecture-governance.md`: every service call must be traceable, policy-ready, auditable, transport-neutral, and free of application/provider/driver/gateway hardcoding.

## Non-Goals

- Do not implement production child process, MCP, HTTP, NATS service-call, or remote A2A transports in Phase 03.
- Do not force all local calls through JSON serialization; local hot paths must remain typed-first.
- Do not migrate all existing LLM, task, driver, skill, gateway, memory, framework, web, or CLI consumers to the service bus in this phase.
- Do not move concrete service behavior into `macaca-kernel`.
- Do not implement Store, Payment, Web3, EVM, GenUI, package installation, entitlement, or plugin marketplace behavior.
- Do not hardcode application names, workflow names, provider names, model names, driver names, gateway names, chain names, or business-specific routing in the bus.
