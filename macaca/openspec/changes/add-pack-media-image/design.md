# Media Image Pack Design

## Context

`pack.media.image.v1` exposes image operations as a Macaca OS serviceized
capability. It lets applications inspect, transform, compose, annotate, redact,
generate, edit, upscale, moderate, export, and replay image work without
embedding ImageMagick, libvips, Sharp, Cloudinary, OpenAI Images, Stability AI,
Adobe Firefly, Google Vision, storage, moderation, or application-specific
image workflows into generic OS layers.

Images can be source evidence, personal media, generated assets, or delivery
artifacts. The pack therefore treats raw image bytes, EXIF/GPS metadata, faces,
biometric signals, prompts, masks, generated outputs, and derived artifacts as
sensitive data. Reads return bounded metadata and handles; side effects use
validated plans and idempotent requests.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| ImageMagick | Identify, resize, crop, rotate, convert, draw, composite, colorspace/profile operations, broad format support | Metadata, transform operations, annotation/composition plans, format capability |
| libvips | High-performance streaming/demand-driven image processing, metadata, ICC/profile handling, resize/crop/composite, encoders | Resource-aware transform Strategy, color/profile DTOs, artifact output |
| Sharp | Node/libvips metadata, resize/crop/rotate/composite, colorspace, output formats | SDK-provider adapter baseline, operation DTOs, output plan |
| Cloudinary | URL/API transformations, resize/crop, overlays, effects, format/quality optimization, derived asset delivery | Remote transform/export provider, derived artifact handle, delivery policy |
| OpenAI Images | Text-to-image generation and image editing with model/size/quality/output controls | Generation/edit plan, prompt safety handle, generated artifact provenance |
| Stability AI / Adobe Firefly | Text/image generation, image-to-image, fill/expand, upscale, creative edits | Alternative generation/edit/upscale Strategy, model capability metadata |
| Google Cloud Vision | SafeSearch, crop hints, labels, OCR-like text detection, face/object/image property annotation | Safety report, image property DTOs, crop hint metadata, optional analysis provider |

The pack exposes provider-neutral contracts. Provider adapters translate to
local libraries, remote transformation APIs, AI generation services, safety
classifiers, storage/artifact providers, or unavailable providers. OS layers
must not branch on provider names, image names, model names, prompt templates,
layer names, brand workflows, or business image workflows.

## Goals

- Provide stable pack id `pack.media.image.v1` and command namespace `image.*`.
- Support provider inspection, image import/open, metadata inspection, geometry/
  color/profile/frame inspection, thumbnail/render planning and requests,
  transform planning and requests, composition/annotation/redaction planning,
  AI generation/edit/upscale planning and requests, safety/moderation
  inspection, export planning/requests, artifact handles, snapshots, health,
  and replay diagnostics.
- Preserve safety with image/artifact scopes, EXIF/GPS stripping, biometric and
  sensitive-image policy, content safety, generated-content provenance,
  prompt/mask redaction, approvals, quotas, bounded output, and sanitized audit.
- Keep concrete image providers behind replaceable service providers.
- Require developer documentation at `docs/developer-packs/media/image.md`.

## Non-Goals

- Do not implement concrete ImageMagick, libvips, Sharp, Cloudinary, OpenAI,
  Stability, Firefly, Vision, OCR, storage, moderation, or export providers in
  this proposal.
- Do not define photo editor, digital asset management, avatar, OCR, ID
  verification, design tool, social media, marketing, e-commerce, or template
  workflows.
- Do not expose raw credentials, raw prompts, private images, EXIF/GPS metadata,
  biometric data, face embeddings, masks, generated image bytes, raw provider
  payloads, prompts, manifests, package bytes, private keys, signatures, or
  unbounded pixel data in observability.
- Do not silently generate, edit, redact, publish, export, watermark, strip
  metadata, or transmit images without typed plan/request, policy checks,
  content safety, version preconditions, and approval where required.

## Ownership And Boundaries

- Pack id: `pack.media.image.v1`.
- Family: `media`.
- Backing service owner: image media service provider.
- SDK surface: `sdk.packs.media.image`.
- Command namespace: `image.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges, artifact
  stores, safety/moderation bridges, decorators, and sanitized diagnostics
  through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `image.inspect_provider` | Inspect provider, codec, transform, generation, safety, and export support | Returns sanitized capability, quota, lifecycle, health, and compatibility metadata |
| `image.import_image_request` | Import image from file/artifact handle | Requires artifact permission, format validation, size policy, metadata policy, and audit |
| `image.open_image` | Resolve image handle and version metadata | Requires image scope and bounded metadata |
| `image.inspect_metadata` | Inspect EXIF/GPS, dimensions, format, color, profile, alpha, animation, and provenance metadata | Requires metadata permission and redaction |
| `image.plan_thumbnail` | Plan thumbnail/render derivative | Validates dimensions, crop mode, metadata stripping, retention, and resources |
| `image.thumbnail_request` | Execute thumbnail/render derivative | Requires plan handle, idempotency key, artifact policy, and audit |
| `image.plan_transform` | Plan resize, crop, rotate, flip, convert, color/profile, and optimization operations | Validates operations, versions, codecs, resources, and approvals |
| `image.transform_request` | Execute a validated transform plan | Requires plan handle, idempotency key, version preconditions, and audit |
| `image.plan_composite` | Plan overlays, watermarks, masks, drawing, labels, and annotations | Validates layer handles, coordinates, fonts, safety, and output policy |
| `image.composite_request` | Execute a validated composite plan | Returns bounded artifact handles and diagnostics |
| `image.plan_redaction` | Plan blur/pixelate/mask/crop/metadata redaction | Validates targets, preview policy, sensitive classes, and approval |
| `image.redaction_request` | Execute a validated redaction plan | Requires plan handle, idempotency key, version preconditions, and audit |
| `image.plan_generation` | Plan text-to-image generation | Validates prompt handle, model capability, safety, output policy, and approvals |
| `image.generation_request` | Execute generation | Returns generated artifact handle and provenance metadata |
| `image.plan_edit` | Plan image-to-image edit, inpaint, outpaint, variation, or upscale | Validates image/mask handles, prompt handle, model support, safety, and resources |
| `image.edit_request` | Execute AI edit/upscale/variation | Returns artifact handle and provenance metadata |
| `image.inspect_safety` | Inspect moderation/safety/property/crop-hint metadata where supported | Requires redaction and no raw biometric output |
| `image.plan_export` | Plan image export or delivery artifact | Validates format, quality, metadata retention, sensitivity, and approvals |
| `image.export_request` | Execute export/delivery | Returns bounded artifact handle and audit metadata |
| `image.get_artifact_handle` | Resolve image/thumbnail/export/generated artifact metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
paged/partial/asynchronous results, typed denied/unavailable/unsupported/
conflict/stale-version/schema-mismatch/format-unsupported/codec-unsupported/
metadata-denied/safety-denied/prompt-denied/generation-denied/redaction-denied/
export-denied/write-denied/artifact-denied/quota/timeout/cancellation/
approval-required/failure results, redaction profile, idempotency semantics for
side effects, and replay metadata.

## DTO Model

Core DTOs:

- `ImageScope`: provider scope handle, image handle, source artifact handle,
  credential reference, network policy, artifact policy, safety policy,
  permission state, rate-limit profile, and health.
- `ImageProviderCapability`: provider class, import/open support, metadata
  support, transform support, composite support, redaction support, generation
  support, edit/upscale support, safety support, export support, formats,
  color/profile support, auth modes, rate limits, lifecycle, and health.
- `ImageHandle`: image handle, provider scope, source artifact handle, format,
  version hash, dimensions class, frame count class, sensitivity class,
  provenance class, redaction class, and freshness.
- `ImageMetadata`: dimensions, orientation, EXIF/GPS presence, color profile,
  color space, alpha state, animation state, embedded thumbnail state,
  provenance handles, checksum handle, and redaction class.
- `ImagePixelGeometry`: width/height class, aspect ratio class, region handle,
  focal/crop hint handles, orientation, and coordinate space.
- `ImageColorProfile`: profile handle, color space, bit depth class, ICC handle,
  conversion compatibility hash, and redaction class.
- `ImageFrame`: frame handle, index, duration class, disposal mode class,
  dimensions class, and redaction class.
- `ImageTransformOperation`: operation handle, operation kind, target image/
  region/profile handle, parameters handle, compatibility hash, and validation
  metadata.
- `ImageCompositeLayer`: layer handle, source artifact handle, mask handle,
  bounds class, opacity class, blend mode, font/style references, and redaction
  class.
- `ImageAnnotationOperation`: operation handle, annotation kind, target region,
  payload handle, style references, and validation metadata.
- `ImageRedactionOperation`: operation handle, target region/metadata/safety
  class, method, preview artifact handle, reason code, and validation metadata.
- `ImageGenerationPlan`: plan handle, prompt handle, model capability hash,
  output profile, safety policy, provenance policy, required approvals,
  idempotency key, and validation diagnostics.
- `ImageEditPlan`: plan handle, source image handle, mask handle, prompt handle,
  edit kind, model capability hash, safety policy, resource estimate,
  idempotency key, and validation diagnostics.
- `ImageSafetyReport`: report handle, image handle, safety classes, sensitive
  content classes, crop hints, label/property handles, confidence classes,
  redaction class, and provider capability hash.
- `ImageExportPlan`: plan handle, source image/artifact handle, output format,
  quality class, metadata retention policy, delivery policy, retention,
  redaction, required approvals, idempotency key, and validation diagnostics.
- `ImageArtifactHandle`: artifact handle, source image/operation handle,
  artifact kind, content type, dimensions class, size class, checksum handle,
  retention, provenance, redaction class, and replay pointer.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `image.provider.inspect`
- `image.import`
- `image.open`
- `image.metadata.read`
- `image.thumbnail`
- `image.transform`
- `image.composite`
- `image.redaction`
- `image.generate`
- `image.edit`
- `image.safety.read`
- `image.export`
- `image.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, image handle, artifact handle when applicable,
  actor handle when available, credential reference, network policy, artifact
  policy, and safety policy.
- Side-effecting transform, composite, redaction, generation, edit, upscale, and
  export commands require plan/request separation, idempotency key, version
  preconditions, metadata retention policy, safety policy, artifact policy, and
  audit reason.
- Private images, faces, biometric signals, minors, identity documents, medical
  or financial images, screenshots, copyrighted media, raw prompts, masks,
  generated content, external delivery, metadata stripping, and destructive
  redaction may require approval.
- EXIF/GPS metadata, prompts, masks, private images, generated image bytes,
  derived artifacts, safety outputs, and provider payloads require redaction and
  bounded output. Raw pixel data must not enter observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, content safety checks, and structured unavailable
  behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
format support, codec support, metadata support, transform support, composite
support, redaction support, generation support, edit/upscale support, safety
support, export support, permission scopes, policy templates, resource limits,
approval rules, provider capability hashes, health, compatibility, diagnostics,
examples, redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/media/image.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, image handles, metadata, EXIF/GPS handling, color profiles,
  frames, transform operations, composite layers, annotations, redactions,
  generation plans, edit/upscale plans, safety reports, export plans, artifacts,
  provider capabilities, and unavailable states
- plan/request lifecycle, version conflicts, codec/format mismatch, metadata
  stripping, prompt/mask redaction, generated-content provenance, content safety,
  approvals, quotas, provider replacement, trace/audit interpretation, and
  conformance tests

Examples must use synthetic images, prompts, masks, safety reports, and
artifacts. They must not include provider names, real credentials, private
images, face/biometric data, EXIF/GPS data, raw prompts, raw generated images,
raw exports, or workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `image_pack_declared`
- `image_pack_admission_validated`
- `image_provider_inspected`
- `image_imported`
- `image_opened`
- `image_metadata_inspected`
- `image_thumbnail_planned`
- `image_thumbnail_requested`
- `image_transform_planned`
- `image_transform_requested`
- `image_composite_planned`
- `image_composite_requested`
- `image_redaction_planned`
- `image_redaction_requested`
- `image_generation_planned`
- `image_generation_requested`
- `image_edit_planned`
- `image_edit_requested`
- `image_safety_inspected`
- `image_export_planned`
- `image_export_requested`
- `image_artifact_handle_resolved`
- `image_pack_policy_decision`
- `image_pack_service_call_requested`
- `image_pack_service_call_succeeded`
- `image_pack_service_call_failed`
- `image_pack_unavailable`
- `image_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, image format
and version hashes, command availability, provider health, policy template hash,
resource counters, bounded metadata/operation/safety/artifact summaries, event
cursors, and sanitized replay pointers. Snapshots must exclude raw credentials,
raw prompts, private images, EXIF/GPS metadata, biometric data, masks,
generated image bytes, raw exports, raw provider payloads, manifests, package
bytes, private keys, signatures, and unbounded pixel data.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, format readers, transform engines,
  composite engines, redaction providers, generation providers, safety
  classifiers, export providers, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  metadata stripping, content safety, prompt redaction, artifact retention, and
  output redaction wrap service calls.
- **Specification**: admission validates provider scope, image format, command
  availability, permissions, version preconditions, codec support, safety
  policy, resource budget, and compatibility.
- **Observer**: provider health, trace, audit, safety events, and artifact
  lifecycle events are subscribable.
- **Memento**: image version hashes, operation plans, generation plans, edit
  plans, artifact handles, safety reports, snapshots, and replay pointers
  preserve recovery state.
- **Abstract Factory**: concrete image providers are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes an ImageMagick/libvips/OpenAI wrapper. Mitigation:
  provider-neutral image/operation/artifact/safety DTOs and Strategy adapters.
- Risk: personal media or EXIF/GPS leaks. Mitigation: handles, metadata
  stripping policy, redaction, bounded summaries, and strict observability
  exclusions.
- Risk: unsafe generated or edited images. Mitigation: safety policy, prompt
  handles, generated-content provenance, approval, and audit.
- Risk: image operations consume excessive memory. Mitigation: resource
  estimates, streaming/asynchronous artifacts, quotas, cancellation, and
  provider capability reporting.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call image APIs directly.
