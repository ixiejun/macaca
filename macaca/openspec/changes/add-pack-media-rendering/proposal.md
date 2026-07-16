# Change: Add Media Rendering Pack

## Why

Developers need `pack.media.rendering.v1` as an industrial rendering capability
for scene/template validation, 2D vector/raster rendering, SVG/static vector
rendering, canvas-like drawing, GPU/CPU rendering, deterministic previews,
single-frame rendering, animation frame sequence rendering, responsive viewport
snapshots, artifact export, job status, cancellation, and replay diagnostics. It
must not be a thin wrapper around Skia, Cairo, Canvas 2D, WebGPU, ImageMagick,
librsvg, Lottie/Skottie, Headless Chrome, or one rendering engine.

Rendering inputs can contain private documents, user content, fonts, embedded
images, linked media, remote URLs, scripts, templates, layout data, generated
assets, licensed fonts, copyrighted media, and untrusted vector data. Rendering
can consume CPU/GPU/memory heavily, fetch network resources, execute unsafe
content if not constrained, or publish derived artifacts. Macaca must therefore
expose rendering only through provider-neutral typed service commands with
declared permissions, asset policy, font policy, network policy, script
blocking, resource reservation, approval, artifact retention, trace, audit,
health, snapshot, replay, and structured unavailable behavior.

## Research And Supplier/API Baseline

Official and supplier references considered for this pack:

- Skia exposes a cross-platform 2D graphics library and canvas-style drawing
  engine used by Chrome, Android, Flutter, and other platforms. References:
  https://skia.org/docs/ and https://api.skia.org/classSkCanvas.html
- Cairo exposes vector drawing over surfaces such as image buffers, PDF,
  PostScript, SVG, and platform windows. References:
  https://www.cairographics.org/manual/ and
  https://www.cairographics.org/manual/cairo-PDF-Surfaces.html
- HTML Canvas 2D and WHATWG Canvas define a script-facing bitmap drawing
  surface for shapes, text, images, and visual composition. References:
  https://www.w3.org/2015/04/2dcontext-lc-sample.html and
  https://html.spec.whatwg.org/multipage/canvas.html
- WebGPU exposes modern GPU-backed rendering and compute with explicit device
  capabilities, resource limits, and shader pipelines. Reference:
  https://www.w3.org/TR/webgpu/
- ImageMagick exposes batch image conversion, composition, resizing, and
  sequence processing across many formats. References:
  https://imagemagick.org/ and
  https://imagemagick.org/command-line-processing/
- librsvg renders SVG documents into Cairo surfaces and highlights safe static
  SVG rendering boundaries. Reference:
  https://gnome.pages.gitlab.gnome.org/librsvg/Rsvg-2.0/overview.html
- Lottie/Skottie expose JSON/vector animation rendering and frame-based
  animation playback. References: https://lottie.airbnb.tech/ and
  https://skia.org/docs/user/modules/skottie/
- Headless Chrome and Chrome DevTools Protocol provide page/surface screenshot
  and PDF-like page capture baselines for viewport rendering. Reference:
  https://chromedevtools.github.io/devtools-protocol/tot/Page/

Macaca maps these supplier concepts into provider-neutral rendering scope,
provider capability, render source, template/scene graph, viewport, surface,
asset handle, font reference, render plan, frame plan, animation plan, preview
plan, export profile, render job status, artifact handle, capability hashes,
event cursors, and diagnostics DTOs. Concrete CPU/GPU engines, browser
renderers, SVG renderers, image processors, animation renderers, font providers,
asset stores, and export providers stay behind replaceable service providers.

## What Changes

- Add provider-neutral `pack.media.rendering.v1` under the `media` family.
- Define command namespace `rendering.*` for:
  - provider capability inspection
  - source/template/scene import and inspection
  - asset and font reference validation
  - render, frame, animation, preview, and export planning
  - side-effecting render/frame/animation/preview/export requests
  - job inspection, job cancellation, artifact handle resolution, snapshots,
    and replay
- Define DTOs for rendering scope, provider capability, render source handle,
  template handle, scene graph summary, viewport, surface profile, asset handle,
  font reference, render plan, frame plan, animation plan, preview plan, export
  plan, render job status, artifact handle, event cursor, and diagnostics.
- Define permission scopes, policy defaults, safe static/vector rendering,
  script/network/font/asset gates, CPU/GPU/memory/storage quotas, deterministic
  preview requirements, artifact retention, SDK discovery, developer
  documentation, trace/audit events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/media/rendering.md` before implementation completion.

## Impact

- Affected specs: `pack-media-rendering`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, rendering service
  provider or unavailable provider, runtime-host provider adapters,
  artifact/font/asset/redaction support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete Skia/Cairo/Canvas/WebGPU/ImageMagick/librsvg/Lottie/
  Headless Chrome/font/storage/export provider implementation in this proposal;
  no application-specific UI renderer, website screenshot product, design tool,
  game engine, video editor, PDF editor, office renderer, or business workflow;
  no provider-name, engine-name, shader-name, template-name, font-name,
  queue-name, URL-name, or workflow-name routing in OS layers beyond
  declarative descriptor data; no raw credentials, raw templates, raw scripts,
  private assets, licensed fonts, raw scene graphs, raw provider payloads,
  manifests, package bytes, private keys, signatures, or unbounded pixel/vector
  output in observability; no SDK/shell/kernel provider construction; no fake
  success when provider, engine, format, font, asset, script, network,
  permission, entitlement, approval, resource, GPU, or host support is absent.
