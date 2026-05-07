## ADDED Requirements

### Requirement: Macaca SHALL define provider-neutral system service descriptors

Macaca SHALL define provider-neutral service descriptor contracts in `macaca-proto` so services can describe type, capabilities, lifecycle state, health, permissions, scopes, trace schema, cleanup policy, and metadata without depending on provider or presentation crates.

#### Scenario: Service descriptor round trips through serde

- **WHEN** a service descriptor is serialized and deserialized
- **THEN** the decoded descriptor SHALL preserve service id, service type, capabilities, lifecycle state, health, permissions, supported scopes, trace schema, cleanup policy, and metadata
- **AND** the descriptor module SHALL NOT depend on `macaca-web`, concrete provider crates, application manifests, or plugin implementations

### Requirement: Macaca SHALL model service types as extensible value objects

Macaca SHALL model service type as an extensible string-backed value object rather than a closed business enum.

#### Scenario: Third-party service type requires no kernel enum edit

- **WHEN** a third-party service descriptor uses a previously unknown service type string
- **THEN** the descriptor SHALL be constructible and serializable
- **AND** kernel service contract code SHALL NOT require source changes to accept the type as descriptor data

### Requirement: Macaca SHALL define a kernel `SystemService` contract

Macaca SHALL define a kernel `SystemService` contract for descriptor export, lifecycle control, health reporting, command-style calls, stop, and cleanup.

#### Scenario: Mock service completes lifecycle and call

- **WHEN** a mock system service is registered, started, called with a valid trace context, stopped, and cleaned up
- **THEN** each step SHALL return structured success data
- **AND** the service descriptor and health state SHALL remain queryable

### Requirement: Macaca SHALL represent service calls as commands

Macaca SHALL represent a service invocation as a `ServiceCommand` carrying command name, payload, trace context, and metadata.

#### Scenario: Service command carries trace and payload

- **WHEN** a service receives a command
- **THEN** the command SHALL expose the command name and JSON payload
- **AND** it SHALL expose the trace context required for auditing and correlation
- **AND** it SHALL avoid provider-specific command structs at the kernel contract boundary

### Requirement: Macaca SHALL reject service calls without trace context

Macaca SHALL reject every system service call that lacks a `TraceContext` before dispatching to service logic.

#### Scenario: Missing trace context fails before dispatch

- **WHEN** a caller submits a service command without trace context
- **THEN** the service call executor SHALL return a structured missing-trace error
- **AND** the target service SHALL NOT receive the command
- **AND** a rejection log or audit record SHALL be produced without leaking secrets

### Requirement: Macaca SHALL emit trace and audit events for service calls

Macaca SHALL emit trace/audit events for accepted, completed, failed, and rejected service calls through a presentation-neutral observer boundary.

#### Scenario: Successful call emits trace event

- **WHEN** a service call with valid trace context succeeds
- **THEN** the trace event SHALL include service id, command name, lifecycle state, call status, and correlation ids
- **AND** the event SHALL NOT require Web SSE, frontend state, or `macaca-web`

#### Scenario: Failed call emits structured failure trace

- **WHEN** a service call fails with a structured service error
- **THEN** the trace/audit event SHALL include error code and reason
- **AND** callers SHALL NOT need panic, hang detection, or provider-specific string parsing to understand the failure

### Requirement: Macaca SHALL log key service execution boundaries

Macaca SHALL record structured logs for key system service execution boundaries without exposing secrets or raw credentials.

#### Scenario: Service call lifecycle is logged

- **WHEN** a service is registered, transitions lifecycle state, accepts a call, rejects a call, completes a call, fails a call, starts cleanup, or finishes cleanup
- **THEN** the implementation SHALL log the service id, command name when present, lifecycle state, status, and error code when present
- **AND** the log SHALL NOT include provider credentials or unredacted secret payloads

### Requirement: Macaca SHALL expose service call middleware as a Chain of Responsibility

Macaca SHALL model service call middleware as an ordered chain so trace validation, policy, budget, metering, logging, and audit checks can be composed without hardcoded provider branches.

#### Scenario: Trace middleware runs before service dispatch

- **WHEN** the middleware chain processes a service command
- **THEN** trace validation SHALL run before the service call is dispatched
- **AND** later policy or metering middleware SHALL be insertable without changing the service implementation

### Requirement: Macaca SHALL provide adapter skeletons for existing built-in service crates

Macaca SHALL provide adapter skeletons that describe existing LLM, Task, Trace, Driver, Skill, Gateway, and Memory capabilities as system services without migrating their existing runtime call paths in Phase 02.

#### Scenario: Built-in adapters export descriptors

- **WHEN** each built-in adapter skeleton is queried
- **THEN** it SHALL export a descriptor with service type, capabilities, health, supported scopes, required permissions, trace schema, and cleanup policy
- **AND** it SHALL NOT hardcode application names, provider names, driver names, gateway names, model names, workflow names, chain names, or business-specific routing in kernel contracts

### Requirement: Macaca SHALL preserve Route C Phase 02 regression baselines

Macaca SHALL implement Phase 02 additively without regressing goal execution, live trace push, skill/MCP trace behavior, or the Route C no-network baseline.

#### Scenario: Phase 02 baseline checks pass

- **WHEN** Phase 02 verification runs
- **THEN** the implementation SHALL preserve regression matrix scenarios `RC-GOAL-001`, `RC-TRACE-001`, and `RC-SKILL-001`
- **AND** the no-network Route C baseline SHALL continue to pass

### Requirement: Macaca SHALL document new service contract code with detailed English comments

Macaca SHALL include detailed English comments in new Phase 02 Rust code explaining service descriptor purpose, lifecycle operation, trace/audit behavior, logging boundaries, policy insertion points, and compatibility limitations.

#### Scenario: Comments explain service contract operation

- **WHEN** a maintainer reads the new service contract and adapter skeleton code
- **THEN** comments SHALL explain what each type or trait represents
- **AND** comments SHALL explain how trace/audit and lifecycle invariants are protected
- **AND** comments SHALL explain why concrete provider execution remains outside kernel
