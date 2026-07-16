# Media Video Pack

`pack.media.video.v1` describes provider-neutral video capabilities. The pack is
descriptor-only until a video provider is installed through the runtime
composition root.

## Manifest Declaration

Declare the pack as required only when video capability is mandatory for
readiness. Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.media.video.v1"]
```

## Permissions

Use the narrowest scope: `video.provider.inspect`, `video.import`,
`video.open`, `video.metadata.read`, `video.track.read`, `video.thumbnail`,
`video.transcode`, `video.segment`, `video.render`, `video.subtitle`,
`video.package`, `video.export`, `video.job.read`, and `video.artifact.read`.

## Capability Model

Macaca models video as scopes, opaque video handles, version hashes, metadata,
tracks, frame handles, timeline ranges, thumbnail plans, transcode plans,
segment plans, render plans, overlay operations, subtitle plans, package plans,
export plans, job statuses, and artifact handles. Raw frames, private video,
faces, voice biometric data, subtitle text containing PII, generated or edited
video bytes, delivery manifests, credentials, and provider payloads stay behind
provider adapters.

## Commands And Results

`video.inspect_provider`, `video.import_video_request`, `video.open_video`,
`video.inspect_metadata`, `video.inspect_tracks`, `video.plan_thumbnail`,
`video.thumbnail_request`, `video.plan_transcode`, `video.transcode_request`,
`video.plan_segment`, `video.segment_request`, `video.plan_render`,
`video.render_request`, `video.plan_subtitles`, `video.subtitles_request`,
`video.plan_package`, `video.package_request`, `video.plan_export`,
`video.export_request`, `video.inspect_job`, and `video.get_artifact_handle`
are descriptor-owned schema names. Result statuses include success, paged,
partial, asynchronous, denied, unavailable, unsupported, conflict,
stale-version, schema-mismatch, format-unsupported, codec-unsupported,
track-denied, subtitle-denied, render-denied, package-denied, export-denied,
write-denied, artifact-denied, quota, timeout, cancellation, approval-required,
and failure.

Plan commands are non-mutating and carry timeline ranges, track mappings,
subtitle redaction profiles, package policies, approval references,
idempotency keys, and resource bounds. Asynchronous jobs and derived outputs are
observed through job status DTOs and artifact handles.

## Platform Comparison

FFmpeg and GStreamer demux/mux/codecs/filtergraphs map to transcode, segment,
render, subtitle, and export plans. WebCodecs maps to codec capability and host
support discovery. AWS Elemental MediaConvert maps to package/job descriptors.
Cloudinary and Mux map to derived assets, adaptive playback manifests, and
artifact retention. Native presets, queues, URLs, raw frames, and provider error
payloads are intentionally not OS semantics.

## App-Facing Examples

- Inspect codec/container, track, render, subtitle, package, and export support.
- Import or open video by handle, then inspect metadata and tracks.
- Plan thumbnails, transcodes, segments, renders, subtitles, packages, and
  exports before issuing request commands.
- Poll job status through `video.inspect_job` and consume outputs by artifact
  handle.
- Handle unavailable, permission, track, subtitle, render, package, quota,
  timeout, and artifact diagnostics without provider-specific fallback APIs.

## App-Facing Example Matrix

Generic examples cover provider inspection, video import/open, metadata
inspection, track inspection, thumbnail planning/request, transcode
planning/request, segment planning/request, render planning/request, subtitle
planning/request, package planning/request, export planning/request, job
inspection, and artifact handles with synthetic video, track, timeline, job,
package, and artifact refs.

Diagnostic examples cover unavailable provider, missing video permission,
metadata redacted, unsupported format, unsupported codec, track denied,
subtitle redacted, stale version, render approval, package denied, export
denied, provider quota, network denied, CPU/GPU unavailable, job timeout, and
artifact denied. Diagnostics must not include provider names, credentials,
private videos, faces, voice biometric data, copyrighted video, raw frames, raw
subtitles, raw exports, or workflow-specific conventions.

## Trace And Audit

Traces should record declaration, admission decision, command name, video id,
version hash, track hash, timeline hash, job id, provider class, capability
hash, result status, and artifact id. They must not record raw frames, faces,
voice biometric data, subtitle PII, generated video bytes, credentials, provider
payloads, or unbounded output.

## Provider Authors

Conformance requires descriptor completeness, video/track/artifact scope
validation, format/codec/container support, metadata stripping, frame extraction
limits, track mapping, transcode validation, segment validation, render
validation, subtitle validation, package validation, export validation, artifact
redaction, bounded resources, policy hooks, trace/audit events, unavailable
behavior, snapshot/replay metadata, and redaction tests.
