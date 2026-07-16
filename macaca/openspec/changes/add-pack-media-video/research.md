# Media Video Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.media.video.v1`. Video support must expose demux/mux, codec/container,
filtergraph, trim, frame extraction, caption/subtitle, packaging, thumbnail,
derived asset, playback manifest, export, and diagnostics through serviceized
commands. It must not become a video editor, livestreaming product, meeting,
surveillance, social, movie, broadcast, or provider-specific preset workflow.

## Source Baseline

- FFmpeg CLI, formats, and filters:
  <https://ffmpeg.org/ffmpeg.html>
  <https://ffmpeg.org/ffmpeg-formats.html>
  <https://ffmpeg.org/ffmpeg-filters.html>
- GStreamer elements and hardware-accelerated playback:
  <https://gstreamer.freedesktop.org/documentation/application-development/basics/elements.html>
  and
  <https://gstreamer.freedesktop.org/documentation/tutorials/playback/hardware-accelerated-video-decoding.html>
- WebCodecs:
  <https://developer.mozilla.org/en-US/docs/Web/API/WebCodecs_API>
  and <https://www.w3.org/TR/webcodecs/>
- AWS Elemental MediaConvert jobs and output groups:
  <https://docs.aws.amazon.com/mediaconvert/latest/ug/setting-up-a-job.html>
  and <https://docs.aws.amazon.com/mediaconvert/latest/ug/outputs-file-ABR.html>
- Cloudinary and Mux video delivery:
  <https://cloudinary.com/documentation/video_manipulation_and_delivery>
  <https://cloudinary.com/documentation/adaptive_bitrate_streaming>
  <https://www.mux.com/docs/core/stream-video-files>
  <https://www.mux.com/docs/api-reference/image/thumbnails/get-thumbnail>

## Supplier API Notes

- FFmpeg contributes demux/mux, codecs, containers, filtergraphs, trimming,
  frame extraction, captions/subtitles, HLS/DASH-like packaging, metadata, and
  structured process failures. Macaca should model transform/package plans, not
  expose CLI arguments.
- GStreamer contributes pipeline graphs, elements, pads, encoders/decoders,
  muxers, filters, streaming, hardware acceleration, zero-copy behavior, and
  asynchronous processing. Macaca should expose provider capability, graph
  state, and resource diagnostics.
- WebCodecs contributes `VideoEncoder`, `VideoDecoder`, `VideoFrame`, encoded
  chunks, browser codec variability, and safety constraints. Macaca should
  model codec capability and frame/chunk handles without browser-native objects.
- AWS Elemental MediaConvert contributes jobs, inputs, output groups, HLS/DASH
  packaging, captions, thumbnails, queues, status, idempotency, quota, and error
  behavior. Macaca should model async job handles, output manifests, and queue
  diagnostics.
- Cloudinary and Mux contribute derived assets, adaptive playback, thumbnails,
  manifests, playback identifiers, signed/public playback policy, and remote
  availability diagnostics.

## Macaca-Owned Abstractions

`pack.media.video.v1` should define `VideoAsset`, `VideoStream`,
`VideoCodecProfile`, `VideoContainerProfile`, `VideoTransformPlan`,
`VideoFilter`, `VideoSegment`, `VideoFrameExtraction`, `VideoCaptionTrack`,
`VideoPackagingPlan`, `VideoThumbnail`, `VideoPlaybackManifest`,
`VideoExportArtifact`, `VideoJobState`, and `VideoProviderCapability`.

The DTOs must carry source handles, stream metadata, codec/container support,
time ranges, filter graphs, frame extraction bounds, caption/subtitle metadata,
packaging targets, thumbnail plans, playback policy, async state, resource
budgets, provider capability hashes, redaction profiles, and replay pointers.
Raw video bytes, raw provider payloads, private frames, credentials, codec
preset pass-through, and unbounded frame/caption dumps are rejected.

## Explicit Non-Goals

- Do not implement concrete FFmpeg, GStreamer, WebCodecs, MediaConvert,
  Cloudinary, Mux, storage, safety, or export providers in this research phase.
- Do not define video-editor, livestreaming, meeting, surveillance, social,
  movie, broadcast, moderation, or application-specific workflows in OS layers.
- Do not expose raw provider presets, queue names, playback ids, filtergraph
  strings, codec-specific command lines, or provider-native payloads as stable
  SDK contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, file handles, media audio, media image, media transcription, and
  media rendering proposals provide reusable substrate.
- Current evidence does not prove video DTOs, providers, SDK helpers, WASM ABI,
  tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
