## ADDED Requirements

### Requirement: Macaca SHALL provide Device Camera as a serviceized industrial pack

Macaca SHALL provide `pack.device.camera.v1` as a provider-neutral industrial pack for authorization, device discovery, capture sessions, preview streams, photo capture, video recording, frame references, controls, media references, revocation, and host camera status. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.device.camera.v1` as required and the device camera service is registered, healthy, entitled, policy-admissible, host-enabled, authorized or promptable, privacy-indicator-compatible, and command-compatible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, authorization state, device descriptor hashes, session/output limits, privacy indicator support, policy template, availability, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, credentials, raw frames, raw media bytes, stable hardware identifiers, raw provider payloads, or unbounded capture data

#### Scenario: Required declaration is unavailable or disabled
- **WHEN** an application declares `pack.device.camera.v1` as required but provider, command support, permission, entitlement, resource, host support, foreground state, privacy indicator, or host authorization is absent
- **THEN** admission SHALL block readiness with structured unavailable, disabled, foreground-required, privacy-indicator-unavailable, prompt-not-allowed, or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to another provider, or fake success

#### Scenario: Optional declaration is degraded
- **WHEN** an application declares `pack.device.camera.v1` as optional and the pack is unavailable, disabled, unauthorized, or command-limited
- **THEN** admission SHALL produce an explicit degraded effective capability report with bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Device Camera SHALL expose supplier-grade provider-neutral commands

`pack.device.camera.v1` SHALL expose typed commands for `camera.inspect_authorization`, `camera.request_authorization`, `camera.list_devices`, `camera.inspect_device`, `camera.open_session`, `camera.start_preview`, `camera.stop_preview`, `camera.capture_photo`, `camera.start_recording`, `camera.stop_recording`, `camera.read_frame`, `camera.set_controls`, `camera.inspect_controls`, `camera.close_session`, and `camera.inspect_host`.

#### Scenario: Authorization inspection reports effective state
- **WHEN** a declared and policy-allowed caller invokes `camera.inspect_authorization`
- **THEN** Macaca SHALL route the command through SDK/facade helpers into service runtime and the active camera provider
- **AND** the result SHALL include permission state, prompt eligibility, limited mode, host disabled reason, privacy indicator state, and provider class

#### Scenario: Authorization request is foreground mediated
- **WHEN** a caller invokes `camera.request_authorization`
- **THEN** Macaca SHALL require foreground/user-mediated policy and requested capability classes
- **AND** it SHALL return granted, denied, limited, prompt-not-allowed, or host-disabled state with trace evidence

#### Scenario: Device discovery returns redacted descriptors
- **WHEN** a caller invokes `camera.list_devices`
- **THEN** Macaca SHALL return opaque camera descriptors with facing mode, output modes, constraints, privacy class, availability, and descriptor hash
- **AND** it SHALL not expose stable hardware identifiers or raw host labels unless policy permits bounded labels

#### Scenario: Session open reserves capture resources
- **WHEN** a caller invokes `camera.open_session`
- **THEN** Macaca SHALL require device/constraint selection, output intents, max duration, foreground policy, privacy indicator support, and resource reservation
- **AND** it SHALL return a scoped `CameraSession` only after policy and resources succeed

#### Scenario: Preview stream is bounded by lease
- **WHEN** a caller invokes `camera.start_preview`
- **THEN** Macaca SHALL require active session, max duration, resolution/fps class, delivery mode, and redaction policy
- **AND** it SHALL return a `CameraPreviewLease` with dropped-frame counters and revocation behavior

#### Scenario: Photo capture returns media reference
- **WHEN** a caller invokes `camera.capture_photo`
- **THEN** Macaca SHALL capture a still image through the service runtime and return a bounded `CameraMediaReference`
- **AND** raw image bytes SHALL NOT enter generic trace or audit records

#### Scenario: Recording lifecycle is explicit
- **WHEN** a caller invokes `camera.start_recording` and `camera.stop_recording`
- **THEN** Macaca SHALL enforce max duration, max size, output policy, media retention, and resource budget
- **AND** stop SHALL finalize a media reference and release recording resources

#### Scenario: Frame read returns reference not raw stream
- **WHEN** a caller invokes `camera.read_frame`
- **THEN** Macaca SHALL enforce frame rate, size, privacy class, redaction, and resource budget
- **AND** the result SHALL contain a bounded `CameraFrameReference` or content reference rather than unbounded raw frames

#### Scenario: Controls validate supported ranges
- **WHEN** a caller invokes `camera.set_controls` or `camera.inspect_controls`
- **THEN** Macaca SHALL validate supported focus, exposure, white balance, zoom, torch, stabilization, and orientation modes
- **AND** unsupported controls SHALL return typed unsupported diagnostics

#### Scenario: Session close is idempotent
- **WHEN** a caller invokes `camera.close_session`
- **THEN** Macaca SHALL close preview/recording/frame outputs, release resources, and emit sanitized audit evidence
- **AND** repeated close calls SHALL return idempotent closed results

### Requirement: Device Camera DTOs SHALL model sessions, media references, controls, and privacy safely

The pack SHALL define provider-neutral DTOs for authorization, device descriptors, constraints, sessions, preview leases, frame references, media references, controls, host status, and structured errors. Provider adapters SHALL translate host-specific APIs into these DTOs and SHALL redact sensitive media by default.

#### Scenario: Session records lifecycle and policy
- **WHEN** a capture session is created
- **THEN** `CameraSession` SHALL include device ids, constraints, output intents, state, max duration, foreground requirement, approval id, resource reservation, privacy indicator state, and revocation state
- **AND** session state SHALL be replayable without raw media

#### Scenario: Media reference records retention
- **WHEN** photo or recording output is produced
- **THEN** `CameraMediaReference` SHALL include media id, kind, duration/size class, format, orientation, thumbnails when allowed, retention class, scan status, and storage/resource reference
- **AND** raw media bytes SHALL remain outside traces, audits, and snapshots

#### Scenario: Frame reference is bounded
- **WHEN** an analysis frame is produced
- **THEN** `CameraFrameReference` SHALL include frame id, timestamp, resolution class, format, orientation, redaction state, content reference, and expiry
- **AND** it SHALL not expose continuous raw frame streams through generic observability

#### Scenario: Structured errors are stable across providers
- **WHEN** providers return prompt not allowed, foreground required, device unavailable, constraint unsatisfied, session expired, privacy indicator unavailable, capture interrupted, media too large, quota, or provider failure states
- **THEN** Macaca SHALL map them to stable `CameraError` variants
- **AND** provider-specific diagnostics SHALL be sanitized and bounded

### Requirement: Device Camera SHALL enforce permission, policy, resource, entitlement, approval, privacy indicator, and revocation

Every command in `pack.device.camera.v1` SHALL run through permission, policy, resource, entitlement, approval, privacy-indicator, metering, and redaction decorators before and during provider use.

#### Scenario: Missing permission denies before provider dispatch
- **WHEN** an application invokes a command without required scope such as `device.camera.preview`, `device.camera.capture_photo`, `device.camera.record_video`, `device.camera.read_frame`, `device.camera.controls`, or `device.camera.session.manage`
- **THEN** Macaca SHALL return a typed denied result before invoking the concrete provider
- **AND** the audit event SHALL include the bounded missing-scope code

#### Scenario: Foreground access is required
- **WHEN** a camera command is requested while the host/application is not foreground-visible and delegated capture is not allowed
- **THEN** Macaca SHALL return foreground-required diagnostics before provider dispatch
- **AND** no capture session SHALL be opened

#### Scenario: Privacy indicator is enforced
- **WHEN** host policy requires a camera privacy indicator and the active provider cannot assert it
- **THEN** Macaca SHALL return privacy-indicator-unavailable diagnostics before capture
- **AND** audit evidence SHALL record the bounded policy reason

#### Scenario: Revocation closes capture outputs
- **WHEN** permission, policy, session, task, host, or user action revokes a camera session
- **THEN** Macaca SHALL stop preview/recording/frame outputs, close the session, release resources, and emit sanitized audit evidence
- **AND** subsequent commands SHALL return session-revoked diagnostics

#### Scenario: Recording quota blocks long capture
- **WHEN** requested duration, media size, frame rate, active sessions, CPU, memory, or retained snapshot budget exceeds policy
- **THEN** Macaca SHALL return quota-exceeded diagnostics before provider dispatch
- **AND** resource counters SHALL be emitted in sanitized trace evidence

### Requirement: Device Camera SHALL preserve canonical service runtime execution

All callable operations SHALL traverse the canonical Macaca service path: application declaration, admission/effective capability projection, SDK/facade command construction, service runtime dispatch, decorators, provider adapter, structured result, trace/audit evidence, capture lifecycle events, and replayable snapshot. SDK helpers SHALL NOT construct providers or create alternate execution paths.

#### Scenario: Command succeeds through the canonical path
- **WHEN** a declared and policy-allowed command is invoked
- **THEN** Macaca SHALL route it through SDK/facade helpers into service runtime dispatch and the active camera provider adapter
- **AND** trace evidence SHALL show declaration, admission, policy, entitlement, resource, provider selection, session/output state, command result, and replay pointer events

#### Scenario: Provider is absent
- **WHEN** no provider is registered for `pack.device.camera.v1`
- **THEN** the unavailable provider SHALL return structured unavailable diagnostics
- **AND** SDK discovery SHALL report unavailable state while preserving the same provider-neutral command/result contract

#### Scenario: Provider supports only a subset
- **WHEN** the active provider supports photo capture but not video recording, frame reads, controls, or privacy indicator assertion
- **THEN** SDK discovery SHALL mark unsupported commands/features as non-callable
- **AND** direct invocation SHALL return typed unsupported diagnostics without falling through to application-specific logic

#### Scenario: Provider is replaced
- **WHEN** a host-native, browser, remote-host, plugin, mock, or unavailable provider is selected
- **THEN** callers SHALL observe the same provider-neutral DTO contract
- **AND** OS-layer code SHALL identify only provider class, descriptor version, output mode, and capability metadata in traces rather than branching on provider names

### Requirement: Device Camera SHALL expose industrial SDK discovery and developer documentation

SDK discovery for `pack.device.camera.v1` SHALL expose pack metadata, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, authorization state, device descriptors, constraints, output limits, control support, privacy indicator support, policy templates, examples, diagnostics, compatibility, and documentation links. The implementation SHALL provide detailed developer documentation under `docs/developer-packs/device/camera.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.device.camera.v1`
- **THEN** it SHALL return command namespace `camera.*`, supported commands, required scopes, authorization state, device descriptors, constraints, output modes, control support, privacy indicator support, examples, lifecycle, health, diagnostics, compatibility metadata, and documentation URL
- **AND** examples SHALL use synthetic media references rather than raw frames, application-specific workflows, or provider-name routing

#### Scenario: Documentation covers app developer usage
- **WHEN** a developer opens `docs/developer-packs/device/camera.md`
- **THEN** the guide SHALL explain manifest declarations, required versus optional behavior, scopes, command DTOs, result DTOs, authorization, device discovery, sessions, preview, photo, video, frame references, controls, privacy indicators, revocation, unavailable diagnostics, trace/audit behavior, and replay workflow
- **AND** it SHALL include minimal app-facing examples using synthetic camera data and canonical SDK calls

#### Scenario: Documentation covers provider authors
- **WHEN** a provider author reads the guide
- **THEN** it SHALL document descriptor fields, host adapter responsibilities, session/preview/recording state machines, conformance tests, unsupported behavior, redaction rules, privacy indicator behavior, health/snapshot behavior, and replacement strategy
- **AND** it SHALL forbid raw media exposure and application-specific business routing in provider-neutral layers

### Requirement: Device Camera observability SHALL be sanitized, replayable, and auditable

The pack SHALL emit sanitized trace, audit, health, capture lifecycle, snapshot, and replay evidence for declaration, admission, policy, authorization, session open/close, preview, photo, recording, frame reference, control changes, revocation, command failures, unavailable state, and snapshot recording.

#### Scenario: Successful command emits bounded evidence
- **WHEN** a camera command succeeds
- **THEN** Macaca SHALL emit sanitized events containing pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when available, provider class, device class, session id hash, media id hash, output mode, resolution/fps class, privacy class, policy decision, latency, and resource counters
- **AND** it SHALL exclude raw frames, raw media, stable hardware identifiers, faces, documents, credentials, secrets, and unbounded provider payloads

#### Scenario: Snapshot records session summaries
- **WHEN** the service runtime records a camera snapshot
- **THEN** the snapshot SHALL include provider health, authorization state, device descriptor hashes, active session summaries, active output summaries, privacy indicator state, resource pressure, policy template hash, unavailable diagnostics, and sanitized replay pointers
- **AND** it SHALL exclude raw frames, raw media bytes, stable hardware identifiers, credentials, and unbounded output

#### Scenario: Replay verifies capture lifecycle
- **WHEN** a session or task is replayed after refresh or restart
- **THEN** Macaca SHALL reconstruct the camera command, session, preview, recording, and media reference chain from bounded trace/audit evidence
- **AND** replay diagnostics SHALL prove the commands used the canonical service runtime path without raw camera media

### Requirement: Device Camera implementation SHALL preserve Macaca architecture boundaries

The `pack.device.camera.v1` implementation SHALL keep concrete host/browser/remote providers behind service/runtime provider adapters. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, provider-specific, host-specific, device-model-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan imports
- **WHEN** dependency-boundary gates scan the implementation
- **THEN** they SHALL find no concrete camera provider, host camera API, browser camera API, media capture API, or remote camera client in the microkernel, SDK, shells, or generic application framework
- **AND** provider construction SHALL appear only in approved runtime composition roots or plugin/remote provider registration paths

#### Scenario: No-direct-provider-call gate scans commands
- **WHEN** no-direct-provider-call gates scan camera commands
- **THEN** every callable operation SHALL be reachable only through descriptor-owned service registrations and typed service runtime dispatch
- **AND** SDK helpers SHALL only build canonical service commands

#### Scenario: Pack remains separate from media and AI capabilities
- **WHEN** architecture review compares camera-related packs
- **THEN** device camera SHALL own host camera authorization, capture sessions, preview, photo/video capture, frame references, controls, and host status
- **AND** media processing, AI vision, OCR, local file persistence, and application-specific capture UX SHALL remain owned by their respective packs or services
