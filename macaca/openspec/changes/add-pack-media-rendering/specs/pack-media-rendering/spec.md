## ADDED Requirements

### Requirement: Macaca SHALL provide the Media Rendering Pack as a serviceized capability

Macaca SHALL provide `pack.media.rendering.v1` as a provider-neutral industrial
pack for provider inspection, render source import/opening, template inspection,
scene graph inspection, asset/font validation, raster/vector rendering,
single-frame rendering, animation rendering, deterministic previews, export,
job inspection, job cancellation, artifact management, snapshot, and replay.
The pack SHALL be declared by applications, resolved by application admission
and catalog services, and invoked only through typed service commands owned by
the rendering service provider.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.media.rendering.v1` as required and the rendering service provider is registered, healthy, entitled, permissioned, resource-admissible, asset-admissible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy templates, resource limits, approval rules, health, compatibility, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider credentials, raw templates, raw scripts, private assets, licensed fonts, raw scene graphs, raw pixels, raw vector output, raw provider payloads, or application-specific workflow metadata

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.media.rendering.v1` as required but provider registration, entitlement, permission, credential reference, resource budget, engine support, asset/font availability, GPU/CPU support, network policy, host capability, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact a concrete provider, fetch remote resources, execute scripts, render pixels, export artifacts, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.media.rendering.v1` as optional and the pack is unavailable or partially available
- **THEN** admission SHALL produce a degraded effective capability memento with unavailable commands, reason codes, provider capability hashes when safe, and remediation metadata
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands while still allowing discovery and diagnostics

### Requirement: Media Rendering Pack commands SHALL use typed canonical service calls

Every `pack.media.rendering.v1` operation SHALL be represented as a typed
`rendering.*` command/result DTO and SHALL traverse the canonical service
runtime path with trace context, policy, entitlement, resource reservation,
approval, metering, health, snapshot, structured errors, and sanitized audit
behavior.

#### Scenario: Provider inspection succeeds through service runtime
- **WHEN** a declared caller invokes `rendering.inspect_provider`
- **THEN** Macaca SHALL route the typed command through SDK/facade helpers into the service runtime and rendering service provider
- **AND** the result SHALL include bounded provider capability, command availability, engine class, raster/vector support, animation support, preview support, export support, surface formats, viewport limits, CPU/GPU classes, shader policy, script/network/asset/font support, quota class, lifecycle, health, and compatibility diagnostics
- **AND** trace and audit events SHALL contain stable trace identifiers and sanitized descriptor metadata only

#### Scenario: Side-effecting command is denied before provider invocation
- **WHEN** policy, permission, entitlement, approval, resource, version, source, asset, font, script, network, shader, GPU, render, export, or artifact checks reject a `rendering.*` command
- **THEN** Macaca SHALL return a typed denied, quota, stale-version, approval-required, unsupported, or unavailable result before invoking any concrete provider
- **AND** the audit trail SHALL include bounded reason codes without raw templates, raw scripts, private assets, licensed font contents, raw pixels, raw vector output, credentials, or provider payloads

#### Scenario: Provider does not support a command
- **WHEN** the active provider descriptor does not support a requested command such as `rendering.plan_animation` or `rendering.plan_export`
- **THEN** Macaca SHALL return a typed unsupported result with descriptor hash, provider capability hash, command name, and safe remediation diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: Media Rendering Pack SHALL expose provider-neutral DTOs and stable hashes

`pack.media.rendering.v1` SHALL define provider-neutral DTOs and deterministic
hashing for `RenderingScope`, `RenderingProviderCapability`,
`RenderSourceHandle`, `RenderTemplateMetadata`, `SceneGraphSummary`,
`RenderViewport`, `RenderSurfaceProfile`, `RenderAssetHandle`,
`RenderFontReference`, `RenderPlan`, `RenderFramePlan`,
`RenderAnimationPlan`, `RenderPreviewPlan`, `RenderExportPlan`,
`RenderJobStatus`, and `RenderArtifactHandle`. Provider-specific extensions
SHALL be bounded as adapter metadata and SHALL NOT drive OS-layer routing.

#### Scenario: Handles and hashes remain replayable
- **WHEN** Macaca records a render source, template metadata, scene summary, render plan, frame plan, animation plan, preview plan, export plan, job status, artifact handle, or service snapshot
- **THEN** it SHALL include stable descriptor, capability, source version, template, scene, viewport, surface, asset/font validation, plan, job, artifact, event cursor, and redaction hashes
- **AND** replay diagnostics SHALL be able to correlate the bounded evidence chain without reconstructing raw templates, private assets, raw pixels, raw vector output, or raw provider payloads

#### Scenario: Provider metadata is bounded
- **WHEN** a provider returns engine, surface, color profile, shader, font, asset, script, network, preview, export, GPU, or job metadata
- **THEN** the rendering service provider SHALL normalize it into provider-neutral DTO fields or bounded `adapter_metadata`
- **AND** the microkernel, SDK, shell, and generic application framework SHALL NOT branch on provider names, engine names, shader names, template names, font names, queue names, URLs, or application workflow names

### Requirement: Media Rendering Pack SHALL separate planning from side-effecting requests

Macaca SHALL require render, frame, animation, preview, and export operations to
use non-mutating plan commands before side-effecting request commands.
Side-effecting request commands SHALL require a validated plan handle,
idempotency key, source version preconditions, asset/font validation, script and
network decisions, resource reservation, approval state when required, artifact
retention policy, and audit reason.

#### Scenario: Render and frame operations use validated plans
- **WHEN** a caller needs raster/vector rendering or a deterministic single-frame render
- **THEN** it SHALL call `rendering.plan_render` or `rendering.plan_frame` before `rendering.render_request` or `rendering.frame_request`
- **AND** the plan SHALL validate source version, viewport, surface profile, output format, asset/font availability, script policy, network policy, resource budget, and approval requirements before any pixels or vector output are produced

#### Scenario: Animation and preview operations use validated plans
- **WHEN** a caller needs frame sequence rendering, animation rendering, or responsive preview snapshots
- **THEN** it SHALL call `rendering.plan_animation` or `rendering.plan_preview` before the corresponding request command
- **AND** the plan SHALL validate timeline, fps class, duration, frame count, viewport set, deterministic preview settings, cache policy, resource budget, asset/font state, and redaction policy before rendering starts

#### Scenario: Export uses a validated plan
- **WHEN** a caller needs artifact export or conversion
- **THEN** it SHALL call `rendering.plan_export` before `rendering.export_request`
- **AND** the plan SHALL validate format, dimensions, color profile, metadata retention, artifact policy, sensitivity, approval, and resource budget before producing an export artifact

### Requirement: Media Rendering Pack SHALL model asynchronous jobs and artifacts explicitly

Long-running rendering operations SHALL return explicit job and artifact handles
rather than blocking indefinitely or exposing provider-native payloads. Job and
artifact state SHALL be inspectable, cancellable where supported, resumable
through snapshots, and replayable through sanitized evidence.

#### Scenario: Request returns an asynchronous job
- **WHEN** `rendering.render_request`, `rendering.animation_request`, `rendering.preview_request`, or `rendering.export_request` is accepted as long-running
- **THEN** Macaca SHALL return `RenderJobStatus` with job handle, command name, provider capability hash, state, progress class, queue class, cancellation state, result artifact handles when available, and redaction class
- **AND** the caller SHALL inspect progress through `rendering.inspect_job` rather than polling provider-specific APIs directly

#### Scenario: Job cancellation is explicit
- **WHEN** a caller invokes `rendering.cancel_job`
- **THEN** Macaca SHALL validate job scope, cancellation policy, current job state, resource cleanup, artifact retention, and audit reason
- **AND** it SHALL return bounded cancellation diagnostics without exposing raw provider payloads

#### Scenario: Artifact handle is resolved safely
- **WHEN** a caller invokes `rendering.get_artifact_handle`
- **THEN** Macaca SHALL enforce artifact permission, retention policy, scope, redaction, entitlement, and export policy before returning bounded artifact metadata
- **AND** the result SHALL NOT include raw pixels, raw vector output, licensed font contents, raw templates, signed provider URLs beyond policy, or unbounded payloads

### Requirement: Media Rendering Pack SHALL enforce permissions, policy, resource, entitlement, and approval gates

Macaca SHALL gate `pack.media.rendering.v1` with explicit permission scopes:
`rendering.provider.inspect`, `rendering.source.import`,
`rendering.source.open`, `rendering.template.read`, `rendering.scene.read`,
`rendering.asset.validate`, `rendering.render`, `rendering.frame`,
`rendering.animation`, `rendering.preview`, `rendering.export`,
`rendering.job.read`, `rendering.job.cancel`, and
`rendering.artifact.read`. Side effects SHALL also pass script, network,
asset, font, resource, entitlement, approval, artifact, and retention policy
checks.

#### Scenario: Template and scene data are redacted by policy
- **WHEN** a caller invokes `rendering.inspect_template` or `rendering.inspect_scene_graph` for private templates, licensed assets, script-bearing sources, or restricted scene data
- **THEN** Macaca SHALL return only bounded, redacted `RenderTemplateMetadata` or `SceneGraphSummary` fields permitted by policy
- **AND** it SHALL include redaction class and reason metadata without exposing raw templates, raw scripts, private assets, licensed fonts, raw scene graphs, location metadata, or provider payloads

#### Scenario: Approval is required for sensitive side effects
- **WHEN** a request involves private templates, licensed fonts, copyrighted assets, remote URL fetching, script-enabled rendering, GPU/shader execution, external delivery, metadata stripping, or artifact publication
- **THEN** Macaca SHALL return `approval_required` or use an approved approval state before invoking the provider
- **AND** the audit evidence SHALL identify the bounded approval reason and operation hash

#### Scenario: Resource budget is insufficient
- **WHEN** source size, node count, layer count, path count, text count, image count, asset count, font count, viewport pixel count, output dimensions, frame count, animation duration, filter/effect count, shader count, CPU/GPU class, memory, storage, network transfer, timeout, provider quota, artifact size, or retained snapshot budget exceeds policy
- **THEN** Macaca SHALL reject the plan or request with a typed quota/resource result
- **AND** the concrete provider SHALL NOT be invoked for rejected side effects

### Requirement: Media Rendering Pack SHALL preserve source, asset, font, script, output, and artifact boundaries

The pack SHALL treat render sources, templates, scene graphs, assets, fonts,
scripts, remote URLs, rendered pixels, vector outputs, animation sequences,
preview artifacts, and export artifacts as scoped resources. Operations SHALL
use handles and bounded metadata across boundaries, while raw content access
remains behind provider, artifact, redaction, and policy controls.

#### Scenario: Import and open create scoped render source handles
- **WHEN** a caller invokes `rendering.import_source_request` or `rendering.open_source`
- **THEN** Macaca SHALL validate source artifact permission, format class, size policy, credential reference, script policy, network policy, asset policy, font policy, artifact policy, and redaction policy
- **AND** it SHALL return a `RenderSourceHandle` with provider scope, source artifact handle, source kind, format class, version hash, size class, asset/font/script presence classes, sensitivity class, provenance class, redaction class, and freshness

#### Scenario: Asset, font, script, or network access is denied
- **WHEN** a render plan references denied assets, denied fonts, scripts, shaders, or remote URLs
- **THEN** Macaca SHALL return `asset_denied`, `font_denied`, `script_denied`, `shader_denied`, or `network_denied` before invoking the provider
- **AND** traces, audits, snapshots, and SDK diagnostics SHALL contain only bounded handles and redaction reasons

#### Scenario: Rendered outputs remain artifact-scoped
- **WHEN** a rendering operation produces raster, vector, preview, frame sequence, animation, or export artifacts
- **THEN** Macaca SHALL represent them as `RenderArtifactHandle` values with retention, entitlement, provenance, redaction, and replay metadata
- **AND** it SHALL NOT leak raw pixels, raw vector output, signed delivery secrets, provider payloads, or unbounded output listings in observability

### Requirement: Media Rendering Pack SHALL provide sanitized trace, audit, health, snapshot, and replay evidence

`pack.media.rendering.v1` SHALL emit sanitized declaration, admission,
provider-inspection, source import/open, template inspection, scene graph
inspection, asset validation, render, frame, animation, preview, export, job,
cancel, artifact, policy, entitlement, resource, approval, health, unavailable,
failure, snapshot, and replay events. Snapshots SHALL be bounded and replayable.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.media.rendering.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, command availability, provider health, policy template hash, resource counters, bounded source/template/scene/plan/job/artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, raw templates, raw scripts, private assets, licensed fonts, raw scene graphs, raw pixels, raw vector outputs, raw provider payloads, manifests, package bytes, private keys, signatures, and unbounded rendering data

#### Scenario: Replay follows the canonical path
- **WHEN** audit replay reconstructs a `rendering.*` command chain
- **THEN** it SHALL show descriptor admission, SDK/facade service call, policy decision, resource/entitlement decision, approval state when applicable, provider dispatch, job/artifact state, and result evidence
- **AND** replay SHALL NOT require direct provider APIs, raw templates, raw assets, raw pixels, raw vector output, provider-native payloads, or shell-owned state

### Requirement: Media Rendering Pack SHALL preserve Macaca architecture boundaries

The `pack.media.rendering.v1` implementation SHALL preserve Macaca's
microkernel, service runtime, application framework, SDK, runtime-host, plugin,
and shell boundaries. Concrete rendering providers SHALL be replaceable Strategy
adapters created only by approved runtime-host composition roots. SDK helpers
SHALL only build typed service commands and SHALL NOT create providers, fetch
remote resources, execute scripts, or render output directly.

#### Scenario: Dependency gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and shell-boundary gates scan the implementation
- **THEN** they SHALL find no concrete Skia, Cairo, Canvas, WebGPU, ImageMagick, librsvg, Lottie/Skottie, Headless Chrome, font, storage, moderation, credential-manager, artifact-provider, or export adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed `rendering.*` service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable rendering provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract, unavailable behavior, health semantics, trace shape, and audit semantics
- **AND** provider-specific details SHALL appear only as sanitized descriptor/capability data, not as OS-layer routing branches

### Requirement: Media Rendering Pack SHALL include industrial developer documentation

Macaca SHALL include detailed developer documentation for
`pack.media.rendering.v1` at `docs/developer-packs/media/rendering.md` before
implementation completion. The documentation SHALL describe capability
declaration, required versus optional behavior, DTOs, commands, permissions,
policy, plan/request lifecycle, asynchronous jobs, cancellation, artifacts,
provider replacement, unavailable states, redaction, trace/audit/replay,
conformance tests, and supplier/API mapping.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/media/rendering.md`
- **THEN** the guide SHALL explain provider scopes, source handles, template metadata, scene graph summaries, viewports, surfaces, asset handles, font references, render plans, frame plans, animation plans, preview plans, export plans, job status, cancellation, artifacts, diagnostics, and operational limits
- **AND** examples SHALL use synthetic sources, templates, scene graphs, assets, fonts, viewports, jobs, and artifacts only

#### Scenario: Provider author checks conformance
- **WHEN** a provider author uses the documentation to implement a provider
- **THEN** the guide SHALL include conformance checks for descriptor completeness, DTO compatibility, command support, stable hashing, scope validation, asset/font validation, script/network enforcement, GPU/shader validation, resource bounds, approval behavior, trace/audit events, unavailable behavior, snapshot/replay, and redaction
- **AND** the guide SHALL map Skia, Cairo, Canvas 2D, WebGPU, ImageMagick, librsvg, Lottie/Skottie, Headless Chrome, font, asset, storage, and export concepts to Macaca abstractions without making supplier-specific behavior OS semantics
