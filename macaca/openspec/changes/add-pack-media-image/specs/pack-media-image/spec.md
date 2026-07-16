## ADDED Requirements

### Requirement: Macaca SHALL expose Media Image as a serviceized industrial pack

Macaca SHALL expose `pack.media.image.v1` as a provider-neutral pack for image
provider inspection, image import/open, metadata inspection, thumbnail planning,
thumbnail requests, transform planning, transform requests, compositing,
annotation, redaction, AI image generation, AI image editing/upscaling, safety
inspection, export planning, export requests, artifact handles, health,
snapshots, and replay diagnostics. The pack SHALL be declared by applications,
resolved by catalog/admission services, and invoked only through
descriptor-owned `image.*` service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.media.image.v1` as required and an image media provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy template hash, resource limits, approval rules, health metadata, compatibility metadata, and replay metadata
- **AND** SDK discovery SHALL expose callable `image.*` commands without leaking credentials, raw prompts, private images, EXIF/GPS metadata, biometric data, masks, generated image bytes, raw exports, raw provider payloads, or provider secrets

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.media.image.v1` as required but provider registration, host support, credential reference, permission, entitlement, resource, safety, codec, model, policy, or approval prerequisites are absent
- **THEN** admission SHALL block readiness with typed unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact a concrete provider, read private images, mutate images, generate outputs, export artifacts, strip metadata, publish assets, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.media.image.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability memento
- **AND** SDK helpers and WASM ABI descriptors SHALL mark unavailable commands as non-callable while preserving structured diagnostics for application recovery

### Requirement: Media Image commands SHALL use typed canonical service calls

Every `pack.media.image.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace context, policy, resource, entitlement, approval, lifecycle, health,
snapshot, structured error, and audit behavior. SDK helpers, WASM ABI handlers,
application admission, web, CLI, and frontend code SHALL only build or submit
canonical service calls and SHALL NOT call image providers directly.

#### Scenario: Provider capability is inspected
- **WHEN** `image.inspect_provider` is invoked with declared scope and trace context
- **THEN** Macaca SHALL return sanitized provider capability metadata for import/open, metadata, transform, composite, redaction, generation, edit/upscale, safety, export, formats, codecs, color/profile support, auth, quota, lifecycle, health, and compatibility support
- **AND** the result SHALL include typed unavailable, unsupported, degraded, retired, format-limited, transform-limited, composite-limited, redaction-limited, generation-limited, safety-limited, export-limited, network-limited, GPU-limited, and quota-limited states when applicable

#### Scenario: Image reads use bounded projections
- **WHEN** `image.open_image`, `image.inspect_metadata`, `image.inspect_safety`, or `image.get_artifact_handle` is invoked
- **THEN** Macaca SHALL enforce image, artifact, prompt, mask, safety, permission, resource, and redaction scopes before provider access
- **AND** results SHALL be bounded, paged, partial, or asynchronous when needed, redacted according to policy, and represented by handles and summaries rather than raw image bytes, EXIF/GPS metadata, biometric data, face embeddings, masks, generated outputs, or unbounded pixel data

#### Scenario: Unsupported command is requested
- **WHEN** a descriptor exists but the active provider does not support the requested `image.*` command, image format, codec, color profile, animation mode, transform operation, composite mode, redaction operation, generation model, edit/upscale mode, safety classifier, export format, or artifact mode
- **THEN** Macaca SHALL return a typed unsupported, format-unsupported, codec-unsupported, or generation-denied result with descriptor and capability diagnostics
- **AND** SDK discovery SHALL report the command or feature as non-callable for the current effective capability set

### Requirement: Media Image DTOs SHALL be provider-neutral and hash-stable

`pack.media.image.v1` SHALL define provider-neutral DTOs for `ImageScope`,
`ImageProviderCapability`, `ImageHandle`, `ImageMetadata`,
`ImagePixelGeometry`, `ImageColorProfile`, `ImageFrame`,
`ImageTransformOperation`, `ImageCompositeLayer`,
`ImageAnnotationOperation`, `ImageRedactionOperation`,
`ImageGenerationPlan`, `ImageEditPlan`, `ImageSafetyReport`,
`ImageExportPlan`, and `ImageArtifactHandle`. DTOs SHALL use stable handles,
version hashes, compatibility hashes, capability hashes, redaction classes,
sensitivity classes, provenance classes, event cursors, and artifact handles
rather than provider object references as OS-layer semantics.

#### Scenario: Provider-specific concepts are mapped
- **WHEN** a provider exposes ImageMagick operations, libvips pipelines, Sharp operation chains, Cloudinary transformations, OpenAI image generation/editing, Stability or Firefly generation/edit/upscale operations, Google Vision annotations, or storage artifact objects
- **THEN** the provider adapter SHALL map those concepts into Macaca provider-neutral DTOs
- **AND** provider-specific extensions SHALL appear only as bounded `adapter_metadata` protected by capability hashes and SHALL NOT drive OS-layer routing

#### Scenario: Hashes preserve compatibility and replay
- **WHEN** Macaca serializes descriptors, provider capabilities, image formats, image versions, geometries, color/profile compatibility, transform plans, composite plans, redaction plans, generation plans, edit plans, safety reports, export plans, artifact handles, event cursors, and redaction profiles
- **THEN** it SHALL produce stable hashes suitable for compatibility checks, stale-version detection, safety diagnostics, audit correlation, and replay diagnostics
- **AND** schema evolution tests SHALL prove older compatible snapshots remain readable or return typed schema-mismatch diagnostics

### Requirement: Media Image side effects SHALL use plan/request separation

Macaca SHALL split thumbnailing, transforming, compositing, redacting,
generating, editing, upscaling, exporting, and other side-effecting image
operations into non-mutating plan commands and side-effecting request commands.
Plan commands SHALL validate image versions, artifact scopes, format/codec
support, prompt/mask handles, safety policy, metadata policy, resource budgets,
approvals, and idempotency before request commands can perform side effects.

#### Scenario: Transform plan validates before mutation
- **WHEN** `image.plan_transform` receives resize, crop, rotate, flip, convert, color/profile, optimization, or frame operations
- **THEN** Macaca SHALL validate operation schema, target handles, image version hash, geometry compatibility, format/codec support, color/profile compatibility, metadata retention policy, provider support, resource budget, and required approvals
- **AND** it SHALL return validation diagnostics without mutating the image, stripping metadata, exporting artifacts, or contacting external delivery systems for side effects

#### Scenario: Composite or redaction plan validates before mutation
- **WHEN** `image.plan_composite` or `image.plan_redaction` receives overlays, watermarks, masks, drawing, labels, blur, pixelation, crop, or metadata-redaction operations
- **THEN** Macaca SHALL validate layer handles, mask handles, target regions, coordinate systems, font/style references, sensitive classes, preview policy, safety policy, resource budget, and approvals
- **AND** it SHALL return plan diagnostics without publishing or modifying source artifacts

#### Scenario: Generation or edit request executes idempotently
- **WHEN** `image.generation_request` or `image.edit_request` is invoked with a valid plan handle, prompt handle, mask handle when required, model capability hash, safety state, idempotency key, trace context, and sufficient permissions
- **THEN** Macaca SHALL execute through the image media service provider and return typed success, safety-denied, prompt-denied, generation-denied, stale-version, conflict, approval-required, quota, timeout, cancellation, or failure results
- **AND** repeated requests with the same idempotency key SHALL NOT duplicate generated or edited outputs

#### Scenario: Export request returns artifact handles only
- **WHEN** `image.export_request`, `image.thumbnail_request`, `image.transform_request`, `image.composite_request`, or `image.redaction_request` produces a derived output
- **THEN** Macaca SHALL return bounded `ImageArtifactHandle` results with provenance and redaction metadata
- **AND** raw image bytes, raw generated outputs, rendered previews, masks, and raw exports SHALL remain in artifact boundaries and SHALL NOT enter trace, audit, snapshots, SDK diagnostics, or examples

### Requirement: Media Image SHALL enforce permission, policy, resource, entitlement, and approval gates

Every `image.*` command SHALL be scoped to application id, tenant id, session
id, task id, trace id, provider scope, image handle, artifact handle when
applicable, actor handle when available, credential reference, network policy,
artifact policy, safety policy, and permission state. Side-effecting commands
SHALL run policy, resource, entitlement, approval, version, safety, metadata,
and idempotency checks before concrete provider calls.

#### Scenario: Permission is denied before provider access
- **WHEN** an application lacks `image.provider.inspect`, `image.import`, `image.open`, `image.metadata.read`, `image.thumbnail`, `image.transform`, `image.composite`, `image.redaction`, `image.generate`, `image.edit`, `image.safety.read`, `image.export`, or `image.artifact.read`
- **THEN** Macaca SHALL return a typed denied result before invoking any provider
- **AND** audit evidence SHALL include bounded reason codes and sanitized scope handles only

#### Scenario: Sensitive operation requires approval
- **WHEN** a command touches private images, faces, biometric signals, minors, identity documents, medical or financial images, screenshots, copyrighted media, raw prompts, masks, generated content, external delivery, metadata stripping, destructive redaction, or operations that publish artifacts
- **THEN** Macaca SHALL require approval when policy marks the operation approval-gated
- **AND** denial, expiration, or missing approval SHALL return typed approval-required diagnostics without side effects

#### Scenario: Resource or entitlement is unavailable
- **WHEN** image size, pixel count, frame count, transform count, layer count, mask size, prompt size, generated output count, render/export size, artifact size, provider quota, network transfer, timeout, CPU/GPU class, memory, storage, streaming output, retained snapshots, entitlement, model access, codec support, or host support is insufficient
- **THEN** Macaca SHALL return typed quota, unavailable, denied, timeout, cancellation, GPU-unavailable, or host-resource diagnostics
- **AND** the provider SHALL NOT be called for side-effecting operations after a failed gate

### Requirement: Media Image artifacts, metadata, prompts, and safety outputs SHALL be bounded and redacted

`pack.media.image.v1` SHALL treat raw images, EXIF/GPS metadata, private
prompts, masks, faces, biometric signals, generated image outputs, safety
reports, exported images, and derived artifacts as sensitive data. The pack
SHALL expose handles, bounded summaries, cursors, redaction classes,
provenance classes, retention metadata, and replay pointers rather than raw
sensitive payloads in observability surfaces.

#### Scenario: Metadata is inspected
- **WHEN** `image.inspect_metadata` is invoked with sufficient permission
- **THEN** Macaca SHALL return bounded metadata classes for dimensions, orientation, format, color profile, alpha state, animation state, checksum handle, provenance handles, and EXIF/GPS presence
- **AND** raw EXIF/GPS values, embedded thumbnails, private file paths, camera serials, and unbounded metadata blobs SHALL NOT enter traces, audits, snapshots, or SDK diagnostics

#### Scenario: Safety report is inspected
- **WHEN** `image.inspect_safety` is invoked with sufficient permission
- **THEN** Macaca SHALL return safety classes, sensitive content classes, crop hints, label/property handles, confidence classes, redaction class, and provider capability hash
- **AND** raw biometric outputs, face embeddings, identity inferences, raw OCR text, and provider-specific safety payloads SHALL NOT become OS-layer semantics

#### Scenario: Artifact metadata is inspected
- **WHEN** `image.get_artifact_handle` resolves an image, thumbnail, generated, edited, redacted, transformed, or exported artifact
- **THEN** Macaca SHALL return artifact kind, source operation handle, content type, dimensions class, size class, checksum handle, retention state, provenance, sensitivity class, and redaction class
- **AND** raw artifact bytes SHALL remain behind artifact boundaries

### Requirement: Media Image SHALL preserve Macaca architecture boundaries

The Media Image pack implementation SHALL preserve the microkernel, service
runtime, SDK/SystemFacade, application framework, runtime-host, plugin, and
shell boundaries defined by Macaca governance. Concrete image providers SHALL
be replaceable Strategy adapters created only in approved runtime-host or
plugin composition roots.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, serviceization, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete ImageMagick, libvips, Sharp, Cloudinary, OpenAI, Stability, Firefly, Vision, storage, moderation, credential, or artifact provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.media.image.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract, permission model, trace/audit schema, snapshot shape, and structured unavailable behavior
- **AND** OS layers SHALL NOT branch on provider names, model names, image names, layer names, prompt templates, application names, or workflow names

### Requirement: Media Image SHALL emit sanitized trace, audit, health, snapshot, and replay evidence

`pack.media.image.v1` SHALL emit sanitized declaration, admission,
provider-inspection, import/open, metadata-inspection, thumbnail, transform,
composite, redaction, generation, edit, safety, export, artifact-handle,
policy, entitlement, resource, approval, health, snapshot, unavailable, and
failure events. Snapshots SHALL contain enough bounded metadata to diagnose and
replay service behavior without storing raw sensitive content.

#### Scenario: Service call evidence is recorded
- **WHEN** any `image.*` command is submitted
- **THEN** Macaca SHALL record trace-required service-call evidence with command name, descriptor version, sanitized scope handles, policy decision, resource decision, provider capability hash, result class, and replay pointer
- **AND** the evidence SHALL exclude raw credentials, raw prompts, private images, EXIF/GPS metadata, biometric data, face embeddings, masks, generated image bytes, raw exports, raw provider payloads, manifests, package bytes, private keys, signatures, and unbounded pixel data

#### Scenario: Snapshot supports recovery diagnostics
- **WHEN** the service runtime records an image snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, image format and version hashes, command availability, provider health, policy template hash, resource counters, bounded metadata/operation/safety/artifact summaries, event cursors, and sanitized replay pointers
- **AND** replay tests SHALL prove every `image.*` command can be correlated through the canonical service path after restart

### Requirement: Media Image SHALL provide industrial developer documentation

The implementation SHALL include a detailed developer guide at
`docs/developer-packs/media/image.md` before `pack.media.image.v1` is marked
complete. The guide SHALL be linked from SDK discovery metadata and the
industrial pack catalog index.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/media/image.md`
- **THEN** the guide SHALL explain purpose, manifest declaration, required versus optional behavior, permissions, provider scopes, image handles, metadata, EXIF/GPS handling, color profiles, frames, transform operations, composite layers, annotations, redactions, generation plans, edit/upscale plans, safety reports, export plans, artifacts, unavailable diagnostics, provider replacement, operational limits, and conformance expectations
- **AND** it SHALL document every command DTO and result DTO with field-level behavior, idempotency, redaction, pagination, streaming/asynchronous artifact behavior, timeout, cancellation, approval, artifact retention, image version preconditions, format/codec compatibility, metadata stripping, prompt/mask safety, generated-content provenance, content safety, structured errors, and trace/audit interpretation

#### Scenario: Supplier mapping is documented
- **WHEN** the documentation describes supplier/API mapping
- **THEN** it SHALL map ImageMagick operations, libvips pipelines, Sharp operation chains, Cloudinary transformations, OpenAI Images, Stability AI, Adobe Firefly, Google Vision annotations, storage, safety, and export concepts to Macaca abstractions
- **AND** it SHALL explicitly document what is intentionally not exposed as OS semantics

#### Scenario: Examples are provided
- **WHEN** the guide provides examples
- **THEN** examples SHALL use only synthetic images, prompts, masks, safety reports, generated artifacts, exported artifacts, and unavailable diagnostics
- **AND** examples SHALL NOT include provider names, real credentials, private images, face or biometric data, EXIF/GPS data, raw prompts, raw generated images, raw exports, or workflow-specific conventions
