## ADDED Requirements

### Requirement: Application Service Contract
The system SHALL provide a provider-neutral Application Service contract for application discovery, load/admission, start, stop, remove, status, snapshot, session start/resume/stop, host dispatch, and GenUI surface lookup.

#### Scenario: Service contract exposes lifecycle operations
- **WHEN** a runtime host registers the Application Service
- **THEN** the service descriptor SHALL advertise stable operation names for discover, load, start, stop, remove, status, snapshot, session lifecycle, host dispatch, and GenUI surface lookup.

#### Scenario: Mutating commands require trace
- **WHEN** a caller creates a start, stop, remove, session, host dispatch, or GenUI command without trace context
- **THEN** the command SHALL be rejected before provider dispatch.

### Requirement: Sanitized Application Views
The system SHALL expose sanitized application views and snapshots that contain ids, names, versions, runtime kind, lifecycle state, compatibility status, entry agent, agent counts/names, session ids, diagnostics, and safe path metadata only.

#### Scenario: Snapshot excludes unsafe payloads
- **WHEN** an Application Service snapshot is requested
- **THEN** the snapshot SHALL NOT include prompt bodies, full manifest bodies, raw agent configs, env values, API keys, secrets, or raw host command payloads.

#### Scenario: Diagnostics are structured
- **WHEN** an application fails to load or start
- **THEN** the result SHALL include structured diagnostics with service id, application scope when known, runtime kind when known, status, and reason.

### Requirement: Application Lifecycle State
The system SHALL use Application ABI lifecycle state as the service lifecycle truth and MAY project legacy `AppStatus` as a compatibility view.

#### Scenario: YAML app starts through service
- **WHEN** a YAML application is started through Application Service
- **THEN** the service SHALL transition the application through traceable lifecycle states and return a running compatibility status when startup succeeds.

#### Scenario: WASM execution unavailable
- **WHEN** a WASM application or package is admitted through Application Service before a real WASM runtime exists
- **THEN** metadata admission MAY succeed but execution SHALL return structured runtime-unavailable.

### Requirement: Application Admission Specifications
The system SHALL validate trace, manifest, runtime kind, application scope, session scope, and compatibility using reusable specification-style admission checks.

#### Scenario: Invalid scope is rejected
- **WHEN** a session lifecycle command has a blank application id or blank session id
- **THEN** the Application Service SHALL reject the command before provider dispatch.

#### Scenario: Unsupported runtime is explicit
- **WHEN** a runtime kind is unsupported for execution
- **THEN** the Application Service SHALL return a structured unavailable or runtime-unavailable result rather than panic, hang, or silently succeed.

