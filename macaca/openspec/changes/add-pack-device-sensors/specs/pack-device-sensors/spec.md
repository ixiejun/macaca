## ADDED Requirements

### Requirement: Macaca SHALL provide Device Sensors as a serviceized industrial pack

Macaca SHALL provide `pack.device.sensors.v1` as a provider-neutral industrial pack for sensor discovery, inspection, one-shot reads, bounded streams, batch reads, calibration inspection, resource leases, and host sensor status. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.device.sensors.v1` as required and the device sensor service is registered, healthy, entitled, policy-admissible, host-enabled, and command-compatible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, host status, sensor descriptor hashes, sampling limits, privacy classes, policy template, availability, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, credentials, raw host API payloads, stable hardware identifiers, raw sample vectors, or unbounded stream data

#### Scenario: Required declaration is unavailable or disabled
- **WHEN** an application declares `pack.device.sensors.v1` as required but provider, command support, permission, entitlement, resource, host support, foreground state, or host permission is absent
- **THEN** admission SHALL block readiness with structured unavailable, disabled, foreground-required, permission-prompt-required, or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to another provider, or fake success

#### Scenario: Optional declaration is degraded
- **WHEN** an application declares `pack.device.sensors.v1` as optional and the pack is unavailable, disabled, or command-limited
- **THEN** admission SHALL produce an explicit degraded effective capability report with bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Device Sensors SHALL expose supplier-grade provider-neutral commands

`pack.device.sensors.v1` SHALL expose typed commands for `sensors.list`, `sensors.inspect`, `sensors.read`, `sensors.open_stream`, `sensors.read_stream`, `sensors.close_stream`, `sensors.read_batch`, `sensors.inspect_calibration`, `sensors.acquire_lease`, `sensors.release_lease`, and `sensors.inspect_host`.

#### Scenario: Sensor discovery returns normalized descriptors
- **WHEN** a declared and policy-allowed caller invokes `sensors.list`
- **THEN** Macaca SHALL route the command through SDK/facade helpers into service runtime and the active sensor provider
- **AND** the result SHALL include normalized `SensorDescriptor` DTOs with type, availability, permission scopes, privacy class, units, axes, frequency limits, batching support, wake behavior, foreground requirement, and descriptor hash

#### Scenario: Sensor inspection explains capabilities
- **WHEN** a caller invokes `sensors.inspect` for a sensor descriptor
- **THEN** Macaca SHALL return sensor type, unit, coordinate frame, min/max frequency, batching support, accuracy classes, calibration support, privacy class, foreground/background policy, and provider limitations
- **AND** unsupported sensors SHALL return typed unsupported diagnostics

#### Scenario: One-shot read enforces freshness and policy
- **WHEN** a caller invokes `sensors.read`
- **THEN** Macaca SHALL enforce permission, foreground/background policy, timeout, freshness, privacy class, and resource budget before provider dispatch
- **AND** the result SHALL include a bounded `SensorReading` with timestamp clock, value, unit, coordinate frame, accuracy, sequence number, redaction state, and provenance

#### Scenario: Stream opening creates a bounded lease
- **WHEN** a caller invokes `sensors.open_stream`
- **THEN** Macaca SHALL require sensor ids, sampling frequency, max duration, max sample count, delivery mode, cancellation behavior, and revocation behavior
- **AND** it SHALL return a `SensorStreamLease` only after policy and resource reservation succeed

#### Scenario: Stream reading returns bounded chunks
- **WHEN** a caller invokes `sensors.read_stream` for an active lease
- **THEN** Macaca SHALL return bounded chunks with sequence numbers, sample count, dropped-sample counters, timestamps, accuracy, truncation state, and lease id reference
- **AND** raw unbounded streams SHALL NOT be emitted in trace or audit events

#### Scenario: Stream close is idempotent
- **WHEN** a caller invokes `sensors.close_stream`
- **THEN** Macaca SHALL release provider resources, mark the lease closed, and emit sanitized audit evidence
- **AND** repeated close calls SHALL return an idempotent closed result rather than leaking provider resources

#### Scenario: Batch read enforces duration and sample limits
- **WHEN** a caller invokes `sensors.read_batch`
- **THEN** Macaca SHALL enforce max duration, max sample count, frequency, foreground/background mode, and resource budget
- **AND** the result SHALL include a `SensorBatch` with bounded readings, dropped-sample count, clock drift warning, and truncation reason when applicable

#### Scenario: Calibration inspection is bounded
- **WHEN** a caller invokes `sensors.inspect_calibration`
- **THEN** Macaca SHALL return calibration state, accuracy class, calibration age, calibration source, and provider limitations when available
- **AND** it SHALL not expose calibration secrets, raw device identifiers, or raw provider payloads

#### Scenario: Lease acquire and release manage resources
- **WHEN** a caller invokes `sensors.acquire_lease` or `sensors.release_lease`
- **THEN** Macaca SHALL manage a revocable resource lease with duration, frequency, sensor types, resource reservation, and state transitions
- **AND** release SHALL close any active streams associated with the lease

#### Scenario: Host status explains disabled states
- **WHEN** a caller invokes `sensors.inspect_host`
- **THEN** Macaca SHALL return provider class, permission state, disabled reason, foreground requirement, active lease summary, resource pressure, command support, and diagnostics
- **AND** disabled host sensors SHALL not appear as fake empty success when policy requires explicit diagnostics

### Requirement: Device Sensors DTOs SHALL model sensor readings, streams, calibration, and host status safely

The pack SHALL define provider-neutral DTOs for sensor descriptors, sensor types, readings, vector values, coordinate frames, accuracy classes, stream leases, batches, calibration, host status, and structured errors. Provider adapters SHALL translate host-specific sensor APIs into these DTOs and SHALL redact or aggregate sensitive values for observability.

#### Scenario: Reading records coordinate frame and accuracy
- **WHEN** a sensor reading is returned
- **THEN** the `SensorReading` SHALL include timestamp, timestamp clock, value, unit, coordinate frame, accuracy, sequence number, sample interval, redaction state, and provenance
- **AND** callers SHALL be able to distinguish device-frame, screen-adjusted, world-frame, magnetic-north, true-north, and provider-defined readings

#### Scenario: Accuracy and calibration are explicit
- **WHEN** provider accuracy or calibration state is degraded or unavailable
- **THEN** Macaca SHALL expose `SensorAccuracy` and `SensorCalibration` diagnostics explicitly
- **AND** it SHALL not silently treat uncalibrated readings as high accuracy

#### Scenario: Lease state is explicit
- **WHEN** a stream lease changes state
- **THEN** Macaca SHALL represent requested, active, draining, closed, expired, revoked, failed, and unavailable states explicitly
- **AND** replay diagnostics SHALL show why a stream ended

#### Scenario: Structured errors are stable across providers
- **WHEN** providers return disabled, permission prompt, foreground required, lease expired, sample rate too high, stream overflow, timeout, quota, calibration unavailable, or provider failure states
- **THEN** Macaca SHALL map them to stable `SensorError` variants
- **AND** provider-specific diagnostics SHALL be sanitized and bounded

### Requirement: Device Sensors SHALL enforce permission, policy, resource, entitlement, approval, and revocation

Every command in `pack.device.sensors.v1` SHALL run through permission, policy, resource, entitlement, metering, approval, and revocation decorators before and during provider use.

#### Scenario: Missing permission denies before provider dispatch
- **WHEN** an application invokes a command without required scope such as `device.sensors.read`, `device.sensors.stream`, `device.sensors.calibration.read`, or `device.sensors.lease.manage`
- **THEN** Macaca SHALL return a typed denied result before invoking the concrete provider
- **AND** the audit event SHALL include the bounded missing-scope code

#### Scenario: High-frequency stream requires budget
- **WHEN** a caller requests a high-frequency stream
- **THEN** Macaca SHALL enforce frequency, sample count, event-buffer, CPU, memory, duration, and privacy policy before provider dispatch
- **AND** quota rejection SHALL return typed quota-exceeded diagnostics without opening a provider stream

#### Scenario: Background access is denied by default
- **WHEN** a caller requests sensor access while the host/application is not foreground-visible
- **THEN** Macaca SHALL deny the command unless the foreground/background host capability and policy explicitly allow it
- **AND** the result SHALL include foreground-required or denied diagnostics

#### Scenario: Revocation closes active streams
- **WHEN** permission, policy, session, task, or host state revokes a sensor lease
- **THEN** Macaca SHALL close active provider streams, mark leases revoked, release resources, and emit sanitized audit evidence
- **AND** subsequent reads SHALL return lease-revoked diagnostics

#### Scenario: Approval is required for sensitive sensors
- **WHEN** host policy marks a sensor command sensitive because of high-frequency motion, environmental monitoring, remote forwarding, background access, or host permission prompt
- **THEN** Macaca SHALL require explicit approval evidence before dispatch
- **AND** denial or missing approval SHALL be traceable without leaking raw samples

### Requirement: Device Sensors SHALL preserve canonical service runtime execution

All callable operations SHALL traverse the canonical Macaca service path: application declaration, admission/effective capability projection, SDK/facade command construction, service runtime dispatch, decorators, provider adapter, structured result, trace/audit evidence, and replayable snapshot. SDK helpers SHALL NOT construct providers or create alternate execution paths.

#### Scenario: Command succeeds through the canonical path
- **WHEN** a declared and policy-allowed command is invoked
- **THEN** Macaca SHALL route it through SDK/facade helpers into service runtime dispatch and the active sensor provider adapter
- **AND** trace evidence SHALL show declaration, admission, policy, entitlement, resource, provider selection, command result, lease state if applicable, and replay pointer events

#### Scenario: Provider is absent
- **WHEN** no provider is registered for `pack.device.sensors.v1`
- **THEN** the unavailable provider SHALL return structured unavailable diagnostics
- **AND** SDK discovery SHALL report unavailable state while preserving the same provider-neutral command/result contract

#### Scenario: Provider supports only a subset
- **WHEN** the active provider supports listing and one-shot reads but not streams, batching, or calibration inspection
- **THEN** SDK discovery SHALL mark unsupported commands as non-callable
- **AND** direct invocation SHALL return typed unsupported diagnostics without falling through to application-specific logic

#### Scenario: Provider is replaced
- **WHEN** a host-native, browser, remote, plugin, mock, or unavailable provider is selected
- **THEN** callers SHALL observe the same provider-neutral DTO contract
- **AND** OS-layer code SHALL identify only provider class, descriptor version, privacy class, and capability metadata in traces rather than branching on provider names

### Requirement: Device Sensors SHALL expose industrial SDK discovery and developer documentation

SDK discovery for `pack.device.sensors.v1` SHALL expose pack metadata, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, host status, sensor descriptors, sampling limits, privacy classes, policy templates, examples, diagnostics, compatibility, and documentation links. The implementation SHALL provide detailed developer documentation under `docs/developer-packs/device/sensors.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.device.sensors.v1`
- **THEN** it SHALL return command namespace `sensors.*`, supported commands, required scopes, host status, sensor descriptors, sampling limits, privacy classes, policy templates, examples, lifecycle, health, diagnostics, compatibility metadata, and documentation URL
- **AND** examples SHALL use generic synthetic data rather than application-specific workflows or provider-name routing

#### Scenario: Documentation covers app developer usage
- **WHEN** a developer opens `docs/developer-packs/device/sensors.md`
- **THEN** the guide SHALL explain manifest declarations, required versus optional behavior, scopes, command DTOs, result DTOs, sensor types, units, coordinate frames, accuracy, calibration, sampling, batching, stream leases, revocation, unavailable diagnostics, trace/audit behavior, and replay workflow
- **AND** it SHALL include minimal app-facing examples that use synthetic sensor data and canonical SDK calls

#### Scenario: Documentation covers provider authors
- **WHEN** a provider author reads the guide
- **THEN** it SHALL document descriptor fields, host adapter responsibilities, stream lease state machine, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy
- **AND** it SHALL forbid application-specific business routing in provider-neutral layers

### Requirement: Device Sensors observability SHALL be sanitized, replayable, and auditable

The pack SHALL emit sanitized trace, audit, health, stream, lease, snapshot, and replay evidence for declaration, admission, policy, entitlement, resource reservation, command request, provider selection, stream open/chunk/close, lease revocation, command result, unavailable state, and snapshot recording.

#### Scenario: Successful command emits bounded evidence
- **WHEN** a sensor command succeeds
- **THEN** Macaca SHALL emit sanitized events containing pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when available, provider class, sensor type, privacy class, frequency class, sample count, dropped count, lease id hash, policy decision, latency, and resource counters
- **AND** it SHALL exclude raw sample vectors, stable hardware identifiers, raw host API payloads, secrets, credentials, and unbounded stream data

#### Scenario: Stream chunk event is aggregated
- **WHEN** a stream chunk is delivered
- **THEN** Macaca SHALL emit only bounded counters, timing metadata, dropped-sample count, sensor type, privacy class, and lease id hash
- **AND** raw vectors and high-frequency sample payloads SHALL remain outside generic trace/audit records

#### Scenario: Snapshot records lease summaries
- **WHEN** the service runtime records a sensor snapshot
- **THEN** the snapshot SHALL include provider health, host status, supported command matrix, sensor descriptor hashes, active lease summaries, resource pressure, policy template hash, unavailable diagnostics, and sanitized replay pointers
- **AND** it SHALL exclude raw sample data, stable hardware identifiers, raw host API payloads, credentials, and unbounded output

#### Scenario: Replay verifies stream lifecycle
- **WHEN** a session or task is replayed after refresh or restart
- **THEN** Macaca SHALL reconstruct the sensor command and stream lease chain from bounded trace/audit evidence
- **AND** replay diagnostics SHALL prove the commands used the canonical service runtime path without raw sensor samples

### Requirement: Device Sensors implementation SHALL preserve Macaca architecture boundaries

The `pack.device.sensors.v1` implementation SHALL keep concrete host/browser/remote providers behind service/runtime provider adapters. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, provider-specific, host-specific, or sensor-model-specific routing branches.

#### Scenario: Boundary gates scan imports
- **WHEN** dependency-boundary gates scan the implementation
- **THEN** they SHALL find no concrete sensor provider, host sensor API, browser sensor API, or remote sensor client in the microkernel, SDK, shells, or generic application framework
- **AND** provider construction SHALL appear only in approved runtime composition roots or plugin/remote provider registration paths

#### Scenario: No-direct-provider-call gate scans commands
- **WHEN** no-direct-provider-call gates scan sensor commands
- **THEN** every callable operation SHALL be reachable only through descriptor-owned service registrations and typed service runtime dispatch
- **AND** SDK helpers SHALL only build canonical service commands

#### Scenario: Pack remains separate from neighboring device packs
- **WHEN** architecture review compares device packs
- **THEN** sensors SHALL own sensor descriptors, readings, streams, batches, calibration, leases, and host sensor status
- **AND** camera, local-files, notifications, foreground/background host capabilities, location, and application lifecycle SHALL remain owned by their respective packs or services
