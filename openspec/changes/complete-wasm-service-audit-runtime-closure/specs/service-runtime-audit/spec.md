## ADDED Requirements

### Requirement: Shared Audit Sink Consistency
The runtime host SHALL use one shared `ServiceCallAuditSink` instance for both WASM `service.call` audit emission and system replay query surfaces within the same host runtime scope.

#### Scenario: Replay sees WASM emitted events
- **WHEN** a WASM host import dispatches `service.call` through the production runtime path
- **AND** routing emits structured audit events
- **THEN** querying `service.audit.replay.trace` with the same `trace_id` returns those events
- **AND** querying `service.audit.replay.session` with the same `session_id` returns those events

### Requirement: Generic System Replay Contract
The runtime host SHALL expose service-call audit replay through provider-neutral system service commands and SHALL NOT require any application-specific code path.

#### Scenario: Replay command contract
- **WHEN** a caller invokes `service.audit.replay.trace` or `service.audit.replay.session`
- **THEN** the system responds with structured replay events from the shared sink
- **AND** the response format remains serializable and audit-safe

### Requirement: Structured Observability for Audit Lifecycle
The runtime host SHALL emit structured logs for audit service startup, sink binding, replay query handling, and replay errors.

#### Scenario: Replay error observability
- **WHEN** replay query handling fails
- **THEN** the runtime emits a structured error log with trace-safe metadata
- **AND** the failure does not leak raw payload content or sensitive fields
