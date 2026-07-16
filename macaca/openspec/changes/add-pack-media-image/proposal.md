# Change: Add Media Image Pack

## Why

Developers need `pack.media.image.v1` as an industrial image capability for
image inspection, metadata extraction, thumbnailing, resize/crop/rotate,
format conversion, color/profile transforms, compositing, annotation, safe
redaction, AI image generation, AI image editing, upscaling, moderation/safety
classification, export, artifact management, and replay diagnostics. It must
not be a thin wrapper around ImageMagick, libvips, Sharp, Cloudinary, OpenAI
Images, Stability AI, Adobe Firefly, Google Vision, or one image library.

Images frequently contain faces, biometrics, location metadata, documents,
screenshots, private designs, minors, copyrighted media, medical/financial
records, and generated content. Transformation can leak EXIF/GPS metadata,
alter evidence, violate brand or copyright constraints, or produce unsafe
content. Macaca must therefore expose image operations only through
provider-neutral typed service commands with permission, policy, entitlement,
resource, approval, content safety, metadata stripping, artifact retention,
trace, audit, health, snapshot, replay, and structured unavailable behavior.

## Research And Supplier/API Baseline

Official and supplier references considered for this pack:

- ImageMagick exposes identify/convert/mogrify/composite-style operations for
  metadata inspection, resizing, cropping, rotation, format conversion,
  colorspace/profile handling, drawing, compositing, and many raster/vector
  formats. Reference: https://imagemagick.org/script/command-line-processing.php
- libvips exposes demand-driven high-performance image processing, metadata,
  ICC/profile operations, resizing, cropping, compositing, and format encoders.
  Reference: https://www.libvips.org/API/current/
- Sharp exposes Node/libvips-based resize, crop, rotate, composite, metadata,
  colorspace, and output format operations. Reference:
  https://sharp.pixelplumbing.com/api-operation/
- Cloudinary transformation APIs expose URL/API-based resize/crop, overlays,
  format/quality optimization, effects, delivery, and derived-asset behavior.
  Reference: https://cloudinary.com/documentation/image_transformations
- OpenAI Images API exposes image generation and editing primitives with model,
  prompt, size, quality, and output handling. Reference:
  https://platform.openai.com/docs/guides/images
- Stability AI and Adobe Firefly APIs provide additional vendor baselines for
  text-to-image, image-to-image, expand/fill, upscale, and creative editing
  workflows. References: https://platform.stability.ai/docs and
  https://developer.adobe.com/firefly-services/docs/
- Google Cloud Vision image annotation APIs expose safe search, crop hints,
  labels, OCR-like text detection, face/object annotations, and image property
  signals. Reference: https://cloud.google.com/vision/docs/reference/rest

Macaca maps these supplier concepts into provider-neutral image scope,
provider capability, image handle, metadata, pixel geometry, color profile,
transform operation, composition layer, annotation operation, redaction plan,
generation request, edit request, safety report, render/export plan, artifact
handle, provider capability, version/freshness metadata, and diagnostics DTOs.
Concrete ImageMagick, libvips, Sharp, Cloudinary, OpenAI, Stability, Firefly,
Vision, storage, moderation, and export providers stay behind replaceable
providers.

## What Changes

- Add provider-neutral `pack.media.image.v1` under the `media` family.
- Define command namespace `image.*` for:
  - provider capability inspection
  - image import/open and metadata inspection
  - pixel geometry, EXIF/GPS, ICC/profile, color, alpha, animation, and format
    inspection
  - thumbnail/render planning and requests
  - resize/crop/rotate/flip/convert/color/profile transform planning and
    requests
  - compositing, watermarking, annotation, masking, and redaction planning
  - AI image generation, image editing, variations, and upscaling planning and
    requests
  - safety/moderation/property inspection where supported
  - export planning, export requests, artifact handle resolution, snapshots,
    and replay
- Define DTOs for image scope, provider capability, image handle, image
  metadata, pixel geometry, color/profile metadata, animation/frame metadata,
  transform operation, composition layer, annotation operation, mask/redaction
  operation, generation plan, edit plan, safety report, export plan, artifact
  handle, event cursor, and diagnostics.
- Define permission scopes, policy defaults, image/artifact scopes, EXIF/GPS
  stripping policy, biometric/sensitive-image handling, prompt/content safety,
  generated-content provenance, approval rules, resource/entitlement behavior,
  SDK discovery, developer documentation, trace/audit events, snapshots,
  replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/media/image.md` before implementation completion.

## Impact

- Affected specs: `pack-media-image`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, image media service
  provider or unavailable provider, runtime-host provider adapters, artifact/
  redaction/moderation support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete ImageMagick/libvips/Sharp/Cloudinary/OpenAI/Stability/
  Firefly/Vision/storage/moderation/export provider implementation in this
  proposal; no app-specific photo editor, DAM, avatar, OCR workflow, design
  workflow, social media workflow, or marketing template logic; no provider-
  name, model-name, image-name, layer-name, prompt-template, or workflow-name
  routing in OS layers; no raw credentials, raw prompts, private images, EXIF/
  GPS metadata, biometric data, generated image bytes, raw provider payloads,
  manifests, package bytes, private keys, signatures, or unbounded pixel data
  in observability; no SDK/shell/kernel provider construction; no fake success
  when provider, codec, format, model, safety, permission, entitlement,
  approval, resource, or host support is absent.
