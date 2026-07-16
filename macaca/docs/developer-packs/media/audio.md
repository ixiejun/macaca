# Media Audio Pack

`pack.media.audio.v1` describes provider-neutral audio capabilities. The pack is
descriptor-only until an audio provider is installed through the runtime
composition root.

## Manifest Declaration

Declare the pack as required only when audio capability is mandatory for
readiness. Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.media.audio.v1"]
```

## Permissions

Use the narrowest scope: `audio.provider.inspect`, `audio.import`,
`audio.open`, `audio.metadata.read`, `audio.waveform.read`, `audio.transcode`,
`audio.segment`, `audio.filter`, `audio.mix`, `audio.synthesize`,
`audio.export`, and `audio.artifact.read`.

## Capability Model

Macaca models audio as scopes, opaque audio handles, version hashes, metadata,
waveform summaries, segments, filter operations, mix sources, mix graphs, voice
capabilities, synthesis plans, export plans, and artifact handles. Raw PCM
samples, private recordings, speaker biometric data, prompt text, generated
audio bytes, provider voices, credentials, and provider payloads stay behind
provider adapters.

## Commands And Results

`audio.inspect_provider`, `audio.import_audio_request`, `audio.open_audio`,
`audio.inspect_metadata`, `audio.inspect_waveform`, `audio.plan_transcode`,
`audio.transcode_request`, `audio.plan_segment`, `audio.segment_request`,
`audio.plan_filter`, `audio.filter_request`, `audio.plan_mix`,
`audio.mix_request`, `audio.plan_synthesis`, `audio.synthesis_request`,
`audio.plan_export`, `audio.export_request`, and `audio.get_artifact_handle`
are descriptor-owned schema names. Result statuses include success, paged,
partial, asynchronous, denied, unavailable, unsupported, conflict,
stale-version, schema-mismatch, format-unsupported, codec-unsupported,
metadata-denied, voice-denied, prompt-denied, synthesis-denied, export-denied,
write-denied, artifact-denied, quota, timeout, cancellation, approval-required,
and failure.

Plan commands are non-mutating and carry idempotency keys, segment ranges,
filter graphs, voice references, synthesis consent state, redaction profiles,
approval references, and resource bounds. Request commands execute only through
the canonical traced service path.

## Platform Comparison

FFmpeg demux/mux/codecs/filters map to transcode, segment, filter, mix, and
export plans. GStreamer pipeline graphs map to provider adapter strategies and
graph validation. Web Audio nodes map to bounded source/filter/mix abstractions.
libsndfile maps to local format and metadata inspection. OpenAI TTS, ElevenLabs,
Google Cloud Text-to-Speech, and Amazon Polly map to synthesis plans and voice
capability descriptors. Native graphs, provider voices, model names, raw prompts,
and provider error payloads are intentionally not OS semantics.

## App-Facing Examples

- Inspect codec/container and feature support before opening audio.
- Import or open audio by handle, then inspect metadata and waveform summaries.
- Plan transcode, segment, filter, mix, synthesis, and export operations before
  issuing request commands.
- Treat voice capability references as consent-gated metadata.
- Consume generated or exported outputs through artifact handles only.
- Handle unavailable, format, codec, voice, prompt, synthesis, quota, network,
  and artifact diagnostics without provider-specific fallback APIs.

## App-Facing Example Matrix

Generic examples cover provider inspection, audio import/open, metadata
inspection, waveform inspection, transcode planning/request, segment
planning/request, filter planning/request, mix planning/request, synthesis
planning/request, export planning/request, and artifact-handle consumption with
synthetic source, voice, graph, job, and artifact refs.

Diagnostic examples cover unavailable provider, missing audio permission,
metadata redacted, unsupported format, unsupported codec, stale version,
waveform redacted, voice denied, prompt denied, synthesis approval, export
denied, provider quota, network denied, CPU/GPU unavailable, and artifact
denied. Diagnostics must not include provider names, credentials, private
recordings, speaker biometric data, copyrighted audio, raw prompts, raw
generated audio, raw exports, or workflow-specific conventions.

## Trace And Audit

Traces should record declaration, admission decision, command name, audio id,
version hash, segment hash, graph hash, voice ref hash, provider class,
capability hash, result status, and artifact id. They must not record raw
samples, private recordings, speaker biometrics, raw prompts, generated audio,
credentials, provider payloads, or unbounded output.

## Provider Authors

Conformance requires descriptor completeness, audio and artifact scope
validation, codec/container support, metadata stripping, waveform projection
limits, transcode validation, segment validation, filter validation, mix
validation, synthesis safety, export validation, artifact redaction, bounded
resources, policy hooks, trace/audit events, unavailable behavior,
snapshot/replay metadata, and redaction tests.
