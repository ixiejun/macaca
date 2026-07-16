# Media Rendering Pack Design

## Context

`pack.media.rendering.v1` exposes rendering as a Macaca OS serviceized
capability. It lets applications validate render sources, inspect templates and
scene graphs, render previews, render frames, render animation sequences, export
raster/vector artifacts, cancel jobs, and replay rendering evidence without
embedding concrete graphics engines, browser renderers, SVG renderers, image
processors, font providers, asset stores, or application-specific UI workflows
into generic OS layers.

Rendering is resource-heavy and trust-sensitive. Inputs can contain untrusted
SVG/vector data, templates, fonts, embedded images, remote URLs, scripts,
licensed assets, generated content, and private document fragments. The pack
treats source templates, scene graphs, assets, fonts, raw pixels, vector output,
rendered frames, animation sequences, and provider payloads as scoped resources.
Reads return bounded metadata and handles; side effects use validated plans,
idempotent requests, asynchronous job handles, and artifact boundaries.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Skia | Cross-platform 2D canvas, paths, text, images, filters, Skottie animation | CPU/GPU render Strategy, scene graph, surface, frame/animation plans |
| Cairo | Vector surfaces, PDF/SVG/PNG-style output, paths, text, image surfaces | Surface profile, vector/raster export plan, deterministic renderer |
| Canvas 2D / WHATWG Canvas | Bitmap drawing, text, images, compositing, viewport-backed rendering | Canvas-like scene operation model, viewport, raster artifact |
| WebGPU | GPU device limits, shader pipelines, buffers, textures, command queues | GPU capability, resource limits, shader policy, render pipeline diagnostics |
| ImageMagick | Image conversion, resize, compose, sequence processing | Raster export/conversion Strategy and bounded artifact generation |
| librsvg | Safe static SVG rendering into Cairo-like surfaces | Static SVG source validation and vector-to-raster Strategy |
| Lottie / Skottie | JSON/vector animation playback and frame rendering | Animation source, frame plan, sequence artifact, timeline diagnostics |
| Headless Chrome / CDP Page | Viewport/page capture, screenshot, print-like output | Browser-render Strategy, viewport snapshot, network/script policy |

The pack exposes provider-neutral contracts. Provider adapters translate to CPU
engines, GPU engines, browser renderers, SVG renderers, image processors,
animation renderers, font resolvers, asset stores, export providers, or
unavailable providers. OS layers must not branch on provider names, engine
names, shader names, template names, font names, queue names, URLs, or business
workflows.

## Goals

- Provide stable pack id `pack.media.rendering.v1` and command namespace
  `rendering.*`.
- Support provider inspection, source/template import/open, template inspection,
  scene graph inspection, asset/font validation, render planning/request,
  frame planning/request, animation planning/request, preview planning/request,
  export planning/request, job inspection, job cancellation, artifact handles,
  health, snapshots, and replay diagnostics.
- Preserve safety with script blocking, remote URL policy, asset scopes, font
  scopes, shader policy, GPU/CPU/memory/storage quotas, deterministic preview
  settings, artifact retention, bounded output, and sanitized audit.
- Keep concrete rendering providers behind replaceable service providers.
- Require developer documentation at `docs/developer-packs/media/rendering.md`.

## Non-Goals

- Do not implement concrete Skia, Cairo, Canvas, WebGPU, ImageMagick, librsvg,
  Lottie/Skottie, Headless Chrome, font, storage, or export providers in this
  proposal.
- Do not define an application UI renderer, website screenshot product, design
  tool, game engine, PDF editor, video editor, office renderer, or
  application-specific workflow.
- Do not expose raw credentials, raw templates, raw scripts, private assets,
  licensed fonts, raw scene graphs, raw pixels, vector payloads, raw provider
  payloads, manifests, package bytes, private keys, signatures, or unbounded
  output in observability.
- Do not silently render, preview, execute scripts, fetch remote resources, use
  fonts/assets, export artifacts, or publish outputs without typed
  plan/request, policy checks, version preconditions, and approval where
  required.

## Ownership And Boundaries

- Pack id: `pack.media.rendering.v1`.
- Family: `media`.
- Backing service owner: rendering service provider.
- SDK surface: `sdk.packs.media.rendering`.
- Command namespace: `rendering.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges, artifact
  stores, font/asset bridges, decorators, and sanitized diagnostics through
  approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `rendering.inspect_provider` | Inspect provider, engine, format, raster/vector, animation, preview, export, CPU/GPU, font, asset, network, and script support | Returns sanitized capability, quota, lifecycle, health, and compatibility metadata |
| `rendering.import_source_request` | Import render source/template/scene from file/artifact handle | Requires artifact permission, format validation, script/network policy, size policy, and audit |
| `rendering.open_source` | Resolve render source handle and version metadata | Requires source scope and bounded metadata |
| `rendering.inspect_template` | Inspect template/scene metadata, required assets, fonts, viewports, formats, and provenance | Requires metadata permission and redaction |
| `rendering.inspect_scene_graph` | Inspect bounded scene graph summary | Requires scene permission, projection limits, and script redaction |
| `rendering.validate_assets` | Validate asset handles, font references, remote URL policy, and license/retention metadata | Requires asset/font permissions and bounded diagnostics |
| `rendering.plan_render` | Plan raster/vector render for a source/template/scene | Validates viewport, surface, output profile, asset/font availability, script/network policy, resource budget, and approvals |
| `rendering.render_request` | Execute a validated render plan | Requires plan handle, idempotency key, version preconditions, and audit |
| `rendering.plan_frame` | Plan a deterministic single-frame render | Validates timeline/frame cursor, viewport, surface, asset/font state, and resource budget |
| `rendering.frame_request` | Execute a validated frame render | Returns bounded artifact/job handles |
| `rendering.plan_animation` | Plan frame sequence or animation rendering | Validates timeline, fps class, duration, frame count, output profile, resource budget, and approvals |
| `rendering.animation_request` | Execute a validated animation plan | Returns job/artifact handles and diagnostics |
| `rendering.plan_preview` | Plan deterministic preview or responsive viewport snapshot | Validates viewport set, fidelity, redaction, cache, and resource policy |
| `rendering.preview_request` | Execute preview plan | Returns bounded preview artifact handles |
| `rendering.plan_export` | Plan artifact export/conversion | Validates format, color profile, dimensions, metadata retention, sensitivity, and approvals |
| `rendering.export_request` | Execute export/conversion | Returns bounded artifact handles |
| `rendering.inspect_job` | Inspect asynchronous rendering job status | Requires job scope and redaction |
| `rendering.cancel_job` | Cancel an active render job | Requires job scope, cancellation policy, and audit |
| `rendering.get_artifact_handle` | Resolve render/frame/animation/preview/export artifact metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
partial/asynchronous results, typed denied/unavailable/unsupported/conflict/
stale-version/schema-mismatch/format-unsupported/asset-denied/font-denied/
script-denied/network-denied/shader-denied/gpu-unavailable/render-denied/
export-denied/write-denied/artifact-denied/quota/timeout/cancellation/
approval-required/failure results, redaction profile, idempotency semantics for
side effects, job status semantics, and replay metadata.

## DTO Model

Core DTOs:

- `RenderingScope`: provider scope, source handle, credential reference,
  network policy, script policy, asset policy, font policy, artifact policy,
  permission state, rate-limit profile, and health.
- `RenderingProviderCapability`: provider class, engine class, raster support,
  vector support, animation support, preview support, export support, surface
  formats, color profiles, viewport limits, GPU/CPU classes, shader support,
  script policy support, remote asset support, font support, auth modes, rate
  limits, lifecycle, and health.
- `RenderSourceHandle`: source handle, provider scope, source artifact handle,
  source kind, format class, version hash, size class, asset count class, font
  count class, script presence class, sensitivity class, provenance class,
  redaction class, and freshness.
- `RenderTemplateMetadata`: template handle, required asset/font handles,
  viewport classes, page/frame count class, animation duration class, color
  profile, embedded script presence, remote reference presence, checksum handle,
  and redaction class.
- `SceneGraphSummary`: scene handle, node count class, layer count class,
  operation classes, text presence, image presence, vector/path presence,
  filter/effect presence, timeline presence, and redaction class.
- `RenderViewport`: width/height class, device scale class, orientation,
  responsive breakpoint class, crop/safe-area, and redaction class.
- `RenderSurfaceProfile`: output kind, format class, color profile, alpha
  policy, compression class, metadata retention, and artifact policy.
- `RenderAssetHandle` and `RenderFontReference`: handles, source scope,
  content/license class, retention, checksum handle, redaction class, and
  validation diagnostics.
- `RenderPlan`, `RenderFramePlan`, `RenderAnimationPlan`, `RenderPreviewPlan`,
  and `RenderExportPlan`: plan handles, source handles, viewport/surface
  profiles, operation list hashes, asset/font validation hashes, script/network
  decisions, version preconditions, resource estimate, required approvals,
  idempotency key, retention, redaction, and validation diagnostics.
- `RenderJobStatus`: job handle, command name, provider capability hash, state,
  progress class, queue class, cancellation state, result artifact handles, and
  redaction class.
- `RenderArtifactHandle`: artifact handle, source operation/job handle,
  artifact kind, content type, dimensions class, frame count class, size class,
  checksum handle, retention, provenance, redaction class, and replay pointer.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `rendering.provider.inspect`
- `rendering.source.import`
- `rendering.source.open`
- `rendering.template.read`
- `rendering.scene.read`
- `rendering.asset.validate`
- `rendering.render`
- `rendering.frame`
- `rendering.animation`
- `rendering.preview`
- `rendering.export`
- `rendering.job.read`
- `rendering.job.cancel`
- `rendering.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, source handle, template/scene handle when
  applicable, job handle when applicable, artifact handle when applicable,
  credential reference, network policy, script policy, asset policy, font
  policy, artifact policy, and permission state.
- Side-effecting render, frame, animation, preview, and export commands require
  plan/request separation, idempotency key, source version preconditions,
  script/network decisions, asset/font validation, artifact retention policy,
  and audit reason.
- Private templates, licensed fonts, copyrighted assets, remote URL fetching,
  script-enabled rendering, GPU/shader execution, external delivery, metadata
  stripping, and publishing artifacts may require approval.
- Raw templates, scripts, private assets, licensed font contents, scene graphs,
  rendered pixels, vector outputs, derived artifacts, job outputs, and provider
  payloads require redaction and bounded output.
- Remote/GPU operations require network/GPU permission, provider quota, rate
  limits, timeout, cancellation, job-status inspection, and structured
  unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
engine classes, raster/vector support, animation support, preview support,
export support, surface formats, viewport limits, CPU/GPU classes, shader
policy, script/network/asset/font policy support, permission scopes, policy
templates, resource limits, approval rules, provider capability hashes, health,
compatibility, diagnostics, examples, redaction profiles, and documentation
links.

The developer guide at `docs/developer-packs/media/rendering.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, source handles, template metadata, scene graph summaries,
  viewports, surfaces, assets, fonts, render plans, frame plans, animation
  plans, preview plans, export plans, jobs, artifacts, provider capabilities,
  and unavailable states
- plan/request lifecycle, asynchronous job lifecycle, cancellation, version
  conflicts, format mismatch, asset/font denial, script/network policy, GPU
  unavailability, deterministic preview, artifact retention, approvals, quotas,
  provider replacement, trace/audit interpretation, and conformance tests

Examples must use synthetic sources, templates, scene graphs, assets, fonts,
viewports, jobs, and artifacts. They must not include provider names, real
credentials, private assets, licensed fonts, raw templates, raw scripts, raw
pixels, raw vector output, or workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `rendering_pack_declared`
- `rendering_pack_admission_validated`
- `rendering_provider_inspected`
- `rendering_source_imported`
- `rendering_source_opened`
- `rendering_template_inspected`
- `rendering_scene_graph_inspected`
- `rendering_assets_validated`
- `rendering_render_planned`
- `rendering_render_requested`
- `rendering_frame_planned`
- `rendering_frame_requested`
- `rendering_animation_planned`
- `rendering_animation_requested`
- `rendering_preview_planned`
- `rendering_preview_requested`
- `rendering_export_planned`
- `rendering_export_requested`
- `rendering_job_inspected`
- `rendering_job_cancelled`
- `rendering_artifact_handle_resolved`
- `rendering_pack_policy_decision`
- `rendering_pack_service_call_requested`
- `rendering_pack_service_call_succeeded`
- `rendering_pack_service_call_failed`
- `rendering_pack_unavailable`
- `rendering_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, command
availability, provider health, policy template hash, resource counters, bounded
source/template/scene/plan/job/artifact summaries, event cursors, and sanitized
replay pointers. Snapshots must exclude raw credentials, raw templates, raw
scripts, private assets, licensed fonts, raw scene graphs, raw pixels, raw
vector outputs, raw provider payloads, manifests, package bytes, private keys,
signatures, and unbounded rendering data.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, CPU renderers, GPU renderers, browser
  renderers, SVG renderers, animation renderers, image processors, font
  resolvers, asset providers, export providers, and unavailable behavior are
  replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  script blocking, network policy, asset/font validation, artifact retention,
  and output redaction wrap service calls.
- **Specification**: admission validates provider scope, source format, command
  availability, permissions, source version, asset/font state, script/network
  policy, surface/profile compatibility, resource budget, and compatibility.
- **Observer**: provider health, trace, audit, job status, and artifact
  lifecycle events are subscribable.
- **State**: render jobs use explicit state machines for planned, queued,
  running, cancelling, cancelled, completed, failed, timed-out, unavailable, and
  replay states.
- **Memento**: source version hashes, plans, job handles, artifact handles,
  snapshots, and replay pointers preserve recovery state.
- **Abstract Factory**: concrete rendering providers are created only by
  approved runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes a wrapper around one graphics engine. Mitigation:
  provider-neutral source/scene/plan/job/artifact DTOs and Strategy adapters.
- Risk: untrusted rendering executes unsafe scripts or network fetches.
  Mitigation: script/network policy, asset/font validation, explicit approvals,
  and provider-side sandbox requirements.
- Risk: rendered pixels or templates leak. Mitigation: handles, redaction,
  bounded summaries, artifact boundaries, and strict observability exclusions.
- Risk: GPU rendering consumes excessive resources. Mitigation: resource
  estimates, provider capability reporting, quotas, cancellation, and
  unavailable states.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call rendering APIs directly.
