## ADDED Requirements

### Requirement: Macaca SHALL expose a provider-neutral application execution protocol service

Macaca SHALL expose `service.application_execution` as the provider-neutral service boundary for starting, controlling, replaying, projecting, and diagnosing application execution sessions. The service SHALL be registered through ServiceRuntime, SHALL require trace context on every call, SHALL evaluate policy before side effects, SHALL expose descriptor/health/snapshot command surfaces, and SHALL return structured unavailable behavior when no provider is active.

#### Scenario: Service starts an execution through a provider-neutral command

- **GIVEN** an application has declared the required application execution capability and requested service permissions
- **AND** a caller submits a traced `StartApplicationExecutionCommand` with application id, session id or session request, task input or payload reference, workspace reference, requested capabilities, provider preference, policy context, actor, and idempotency key
- **WHEN** `service.application_execution` accepts the command
- **THEN** it SHALL create or reuse a session/run envelope
- **AND** it SHALL evaluate policy, resource, capability, entitlement, and provider admission before provider side effects
- **AND** it SHALL select a provider through the provider strategy registry
- **AND** it SHALL append a durable session event for the accepted execution before returning success
- **AND** it SHALL return session id, run id, provider id, provider kind, event cursor, control reference, workspace reference, and traceable status.

#### Scenario: Service is unavailable

- **GIVEN** `service.application_execution` has no active provider stack or is disabled by configuration
- **WHEN** a caller submits start, control, replay, current-state, provider-health, snapshot, or gateway-ingress commands
- **THEN** the service SHALL return a structured unavailable result
- **AND** it SHALL NOT crash, hang, fake success, silently fall back to browser execution, or bypass policy.

### Requirement: Macaca SHALL support three application execution provider strategies in the same protocol

Macaca SHALL implement `macaca_hosted`, `external_app_backend`, and `remote_agent` provider strategies behind one `ApplicationExecutionProvider` contract. Provider differences SHALL be isolated behind Adapter/Strategy implementations and SHALL NOT require OS-layer branches on application id, workflow name, business domain, model name, driver name, gateway name, or concrete provider name.

#### Scenario: Macaca-hosted provider executes on the backend

- **GIVEN** an application manifest or execution profile selects or permits `macaca_hosted`
- **WHEN** `service.application_execution` assigns the run to that provider
- **THEN** runtime-host SHALL start a backend-owned execution task through approved application ABI/runtime-host boundaries
- **AND** the task SHALL continue independently of browser subscribers
- **AND** application service calls SHALL pass through declared capabilities and ServiceRuntime
- **AND** lifecycle, service-call, approval, checkpoint, completion, failure, and cancellation facts SHALL be appended as durable application execution events.

#### Scenario: External app backend provider executes through gateway protocol

- **GIVEN** an application manifest declares an external backend endpoint, protocol version, callback identity reference, heartbeat policy, timeout, supported controls, and event schema version
- **WHEN** `service.application_execution` assigns the run to `external_app_backend`
- **THEN** Macaca SHALL call the backend start endpoint with session/run/workspace/callback envelope and scoped callback identity reference
- **AND** the backend SHALL write authoritative execution facts only through Macaca gateway ingress
- **AND** Macaca SHALL forward control commands to the backend control endpoint through the same control protocol
- **AND** callbacks that fail identity, lease, schema, idempotency, session/run binding, or sanitization checks SHALL be rejected before event append.

#### Scenario: Remote agent provider executes under lease

- **GIVEN** a remote agent registers a descriptor with supported capabilities, protocol version, transport metadata, heartbeat policy, resource profile, and control support
- **WHEN** `service.application_execution` assigns a run to `remote_agent`
- **THEN** Macaca SHALL issue a scoped execution lease with expiry, allowed event types, allowed controls, callback identity reference, and trace context
- **AND** the remote agent SHALL append events, heartbeat, snapshots, approval requests, completion, and failure through gateway ingress
- **AND** stale leases or invalid callback identities SHALL be rejected
- **AND** heartbeat timeout SHALL produce a structured provider failure or stale-lease state.

### Requirement: EventLog SHALL be the durable source of truth for application execution events

Macaca SHALL persist application execution events before realtime delivery. Realtime/SSE/WebSocket delivery SHALL be an Observer projection over durable EventLog/session events, not the authority for execution state.

#### Scenario: Browser disconnect does not stop backend execution

- **GIVEN** an application execution has started and a frontend subscriber is receiving realtime events
- **WHEN** the browser tab closes, the iframe unloads, or the subscriber disconnects without sending a cancel control command
- **THEN** backend execution SHALL continue according to provider policy
- **AND** provider events SHALL continue to be appended to the durable session event store
- **AND** no browser-local event buffer SHALL be required for execution progress.

#### Scenario: Replay reconstructs execution after refresh

- **GIVEN** an application execution has persisted events for a session/run
- **WHEN** a shell or application UI reconnects with the same session id and replay cursor
- **THEN** Macaca SHALL replay events in deterministic order from the requested cursor
- **AND** Macaca SHALL provide current state derived from persisted events
- **AND** pending approvals, active controls, latest provider heartbeat, latest checkpoint, summarized LLM/tool steps, and terminal outcome SHALL match the event history.

### Requirement: Application execution events SHALL be schema-validated, idempotent, traceable, and sanitized

Macaca SHALL validate every application execution event envelope before append. Events SHALL include application id, session id, run id, sequence or idempotency key, timestamp, event type, trace id, actor, provider id, provider kind, visibility, causality, sanitized payload or payload reference, and schema version. Events SHALL be idempotent and SHALL NOT leak raw sensitive data.

#### Scenario: Duplicate event append is idempotent

- **GIVEN** a provider or gateway caller submits an event with an idempotency key that has already been accepted for the same application/session/run
- **WHEN** Macaca processes the duplicate append
- **THEN** Macaca SHALL return the original event cursor or structured duplicate result
- **AND** it SHALL NOT append a second authoritative event
- **AND** it SHALL NOT redeliver provider side effects.

#### Scenario: Unsafe payload is rejected or redacted

- **GIVEN** an event payload contains raw secrets, credentials, callback tokens, raw prompts, raw provider payloads, package bytes, WASM bytes, private keys, or unbounded output
- **WHEN** the event append path validates the envelope
- **THEN** Macaca SHALL reject the event with `sanitization_failed` or replace sensitive content with bounded summaries, hashes, redacted fields, or policy-governed payload references
- **AND** the unsafe raw payload SHALL NOT enter EventLog, trace, audit, logs, snapshots, diagnostics, or realtime payloads.

### Requirement: Execution control SHALL be typed, audited, idempotent, and provider-routed

Macaca SHALL route `cancel`, `approve`, `reject`, `pause`, `resume`, `retry`, and `inject_input` through typed `ApplicationExecutionControlCommand` values. Control commands SHALL include application/session/run scope, actor, reason code, trace context, policy context, idempotency key, and bounded payload or payload reference.

#### Scenario: Approval is delivered through the selected provider

- **GIVEN** an application execution has emitted `approval.requested`
- **WHEN** a user or system actor submits an `approve` control command
- **THEN** `service.application_execution` SHALL evaluate policy
- **AND** it SHALL append `control.requested`
- **AND** it SHALL deliver the command to the selected provider adapter
- **AND** it SHALL append `control.delivered` and `control.completed` or structured delivery failure
- **AND** the approval outcome SHALL be replayable from durable events.

#### Scenario: Duplicate cancel is not delivered twice

- **GIVEN** a caller submits `cancel` with an idempotency key that was already accepted for the same run
- **WHEN** the command is processed again
- **THEN** Macaca SHALL return the original control outcome
- **AND** it SHALL NOT deliver a second cancel side effect to the provider
- **AND** audit evidence SHALL show the duplicate decision.

### Requirement: Gateway ingress SHALL constrain external backends and remote agents

Macaca SHALL require external app backends and remote agents to report execution facts through gateway ingress commands. Gateway ingress SHALL validate callback identity, application/session/run binding, provider lease, schema version, idempotency key, event type allowlist, payload size, payload sanitization, trace continuation, policy, resource, and entitlement constraints before appending events.

#### Scenario: Invalid external callback is rejected

- **GIVEN** an external backend callback has an invalid signature, unknown session/run binding, stale lease, unsupported event type, invalid schema version, or oversized unsafe payload
- **WHEN** it calls gateway ingress
- **THEN** Macaca SHALL reject the callback with structured denied, stale_lease, unsupported, invalid_schema, or sanitization_failed result
- **AND** no event SHALL be appended
- **AND** sanitized audit evidence SHALL record the rejection reason.

#### Scenario: Valid remote heartbeat extends lease visibility

- **GIVEN** a remote agent has an active lease for an application execution
- **WHEN** it reports heartbeat through gateway ingress before the heartbeat deadline
- **THEN** Macaca SHALL validate the lease and callback identity
- **AND** it SHALL append or update durable heartbeat evidence
- **AND** current-state projection SHALL expose the latest provider heartbeat without exposing callback credentials.

### Requirement: Shells and app-owned UIs SHALL remain interaction adapters

Web, CLI, frontend, app-owned UI bundles, and iframe bridges SHALL only start tasks, subscribe to session events, query replay/current state, render event-derived UI, and send typed control commands. They SHALL NOT own production execution loops, provider assignment, EventLog persistence, approval/cancel semantics, or authoritative execution state.

#### Scenario: App-owned UI starts execution without owning the loop

- **GIVEN** an app-owned UI bundle declares the application execution bridge capability
- **WHEN** it starts a task from the UI
- **THEN** the shell SHALL translate the input into a typed application execution command
- **AND** the shell SHALL call SDK/SystemFacade or a focused client
- **AND** backend service/provider infrastructure SHALL own the execution loop
- **AND** the UI SHALL render only replay/current-state/subscription output derived from persisted events.

#### Scenario: Shell direct loop is rejected as production path

- **GIVEN** a browser-side loop attempts to act as the production owner of LLM/tool execution, durable event append, provider assignment, approval state, or cancellation semantics
- **WHEN** boundary tests or runtime admission checks evaluate the path
- **THEN** the path SHALL be rejected, disabled, or classified as debug-only
- **AND** production validation SHALL use `service.application_execution`.

### Requirement: The platform SHALL prove application-neutral Codex-class execution

Macaca SHALL validate the new platform with `CODEX-WASM-WORKBENCH` as an application-layer proof while keeping OS services application-neutral. The proof SHALL demonstrate real task execution, backend-owned execution continuity, durable event persistence, replay/current-state recovery, and control command handling without Codex-specific OS branches.

#### Scenario: Workbench task survives browser close and replays

- **GIVEN** `CODEX-WASM-WORKBENCH` starts a real programming task through the generic application execution protocol
- **WHEN** the browser subscriber closes before the task completes
- **THEN** backend execution SHALL continue
- **AND** LLM/tool/service events SHALL be persisted through durable application execution events
- **AND** reopening the UI SHALL replay the complete history and current state from EventLog-derived projection
- **AND** no Macaca OS service SHALL branch on the Workbench application id or Codex-specific workflow names.

#### Scenario: All three provider kinds are exercised

- **GIVEN** the validation suite runs platform proof cases
- **WHEN** it executes Workbench or equivalent application-neutral fixtures through `macaca_hosted`, `external_app_backend`, and `remote_agent`
- **THEN** each provider kind SHALL use the same start/control/event/replay protocol
- **AND** provider-specific transport behavior SHALL remain behind provider adapters
- **AND** results SHALL be comparable through the same durable session event and current-state projection APIs.

### Requirement: Boundary and dependency gates SHALL prevent ownership regression

Macaca SHALL add tests or static gates proving that the application execution protocol platform preserves microkernel, serviceization, application-framework, runtime-host, and shell boundaries.

#### Scenario: Generic OS services do not import application-specific code

- **WHEN** dependency-boundary tests scan kernel, foundation, SDK, service runtime, runtime-host generic service modules, Web shell adapters, and CLI adapters
- **THEN** they SHALL reject dependencies on app package directories or application-specific modules
- **AND** they SHALL reject hardcoded routing branches based on application id, Codex/Workbench names, workflow names, model names, provider names, driver names, gateway names, or business domains.

#### Scenario: Presentation shells cannot own execution semantics

- **WHEN** dependency-boundary tests scan Web, CLI, frontend, and app UI bridge code
- **THEN** shells MAY contain input parsing, API/SSE mapping, replay rendering, approval rendering, diagnostics display, and control-command submission
- **AND** shells SHALL NOT contain production provider loops, durable EventLog append ownership, provider assignment logic, approval state machines, cancellation semantics, or direct external backend authoritative-state subscriptions.
