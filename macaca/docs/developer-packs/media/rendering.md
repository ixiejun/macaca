# Media Rendering Pack

`pack.media.rendering.v1` describes provider-neutral rendering capabilities.
The pack is descriptor-only until a rendering provider is installed through the
runtime composition root.

## Manifest Declaration

Declare the pack as required only when rendering capability is mandatory for
readiness. Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.media.rendering.v1"]
```

## Permissions

Use the narrowest scope: `rendering.provider.inspect`,
`rendering.source.import`, `rendering.source.open`, `rendering.template.read`,
`rendering.scene.read`, `rendering.asset.validate`, `rendering.render`,
`rendering.frame`, `rendering.animation`, `rendering.preview`,
`rendering.export`, `rendering.job.read`, `rendering.job.cancel`, and
`rendering.artifact.read`.

## Capability Model

Macaca models rendering as scopes, source handles, template metadata, scene
graph summaries, viewports, surface profiles, asset handles, font references,
render plans, frame plans, animation plans, preview plans, export plans, job
statuses, and artifact handles. Raw templates, scripts, private assets, licensed
fonts, scene graphs, pixels, vector outputs, shader code, credentials, and
provider payloads stay behind provider adapters.

## Commands And Results

`rendering.inspect_provider`, `rendering.import_source_request`,
`rendering.open_source`, `rendering.inspect_template`,
`rendering.inspect_scene_graph`, `rendering.validate_assets`,
`rendering.plan_render`, `rendering.render_request`, `rendering.plan_frame`,
`rendering.frame_request`, `rendering.plan_animation`,
`rendering.animation_request`, `rendering.plan_preview`,
`rendering.preview_request`, `rendering.plan_export`,
`rendering.export_request`, `rendering.inspect_job`, `rendering.cancel_job`,
and `rendering.get_artifact_handle` are descriptor-owned schema names. Result
statuses include success, paged, partial, asynchronous, denied, unavailable,
unsupported, conflict, stale-version, schema-mismatch, format-unsupported,
asset-denied, font-denied, script-denied, network-denied, shader-denied,
gpu-unavailable, render-denied, export-denied, write-denied, artifact-denied,
quota, timeout, cancellation, approval-required, and failure.

Plan commands are non-mutating and carry viewport limits, surface profiles,
script/network policy, asset and font validation refs, GPU/CPU requirements,
idempotency keys, approval references, and resource bounds. Jobs and outputs are
observed through job status DTOs and artifact handles.

## Platform Comparison

Skia, Cairo, Canvas 2D, WebGPU, ImageMagick, librsvg, Lottie/Skottie, Headless
Chrome capture, font providers, asset stores, and export providers map to engine
classes, surface profiles, render/frame/animation/preview/export plans, job
status, and artifact handles. Native scene trees, scripts, shaders, font files,
URLs, pixels, vectors, and provider error payloads are intentionally not OS
semantics.

## App-Facing Examples

- Inspect raster, vector, animation, preview, export, CPU/GPU, script, network,
  asset, and font support before opening a source.
- Import or open a source by handle, then inspect template metadata and scene
  summaries.
- Validate assets and fonts before render, frame, animation, preview, or export
  requests.
- Poll job status or cancel jobs through descriptor-owned commands.
- Consume rendered outputs through artifact handles.
- Handle unavailable, asset, font, script, network, shader, GPU, render, export,
  quota, cancellation, and artifact diagnostics generically.

## App-Facing Example Matrix

Generic examples cover provider inspection, source import/open, template
inspection, scene graph inspection, asset/font validation, render
planning/request, frame planning/request, animation planning/request, preview
planning/request, export planning/request, job inspection, cancellation, and
artifact handles with synthetic source, template, scene, surface, job, and
artifact refs.

Diagnostic examples cover unavailable provider, missing source permission,
template redacted, scene redacted, unsupported format, asset denied, font
denied, script denied, network denied, shader denied, GPU unavailable, render
approval, export denied, provider quota, job timeout, cancellation, and
artifact denied. Diagnostics must not include provider names, credentials,
private assets, licensed fonts, raw templates, raw scripts, raw pixels, raw
vector output, or workflow-specific conventions.

## Trace And Audit

Traces should record declaration, admission decision, command name, source id,
version hash, scene summary hash, viewport hash, surface hash, job id, provider
class, capability hash, result status, and artifact id. They must not record raw
templates, scripts, private assets, licensed fonts, scene graphs, pixels,
vectors, credentials, provider payloads, or unbounded rendering data.

## Provider Authors

Conformance requires descriptor completeness, source/template/scene/job and
artifact scope validation, format and surface support, asset/font validation,
script/network enforcement, GPU/shader validation, render validation, animation
validation, preview determinism, export validation, artifact redaction, bounded
resources, policy hooks, trace/audit events, unavailable behavior,
snapshot/replay metadata, and redaction tests.
