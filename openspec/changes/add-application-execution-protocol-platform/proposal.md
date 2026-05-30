# Change: Add Application Execution Protocol Platform

## Why

Macaca currently has strong serviceization, application ABI, WASM runtime, workbench service, session, EventLog, and thin-shell foundations, but application execution is still not governed by one durable protocol platform. The latest `CODEX-WASM-WORKBENCH` validation exposed the boundary issue clearly: an app-owned UI can start or stream work, but a real industrial application must not depend on the browser, iframe bridge, or frontend local state to keep long-running execution alive, persist trace/session evidence, or coordinate approval/cancel commands.

Macaca OS should behave as a protocol platform for application execution. Applications must be free to run through Macaca-hosted runtime components, application-owned backend services, or remote agent/runtime workers, but every provider shape must participate in the same session, event, replay, control, capability, policy, trace, and audit protocol. This gives upper-layer applications maximum flexibility while keeping Macaca OS provider-neutral, application-neutral, auditable, replayable, and shell-independent.

## What Changes

This change adds a generic **Application Execution Protocol Platform**. The platform defines typed protocol contracts, a service boundary, provider strategy adapters, gateway ingress rules, durable session event semantics, replay/current-state projection, and shell adapter restrictions. It is not a Codex-specific workbench feature.

The change will:

- Add provider-neutral protocol DTOs for application execution sessions, execution start commands, execution results, session event envelopes, control commands, checkpoints, snapshots, provider descriptors, provider leases, gateway ingress callbacks, structured errors, and sanitized current-state projections.
- Add a new system service boundary, expected service id `service.application_execution`, implemented through `ServiceRuntime`, typed SDK/SystemFacade clients, descriptors, health checks, snapshots, trace-required calls, policy gates, and structured unavailable provider behavior.
- Implement all three required execution provider strategies behind one `ApplicationExecutionProvider` contract:
  - `macaca_hosted`: Macaca runtime-host executes the application component or adapter on the backend.
  - `external_app_backend`: an application-owned backend executes the loop but must write events through Macaca gateway ingress and receive control commands through Macaca.
  - `remote_agent`: a remote agent/runtime worker executes under Macaca-issued lease, heartbeat, capability delegation, and control protocol.
- Make EventLog the durable source of truth for application execution facts. Realtime/SSE/WebSocket surfaces become observers over persisted events, not the place where execution is owned.
- Add replay and current-state projection APIs derived from persisted session events so shells can recover after refresh/reconnect and users can inspect historical execution without depending on frontend state.
- Add control-command APIs for `cancel`, `approve`, `reject`, `pause`, `resume`, `retry`, and `inject_input` with idempotency, policy, audit, provider routing, and structured outcomes.
- Add gateway ingress APIs for external backends and remote agents to append sanitized events, report heartbeat, report snapshots, request approval, report completion, and report failure.
- Add protocol admission gates that reject providers, manifests, callbacks, or remote agents that do not declare capabilities, callback identity, provider descriptor, supported commands, event schema version, heartbeat policy, and audit-safe payload behavior.
- Update app-owned UI and Web shell paths so application frontends only start tasks, subscribe to session events, render replay/current state, and send control commands. Shells must not own long-running execution loops or authoritative events.
- Migrate `CODEX-WASM-WORKBENCH` validation onto the generic protocol as the first proof application without adding any Codex-specific logic to Macaca OS.
- Add tests and regression gates proving execution continues when the browser closes, replay works after refresh, all three providers obey the same protocol, unavailable providers return structured unavailable, and no application-specific branches enter OS services.

## Non-Goals

- Do not add Codex-specific, Workbench-specific, model-specific, provider-specific, or workflow-specific logic to Macaca OS services, runtime host, SDK, Web, CLI, or kernel crates.
- Do not replace existing `service.agent_execution`, `service.execution_control`, `service.llm`, `service.file`, `service.process`, `service.sandbox`, `service.approval`, `service.app_protocol`, `service.realtime`, or EventLog capabilities. This change composes them through a higher-level application execution protocol.
- Do not make Web, CLI, frontend, iframe bridge, or browser storage responsible for long-running execution, event persistence, replay truth, approval state, or cancellation semantics.
- Do not expose raw prompts, secrets, provider payloads, package bytes, WASM bytes, callback tokens, private keys, credentials, or unbounded tool output in EventLog, traces, snapshots, logs, diagnostics, or realtime payloads.
- Do not require every application to use Macaca-hosted execution. External application backends and remote agents are first-class provider strategies in this change.

## Impact

- Affected specs:
  - `application-execution-protocol-platform` (new capability)
  - Existing adjacent capabilities consumed by this change: `service-runtime`, `execution-control-service`, `web-cli-thin-shell-completion`, `web-cli-thin-shell-v0`, `sdk-system-facade`
- Affected code areas expected during implementation:
  - Protocol/foundation crates: `macaca/crates/foundation/macaca-proto/src/...`
  - SDK/facade crates: `macaca/crates/facade/macaca-sdk/src/...`
  - Runtime host service/provider crates: `macaca/crates/runtime/macaca-runtime-host/src/...`
  - Application framework/ABI adapters: `macaca/crates/application/macaca-app/src/...`
  - Web shell routes and bridges: `macaca/crates/shells/macaca-web/src/...`
  - Frontend app-owned UI bridge surfaces where they subscribe/render/control only: `frontend/lib/...`, `frontend/components/...`
  - Generic app package validation and first proof application: `apps/codex-wasm-workbench/...`
  - Integration tests: `macaca/crates/integration-tests/...` and focused crate tests near the implemented modules
  - Governance docs only where the implementation introduces or narrows ownership/dependency rules: `macaca/docs/macaca-os-architecture-governance.md`, `macaca/docs/macaca-os-microkernel-boundaries.md`, `macaca/docs/macaca-os-serviceization-allowlist.md`

## Governance Constraints

Implementation MUST comply with:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

Required ownership boundaries:

- Microkernel may only keep identity, registry, service-call facade, policy facade, trace/audit bus, session/task primitives, and package admission invariants.
- `service.application_execution` owns the replaceable application execution capability family and provider strategy routing.
- Runtime host owns provider wrappers, composition roots, service decorators, gateway ingress adapters, WASM/application host adapters, leases, and sanitized diagnostics.
- Application framework owns manifests, ABI, app lifecycle metadata, app-scoped permissions, session envelope, and UI surface metadata.
- Applications own product behavior and may orchestrate declared services only through capability and service boundaries.
- Web/CLI/frontend/gateway shells are adapters only: task start input, subscription, replay/current-state rendering, approval/cancel/control input, diagnostics display.

Required design patterns:

- Facade for SDK/SystemFacade and shell-facing clients.
- Command for every cross-boundary operation.
- Adapter/Bridge for provider transports and shell/app/backend integrations.
- Strategy for provider selection among `macaca_hosted`, `external_app_backend`, and `remote_agent`.
- Decorator for trace, policy, resource, entitlement, metering, and audit around service calls.
- State for session, run, provider, approval, lease, checkpoint, and lifecycle transitions.
- Observer for EventLog/realtime/session subscriptions.
- Memento for checkpoints, snapshots, replay, and audit records.
- Specification for manifest admission, provider admission, event schema validation, callback identity, and version compatibility.
- Null Object for unavailable providers and disabled optional modules.

## Success Criteria

The change is complete only when all of the following are true:

1. A frontend/app-owned UI can start an application execution and then close the browser while backend execution continues and persists events.
2. Reopening the UI with the same session id can replay the complete event history and render current state from EventLog-derived projection.
3. `macaca_hosted`, `external_app_backend`, and `remote_agent` providers are all implemented, registered, test-covered, and exercised through the same provider-neutral command/control/event protocol.
4. `cancel`, `approve`, `reject`, `pause`, `resume`, `retry`, and `inject_input` commands are typed, idempotent, audited, policy-gated, and routed to the selected provider without shell-owned semantics.
5. External backend and remote agent callbacks can append events only through Macaca gateway ingress with session/run binding, identity validation, schema validation, idempotency, payload sanitization, trace injection, and audit evidence.
6. Realtime transport emits event notifications from persisted EventLog rows or equivalent durable session events, not from frontend-local arrays or browser-owned loops.
7. Provider absence, callback denial, unsupported command, stale lease, duplicate idempotency key, invalid schema, policy denial, and backend timeout all return structured unavailable/denied/unsupported/failure states.
8. `CODEX-WASM-WORKBENCH` is validated as an application-layer proof that uses the generic platform and does not introduce Codex-specific branches inside OS services.
9. Dependency-boundary tests prove Web/CLI/frontend do not own application execution semantics and generic OS services do not import application-specific code.
10. Logs, traces, snapshots, diagnostics, EventLog rows, and SSE/realtime payloads are bounded and sanitized.
