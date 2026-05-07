## ADDED Requirements

### Requirement: Macaca SHALL define service bus envelopes for traced system service calls

Macaca SHALL define service bus envelope contracts that wrap a Phase 02 `ServiceCommand` with source identity, target service id, optional session context, optional task context, permission scope, deadline, idempotency key, trace context, and metadata.

#### Scenario: Service envelope preserves call context

- **WHEN** a service envelope is serialized and deserialized
- **THEN** the decoded envelope SHALL preserve envelope id, source identity, target `KernelServiceId`, command name, command payload, trace context, session id when present, task id when present, permission scope, deadline, idempotency key, and metadata
- **AND** the envelope contract SHALL NOT depend on `macaca-web`, frontend code, concrete provider crates, application manifests, driver implementations, gateway implementations, chain implementations, or business workflows

### Requirement: Macaca SHALL route service calls through a transport-neutral service bus facade

Macaca SHALL provide a service bus facade that dispatches service envelopes through a `ServiceTransport` bridge rather than through concrete local, child process, MCP, HTTP, or remote A2A transport details.

#### Scenario: Local service call uses the transport bridge

- **WHEN** a caller submits a valid service envelope to the service bus
- **THEN** the bus SHALL select a transport through the transport bridge
- **AND** the caller SHALL receive a structured service reply without depending on the concrete transport implementation
- **AND** route selection SHALL use service identity and transport capability data rather than application names, provider names, driver names, gateway names, model names, workflow names, chain names, or business-specific routing

### Requirement: Macaca SHALL provide local typed service transport without mandatory serialization

Macaca SHALL provide an in-process local service transport that dispatches typed service envelopes to registered local services without requiring every local call to serialize through JSON or another wire format.

#### Scenario: Mock local service completes through typed transport

- **WHEN** a mock system service is registered with the local service transport
- **AND** a valid traced envelope targets that service
- **THEN** the transport SHALL dispatch the command to the service
- **AND** the reply SHALL include success status, output, trace context, transport kind, and metadata
- **AND** the local dispatch path SHALL NOT require a wire-format encode/decode step before service execution

### Requirement: Macaca SHALL reject service bus calls without trace context before dispatch

Macaca SHALL reject service bus calls that lack trace context before selecting or dispatching a target service.

#### Scenario: Missing trace is rejected before routing

- **WHEN** a caller submits a service envelope without envelope trace context and without command trace context
- **THEN** the service bus SHALL return a structured missing-trace error
- **AND** no target service SHALL receive the command
- **AND** the rejection SHALL produce a structured log or audit record without leaking secrets or raw credentials

### Requirement: Macaca SHALL emit trace and audit events for service bus lifecycle

Macaca SHALL emit presentation-neutral trace/audit events for service bus accepted, routed, completed, failed, rejected, timed-out, and unavailable outcomes.

#### Scenario: Successful service bus call emits lifecycle trace

- **WHEN** a traced service envelope completes successfully through the service bus
- **THEN** trace/audit events SHALL include envelope id, source identity, target service id, command name, transport kind, status, duration, and correlation ids
- **AND** the events SHALL NOT require Web SSE, frontend state, CLI output, or `macaca-web`

#### Scenario: Failed service bus call emits structured failure trace

- **WHEN** a service bus call fails with a structured bus or service error
- **THEN** trace/audit events SHALL include envelope id, target service id, command name, transport kind when known, error code, reason, and correlation ids
- **AND** callers SHALL NOT need panic, hang detection, or provider-specific string parsing to understand the failure

### Requirement: Macaca SHALL expose service bus middleware as a Chain of Responsibility

Macaca SHALL expose ordered service bus middleware so trace validation, policy, budget, metering, deadline, idempotency, logging, and audit behavior can be composed without hardcoded provider branches.

#### Scenario: Trace middleware runs before dispatch

- **WHEN** the middleware chain processes a service envelope
- **THEN** trace validation SHALL run before routing and service dispatch
- **AND** later policy, budget, metering, or entitlement middleware SHALL be insertable without changing concrete service implementations

#### Scenario: Policy denial prevents dispatch

- **WHEN** policy middleware denies a service envelope
- **THEN** the service bus SHALL return a structured policy-denied error
- **AND** the target service SHALL NOT receive the command
- **AND** the denial SHALL be logged and emitted through the trace/audit boundary

### Requirement: Macaca SHALL enforce service bus deadlines as structured errors

Macaca SHALL treat expired or exceeded service bus deadlines as structured timeout/deadline errors.

#### Scenario: Expired deadline fails locally before service execution

- **WHEN** a service envelope reaches the local service bus with an already expired deadline
- **THEN** the bus SHALL return a structured deadline-exceeded error
- **AND** no target service SHALL receive the command
- **AND** the timeout outcome SHALL be logged and traceable

### Requirement: Macaca SHALL define future service transport extension points without implementing remote transports

Macaca SHALL define extension points for future local, child process, MCP, HTTP, signed remote A2A, and custom service transports while implementing only the local typed transport in Phase 03.

#### Scenario: Mock remote transport satisfies the bridge contract

- **WHEN** a test transport declares a non-local transport kind and implements the service transport trait
- **THEN** it SHALL compile and route through the same service bus facade
- **AND** Phase 03 SHALL NOT require production child process, MCP, HTTP, signed remote A2A, entitlement, or remote identity verification behavior

### Requirement: Macaca SHALL bridge service bus calls to Phase 02 `SystemService` contracts additively

Macaca SHALL provide an additive bridge that lets local service bus calls invoke Phase 02 `SystemService` contracts through the existing service call execution path without migrating all production consumers in Phase 03.

#### Scenario: Bus bridge invokes a mock system service

- **WHEN** a valid traced service envelope targets a mock Phase 02 `SystemService`
- **THEN** the bridge SHALL invoke the service through the service call executor or equivalent Phase 02 middleware path
- **AND** the reply SHALL preserve trace context and structured result data
- **AND** existing direct service call paths SHALL continue to compile and behave as before

### Requirement: Macaca SHALL log key service bus execution boundaries

Macaca SHALL record structured logs for key service bus execution boundaries without exposing secrets, raw provider credentials, or unredacted sensitive payloads.

#### Scenario: Service bus call lifecycle is logged

- **WHEN** a service envelope is accepted, rejected, routed, dispatched, completed, failed, times out, or targets an unavailable transport
- **THEN** logs SHALL include envelope id, source identity, target service id, command name, transport kind when known, status, duration when known, and error code when present
- **AND** logs SHALL NOT include provider credentials, raw secrets, or unredacted sensitive payloads

### Requirement: Macaca SHALL document new service bus code with detailed English comments

Macaca SHALL include detailed English comments in new Phase 03 Rust code explaining bus contracts, envelope validation, transport bridge operation, local typed dispatch, trace/audit behavior, policy insertion points, deadline behavior, and compatibility limitations.

#### Scenario: Maintainer can understand service bus invariants from comments

- **WHEN** a maintainer reads the new service bus modules
- **THEN** comments SHALL explain what each public type or trait represents
- **AND** comments SHALL explain how trace, audit, deadline, policy, routing, and local dispatch invariants are protected
- **AND** comments SHALL explain why concrete remote transports and provider execution remain outside the Phase 03 bus implementation

### Requirement: Macaca SHALL preserve Route C Phase 03 regression baselines

Macaca SHALL implement Phase 03 additively without regressing goal execution, live trace push, or the no-network pipeline baseline.

#### Scenario: Phase 03 baseline checks pass

- **WHEN** Phase 03 verification runs
- **THEN** the implementation SHALL preserve regression matrix scenarios `RC-GOAL-001`, `RC-TRACE-001`, and `RC-PIPE-001`
- **AND** existing YAML application, `/api/chat/v2`, trace, task board, resume, driver, and skill/MCP behavior SHALL continue to compile and run through existing paths until explicitly migrated by later changes
