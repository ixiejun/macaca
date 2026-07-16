# Change: Add AI Vision Pack

## Why

Developers need `pack.ai.vision.v1` as a real industrial capability for image/video understanding, OCR, object detection, moderation, and visual evidence extraction. The pack must not be a catalog label only: if an application declares it and the provider is installed, the SDK must expose callable typed commands; if policy, entitlement, provider, or host support is absent, Macaca must return explicit unavailable or denied diagnostics.

This child proposal intentionally narrows the broad industrial catalog into one implementable service-backed pack. It records the research basis, capability boundary, command surface, permission model, observability model, and acceptance gates needed before this pack can be called production-ready.

## Research And Borrowed Platform Patterns

The proposal borrows stable ideas from mature application platforms without copying their product-specific APIs:

- Platform privacy models: raw user data must not enter logs or diagnostics when processed by intelligent services.
- Android runtime permission pattern: AI operations that touch private data inherit source permissions.
- Windows capability declaration pattern: model-backed operations must be visible in app capability metadata.
- Apple entitlement/privacy pattern: sensitive processing requires policy and developer-declared purpose.

The Macaca interpretation is: app manifests declare pack intent; admission validates capability and permission metadata; runtime permission and approval gates happen before sensitive side effects; concrete providers stay behind service descriptors; traces and audits prove the execution path.

## What Changes

- Add the provider-neutral `pack.ai.vision.v1` contract under the `ai` family.
- Define a concrete command namespace, initial industrial command set, result/error DTO requirements, permission scopes, policy defaults, resource/entitlement behavior, and unavailable diagnostics.
- Bind implementation ownership to vision service provider; the kernel, SDK, shells, and generic application framework remain provider-neutral.
- Add SDK discovery metadata and examples for analyze image, analyze video, ocr, detect objects, moderate visual.
- Add trace, audit, health, snapshot, replay, and boundary gates proving the pack uses the canonical service path.

## Impact

- Affected specs: `pack-ai-vision`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs in protocol crates, descriptor/admission validators, SDK discovery/command helpers, vision service provider, optional provider registration, trace/audit tests, and dependency-boundary gates.
- Non-goals: no application-specific workflow, no provider-name routing in OS layers, no concrete provider construction in SDK/shell/kernel, and no fake success for absent providers.

## Supplier/API Baseline To Match

- Google Cloud Vision and Azure Computer Vision style APIs: OCR, object
  detection, labels, moderation/safety signals, bounding boxes, page layout, and
  confidence metadata.
- AWS Rekognition style APIs: image/video analysis, moderation labels,
  asynchronous video jobs, and result pagination.
- Apple Vision and Android ML Kit style APIs: local OCR/object detection,
  coordinate systems, confidence scores, and privacy-sensitive on-device
  processing.
- Multimodal vision APIs: image understanding, visual question answering,
  evidence extraction, and redacted reasoning/output metadata.

Macaca's contract normalizes visual analysis outputs while preserving privacy,
coordinate-system clarity, evidence provenance, and modality-specific resource
limits.

## Industrial Acceptance Bar

This proposal is not complete until implementation tasks produce a developer
guide at `docs/developer-packs/ai/vision.md`, typed visual input/region/OCR/
object/moderation/evidence/job DTOs, deterministic tests for coordinate
conversion and async video jobs, and replay evidence proving raw media never
enters logs, traces, snapshots, or SDK diagnostics.
