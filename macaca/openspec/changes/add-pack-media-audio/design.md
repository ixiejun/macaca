# Media Audio Pack Design

## Context

`pack.media.audio.v1` exposes audio operations as a Macaca OS serviceized
capability. It lets applications inspect, transcode, segment, normalize, filter,
mix, synthesize, export, and replay audio work without embedding FFmpeg,
GStreamer, Web Audio, libsndfile, OpenAI TTS, ElevenLabs, Google TTS, Amazon
Polly, storage, moderation, or application-specific audio workflows into
generic OS layers.

Audio is time-based, codec/container-dependent, and often privacy-sensitive.
The pack treats raw recordings, speaker identity, voice biometric features,
prompts, generated speech, waveforms, and derived artifacts as sensitive data.
Reads return bounded metadata and handles; side effects use validated plans and
idempotent requests.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| FFmpeg | Demux/mux, codec conversion, filters, resampling, trimming, loudness normalization, mixing, metadata | Codec/container capability, filter operation, segment plan, export plan |
| GStreamer | Pipeline graphs, elements, pads, encoders/decoders, mixers, streaming, device/transport adapters | Mix graph, processing Strategy, streaming/asynchronous artifact behavior |
| Web Audio API | Audio nodes, buffers, gain, filters, analyzers, offline rendering | Provider-neutral graph model, analyzer/waveform summary, offline render plan |
| libsndfile | Local audio file read/write, format/subtype inspection, PCM-oriented processing | Local provider capability, stream metadata, read/write compatibility |
| OpenAI / ElevenLabs / Google TTS / Amazon Polly | Text-to-speech synthesis, voices/models, formats, streaming, quotas | Voice synthesis plan, voice capability, generated audio provenance |

The pack exposes provider-neutral contracts. Provider adapters translate to
local processing libraries, browser/offline audio graphs, remote TTS providers,
storage/artifact providers, safety classifiers, or unavailable providers. OS
layers must not branch on provider names, model names, voice names, file names,
codec names, music/podcast/call-center workflows, or business audio workflows.

## Goals

- Provide stable pack id `pack.media.audio.v1` and command namespace `audio.*`.
- Support provider inspection, audio import/open, metadata inspection, waveform
  and loudness inspection, transcode planning/requests, segment/trim planning/
  requests, normalize/filter planning/requests, mix planning/requests, synthesis
  planning/requests, export planning/requests, artifact handles, snapshots,
  health, and replay diagnostics.
- Preserve safety with audio/artifact scopes, voice/speaker safety, consent and
  copyright metadata, generated-voice provenance, prompt redaction, approvals,
  quotas, bounded output, and sanitized audit.
- Keep concrete audio providers behind replaceable service providers.
- Require developer documentation at `docs/developer-packs/media/audio.md`.

## Non-Goals

- Do not implement concrete FFmpeg, GStreamer, Web Audio, libsndfile, OpenAI,
  ElevenLabs, Google TTS, Polly, storage, moderation, or export providers in
  this proposal.
- Do not define transcription, speech recognition, diarization, speaker ID,
  voice cloning, music generation, podcast, call-center, meeting, or audio
  editor workflows.
- Do not expose raw credentials, raw prompts, private recordings, speaker
  biometric features, generated audio bytes, raw provider payloads, prompts,
  manifests, package bytes, private keys, signatures, or unbounded PCM/sample
  data in observability.
- Do not silently synthesize, mix, normalize, publish, export, watermark, strip
  metadata, or transmit audio without typed plan/request, policy checks,
  consent/copyright metadata, version preconditions, and approval where
  required.

## Ownership And Boundaries

- Pack id: `pack.media.audio.v1`.
- Family: `media`.
- Backing service owner: audio media service provider.
- SDK surface: `sdk.packs.media.audio`.
- Command namespace: `audio.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges, artifact
  stores, voice/safety bridges, decorators, and sanitized diagnostics through
  approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `audio.inspect_provider` | Inspect provider, codec/container, filter, mix, synthesis, and export support | Returns sanitized capability, quota, lifecycle, health, and compatibility metadata |
| `audio.import_audio_request` | Import audio from file/artifact handle | Requires artifact permission, format validation, size/duration policy, and audit |
| `audio.open_audio` | Resolve audio handle and version metadata | Requires audio scope and bounded metadata |
| `audio.inspect_metadata` | Inspect duration, sample rate, channels, codec, container, tags, loudness presence, and provenance | Requires metadata permission and redaction |
| `audio.inspect_waveform` | Inspect bounded waveform/loudness/silence summary | Requires projection limits, privacy policy, and no raw samples |
| `audio.plan_transcode` | Plan codec/container/sample-rate/channel conversion | Validates codec support, resource budget, metadata policy, and approvals |
| `audio.transcode_request` | Execute a validated transcode plan | Requires plan handle, idempotency key, version preconditions, and audit |
| `audio.plan_segment` | Plan trim, split, concatenate, or silence-based segmentation | Validates time ranges, source versions, output policy, and resources |
| `audio.segment_request` | Execute validated segmentation | Returns bounded artifact handles |
| `audio.plan_filter` | Plan normalize, resample, gain, fade, equalizer, denoise, or channel operations | Validates filter graph, resource budget, and output policy |
| `audio.filter_request` | Execute validated filter plan | Returns artifact handles and diagnostics |
| `audio.plan_mix` | Plan multi-source mix graph | Validates source handles, timing, gain, pan, fade, sample-rate compatibility, and rights metadata |
| `audio.mix_request` | Execute validated mix plan | Returns mixed artifact handle and audit metadata |
| `audio.plan_synthesis` | Plan text-to-speech synthesis | Validates prompt handle, voice/model capability, consent/safety, output profile, and approvals |
| `audio.synthesis_request` | Execute speech synthesis | Returns generated audio artifact and provenance metadata |
| `audio.plan_export` | Plan audio export or delivery artifact | Validates format, quality, metadata retention, sensitivity, and approvals |
| `audio.export_request` | Execute export/delivery | Returns bounded artifact handle and audit metadata |
| `audio.get_artifact_handle` | Resolve audio/segment/mix/generated/export artifact metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
paged/partial/asynchronous results, typed denied/unavailable/unsupported/
conflict/stale-version/schema-mismatch/format-unsupported/codec-unsupported/
metadata-denied/voice-denied/prompt-denied/synthesis-denied/export-denied/
write-denied/artifact-denied/quota/timeout/cancellation/approval-required/
failure results, redaction profile, idempotency semantics for side effects, and
replay metadata.

## DTO Model

Core DTOs:

- `AudioScope`: provider scope handle, audio handle, source artifact handle,
  credential reference, network policy, artifact policy, voice/safety policy,
  permission state, rate-limit profile, and health.
- `AudioProviderCapability`: provider class, import/open support, metadata
  support, waveform/loudness support, transcode support, segment support,
  filter support, mix support, synthesis support, export support, codecs,
  containers, sample-rate/channel support, auth modes, rate limits, lifecycle,
  and health.
- `AudioHandle`: audio handle, provider scope, source artifact handle,
  container, codec, version hash, duration class, channel layout class,
  sample-rate class, sensitivity class, provenance class, redaction class, and
  freshness.
- `AudioMetadata`: duration, sample rate, channels, bit depth class, bitrate
  class, codec/container, tags presence, loudness summary handle, waveform
  summary handle, provenance handles, checksum handle, and redaction class.
- `AudioWaveformSummary`: summary handle, audio handle, time window, peak/RMS
  classes, silence ranges, loudness class, sample projection resolution, and
  redaction class.
- `AudioSegment`: segment handle, source audio handle, start/end time, duration
  class, reason code, version hash, and redaction class.
- `AudioFilterOperation`: operation handle, operation kind, target audio/segment
  handle, parameters handle, compatibility hash, and validation metadata.
- `AudioMixSource`: source handle, audio/segment artifact handle, start time,
  gain class, pan class, fade handles, rights/consent class, and redaction class.
- `AudioMixGraph`: graph handle, source handles, filter operations, output
  profile, compatibility hash, resource estimate, and validation diagnostics.
- `AudioVoiceCapability`: voice handle, provider scope, language/locale,
  voice class, model capability hash, streaming support, consent policy, and
  redaction class.
- `AudioSynthesisPlan`: plan handle, prompt handle, voice capability hash,
  output profile, safety policy, provenance policy, required approvals,
  idempotency key, and validation diagnostics.
- `AudioExportPlan`: plan handle, source audio/artifact handle, output format,
  codec/container profile, quality class, metadata retention policy, delivery
  policy, retention, redaction, required approvals, idempotency key, and
  validation diagnostics.
- `AudioArtifactHandle`: artifact handle, source audio/operation handle,
  artifact kind, content type, duration class, codec/container, size class,
  checksum handle, retention, provenance, redaction class, and replay pointer.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `audio.provider.inspect`
- `audio.import`
- `audio.open`
- `audio.metadata.read`
- `audio.waveform.read`
- `audio.transcode`
- `audio.segment`
- `audio.filter`
- `audio.mix`
- `audio.synthesize`
- `audio.export`
- `audio.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, audio handle, artifact handle when applicable,
  actor handle when available, credential reference, network policy, artifact
  policy, voice/safety policy, and permission state.
- Side-effecting transcode, segment, filter, mix, synthesis, and export commands
  require plan/request separation, idempotency key, version preconditions,
  metadata retention policy, consent/copyright metadata, voice safety policy,
  artifact policy, and audit reason.
- Private recordings, human voices, speaker identity, minors, legal/medical/
  financial calls, copyrighted music, raw prompts, generated voices, external
  delivery, metadata stripping, and destructive edits may require approval.
- Raw audio samples, waveforms above bounded summaries, prompts, private
  recordings, generated audio bytes, derived artifacts, and provider payloads
  require redaction and bounded output. Raw PCM/sample data must not enter
  observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, voice/content safety checks, and structured unavailable
  behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
codec/container support, metadata support, waveform/loudness support, transcode
support, segment support, filter support, mix support, synthesis support, export
support, permission scopes, policy templates, resource limits, approval rules,
provider capability hashes, health, compatibility, diagnostics, examples,
redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/media/audio.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, audio handles, metadata, codecs, containers, sample rates,
  channels, waveform summaries, loudness reports, segments, filter operations,
  mix graphs, voice capabilities, synthesis plans, export plans, artifacts,
  provider capabilities, and unavailable states
- plan/request lifecycle, version conflicts, codec/container mismatch, metadata
  stripping, prompt redaction, voice consent/safety, generated-voice provenance,
  approvals, quotas, provider replacement, trace/audit interpretation, and
  conformance tests

Examples must use synthetic audio, prompts, voices, waveform summaries, and
artifacts. They must not include provider names, real credentials, private
recordings, speaker biometric data, copyrighted audio, raw prompts, raw
generated audio, raw exports, or workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `audio_pack_declared`
- `audio_pack_admission_validated`
- `audio_provider_inspected`
- `audio_imported`
- `audio_opened`
- `audio_metadata_inspected`
- `audio_waveform_inspected`
- `audio_transcode_planned`
- `audio_transcode_requested`
- `audio_segment_planned`
- `audio_segment_requested`
- `audio_filter_planned`
- `audio_filter_requested`
- `audio_mix_planned`
- `audio_mix_requested`
- `audio_synthesis_planned`
- `audio_synthesis_requested`
- `audio_export_planned`
- `audio_export_requested`
- `audio_artifact_handle_resolved`
- `audio_pack_policy_decision`
- `audio_pack_service_call_requested`
- `audio_pack_service_call_succeeded`
- `audio_pack_service_call_failed`
- `audio_pack_unavailable`
- `audio_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, audio codec/
container and version hashes, command availability, provider health, policy
template hash, resource counters, bounded metadata/waveform/operation/artifact
summaries, event cursors, and sanitized replay pointers. Snapshots must exclude
raw credentials, raw prompts, private recordings, speaker biometric data, raw
generated audio, raw exports, raw provider payloads, manifests, package bytes,
private keys, signatures, and unbounded PCM/sample data.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, codec readers, transcoders, segmenters,
  filter engines, mix engines, synthesis providers, export providers, and
  unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  metadata stripping, voice safety, prompt redaction, artifact retention, and
  output redaction wrap service calls.
- **Specification**: admission validates provider scope, audio format, command
  availability, permissions, version preconditions, codec/container support,
  voice safety policy, resource budget, and compatibility.
- **Observer**: provider health, trace, audit, and artifact lifecycle events are
  subscribable.
- **Memento**: audio version hashes, operation plans, mix graphs, synthesis
  plans, export plans, artifact handles, waveform summaries, snapshots, and
  replay pointers preserve recovery state.
- **Abstract Factory**: concrete audio providers are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes an FFmpeg/OpenAI wrapper. Mitigation: provider-neutral
  audio/operation/artifact/voice DTOs and Strategy adapters.
- Risk: private recordings or speaker identity leak. Mitigation: handles,
  bounded waveform summaries, redaction, artifact boundaries, and strict
  observability exclusions.
- Risk: misleading or unauthorized generated voices. Mitigation: voice consent
  policy, prompt handles, generated-voice provenance, approval, and audit.
- Risk: audio processing consumes excessive CPU/GPU/memory. Mitigation:
  resource estimates, streaming/asynchronous artifacts, quotas, cancellation,
  and provider capability reporting.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call audio APIs directly.
