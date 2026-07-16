# Change: Add Media Video Pack

## Why

Developers need `pack.media.video.v1` as an industrial video capability for
video provider inspection, import/opening, metadata and track inspection,
thumbnail/proxy creation, frame extraction, transcoding, trimming/segmentation,
timeline rendering, overlays/composition, subtitle/caption handling, adaptive
streaming packaging, safe export, artifact management, and replay diagnostics.
It must not be a thin wrapper around FFmpeg, GStreamer, WebCodecs, AWS
Elemental MediaConvert, Cloudinary, Mux, or one video processing library.

Video can contain private people, faces, voice, location metadata, copyrighted
content, regulated recordings, subtitles, embedded audio, and generated or
edited material. Processing can leak frames, alter evidence, publish content,
or produce expensive long-running jobs. Macaca must therefore expose video
operations only through provider-neutral typed service commands with
permission, policy, entitlement, resource, approval, content safety, metadata
stripping, artifact retention, trace, audit, health, snapshot, replay, and
structured unavailable behavior.

## Research And Supplier/API Baseline

Official and supplier references considered for this pack:

- FFmpeg exposes demux/mux, codec conversion, filtergraphs, trimming, frame
  extraction, subtitles, HLS/DASH packaging, and broad format/container support.
  References: https://ffmpeg.org/ffmpeg.html,
  https://ffmpeg.org/ffmpeg-formats.html, and
  https://ffmpeg.org/ffmpeg-codecs.html
- GStreamer exposes pipeline/element graph media processing, encoding/muxing,
  streaming, filters, and hardware-accelerated provider patterns. Reference:
  https://gstreamer.freedesktop.org/documentation/
- WebCodecs exposes browser-level `VideoEncoder`, `VideoDecoder`,
  `VideoFrame`, and encoded chunk abstractions without mandating a codec,
  making it a useful host-capability baseline. Reference:
  https://www.w3.org/TR/webcodecs/
- AWS Elemental MediaConvert exposes job-based cloud transcoding, output groups,
  HLS/DASH packaging, captions/subtitles, thumbnails, queues, status, and
  idempotent request behavior. Reference:
  https://docs.aws.amazon.com/mediaconvert/latest/apireference/jobs.html
- Cloudinary and Mux provide remote video transformation, adaptive streaming,
  thumbnails, playback/delivery artifacts, and derived-asset behavior. These
  are provider baselines, not OS semantics.

Macaca maps these supplier concepts into provider-neutral video scope,
provider capability, video handle, stream/track metadata, timeline range,
frame handle, thumbnail/proxy plan, transcode plan, segment plan, render plan,
subtitle track, overlay/composition operation, streaming package plan, export
plan, artifact handle, provider capability, version/freshness metadata, and
diagnostics DTOs. Concrete FFmpeg, GStreamer, WebCodecs, MediaConvert,
Cloudinary, Mux, storage, moderation, and export providers stay behind
replaceable providers.

## What Changes

- Add provider-neutral `pack.media.video.v1` under the `media` family.
- Define command namespace `video.*` for:
  - provider capability inspection
  - video import/open and metadata/track inspection
  - thumbnail/proxy/frame extraction planning and requests
  - transcode, trim, split, concatenate, filter, subtitle, overlay, and timeline
    render planning and requests
  - adaptive streaming packaging for HLS/DASH-like outputs where supported
  - export planning, export requests, artifact handle resolution, snapshots,
    and replay
- Define DTOs for video scope, provider capability, video handle, media
  metadata, video track, audio track reference, subtitle track, frame handle,
  timeline range, thumbnail plan, transcode plan, segment plan, render graph,
  overlay operation, package plan, export plan, artifact handle, job status,
  event cursor, and diagnostics.
- Define permission scopes, policy defaults, video/track/artifact scopes,
  metadata stripping, face/voice/sensitive-video policy, subtitles/captions
  redaction, generated/edited-content provenance, approval rules, resource/
  entitlement behavior, SDK discovery, developer documentation, trace/audit
  events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/media/video.md` before implementation completion.

## Impact

- Affected specs: `pack-media-video`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, video media service
  provider or unavailable provider, runtime-host provider adapters,
  artifact/redaction/moderation support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete FFmpeg/GStreamer/WebCodecs/MediaConvert/Cloudinary/
  Mux/storage/moderation/export provider implementation in this proposal; no
  video editor, livestreaming app, meeting app, surveillance workflow, social
  media workflow, movie workflow, broadcast workflow, or application-specific
  render logic; no provider-name, codec-name, track-name, preset-name, queue-
  name, or workflow-name routing in OS layers beyond declarative descriptor
  data; no raw credentials, private videos, raw frames, faces, voice biometric
  features, subtitles containing PII, raw generated/edited video, raw provider
  payloads, manifests, package bytes, private keys, signatures, or unbounded
  frame/pixel data in observability; no SDK/shell/kernel provider construction;
  no fake success when provider, codec, container, track, caption, packaging,
  permission, entitlement, approval, resource, or host support is absent.
