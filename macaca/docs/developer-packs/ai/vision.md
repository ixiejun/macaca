# AI Vision Pack

`pack.ai.vision.v1` provides provider-neutral image analysis, video analysis,
OCR, object detection, visual moderation, and visual evidence extraction. The
pack is descriptor-only until a serviceized vision provider is registered.

Applications pass media handles and region refs; they do not pass raw pixels,
frames, OCR text, biometrics, or provider payloads through OS diagnostics.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.ai.vision.v1"]
```

Optional declarations degrade with `ai_vision_provider_not_installed`.

## Permission Scopes

- `ai.vision.invoke`: image and video analysis.
- `ai.vision.ocr`: OCR and layout extraction.
- `ai.vision.moderate`: visual moderation and sensitive-category decisions.

## Commands

- `vision.analyze_image`: analyzes an image handle.
- `vision.analyze_video`: starts or inspects a bounded video analysis job.
- `vision.ocr`: returns OCR span refs and layout refs.
- `vision.detect_objects`: returns detected object refs and normalized regions.
- `vision.moderate_visual`: returns moderation result refs.
- `vision.extract_visual_evidence`: returns redacted evidence handles.

## DTOs And Results

Core DTOs include `VisualInput`, `VisualRegion`, `OcrTextSpan`,
`DetectedObject`, `VisualModerationResult`, `VisualEvidenceRef`, and
`VisionJob`. Statuses cover success, partial, denied, unavailable, unsupported,
conflict, quota exceeded, invalid region, moderation blocked, job pending,
cancelled jobs, and provider failure.

## Examples

Minimal declaration:

```toml
[service_contract]
optional_packs = ["pack.ai.vision.v1"]
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.ai.vision.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "ai_vision_provider_not_installed"
}
```

Canonical OCR command payload:

```json
{
  "subject_ref": "media:image-ref",
  "parameters": {
    "input_ref": "visual-input-ref",
    "region_ref": "normalized-region-ref"
  },
  "idempotency_key": "vision-ocr-key"
}
```

## Trace And Audit

Trace evidence may include media refs, region refs, job refs, evidence refs,
moderation action codes, progress counters, and descriptor hashes. It must not
include raw image bytes, video frames, OCR text, face/biometric payloads,
credentials, or provider-native responses.

## Provider Replacement

Provider classes include `hosted-model`, `local-runtime`,
`moderation-service`, `mock`, and `unavailable`. Concrete OCR, visual
moderation, or multimodal adapters stay behind service runtime registration.
