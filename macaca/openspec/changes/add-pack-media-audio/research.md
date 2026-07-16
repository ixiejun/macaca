# Media Audio Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.media.audio.v1`. Audio support must expose decode, encode, transform,
mix, normalize, segment, synthesize, metadata, export, and diagnostics through
serviceized commands. It must not implement transcription, voice cloning,
podcast, music, call-center, or provider-specific voice workflows in OS-layer
code.

## Source Baseline

- FFmpeg CLI, formats, and filters:
  <https://ffmpeg.org/ffmpeg.html>
  <https://ffmpeg.org/ffmpeg-formats.html>
  <https://ffmpeg.org/ffmpeg-filters.html>
- GStreamer pipeline and element model:
  <https://gstreamer.freedesktop.org/documentation/application-development/basics/elements.html>
  and
  <https://gstreamer.freedesktop.org/documentation/additional/design/overview.html>
- W3C/MDN Web Audio API and `OfflineAudioContext`:
  <https://www.w3.org/TR/webaudio-1.1/>
  and <https://developer.mozilla.org/en-US/docs/Web/API/OfflineAudioContext>
- libsndfile API:
  <https://libsndfile.github.io/libsndfile/api.html>
- OpenAI audio/TTS, ElevenLabs TTS, Google Cloud Text-to-Speech, and Amazon
  Polly:
  <https://developers.openai.com/api/docs/guides/audio>
  <https://elevenlabs.io/docs/overview/capabilities/text-to-speech>
  <https://docs.cloud.google.com/text-to-speech/docs/list-voices-and-types>
  <https://docs.aws.amazon.com/polly/latest/dg/using-speechmarks.html>

## Supplier API Notes

- FFmpeg contributes demux/mux, codecs, resampling, filtergraphs, trimming,
  segmentation, loudness normalization, mixing, metadata, container operations,
  and structured process failures. Macaca should model an audio operation plan,
  not expose raw command-line arguments.
- GStreamer contributes pipeline graphs, elements, pads, encoders/decoders,
  mixers, filters, streaming, async lifecycle, and hardware/provider
  variability. Macaca should model graph capability, streaming handles, and
  provider health.
- Web Audio contributes source nodes, buffer graphs, gain/filter/analyser nodes,
  offline rendering, origin/host safety constraints, and browser resource
  limits. Macaca should expose graph-style transformations without browser node
  types becoming the stable DTO.
- libsndfile contributes local sampled-file read/write, format/subtype
  inspection, PCM-oriented access, and limited metadata. Macaca should model
  format capability and metadata limitations explicitly.
- TTS providers contribute text-to-speech synthesis, voices/models, output
  formats, streaming, speech marks/timestamps, SSML-like controls, quotas, and
  safety/consent constraints. Macaca should expose voice capability metadata and
  synthesis policy without provider/model/voice-specific routing.

## Macaca-Owned Abstractions

`pack.media.audio.v1` should define `AudioAsset`, `AudioStream`,
`AudioCodecProfile`, `AudioContainerProfile`, `AudioTransformPlan`,
`AudioFilter`, `AudioMixPlan`, `AudioSegment`, `AudioLoudnessReport`,
`AudioMetadata`, `AudioSynthesisRequest`, `AudioVoiceCapability`,
`AudioExportArtifact`, and `AudioProviderCapability`.

The DTOs must carry source handles, codec/container constraints, stream
metadata, sample rate/channel/bit-depth, transform graph, filter/mix bounds,
TTS voice capability, speech-mark support, output artifact handles, resource
budgets, provider capability hashes, redaction profiles, and replay pointers.
Raw audio bytes, raw prompts, provider payloads, credentials, voice-cloning
material, and unbounded waveform/sample dumps are rejected.

## Explicit Non-Goals

- Do not implement concrete FFmpeg, GStreamer, Web Audio, libsndfile, OpenAI,
  ElevenLabs, Google TTS, Polly, storage, or export providers in this research
  phase.
- Do not define transcription, speech recognition, diarization, speaker ID,
  voice cloning, music generation, podcast, audiobook, call-center, or
  application-specific audio workflows in OS layers.
- Do not expose raw filtergraph strings, command-line arguments, provider
  voice ids, or provider-native responses as stable SDK contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, file handles, secrets-reference handles, and media transcription
  adjacency provide reusable substrate.
- Current evidence does not prove audio DTOs, providers, SDK helpers, WASM ABI,
  tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
