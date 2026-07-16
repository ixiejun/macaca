# Media Video Pack Design

## Context

`pack.media.video.v1` exposes video operations as a Macaca OS serviceized
capability. It lets applications inspect, thumbnail, transcode, trim, segment,
render, subtitle, package, export, and replay video work without embedding
FFmpeg, GStreamer, WebCodecs, AWS MediaConvert, Cloudinary, Mux, storage,
moderation, or application-specific video workflows into generic OS layers.

Video is long-running, multi-track, codec/container-dependent, and expensive.
The pack treats raw frames, private video, faces, audio tracks, subtitles,
metadata, generated/edited content, and delivery artifacts as sensitive data.
Reads return bounded metadata and handles; side effects use validated plans,
idempotent requests, asynchronous job handles, and artifact boundaries.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| FFmpeg | Demux/mux, codecs, filtergraphs, trim, frame extraction, subtitles, HLS/DASH-like packaging | Codec/container capability, track metadata, filter/render graph, package/export plan |
| GStreamer | Pipeline graphs, elements, encoders/decoders, muxers, streaming, hardware acceleration | Processing Strategy, graph validation, streaming/asynchronous artifact behavior |
| WebCodecs | Browser `VideoEncoder`, `VideoDecoder`, `VideoFrame`, encoded chunks, low-level codec access | Host capability, frame handle, browser-safe decode/encode Strategy |
| AWS Elemental MediaConvert | Job-based transcode, output groups, HLS/DASH, captions, thumbnails, queues, status | Remote job plan/request, package plan, job status, idempotency key |
| Cloudinary / Mux | Remote transformations, adaptive streaming, thumbnails, playback/delivery artifacts | Derived artifact handle, delivery policy, remote quota/availability diagnostics |

The pack exposes provider-neutral contracts. Provider adapters translate to
local pipelines, browser codecs, remote job services, cloud delivery providers,
safety classifiers, storage/artifact providers, or unavailable providers. OS
layers must not branch on provider names, codec names, presets, queue names,
file names, broadcast workflows, or business video workflows.

## Goals

- Provide stable pack id `pack.media.video.v1` and command namespace `video.*`.
- Support provider inspection, video import/open, metadata/track inspection,
  thumbnail/proxy/frame extraction planning and requests, transcode planning/
  requests, segment/trim planning/requests, filter/render/composition planning/
  requests, subtitle/caption planning/requests, adaptive package planning/
  requests, export planning/requests, job inspection, artifact handles,
  snapshots, health, and replay diagnostics.
- Preserve safety with video/track/artifact scopes, face/voice/sensitive-video
  policy, subtitle redaction, metadata stripping, generated/edited-content
  provenance, approvals, quotas, bounded output, and sanitized audit.
- Keep concrete video providers behind replaceable service providers.
- Require developer documentation at `docs/developer-packs/media/video.md`.

## Non-Goals

- Do not implement concrete FFmpeg, GStreamer, WebCodecs, MediaConvert,
  Cloudinary, Mux, storage, moderation, or export providers in this proposal.
- Do not define video editor, livestreaming, meeting, surveillance, social
  media, movie, broadcast, avatar, or application-specific render workflows.
- Do not expose raw credentials, private videos, raw frames, faces, voice
  biometric features, subtitles containing PII, generated/edited video bytes,
  raw provider payloads, manifests, package bytes, private keys, signatures, or
  unbounded frame/pixel data in observability.
- Do not silently transcode, trim, render, subtitle, package, publish, export,
  strip metadata, or transmit video without typed plan/request, policy checks,
  version preconditions, and approval where required.

## Ownership And Boundaries

- Pack id: `pack.media.video.v1`.
- Family: `media`.
- Backing service owner: video media service provider.
- SDK surface: `sdk.packs.media.video`.
- Command namespace: `video.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges, artifact
  stores, safety/moderation bridges, decorators, and sanitized diagnostics
  through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `video.inspect_provider` | Inspect provider, codec/container, track, render, package, and export support | Returns sanitized capability, quota, lifecycle, health, and compatibility metadata |
| `video.import_video_request` | Import video from file/artifact handle | Requires artifact permission, format validation, size/duration policy, and audit |
| `video.open_video` | Resolve video handle and version metadata | Requires video scope and bounded metadata |
| `video.inspect_metadata` | Inspect duration, dimensions, frame rate, tracks, codec/container, subtitles, tags, and provenance | Requires metadata permission and redaction |
| `video.inspect_tracks` | Inspect bounded video/audio/subtitle/data track metadata | Requires track permission, projection limits, and redaction |
| `video.plan_thumbnail` | Plan thumbnails, poster frames, proxy frames, or frame extraction | Validates time/frame ranges, output format, metadata policy, and resources |
| `video.thumbnail_request` | Execute thumbnail/frame extraction | Returns bounded artifact handles |
| `video.plan_transcode` | Plan codec/container/resolution/bitrate/frame-rate conversion | Validates codec support, resource budget, packaging intent, and approvals |
| `video.transcode_request` | Execute a validated transcode plan | Requires plan handle, idempotency key, version preconditions, and audit |
| `video.plan_segment` | Plan trim, split, concatenate, or scene/silence-based segmentation | Validates timeline ranges, track mapping, output policy, and resources |
| `video.segment_request` | Execute validated segmentation | Returns bounded artifact handles |
| `video.plan_render` | Plan timeline render, filters, overlays, watermarks, burn-in subtitles, or track composition | Validates render graph, sources, timing, fonts, tracks, safety, and resources |
| `video.render_request` | Execute validated render plan | Returns job/artifact handles and diagnostics |
| `video.plan_subtitles` | Plan subtitle/caption import, conversion, burn-in, or sidecar export | Validates caption format, language, redaction, accessibility metadata, and approvals |
| `video.subtitles_request` | Execute validated subtitle operation | Returns artifact or track handles |
| `video.plan_package` | Plan HLS/DASH-like adaptive package | Validates rendition ladder, manifests, captions, DRM/entitlement references, and retention |
| `video.package_request` | Execute package plan | Returns manifest/artifact handles and job status |
| `video.plan_export` | Plan video export or delivery artifact | Validates format, quality, metadata retention, sensitivity, and approvals |
| `video.export_request` | Execute export/delivery | Returns bounded artifact handle and audit metadata |
| `video.inspect_job` | Inspect asynchronous video job status | Requires job scope and redaction |
| `video.get_artifact_handle` | Resolve video/thumbnail/proxy/render/package/export artifact metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
paged/partial/asynchronous results, typed denied/unavailable/unsupported/
conflict/stale-version/schema-mismatch/format-unsupported/codec-unsupported/
track-denied/subtitle-denied/render-denied/package-denied/export-denied/
write-denied/artifact-denied/quota/timeout/cancellation/approval-required/
failure results, redaction profile, idempotency semantics for side effects, job
status semantics, and replay metadata.

## DTO Model

Core DTOs:

- `VideoScope`: provider scope handle, video handle, source artifact handle,
  credential reference, network policy, artifact policy, safety policy,
  permission state, rate-limit profile, and health.
- `VideoProviderCapability`: provider class, import/open support, metadata
  support, track support, thumbnail support, transcode support, segment support,
  render support, subtitle support, packaging support, export support, codecs,
  containers, hardware acceleration state, auth modes, rate limits, lifecycle,
  and health.
- `VideoHandle`: video handle, provider scope, source artifact handle,
  container, codec summary, version hash, duration class, dimensions class,
  frame-rate class, track count class, sensitivity class, provenance class,
  redaction class, and freshness.
- `VideoMetadata`: duration, dimensions, frame rate, bitrate class, codec/
  container, track summaries, subtitle presence, tags presence, chapter
  presence, provenance handles, checksum handle, and redaction class.
- `VideoTrack`: track handle, video handle, track kind, codec, language,
  dimensions/sample profile, duration class, default/enabled state, sensitivity
  class, and redaction class.
- `VideoFrameHandle`: frame handle, video handle, timestamp, frame index class,
  dimensions class, keyframe state, thumbnail artifact handle, and redaction
  class.
- `VideoTimelineRange`: range handle, video handle, start/end time, frame range,
  track selection, reason code, and redaction class.
- `VideoThumbnailPlan`, `VideoTranscodePlan`, `VideoSegmentPlan`,
  `VideoRenderPlan`, `VideoSubtitlePlan`, `VideoPackagePlan`, and
  `VideoExportPlan`: plan handles, source handles, operation list hashes,
  version preconditions, track mapping, output profile, resource estimate,
  required approvals, idempotency key, retention, redaction, and validation
  diagnostics.
- `VideoOverlayOperation`: operation handle, source artifact handle, target
  range/region, z-order class, opacity class, blend mode, font/style
  references, and validation metadata.
- `VideoJobStatus`: job handle, command name, provider capability hash, state,
  progress class, queue class, cancellation state, result artifact handles, and
  redaction class.
- `VideoArtifactHandle`: artifact handle, source video/operation/job handle,
  artifact kind, content type, duration class, dimensions class, codec/
  container, size class, checksum handle, retention, provenance, redaction
  class, and replay pointer.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `video.provider.inspect`
- `video.import`
- `video.open`
- `video.metadata.read`
- `video.track.read`
- `video.thumbnail`
- `video.transcode`
- `video.segment`
- `video.render`
- `video.subtitle`
- `video.package`
- `video.export`
- `video.job.read`
- `video.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, video handle, track handle when applicable, artifact
  handle when applicable, actor handle when available, credential reference,
  network policy, artifact policy, safety policy, and permission state.
- Side-effecting thumbnail, transcode, segment, render, subtitle, package, and
  export commands require plan/request separation, idempotency key, version
  preconditions, metadata retention policy, safety policy, artifact policy, and
  audit reason.
- Private videos, faces, voice, minors, legal/medical/financial recordings,
  copyrighted media, subtitles containing PII, generated/edited content,
  external delivery, metadata stripping, and destructive edits may require
  approval.
- Raw frames, private videos, subtitles, audio tracks, derived artifacts, job
  outputs, and provider payloads require redaction and bounded output. Raw
  frame/pixel data must not enter observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, job-status inspection, and structured unavailable
  behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
codec/container support, metadata support, track support, thumbnail support,
transcode support, segment support, render support, subtitle support, package
support, export support, permission scopes, policy templates, resource limits,
approval rules, provider capability hashes, health, compatibility, diagnostics,
examples, redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/media/video.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, video handles, metadata, tracks, codecs, containers, frame
  rates, dimensions, frame handles, timeline ranges, thumbnail plans, transcode
  plans, segment plans, render graphs, subtitle plans, package plans, export
  plans, job status, artifacts, provider capabilities, and unavailable states
- plan/request lifecycle, asynchronous job lifecycle, version conflicts, codec/
  container mismatch, track mapping, metadata stripping, subtitle redaction,
  generated/edited-content provenance, approvals, quotas, provider replacement,
  trace/audit interpretation, and conformance tests

Examples must use synthetic videos, tracks, subtitles, frames, jobs, and
artifacts. They must not include provider names, real credentials, private
videos, faces, voice biometric data, copyrighted video, raw frames, raw
subtitles, raw exports, or workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `video_pack_declared`
- `video_pack_admission_validated`
- `video_provider_inspected`
- `video_imported`
- `video_opened`
- `video_metadata_inspected`
- `video_tracks_inspected`
- `video_thumbnail_planned`
- `video_thumbnail_requested`
- `video_transcode_planned`
- `video_transcode_requested`
- `video_segment_planned`
- `video_segment_requested`
- `video_render_planned`
- `video_render_requested`
- `video_subtitles_planned`
- `video_subtitles_requested`
- `video_package_planned`
- `video_package_requested`
- `video_export_planned`
- `video_export_requested`
- `video_job_inspected`
- `video_artifact_handle_resolved`
- `video_pack_policy_decision`
- `video_pack_service_call_requested`
- `video_pack_service_call_succeeded`
- `video_pack_service_call_failed`
- `video_pack_unavailable`
- `video_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, video codec/
container and version hashes, command availability, provider health, policy
template hash, resource counters, bounded metadata/track/operation/job/artifact
summaries, event cursors, and sanitized replay pointers. Snapshots must exclude
raw credentials, private videos, raw frames, faces, voice biometric features,
subtitles containing PII, generated/edited video bytes, raw exports, raw
provider payloads, manifests, package bytes, private keys, signatures, and
unbounded frame/pixel data.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, demuxers, transcoders, segmenters, render
  engines, subtitle handlers, package providers, export providers, and
  unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  metadata stripping, content safety, artifact retention, and output redaction
  wrap service calls.
- **Specification**: admission validates provider scope, video format, command
  availability, permissions, version preconditions, codec/container support,
  track mapping, resource budget, and compatibility.
- **Observer**: provider health, trace, audit, job status, and artifact
  lifecycle events are subscribable.
- **Memento**: video version hashes, operation plans, job handles, artifact
  handles, snapshots, and replay pointers preserve recovery state.
- **Abstract Factory**: concrete video providers are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes an FFmpeg/MediaConvert wrapper. Mitigation:
  provider-neutral video/track/job/artifact DTOs and Strategy adapters.
- Risk: private frames, subtitles, or voices leak. Mitigation: handles,
  redaction, bounded summaries, artifact boundaries, and strict observability
  exclusions.
- Risk: long-running jobs cannot be recovered. Mitigation: job handles,
  idempotency keys, snapshots, cancellation, and replay pointers.
- Risk: video operations consume excessive CPU/GPU/storage. Mitigation:
  resource estimates, asynchronous artifacts, quotas, cancellation, and
  provider capability reporting.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call video APIs directly.
