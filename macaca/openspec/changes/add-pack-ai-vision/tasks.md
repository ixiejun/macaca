## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Record the borrowed platform patterns relevant to `pack.ai.vision.v1` and map them to Macaca descriptors, permissions, policy, service calls, and audit records.
- [x] 1.3 Inventory existing service descriptors, SDK clients, optional packages, plugins, and unavailable providers that can back vision service provider.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And Schema

- [x] 2.1 Define provider-neutral command DTOs for: `vision.analyze_image`, `vision.analyze_video`, `vision.ocr`, `vision.detect_objects`, `vision.moderate_visual`, `vision.extract_visual_evidence`.
- [x] 2.2 Define typed success, partial, denied, unavailable, unsupported, conflict, quota, and failure result DTOs.
- [x] 2.3 Define descriptor metadata for pack id, family, lifecycle, stability, command schemas, permissions, policy template, resource budget, data governance, SDK metadata, compatibility, and diagnostics.
- [x] 2.4 Add stable descriptor hashing and version compatibility checks.
- [x] 2.5 Add unit tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema compatibility.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for scopes: `ai.vision.invoke`, `ai.vision.ocr`, `ai.vision.moderate`.
- [x] 3.2 Add policy checks before side effects and resource reservation before provider calls.
- [x] 3.3 Add entitlement checks and explicit unavailable/denied diagnostics for missing provider, missing permission, missing entitlement, disabled host capability, and unsupported command.
- [x] 3.4 Add approval behavior for sensitive, external, host, identity, financial, irreversible, or long-running side effects.
- [x] 3.5 Add tests proving denied/unavailable paths do not call concrete providers.

## 4. Service Provider Or Unavailable Provider

- [x] 4.1 Implement or bind vision service provider through the service runtime; do not construct providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, and bounded streaming behavior where applicable.
- [x] 4.3 Add structured provider capability reporting so discovery can distinguish available, degraded, preview, unavailable, unsupported, and retired states.
- [x] 4.4 Add mock and unavailable providers for deterministic tests.

## 5. SDK, Admission, Examples, And Documentation

- [x] 5.1 Extend SDK discovery for `pack.ai.vision.v1` with command schemas, examples, availability, diagnostics, docs metadata, provider class, and compatibility.
- [x] 5.2 Extend application admission so required declarations block when unavailable and optional declarations degrade explicitly.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls for declared callable commands.
- [x] 5.4 Add examples for analyze image, analyze video, ocr, detect objects using generic data and without hardcoded application or provider behavior.

## 6. Trace, Audit, Replay, And Gates

- [x] 6.1 Emit sanitized declaration, admission, policy, entitlement, resource, service-call, health, snapshot, and unavailable events.
- [x] 6.2 Add replay tests proving `pack.ai.vision.v1` calls are trace-addressable through the canonical service path.
- [x] 6.3 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete providers.
- [x] 6.4 Add no-direct-provider-call gates and canonical execution-path tests for all commands.
- [x] 6.5 Run `openspec validate add-pack-ai-vision --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create or update the detailed developer guide for `pack.ai.vision.v1` under `docs/developer-packs/`, covering purpose, manifest declaration, permission scopes, command DTOs, result DTOs, examples, unavailable diagnostics, trace/audit behavior, and provider replacement notes.
- [x] 7.2 Add at least one minimal app-facing example and one provider/unavailable diagnostic example that use generic data and do not hardcode application business logic.
- [x] 7.3 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-ai-vision` complete.

## 8. Industrial Vision Semantics

- [x] 8.1 Model `VisualInput`, `VisualRegion`, `OcrTextSpan`, `DetectedObject`, `VisualModerationResult`, `VisualEvidenceRef`, and `VisionJob`.
- [x] 8.2 Add coordinate-system and region-normalization tests for images, pages, frames, rotation, and scaling.
- [x] 8.3 Add OCR layout tests preserving page/block/line/span order with bounded text redaction.
- [x] 8.4 Add async video job tests for start, inspect, partial result, timeout, cancellation, and unavailable provider.
- [x] 8.5 Add moderation and sensitive visual category policy tests before provider invocation.
