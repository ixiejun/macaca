# Media Image Pack

`pack.media.image.v1` describes provider-neutral image capabilities. The pack is
descriptor-only until an image provider is installed through the runtime
composition root.

## Manifest Declaration

Declare the pack as required only when image processing is mandatory for
readiness. Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.media.image.v1"]
```

## Permissions

Use the narrowest scope: `image.provider.inspect`, `image.import`,
`image.open`, `image.metadata.read`, `image.thumbnail`, `image.transform`,
`image.composite`, `image.redaction`, `image.generate`, `image.edit`,
`image.safety.read`, `image.export`, and `image.artifact.read`.

## Capability Model

Macaca models images as scopes, opaque image handles, version hashes, metadata
refs, pixel geometry, color profiles, frames, transform operations, composite
layers, annotation operations, redaction operations, generation plans, edit
plans, safety reports, export plans, and artifact handles. Raw pixels, EXIF/GPS
payloads, face or biometric signals, masks, prompts, generated image bytes,
credentials, and provider payloads stay behind provider adapters.

## Commands And Results

`image.inspect_provider`, `image.import_image_request`, `image.open_image`,
`image.inspect_metadata`, `image.plan_thumbnail`, `image.thumbnail_request`,
`image.plan_transform`, `image.transform_request`, `image.plan_composite`,
`image.composite_request`, `image.plan_redaction`, `image.redaction_request`,
`image.plan_generation`, `image.generation_request`, `image.plan_edit`,
`image.edit_request`, `image.inspect_safety`, `image.plan_export`,
`image.export_request`, and `image.get_artifact_handle` are descriptor-owned
schema names. Result statuses include success, paged, partial, asynchronous,
denied, unavailable, unsupported, conflict, stale-version, schema-mismatch,
format-unsupported, codec-unsupported, metadata-denied, safety-denied,
prompt-denied, generation-denied, redaction-denied, export-denied,
write-denied, artifact-denied, quota, timeout, cancellation, approval-required,
and failure.

Plan commands are non-mutating and carry version preconditions, idempotency
keys, redaction profile references, approval references, and resource bounds.
Request commands execute only after policy, entitlement, resource, and approval
checks pass. Paged or asynchronous outputs use cursors and artifact handles
instead of embedding media data.

## Platform Comparison

ImageMagick identify/convert/mogrify/composite map to metadata, transform,
composite, and export plans. libvips and Sharp demand-driven pipelines map to
bounded transform and export operations. Cloudinary transformations map to
derived-asset plans and artifact retention. OpenAI Images, Stability AI, and
Adobe Firefly map to generation/edit plans plus safety and provenance
requirements. Google Vision annotations map to safety and annotation reports.
Provider-specific model names, transformation URLs, prompts, masks, storage
URLs, and native error payloads are intentionally not OS semantics.

## App-Facing Examples

- Inspect provider metadata and supported provider classes before opening an
  image.
- Import or open an image through an opaque handle and version hash.
- Inspect metadata while treating EXIF/GPS and color-profile payloads as
  redacted references.
- Plan thumbnails, transforms, composites, redactions, generation, edits, and
  exports before issuing request commands.
- Use safety reports and approval references before publishing generated or
  redacted artifacts.
- Handle unavailable, permission, format, codec, safety, prompt, quota, and
  artifact diagnostics without provider-specific fallback APIs.

## App-Facing Example Matrix

Generic examples cover provider inspection, image import/open, metadata
inspection, thumbnail planning/request, transform planning/request, composite
planning/request, redaction planning/request, generation planning/request, edit
planning/request, safety inspection, export planning/request, and artifact
handles with synthetic image, geometry, safety, job, and artifact refs.

Diagnostic examples cover unavailable provider, missing image permission,
EXIF redacted, GPS stripped, unsupported format, unsupported codec, stale
version, safety denied, prompt denied, generation approval, redaction approval,
export denied, provider quota, network denied, GPU unavailable, and artifact
denied. Diagnostics must not include provider names, credentials, private
images, face/biometric data, EXIF/GPS data, raw prompts, raw generated images,
raw exports, or workflow-specific conventions.

## Trace And Audit

Traces should record declaration, admission decision, command name, image id,
version hash, geometry hash, color/profile hash, provider class, capability
hash, result status, and artifact id. They must not record raw pixels, EXIF/GPS
payloads, biometric signals, raw prompts, masks, generated images, credentials,
provider payloads, or unbounded output.

## Provider Authors

Conformance requires descriptor completeness, image and artifact scope
validation, format and codec support, EXIF/GPS stripping policy, color/profile
compatibility, transform validation, composite validation, redaction validation,
generation and edit safety, safety reports, export validation, artifact
redaction, bounded resources, policy hooks, trace/audit events, unavailable
behavior, snapshot/replay metadata, and redaction tests.
