# Media Rendering Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.media.rendering.v1`. Rendering support must expose CPU/GPU/vector/browser
render plans, surfaces, assets, fonts, captures, animation frames, export
artifacts, and diagnostics through serviceized commands, not provider-native
canvas, shader, URL, or UI workflow pass-through.

## Source Baseline

- Skia canvas creation and `SkCanvas` overview:
  <https://skia.org/docs/user/api/skcanvas_overview/>
  and <https://skia.org/docs/user/api/skcanvas_creation/>
- Cairo tutorial and SVG surfaces:
  <https://www.cairographics.org/tutorial/>
  and <https://www.cairographics.org/manual-1.12.4/cairo-SVG-Surfaces.html>
- WHATWG Canvas and Canvas 2D:
  <https://html.spec.whatwg.org/multipage/canvas.html>
  and <https://www.w3.org/TR/2021/SPSD-2dcontext-20210128/>
- WebGPU:
  <https://www.w3.org/TR/webgpu/>
  and <https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API>
- ImageMagick, librsvg, Lottie/Skottie, and Headless Chrome/CDP are treated as
  provider candidates for image conversion, SVG rendering, animation frame
  rendering, and viewport capture.

## Supplier API Notes

- Skia contributes canvas-style 2D drawing, paths, text, images, filters,
  surfaces, CPU/GPU backends, Skottie animation, and backend lifecycle. Macaca
  should model render surfaces, draw plans, animation plans, and provider
  backend capability.
- Cairo contributes deterministic vector drawing, image/PDF/SVG surfaces,
  paths, text, and explicit surface lifecycle. Macaca should model surface kind,
  output artifact, and deterministic replay metadata.
- Canvas 2D contributes bitmap surfaces, shapes, text, images, compositing,
  dimensions, origin-clean constraints, and export behavior. Macaca should
  encode origin/asset safety and export eligibility as policy metadata.
- WebGPU contributes adapter/device availability, device limits, shaders,
  pipelines, buffers, textures, command encoders, command queues, and async
  error/resource behavior. Macaca should model GPU capability and resource
  reservations, not expose shader-specific routing.
- ImageMagick, librsvg, Lottie/Skottie, and browser capture providers
  contribute conversion, SVG static rendering, animation frames, viewport
  snapshots, remote resource restrictions, and export artifact behavior.

## Macaca-Owned Abstractions

`pack.media.rendering.v1` should define `RenderSurface`, `RenderAsset`,
`RenderFont`, `RenderPlan`, `RenderCommand`, `RenderLayer`,
`RenderAnimationPlan`, `RenderFrame`, `RenderCaptureRequest`,
`RenderExportArtifact`, `RenderResourceBudget`, `RenderSafetyReport`, and
`RenderProviderCapability`.

The DTOs must carry surface dimensions, color space, asset/font handles,
operation graphs, animation timeline, frame bounds, GPU/CPU resource
reservation, origin/remote-resource policy, output artifacts, capability
hashes, redaction profiles, and replay pointers. Raw pixels, raw vectors, raw
URLs, shaders, templates, private fonts, provider payloads, and unbounded frame
exports are rejected.

## Explicit Non-Goals

- Do not implement concrete Skia, Cairo, Canvas, WebGPU, ImageMagick, librsvg,
  Lottie/Skottie, Headless Chrome, font, asset, or browser providers in this
  research phase.
- Do not define UI renderer, design tool, game engine, video editor, PDF,
  office, charting, or application-specific rendering workflows in OS layers.
- Do not expose provider-native canvas APIs, shader code, browser URLs, raw
  screenshots, or engine-specific object ids as stable SDK contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, file handles, browser automation, office PDF, media image, and
  media video proposals provide reusable substrate.
- Current evidence does not prove rendering DTOs, providers, SDK helpers, WASM
  ABI, tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
