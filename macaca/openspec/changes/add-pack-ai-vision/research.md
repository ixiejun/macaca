# AI Vision Pack Research

## Purpose

This note records borrowed platform patterns, Macaca mapping, existing platform
inventory, and GitNexus memo evidence for `pack.ai.vision.v1`. The pack must
provide image/video analysis, OCR, object detection, visual moderation, and
visual evidence extraction through provider-neutral commands. It must not become
media storage, UI rendering, surveillance workflow, or provider-native vision
SDK exposure.

## Source Baseline

- OpenAI vision-capable model documentation:
  <https://platform.openai.com/docs/guides/vision>
- Azure AI Vision image analysis and OCR documentation:
  <https://learn.microsoft.com/en-us/azure/ai-services/computer-vision/>
- Google Cloud Vision and Vertex AI multimodal documentation:
  <https://cloud.google.com/vision/docs>
  and <https://cloud.google.com/vertex-ai/generative-ai/docs/multimodal/overview>
- AWS Rekognition image/video analysis documentation:
  <https://docs.aws.amazon.com/rekognition/latest/dg/what-is.html>
- Platform privacy and permission patterns from Android, Apple, and Windows
  inform source-permission inheritance and declared capability metadata.

## Borrowed Platform Patterns

- Vision APIs converge on image/video references, region coordinates, OCR text
  blocks, object labels, moderation categories, confidence values, asynchronous
  video jobs, and evidence/provenance metadata.
- Providers differ in coordinate systems, rotation handling, page/frame ids,
  video job lifecycle, and label taxonomies. Macaca should normalize coordinate
  metadata and provider capability reports.
- OCR and visual moderation can expose sensitive content. Macaca should store
  redacted text/evidence references, hashes, region metadata, and confidence
  bands in observability, not raw images or unbounded extracted text.
- Video analysis is often asynchronous. Macaca should model jobs, progress,
  cancellation, partial results, timeout, and unavailable behavior.
- Media persistence and rendering remain media/file services; vision consumes
  references and emits evidence references.

## Macaca Mapping

- Descriptor: `pack.ai.vision.v1`, command namespace `vision.*`, scopes
  `ai.vision.invoke`, `ai.vision.ocr`, and `ai.vision.moderate`.
- Commands: `vision.analyze_image`, `vision.analyze_video`, `vision.ocr`,
  `vision.detect_objects`, `vision.moderate_visual`, and
  `vision.extract_visual_evidence`.
- DTOs: `VisualInput`, `VisualRegion`, `OcrTextSpan`, `DetectedObject`,
  `VisualModerationResult`, `VisualEvidenceRef`, and `VisionJob`.
- Policy: validate source handle permission, media size/duration, frame/page
  selection, sensitive category scope, moderation policy, resource budget,
  entitlement, and provider capability before dispatch.
- Trace/audit: record media hash, dimension/duration counters, region count,
  category ids, confidence bands, job refs, provider class, latency, and bounded
  errors only.

## Existing Macaca Platform Inventory

- The repo has generic service descriptors, service-call trace enforcement,
  unavailable clients, SDK facade patterns, permission command objects, and
  runtime-host composition roots that can host a future vision provider.
- Existing media/file/domain-pack work can provide content handles later, but
  no current evidence proves vision-specific DTOs, service provider, SDK helper,
  WASM ABI, audit redaction, or developer docs are complete.
- The application package admission and conformance checker patterns are useful
  for declared visual-source permissions and redaction profiles.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
