## ADDED Requirements

### Requirement: Macaca SHALL provide the AI Vision Pack as a serviceized capability

Macaca SHALL provide `pack.ai.vision.v1` as a provider-neutral industrial pack for image/video understanding, OCR, object detection, moderation, and visual evidence extraction. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.ai.vision.v1` as required and vision service provider is registered, healthy, entitled, and policy-admissible
- **THEN** admission SHALL expose `pack.ai.vision.v1` in the effective capability set with command schemas, permission scopes, policy template, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets or raw provider payloads

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.ai.vision.v1` as required but provider, permission, entitlement, resource, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.ai.vision.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: AI Vision Pack commands SHALL use typed canonical service calls

Every `pack.ai.vision.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `vision.analyze_image` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and vision service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, or resource checks reject a `pack.ai.vision.v1` command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** the audit trail SHALL include the bounded reason code without raw user data or provider payloads

#### Scenario: Command is unsupported by the active provider
- **WHEN** a descriptor exists but the active provider does not support a requested command
- **THEN** Macaca SHALL return a typed unsupported result with descriptor and provider capability diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: AI Vision Pack SHALL expose concrete industrial metadata

`pack.ai.vision.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots, and unavailable diagnostics.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.ai.vision.v1`
- **THEN** it SHALL return the command namespace `vision.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, and compatibility metadata
- **AND** examples SHALL use generic handles or synthetic data rather than application-specific workflows

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.ai.vision.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

### Requirement: AI Vision Pack implementation SHALL preserve Macaca boundaries

The `pack.ai.vision.v1` implementation SHALL remain owned by vision service provider; the microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.ai.vision.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class and descriptor metadata rather than provider-specific business branches

### Requirement: AI Vision Pack SHALL normalize visual inputs, regions, and evidence

`pack.ai.vision.v1` SHALL expose typed visual inputs, coordinate systems, OCR spans, object detections, moderation results, evidence references, and asynchronous job records.

#### Scenario: Region coordinate system is explicit
- **WHEN** a vision command returns a region
- **THEN** each region SHALL include coordinate system, frame or page id, dimensions, rotation, scale, confidence band, and region shape
- **AND** downstream consumers SHALL NOT need provider-specific coordinate assumptions

#### Scenario: OCR preserves layout
- **WHEN** `vision.ocr` processes a page or image with multiple text regions
- **THEN** Macaca SHALL return page, block, line, and span ordering when supported
- **AND** trace/audit SHALL store only bounded text references, hashes, language hints, and confidence bands

#### Scenario: Raw media is redacted
- **WHEN** any vision command emits observability events
- **THEN** Macaca SHALL record media hashes, dimensions, duration bands, region metadata, confidence bands, and policy decisions
- **AND** raw images, video frames, credentials, provider payloads, and unbounded OCR text SHALL NOT be recorded

#### Scenario: Unsupported modality is explicit
- **WHEN** `vision.analyze_video` is invoked against a provider that only supports images
- **THEN** Macaca SHALL return a typed unsupported result
- **AND** SDK discovery SHALL mark video analysis unavailable for the current effective capability set

### Requirement: AI Vision Pack SHALL support asynchronous jobs and moderation gates

`pack.ai.vision.v1` SHALL support long-running visual analysis through job state and SHALL gate sensitive visual categories through policy.

#### Scenario: Video job is inspected
- **WHEN** `vision.inspect_job` is invoked for a visible asynchronous job
- **THEN** Macaca SHALL return job state, progress band, partial result references, cancellation support, and bounded diagnostics
- **AND** hidden frames, raw video, and provider payloads SHALL NOT be returned

#### Scenario: Video job cancellation is terminal
- **WHEN** a caller cancels a visible active vision job
- **THEN** Macaca SHALL move it to cancelled or already-terminal state with replay evidence
- **AND** late provider results SHALL be ignored or retained only as sanitized diagnostics

#### Scenario: Moderation policy denies sensitive category
- **WHEN** `vision.moderate_visual` requests a sensitive category not allowed by manifest scope, tenant policy, or entitlement
- **THEN** Macaca SHALL return denied before provider side effects
- **AND** the audit event SHALL include bounded category and policy reason codes only
