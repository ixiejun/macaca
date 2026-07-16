# Media Image Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.media.image.v1`. Image support must expose inspect, transform, compose,
generate, edit, annotate, optimize, export, and safety diagnostics through typed
service commands. It must not become a photo editor, DAM, avatar, OCR, design,
social, marketing, or provider-specific model workflow.

## Source Baseline

- ImageMagick command processing, formats, and mogrify behavior:
  <https://imagemagick.org/command-line-processing/>
  <https://imagemagick.org/formats/>
  <https://imagemagick.org/mogrify/>
- libvips demand-driven processing:
  <https://www.libvips.org/>
  and <https://www.libvips.org/API/8.16/How-it-works.html>
- Sharp API:
  <https://sharp.pixelplumbing.com/api-output/>
- Cloudinary image transformations:
  <https://cloudinary.com/documentation/image_transformations>
  and <https://cloudinary.com/documentation/transformation_reference>
- OpenAI Images, Stability AI, Adobe Firefly, and Google Cloud Vision:
  <https://developers.openai.com/api/docs/guides/image-generation>
  <https://platform.stability.ai/docs/api-reference>
  <https://developer.adobe.com/firefly-services/docs/firefly-api/>
  <https://docs.cloud.google.com/vision/docs/features-list>

## Supplier API Notes

- ImageMagick contributes identify/convert/mogrify/composite behavior,
  metadata, resize/crop/rotate, colorspace/profile handling, drawing,
  compositing, format negotiation, and delegated format boundaries. Macaca
  should model operation plans and supported formats instead of raw CLI syntax.
- libvips contributes demand-driven, horizontally threaded, low-memory image
  processing with metadata/profile handling and long pipeline composition.
  Macaca should represent streaming/resource behavior and operation graphs.
- Sharp contributes Node/libvips APIs for metadata, resize/crop/rotate,
  composite, colorspace, output formats, and default metadata stripping.
  Macaca should expose redaction/default-metadata policy explicitly.
- Cloudinary contributes remote transformation URLs, resize/crop, overlays,
  effects, quality/format optimization, derived assets, delivery policy, quota,
  and error behavior. Macaca should model derived artifact handles and remote
  quota diagnostics without URL pass-through.
- Generative and annotation providers contribute text-to-image, edits,
  variations, fill/expand, upscale, prompt safety, model capability, artifacts,
  provenance, SafeSearch, crop hints, labels, text/face/object/image-property
  annotations, and sensitive-output constraints.

## Macaca-Owned Abstractions

`pack.media.image.v1` should define `ImageAsset`, `ImageMetadata`,
`ImageFormatProfile`, `ImageTransformPlan`, `ImageResize`, `ImageCrop`,
`ImageComposite`, `ImageColorProfile`, `ImageGenerationRequest`,
`ImageEditRequest`, `ImageSafetyReport`, `ImageAnnotation`,
`ImageExportArtifact`, and `ImageProviderCapability`.

The DTOs must carry source handles, dimensions, format/color/ICC metadata,
operation graphs, generated-artifact provenance, prompt safety state, annotation
redaction, output artifact handles, resource budgets, provider capability
hashes, and replay pointers. Raw images, raw prompts, provider payloads,
credentials, private visual content, and unbounded pixel dumps are rejected.

## Explicit Non-Goals

- Do not implement concrete ImageMagick, libvips, Sharp, Cloudinary, OpenAI,
  Stability, Firefly, Vision, storage, or export providers in this research
  phase.
- Do not define photo-editor, DAM, avatar, OCR, design tool, social media,
  marketing, brand, or application-specific image workflows in OS layers.
- Do not expose raw CLI options, transformation URLs, model ids, prompts,
  provider annotations, or provider-native artifact payloads as stable SDK API.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, file handles, secrets-reference handles, media rendering, and AI
  vision proposals provide reusable substrate.
- Current evidence does not prove image DTOs, providers, SDK helpers, WASM ABI,
  tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
