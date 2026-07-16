# Media Transcription Pack Design

## Context

`pack.media.transcription.v1` exposes transcription as a Macaca OS
serviceized capability. It lets applications transcribe audio/video, run live
streaming transcription, execute asynchronous batch transcription, diarize
speakers, label channels, align timestamps, apply redaction, export subtitles,
handoff translation, inspect jobs, and resolve transcript artifacts without
embedding cloud speech APIs, local model runtimes, storage providers, or
application-specific conversation workflows into generic OS layers.

Transcription is privacy-sensitive and often long-running. The pack treats raw
audio chunks, private video/audio, voice biometric features, speaker labels,
channel labels, PII, regulated recordings, generated transcripts, captions, and
provider job payloads as sensitive. Reads return bounded metadata and handles;
side effects use validated plans, idempotent requests, streaming session
handles, asynchronous job handles, and artifact boundaries.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Amazon Transcribe | Batch/streaming jobs, speaker partitioning, vocabulary, language support, PII redaction, subtitles, word timing/confidence | Batch/stream plans, speaker labels, vocabulary refs, redaction profiles, subtitle export plans, job/artifact handles |
| Google Cloud Speech-to-Text | Synchronous, long-running, streaming recognition, word time offsets, diarization, adaptation, recognition config | Recognition Strategy, long-running job, token timing, adaptation references, provider capability |
| Azure AI Speech | Batch and fast transcription, diarization, word-level timestamps, multi-file jobs, storage-backed artifacts | Batch job Strategy, source/artifact handles, job state, timestamp model |
| OpenAI audio transcription | Response formats, word/segment timestamp granularities, optional streaming, logprobs for supported models, diarized JSON for supported models | Transcript format plan, token/segment timing, confidence diagnostics, streaming session |
| Deepgram / AssemblyAI / Rev AI / Speechmatics | Live streaming, interim results, endpointing, diarization, entity/redaction, webhooks, subtitle exports, provider-specific model controls | Streaming session, partial result cursor, redaction plan, callback boundary, subtitle artifact |

The pack exposes provider-neutral contracts. Provider adapters translate to
cloud APIs, local speech models, browser/host speech engines, streaming sockets,
remote jobs, artifact stores, redaction services, translation services, or
unavailable providers. OS layers must not branch on provider names, model names,
vocabulary names, queue names, webhook names, speaker names, channel names, file
names, or business workflows.

## Goals

- Provide stable pack id `pack.media.transcription.v1` and command namespace
  `transcription.*`.
- Support provider inspection, source import/open, media metadata inspection,
  batch transcription planning/request, streaming session planning/start/append/
  finish/cancel, diarization/channel-label planning/request, timestamp
  alignment, transcript normalization, redaction planning/request,
  subtitle/caption export planning/request, translation handoff planning/
  request, job inspection, artifact handles, health, snapshots, and replay.
- Preserve privacy with consent policy, voice/biometric handling, PII
  redaction, regulated-recording gates, streaming retention, artifact scopes,
  callback boundaries, approval rules, bounded output, and sanitized audit.
- Keep concrete transcription providers behind replaceable service providers.
- Require developer documentation at
  `docs/developer-packs/media/transcription.md`.

## Non-Goals

- Do not implement concrete Amazon, Google, Azure, OpenAI, Deepgram, AssemblyAI,
  Rev AI, Speechmatics, local model, storage, moderation, translation, or export
  providers in this proposal.
- Do not define meeting, call-center, courtroom, medical dictation, subtitle
  editing, surveillance, voice identification, speaker verification, or
  application-specific workflows.
- Do not expose raw credentials, private audio/video, raw audio chunks, voice
  biometric features, raw transcripts containing PII, raw subtitles, raw
  provider payloads, manifests, package bytes, private keys, signatures, or
  unbounded transcript/audio data in observability.
- Do not silently transcribe, stream, diarize, redact, translate, export, or
  publish transcripts without typed plan/request, policy checks, consent or
  approval where required, version preconditions, and artifact retention.

## Ownership And Boundaries

- Pack id: `pack.media.transcription.v1`.
- Family: `media`.
- Backing service owner: transcription service provider.
- SDK surface: `sdk.packs.media.transcription`.
- Command namespace: `transcription.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges, artifact
  stores, redaction bridges, decorators, and sanitized diagnostics through
  approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `transcription.inspect_provider` | Inspect provider, language, model, streaming, diarization, timestamp, redaction, subtitle, export, and translation support | Returns sanitized capability, quota, lifecycle, health, and compatibility metadata |
| `transcription.import_source_request` | Import audio/video source from file/artifact handle | Requires artifact permission, consent policy, format validation, size/duration policy, and audit |
| `transcription.open_source` | Resolve transcription source handle and version metadata | Requires source scope and bounded metadata |
| `transcription.inspect_media` | Inspect media duration, channels, sample rate class, codec/container, language hints, and provenance | Requires metadata permission and redaction |
| `transcription.plan_batch` | Plan asynchronous transcription | Validates language, model class, channel mapping, diarization, timestamp granularity, redaction, resource budget, and approvals |
| `transcription.batch_request` | Execute a validated batch transcription plan | Requires plan handle, idempotency key, version preconditions, and audit |
| `transcription.plan_stream` | Plan live or incremental streaming transcription | Validates chunk format, interim results, endpointing, retention, network, consent, and resources |
| `transcription.start_stream` | Start a streaming transcription session | Returns streaming session handle and bounded initial status |
| `transcription.append_stream_chunk` | Append an audio chunk or chunk handle to a streaming session | Requires session state, sequence id, chunk bounds, redaction, and resource accounting |
| `transcription.finish_stream` | Finish a streaming session and finalize transcript artifacts | Requires session state and retention policy |
| `transcription.cancel_stream` | Cancel a streaming session | Returns bounded cancellation diagnostics and audit evidence |
| `transcription.plan_diarization` | Plan speaker/channel labeling or re-labeling | Validates speaker count hints, channel mapping, consent, and sensitivity |
| `transcription.diarization_request` | Execute diarization/channel-label plan | Returns transcript/job/artifact handles |
| `transcription.align_timestamps` | Align token, word, segment, or caption timestamps | Requires transcript/source handles, version preconditions, and timestamp support |
| `transcription.normalize_transcript` | Normalize transcript text, punctuation, casing, formatting, or confidence projection | Requires redaction policy and bounded output |
| `transcription.plan_redaction` | Plan PII/entity/phrase redaction | Validates redaction profile, locale, sensitivity, and approval |
| `transcription.redaction_request` | Execute transcript redaction | Returns redacted transcript artifact handles |
| `transcription.plan_subtitle_export` | Plan SRT/VTT/TTML-like subtitle or caption export | Validates timing, line length, language, redaction, and artifact retention |
| `transcription.subtitle_export_request` | Execute subtitle/caption export | Returns subtitle artifact handles |
| `transcription.plan_translation_handoff` | Plan transcript translation handoff to a translation provider | Validates target languages, redaction, consent, and provider availability |
| `transcription.translation_handoff_request` | Execute translation handoff via typed service boundary | Returns translation job/artifact handle without bypassing policy |
| `transcription.inspect_job` | Inspect asynchronous transcription, redaction, subtitle, or translation job status | Requires job scope and redaction |
| `transcription.get_artifact_handle` | Resolve transcript/subtitle/export artifact metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
partial/streaming/asynchronous results, typed denied/unavailable/unsupported/
conflict/stale-version/schema-mismatch/format-unsupported/language-unsupported/
model-unsupported/diarization-unsupported/timestamp-unsupported/redaction-
denied/translation-denied/export-denied/write-denied/artifact-denied/quota/
timeout/cancellation/approval-required/failure results, redaction profile,
idempotency semantics for side effects, job/session status semantics, and replay
metadata.

## DTO Model

Core DTOs:

- `TranscriptionScope`: provider scope, source handle, credential reference,
  network policy, artifact policy, consent policy, redaction policy, permission
  state, rate-limit profile, and health.
- `TranscriptionProviderCapability`: provider class, source formats, languages,
  model classes, batch support, streaming support, diarization support, channel
  labeling support, timestamp granularities, custom vocabulary/adaptation
  support, redaction support, subtitle export support, translation handoff
  support, auth modes, rate limits, lifecycle, and health.
- `TranscriptionSourceHandle`: source handle, provider scope, source artifact
  handle, media kind, format class, duration class, channel count class,
  sample-rate class, version hash, sensitivity class, provenance class,
  redaction class, and freshness.
- `TranscriptionMediaMetadata`: duration, codec/container, channels, sample
  rate class, language hint class, embedded caption presence, provenance
  handles, checksum handle, and redaction class.
- `TranscriptionPlan`: plan handle, source handle, operation kind, language
  profile, model class, timestamp granularity, diarization options, channel
  mapping, redaction profile, vocabulary references, version preconditions,
  resource estimate, required approvals, idempotency key, retention, and
  validation diagnostics.
- `TranscriptionStreamingSession`: session handle, plan handle, provider
  capability hash, state, sequence cursor, interim-result policy, endpointing
  class, retention, cancellation state, and redaction class.
- `TranscriptionAudioChunkHandle`: chunk handle, session handle, sequence id,
  duration class, size class, checksum handle, retention, and redaction class.
- `TranscriptDocument`: transcript handle, source handle, job/session handle,
  language profile, segment count class, token count class, speaker label
  class, channel label class, redaction class, confidence class, version hash,
  and artifact handle.
- `TranscriptSegment`: segment handle, transcript handle, start/end time, text
  projection, confidence class, speaker label handle, channel label handle,
  redaction class, and provenance pointer.
- `TranscriptToken`: token handle, segment handle, text projection, start/end
  time, confidence/logprob class, redaction class, and provenance pointer.
- `SpeakerLabel` and `ChannelLabel`: label handles, stable label class,
  sensitivity class, confidence class, and redaction class. They must not encode
  biometric identity unless a separate approved identity capability is declared.
- `TranscriptionRedactionProfile`: profile handle, entity classes, phrase
  classes, locale, replacement policy, audit reason, and approval state.
- `TranscriptionSubtitleExportPlan` and `TranscriptionTranslationHandoffPlan`:
  plan handles, source transcript handles, target format/language, timing
  constraints, redaction policy, resource estimate, retention, and diagnostics.
- `TranscriptionJobStatus`: job handle, command name, provider capability hash,
  state, progress class, queue class, cancellation state, result artifact
  handles, and redaction class.
- `TranscriptionArtifactHandle`: artifact handle, source operation/job handle,
  artifact kind, content type, size class, checksum handle, retention,
  provenance, redaction class, and replay pointer.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `transcription.provider.inspect`
- `transcription.source.import`
- `transcription.source.open`
- `transcription.media.read`
- `transcription.batch`
- `transcription.stream`
- `transcription.stream.append`
- `transcription.stream.cancel`
- `transcription.diarization`
- `transcription.timestamp.align`
- `transcription.normalize`
- `transcription.redaction`
- `transcription.subtitle.export`
- `transcription.translation.handoff`
- `transcription.job.read`
- `transcription.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, source handle, transcript handle when applicable,
  job/session handle when applicable, artifact handle when applicable, actor
  handle when available, credential reference, network policy, artifact policy,
  consent policy, redaction policy, and permission state.
- Side-effecting batch, stream, diarization, redaction, subtitle export, and
  translation handoff commands require plan/request separation, idempotency key,
  version preconditions, consent policy, artifact retention policy, and audit
  reason.
- Private conversations, voice biometric risk, minors, medical/legal/financial
  recordings, customer data, regulated calls, subtitles containing PII,
  external delivery, persistent transcripts, and translation handoff may require
  approval.
- Raw audio, private video, raw transcript text, subtitle text, speaker labels,
  channel labels, derived artifacts, streaming chunks, job outputs, and provider
  payloads require redaction and bounded output.
- Remote and streaming operations require network permission, provider quota,
  rate limits, timeout, cancellation, job/session inspection, and structured
  unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
language/model support, streaming support, batch support, diarization support,
timestamp granularities, vocabulary/adaptation support, redaction support,
subtitle export support, translation handoff support, permission scopes, policy
templates, resource limits, approval rules, provider capability hashes, health,
compatibility, diagnostics, examples, redaction profiles, and documentation
links.

The developer guide at `docs/developer-packs/media/transcription.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, source handles, media metadata, batch plans, streaming
  sessions, chunk handles, transcript documents, segments, tokens, speaker
  labels, channel labels, language profiles, vocabulary references, redaction
  profiles, subtitle export plans, translation handoff plans, jobs, artifacts,
  provider capabilities, and unavailable states
- plan/request lifecycle, streaming lifecycle, asynchronous job lifecycle,
  version conflicts, language/model mismatch, diarization limitations, timestamp
  granularities, confidence/logprob semantics, redaction, consent, approvals,
  quotas, provider replacement, trace/audit interpretation, and conformance
  tests

Examples must use synthetic audio/video sources, speakers, channels, chunks,
transcripts, jobs, subtitles, and artifacts. They must not include provider
names, real credentials, private conversations, voice biometric data, customer
data, raw transcripts containing PII, copyrighted recordings, raw provider
payloads, or workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `transcription_pack_declared`
- `transcription_pack_admission_validated`
- `transcription_provider_inspected`
- `transcription_source_imported`
- `transcription_source_opened`
- `transcription_media_inspected`
- `transcription_batch_planned`
- `transcription_batch_requested`
- `transcription_stream_planned`
- `transcription_stream_started`
- `transcription_stream_chunk_appended`
- `transcription_stream_finished`
- `transcription_stream_cancelled`
- `transcription_diarization_planned`
- `transcription_diarization_requested`
- `transcription_timestamps_aligned`
- `transcription_transcript_normalized`
- `transcription_redaction_planned`
- `transcription_redaction_requested`
- `transcription_subtitle_export_planned`
- `transcription_subtitle_export_requested`
- `transcription_translation_handoff_planned`
- `transcription_translation_handoff_requested`
- `transcription_job_inspected`
- `transcription_artifact_handle_resolved`
- `transcription_pack_policy_decision`
- `transcription_pack_service_call_requested`
- `transcription_pack_service_call_succeeded`
- `transcription_pack_service_call_failed`
- `transcription_pack_unavailable`
- `transcription_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, command
availability, provider health, policy template hash, resource counters, bounded
source/transcript/session/job/artifact summaries, event cursors, and sanitized
replay pointers. Snapshots must exclude raw credentials, private audio/video,
raw audio chunks, voice biometric features, raw transcripts containing PII,
subtitle text containing PII, raw provider payloads, manifests, package bytes,
private keys, signatures, and unbounded transcript/audio data.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, batch processors, streaming processors,
  diarizers, aligners, redactors, subtitle exporters, translation handoff
  adapters, artifact providers, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  consent, redaction, artifact retention, and output bounding wrap service
  calls.
- **Specification**: admission validates provider scope, source format,
  commands, permissions, language/model support, timestamp support, resource
  budget, consent, and compatibility.
- **Observer**: provider health, trace, audit, streaming partials, job status,
  and artifact lifecycle events are subscribable.
- **State**: streaming sessions and asynchronous jobs use explicit state
  machines for start, append, finish, cancel, failure, and replay.
- **Memento**: source version hashes, operation plans, stream cursors, job
  handles, artifact handles, snapshots, and replay pointers preserve recovery
  state.
- **Abstract Factory**: concrete transcription providers are created only by
  approved runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes a wrapper around one speech API. Mitigation:
  provider-neutral source/transcript/session/job/artifact DTOs and Strategy
  adapters.
- Risk: voice, transcript, or PII leaks. Mitigation: handles, consent policy,
  redaction profiles, bounded summaries, artifact boundaries, and strict
  observability exclusions.
- Risk: streaming creates hidden second paths. Mitigation: streaming append,
  finish, cancel, partial results, and events are typed service commands/events.
- Risk: long-running jobs cannot be recovered. Mitigation: job handles,
  idempotency keys, snapshots, cancellation, and replay pointers.
- Risk: SDK helpers bypass policy. Mitigation: helpers build canonical service
  commands and never call transcription APIs directly.
