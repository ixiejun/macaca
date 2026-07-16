# AI Vision Pack Design

## Context

`pack.ai.vision.v1` is a child proposal of the developer-pack industrial capability catalog. It makes image/video understanding, OCR, object detection, moderation, and visual evidence extraction available through Macaca's microkernel-compatible service model. The pack must behave like an operating-system capability: declarative in application manifests, policy checked before use, provider-neutral at the SDK boundary, observable through trace/audit evidence, and replaceable by built-in, plugin, remote, mock, or unavailable providers.

## Research Synthesis

Mature platforms converge on the same pattern:

- Platform privacy models: raw user data must not enter logs or diagnostics when processed by intelligent services.
- Android runtime permission pattern: AI operations that touch private data inherit source permissions.
- Windows capability declaration pattern: model-backed operations must be visible in app capability metadata.
- Apple entitlement/privacy pattern: sensitive processing requires policy and developer-declared purpose.

Macaca should adapt those ideas as pack descriptors, service commands, policy decorators, and replayable audit records. The design does not copy platform API names; it preserves Macaca's own microkernel boundary and service runtime execution path.

## Goals

- Provide image/video understanding, OCR, object detection, moderation, and visual evidence extraction.
- Expose stable pack id `pack.ai.vision.v1`, command namespace `vision.*`, permission scopes, SDK metadata, health, snapshot, and unavailable diagnostics.
- Route every operation through `SystemFacade` or focused SDK clients into the canonical service runtime path.
- Return structured `unavailable`, `unsupported`, `denied`, `conflict`, `quota_exceeded`, and `failure` results.
- Emit sanitized trace/audit events for declaration, admission, policy, entitlement/resource decisions, service calls, provider health, snapshots, and replay.

## Non-Goals

- Do not implement an application-specific feature, workflow, UI, or business rule.
- Do not put concrete provider construction in the microkernel, SDK, shells, or generic application framework.
- Do not expose raw secrets, prompts, manifests, package bytes, credentials, raw provider payloads, private keys, signatures, or unbounded output in logs, traces, snapshots, SDK diagnostics, or examples.
- Do not silently degrade to a different provider, fake success, or bypass policy when the declared provider is absent.

## Ownership And Boundaries

- Pack id: `pack.ai.vision.v1`.
- Family: `ai`.
- Backing service owner: vision service provider.
- SDK surface: `sdk.packs.ai.vision`.
- Command namespace: `vision.*`.
- Microkernel ownership: identity, service-call evidence, policy facade, trace/audit primitives only.
- Application framework ownership: manifest declaration, app-scoped permission declarations, lifecycle/effective-capability projection.
- Runtime-host ownership: provider registration and decorators only through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `vision.analyze_image` | Typed command/result DTO for analyze image | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `vision.analyze_video` | Typed command/result DTO for analyze video | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `vision.ocr` | Typed command/result DTO for ocr | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `vision.detect_objects` | Typed command/result DTO for detect objects | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `vision.moderate_visual` | Typed command/result DTO for moderate visual | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `vision.extract_visual_evidence` | Typed command/result DTO for extract visual evidence | Requires trace, policy decision, structured result, and sanitized audit evidence |

Every command must define a typed command DTO, typed success result, typed partial-result shape when streaming or asynchronous, typed error result, redaction policy, idempotency semantics where side effects exist, and replay metadata.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `ai.vision.invoke`
- `ai.vision.ocr`
- `ai.vision.moderate`

Policy defaults:

- Scope every command to application id, tenant id, session id, task id, and trace id when available.
- Apply model/provider neutrality, budget/rate policy, prompt/output redaction, evaluation trace, and no model-name routing in OS code.
- Require explicit approval for commands that cross user-sensitive, financial, identity, device, host, network, external-recipient, or irreversible side-effect boundaries.
- Enforce resource budgets for time, memory, storage, network, provider quota, streaming output, and retained snapshots.
- Return `denied` for policy rejection, `unavailable` for absent providers or entitlements, `unsupported` for unknown commands, and `quota_exceeded` for bounded-resource rejection.

## SDK Discovery And Examples

SDK discovery must return pack metadata, lifecycle, service mappings, command schemas, permission scopes, policy templates, examples, availability, health, provider class, version compatibility, and sanitized diagnostics.

- SDK helper example: `sdk.packs.ai.vision.analyze_image(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.ai.vision.analyze_video(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.ai.vision.ocr(command)` builds a canonical traced service call; it never constructs providers.

Examples must use generic handles and synthetic data. They must not bake in application names, provider names, credentials, business workflows, or domain-specific routing.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `vision_pack_declared`
- `vision_pack_admission_validated`
- `vision_pack_policy_decision`
- `vision_pack_service_call_requested`
- `vision_pack_service_call_succeeded`
- `vision_pack_service_call_failed`
- `vision_pack_unavailable`
- `vision_pack_snapshot_recorded`

Events include pack id, service id, command name, trace id, application/session/task/tenant identifiers when available, policy decision, provider class, latency, bounded resource counters, stable capability hash, and bounded error code. Snapshots include descriptor version, provider health, command availability, policy template hash, and sanitized replay pointers.

## Implementation Slices

1. Descriptor and contract slice: pack descriptor, command schemas, permissions, policy template, health/snapshot DTOs, unavailable diagnostics.
2. Admission and resolver slice: required/optional declaration handling, lifecycle checks, service mapping checks, permission validation, effective capability memento.
3. Service slice: vision service provider command handlers or unavailable provider, lifecycle, health, snapshot, shutdown, and structured error behavior.
4. SDK slice: discovery APIs, typed command helper builders, examples, diagnostics, and Null Object behavior.
5. Observability slice: trace/audit events, replay tests, snapshot sanitization, and metrics.
6. Gates slice: OpenSpec validation, DTO compatibility, dependency-boundary tests, no-direct-provider-call tests, canonical execution-path tests, file-size gates.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders; it does not construct providers.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider selection, unavailable behavior, policy routing, and version compatibility are replaceable.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **Specification**: admission validates pack id, lifecycle, commands, permissions, policy, and service mapping.
- **Observer**: trace, audit, health, and service events are subscribable and replayable.
- **Memento**: effective capability reports and snapshots preserve bounded recovery state.
- **Abstract Factory**: optional providers register only through approved composition roots.

## Risks And Mitigations

- Risk: broad capability becomes an OS-layer business workflow. Mitigation: keep the pack contract generic and place domain/provider semantics in replaceable services.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only build canonical service-call commands and are covered by no-direct-provider-call gates.
- Risk: preview or unavailable providers look callable. Mitigation: availability validators require descriptor, service registration, command schema, permission, entitlement, and health evidence before callable state.
- Risk: observability leaks sensitive data. Mitigation: event schema permits identifiers, hashes, counters, bounded codes, and sanitized snippets only.

## Supplier Capability Matrix

| Supplier pattern | Macaca contract element |
| --- | --- |
| OCR pages/blocks/lines | `OcrResult`, `VisualRegion`, text-span references |
| Object detection | `DetectedObject`, bounding region, confidence band |
| Moderation labels | `VisualModerationResult`, policy category, action hint |
| Video analysis jobs | `VisionJob`, frame interval, async status, cancellation |
| Visual evidence extraction | `VisualEvidenceRef`, provenance, redaction profile |

## Domain Model

- `VisualInput`: image, video, frame, page, or content reference with media hash,
  dimensions, duration, frame/page selection, and redaction profile.
- `VisualRegion`: normalized coordinate system, bounding polygon/box, page/frame
  id, rotation, scale, and confidence band.
- `OcrTextSpan`: text reference, language hint, region reference, confidence,
  reading order, and redaction profile.
- `DetectedObject`: category, region, confidence, source frame/page, and policy
  sensitivity metadata.
- `VisionJob`: asynchronous analysis job with state, progress, cancellation, and
  bounded diagnostics.

## Additional Industrial Commands

- `vision.start_video_job`: start asynchronous video analysis with frame
  sampling and cancellation policy.
- `vision.inspect_job`: query job state, progress, partial result references,
  and sanitized diagnostics.
- `vision.normalize_regions`: convert provider coordinates to canonical
  coordinate metadata for downstream consumers.

## Vision-Specific Risks

- Risk: visual pack becomes media storage. Mitigation: inputs and outputs use
  content/evidence references; media persistence remains owned by media or file
  services.
- Risk: sensitive visual detection bypasses policy. Mitigation: moderation and
  sensitive visual categories require explicit scope and policy templates.
- Risk: coordinate ambiguity breaks downstream tools. Mitigation: every region
  declares coordinate system, frame/page id, rotation, and scale.
