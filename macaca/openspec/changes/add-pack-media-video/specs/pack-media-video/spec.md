## ADDED Requirements

### Requirement: Macaca SHALL provide the Media Video Pack as a serviceized capability

Macaca SHALL provide `pack.media.video.v1` as a provider-neutral industrial pack
for video provider inspection, import/opening, metadata and track inspection,
thumbnail/proxy/frame extraction, transcoding, segmentation, rendering,
subtitle/caption handling, adaptive packaging, export, artifact management, job
inspection, snapshot, and replay. The pack SHALL be declared by applications,
resolved by application admission and catalog services, and invoked only through
typed service commands owned by the video media service provider.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.media.video.v1` as required and the video media service provider is registered, healthy, entitled, permissioned, resource-admissible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy templates, resource limits, approval rules, health, compatibility, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider credentials, raw video bytes, raw frames, raw subtitles, raw exports, raw provider payloads, or application-specific workflow metadata

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.media.video.v1` as required but provider registration, entitlement, permission, credential reference, resource budget, codec/container support, network policy, host capability, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact a concrete provider, process video, export artifacts, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.media.video.v1` as optional and the pack is unavailable or partially available
- **THEN** admission SHALL produce a degraded effective capability memento with unavailable commands, reason codes, provider capability hashes when safe, and remediation metadata
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands while still allowing discovery and diagnostics

### Requirement: Media Video Pack commands SHALL use typed canonical service calls

Every `pack.media.video.v1` operation SHALL be represented as a typed
`video.*` command/result DTO and SHALL traverse the canonical service runtime
path with trace context, policy, entitlement, resource reservation, approval,
metering, health, snapshot, structured errors, and sanitized audit behavior.

#### Scenario: Provider inspection succeeds through service runtime
- **WHEN** a declared caller invokes `video.inspect_provider`
- **THEN** Macaca SHALL route the typed command through SDK/facade helpers into the service runtime and video media service provider
- **AND** the result SHALL include bounded provider capability, command availability, codec/container classes, track support, render support, package support, export support, quota class, lifecycle, health, and compatibility diagnostics
- **AND** trace and audit events SHALL contain stable trace identifiers and sanitized descriptor metadata only

#### Scenario: Side-effecting command is denied before provider invocation
- **WHEN** policy, permission, entitlement, approval, resource, version, track, subtitle, package, export, or artifact checks reject a `video.*` command
- **THEN** Macaca SHALL return a typed denied, quota, stale-version, approval-required, unsupported, or unavailable result before invoking any concrete provider
- **AND** the audit trail SHALL include bounded reason codes without raw video, raw frame, subtitle PII, voice biometric data, faces, raw exports, credentials, or provider payloads

#### Scenario: Provider does not support a command
- **WHEN** the active provider descriptor does not support a requested command such as `video.plan_package` or `video.render_request`
- **THEN** Macaca SHALL return a typed unsupported result with descriptor hash, provider capability hash, command name, and safe remediation diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: Media Video Pack SHALL expose provider-neutral DTOs and stable hashes

`pack.media.video.v1` SHALL define provider-neutral DTOs and deterministic
hashing for `VideoScope`, `VideoProviderCapability`, `VideoHandle`,
`VideoMetadata`, `VideoTrack`, `VideoFrameHandle`, `VideoTimelineRange`,
`VideoThumbnailPlan`, `VideoTranscodePlan`, `VideoSegmentPlan`,
`VideoRenderPlan`, `VideoSubtitlePlan`, `VideoPackagePlan`, `VideoExportPlan`,
`VideoOverlayOperation`, `VideoJobStatus`, and `VideoArtifactHandle`.
Provider-specific extensions SHALL be bounded as adapter metadata and SHALL NOT
drive OS-layer routing.

#### Scenario: Handles and hashes remain replayable
- **WHEN** Macaca records a video operation plan, job status, artifact handle, or service snapshot
- **THEN** it SHALL include stable descriptor, capability, video version, track mapping, timeline range, plan, job, artifact, event cursor, and redaction hashes
- **AND** replay diagnostics SHALL be able to correlate the bounded evidence chain without reconstructing private video content or raw provider payloads

#### Scenario: Provider metadata is bounded
- **WHEN** a provider returns codec, container, stream, render, subtitle, packaging, or delivery metadata
- **THEN** the video service provider SHALL normalize it into provider-neutral DTO fields or bounded `adapter_metadata`
- **AND** the microkernel, SDK, shell, and generic application framework SHALL NOT branch on provider names, codec names, preset names, queue names, file names, or application workflow names

### Requirement: Media Video Pack SHALL separate planning from side-effecting requests

Macaca SHALL require thumbnail, frame extraction, transcode, segment, render,
subtitle, package, and export operations to use non-mutating plan commands
before side-effecting request commands. Side-effecting request commands SHALL
require a validated plan handle, idempotency key, video version preconditions,
resource reservation, approval state when required, artifact retention policy,
and audit reason.

#### Scenario: Thumbnail and frame extraction uses a validated plan
- **WHEN** a caller needs thumbnails, poster frames, proxy frames, or frame extraction
- **THEN** it SHALL call `video.plan_thumbnail` before `video.thumbnail_request`
- **AND** the plan SHALL validate timestamp or frame ranges, output profile, metadata retention, safety policy, artifact policy, and resource estimate before any frame is extracted

#### Scenario: Transcode uses a validated plan
- **WHEN** a caller needs codec, container, resolution, bitrate, frame-rate, or rendition conversion
- **THEN** it SHALL call `video.plan_transcode` before `video.transcode_request`
- **AND** the plan SHALL validate codec/container support, track mapping, quality profile, version preconditions, resource budget, output policy, and approval requirements before any transcode job starts

#### Scenario: Segment and render use validated plans
- **WHEN** a caller needs trim, split, concatenate, filter, overlay, watermark, burn-in subtitle, or timeline composition behavior
- **THEN** it SHALL call `video.plan_segment` or `video.plan_render` before `video.segment_request` or `video.render_request`
- **AND** the plan SHALL validate timeline ranges, track selection, render graph, overlay sources, fonts, styles, safety state, artifact policy, and resource budget without mutating or publishing video

#### Scenario: Subtitle, package, and export use validated plans
- **WHEN** a caller needs subtitle conversion, sidecar captions, burn-in captions, HLS/DASH-like packaging, delivery artifacts, or exported video
- **THEN** it SHALL call `video.plan_subtitles`, `video.plan_package`, or `video.plan_export` before the corresponding request command
- **AND** the plan SHALL validate subtitle format, language, redaction, rendition ladder, package manifests, entitlement references, metadata stripping, provenance, delivery policy, retention, approval, and resource budget before producing artifacts

### Requirement: Media Video Pack SHALL model asynchronous jobs and artifacts explicitly

Long-running video commands SHALL return explicit job and artifact handles rather
than blocking indefinitely or exposing provider-native job payloads. Job and
artifact state SHALL be inspectable, cancellable where supported, resumable
through snapshots, and replayable through sanitized evidence.

#### Scenario: Request returns an asynchronous job
- **WHEN** `video.transcode_request`, `video.render_request`, `video.package_request`, or another long-running request is accepted
- **THEN** Macaca SHALL return `VideoJobStatus` with job handle, command name, provider capability hash, state, progress class, queue class, cancellation state, result artifact handles when available, and redaction class
- **AND** the caller SHALL inspect progress through `video.inspect_job` rather than polling provider-specific APIs directly

#### Scenario: Artifact handle is resolved safely
- **WHEN** a caller invokes `video.get_artifact_handle`
- **THEN** Macaca SHALL enforce artifact permission, retention policy, scope, redaction, entitlement, and export policy before returning bounded artifact metadata
- **AND** the result SHALL NOT include raw video bytes, raw frames, raw subtitles, raw exports, signed provider URLs beyond policy, or unbounded payloads

#### Scenario: Job fails or times out
- **WHEN** a video job fails, times out, is cancelled, exceeds quota, loses provider availability, or violates resource policy
- **THEN** Macaca SHALL return typed timeout, cancellation, quota, unavailable, or failure diagnostics with replay pointers
- **AND** the service SHALL preserve enough bounded state for recovery diagnostics without leaking provider-native payloads

### Requirement: Media Video Pack SHALL enforce permissions, policy, resource, entitlement, and approval gates

Macaca SHALL gate `pack.media.video.v1` with explicit permission scopes:
`video.provider.inspect`, `video.import`, `video.open`,
`video.metadata.read`, `video.track.read`, `video.thumbnail`,
`video.transcode`, `video.segment`, `video.render`, `video.subtitle`,
`video.package`, `video.export`, `video.job.read`, and
`video.artifact.read`. Side effects SHALL also pass resource, entitlement,
approval, safety, metadata, artifact, network, and retention policy checks.

#### Scenario: Metadata and tracks are redacted by policy
- **WHEN** a caller invokes `video.inspect_metadata` or `video.inspect_tracks` for private videos, sensitive tracks, subtitles containing PII, or restricted provenance
- **THEN** Macaca SHALL return only bounded, redacted `VideoMetadata` or `VideoTrack` fields permitted by policy
- **AND** it SHALL include redaction class and reason metadata without exposing raw frames, faces, voice biometric features, raw subtitles, location metadata, or provider payloads

#### Scenario: Approval is required for sensitive side effects
- **WHEN** a request involves private videos, faces, voice, minors, legal/medical/financial recordings, copyrighted media, subtitle PII, generated or edited content, destructive edits, external delivery, package publication, metadata stripping, or artifact publication
- **THEN** Macaca SHALL return `approval_required` or use an approved approval state before invoking the provider
- **AND** the audit evidence SHALL identify the bounded approval reason and operation hash

#### Scenario: Resource budget is insufficient
- **WHEN** duration, dimensions, frame count, track count, subtitle size, filter count, overlay count, rendition count, CPU/GPU class, memory, storage, network transfer, timeout, provider quota, or retained snapshot budget exceeds policy
- **THEN** Macaca SHALL reject the plan or request with a typed quota/resource result
- **AND** the concrete provider SHALL NOT be invoked for rejected side effects

### Requirement: Media Video Pack SHALL preserve artifact, track, subtitle, package, and private-video boundaries

The pack SHALL treat source videos, tracks, frames, subtitles, generated or
edited videos, package manifests, and derived artifacts as scoped resources.
Operations SHALL use handles and bounded metadata across boundaries, while raw
content access remains behind provider, artifact, redaction, and policy
controls.

#### Scenario: Import and open create scoped handles
- **WHEN** a caller invokes `video.import_video_request` or `video.open_video`
- **THEN** Macaca SHALL validate source artifact permission, format class, size/duration policy, credential reference, artifact policy, and safety policy
- **AND** it SHALL return a `VideoHandle` with provider scope, source artifact handle, container/codec summary, version hash, duration/dimension/frame-rate classes, track count class, sensitivity class, provenance class, redaction class, and freshness

#### Scenario: Track or subtitle access is denied
- **WHEN** a caller requests a denied audio, video, subtitle, data, chapter, or metadata track
- **THEN** Macaca SHALL return `track_denied` or `subtitle_denied` before exposing the restricted track content
- **AND** traces, audits, snapshots, and SDK diagnostics SHALL contain only bounded track handles and redaction reasons

#### Scenario: Package manifests remain artifact-scoped
- **WHEN** a packaging operation produces adaptive playback manifests or rendition artifacts
- **THEN** Macaca SHALL represent them as `VideoArtifactHandle` values with retention, entitlement, provenance, redaction, and replay metadata
- **AND** it SHALL NOT leak raw package manifests, signed delivery secrets, provider payloads, or unbounded rendition listings in observability

### Requirement: Media Video Pack SHALL provide sanitized trace, audit, health, snapshot, and replay evidence

`pack.media.video.v1` SHALL emit sanitized declaration, admission,
provider-inspection, import/open, metadata, track, thumbnail, transcode,
segment, render, subtitle, package, export, job, artifact, policy, entitlement,
resource, approval, health, unavailable, failure, snapshot, and replay events.
Snapshots SHALL be bounded and replayable.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.media.video.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, command availability, provider health, policy template hash, resource counters, bounded video/track/operation/job/artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, private videos, raw frames, faces, voice biometric features, subtitle PII, generated or edited video bytes, raw exports, raw provider payloads, manifests, package bytes, private keys, signatures, and unbounded frame/pixel data

#### Scenario: Replay follows the canonical path
- **WHEN** audit replay reconstructs a `video.*` command chain
- **THEN** it SHALL show descriptor admission, SDK/facade service call, policy decision, resource/entitlement decision, approval state when applicable, provider dispatch, job/artifact state, and result evidence
- **AND** replay SHALL NOT require direct provider APIs, raw video content, provider-native payloads, or shell-owned state

### Requirement: Media Video Pack SHALL preserve Macaca architecture boundaries

The `pack.media.video.v1` implementation SHALL preserve Macaca's microkernel,
service runtime, application framework, SDK, runtime-host, plugin, and shell
boundaries. Concrete video providers SHALL be replaceable Strategy adapters
created only by approved runtime-host composition roots. SDK helpers SHALL only
build typed service commands and SHALL NOT create providers or access private
video data directly.

#### Scenario: Dependency gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and shell-boundary gates scan the implementation
- **THEN** they SHALL find no concrete FFmpeg, GStreamer, WebCodecs, MediaConvert, Cloudinary, Mux, storage, moderation, credential-manager, artifact-provider, or export-provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed `video.*` service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable video provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract, unavailable behavior, health semantics, trace shape, and audit semantics
- **AND** provider-specific details SHALL appear only as sanitized descriptor/capability data, not as OS-layer routing branches

### Requirement: Media Video Pack SHALL include industrial developer documentation

Macaca SHALL include detailed developer documentation for
`pack.media.video.v1` at `docs/developer-packs/media/video.md` before
implementation completion. The documentation SHALL describe capability
declaration, required versus optional behavior, DTOs, commands, permissions,
policy, plan/request lifecycle, asynchronous jobs, artifacts, provider
replacement, unavailable states, redaction, trace/audit/replay, conformance
tests, and supplier/API mapping.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/media/video.md`
- **THEN** the guide SHALL explain provider scopes, video handles, metadata, tracks, codecs, containers, frame rates, dimensions, frames, timeline ranges, thumbnail plans, transcode plans, segment plans, render graphs, subtitle plans, package plans, export plans, job status, artifacts, diagnostics, and operational limits
- **AND** examples SHALL use synthetic videos, tracks, subtitles, jobs, frames, and artifacts only

#### Scenario: Provider author checks conformance
- **WHEN** a provider author uses the documentation to implement a provider
- **THEN** the guide SHALL include conformance checks for descriptor completeness, DTO compatibility, command support, stable hashing, scope validation, policy hooks, resource bounds, approval behavior, trace/audit events, unavailable behavior, snapshot/replay, and redaction
- **AND** the guide SHALL map FFmpeg, GStreamer, WebCodecs, AWS MediaConvert, Cloudinary, Mux, storage, safety, and export concepts to Macaca abstractions without making supplier-specific behavior OS semantics
