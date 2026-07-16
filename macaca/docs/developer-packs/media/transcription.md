# Media Transcription Pack

`pack.media.transcription.v1` describes provider-neutral transcription
capabilities. The pack is descriptor-only until a transcription provider is
installed through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when transcription capability is mandatory for
readiness. Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.media.transcription.v1"]
```

## Permissions

Use the narrowest scope: `transcription.provider.inspect`,
`transcription.source.import`, `transcription.source.open`,
`transcription.media.read`, `transcription.batch`, `transcription.stream`,
`transcription.stream.append`, `transcription.stream.cancel`,
`transcription.diarization`, `transcription.timestamp.align`,
`transcription.normalize`, `transcription.redaction`,
`transcription.subtitle.export`, `transcription.translation.handoff`,
`transcription.job.read`, and `transcription.artifact.read`.

## Capability Model

Macaca models transcription as scopes, source handles, media metadata, batch
plans, streaming sessions, audio chunk handles, transcript documents, segments,
tokens, speaker labels, channel labels, language profiles, vocabulary
references, redaction profiles, subtitle export plans, translation handoff
plans, job statuses, and artifact handles. Raw audio/video, raw chunks,
speaker biometric features, raw transcript text, subtitle PII, provider models,
credentials, and provider payloads stay behind provider adapters.

## Commands And Results

`transcription.inspect_provider`, `transcription.import_source_request`,
`transcription.open_source`, `transcription.inspect_media`,
`transcription.plan_batch`, `transcription.batch_request`,
`transcription.plan_stream`, `transcription.start_stream`,
`transcription.append_stream_chunk`, `transcription.finish_stream`,
`transcription.cancel_stream`, `transcription.plan_diarization`,
`transcription.diarization_request`, `transcription.align_timestamps`,
`transcription.normalize_transcript`, `transcription.plan_redaction`,
`transcription.redaction_request`, `transcription.plan_subtitle_export`,
`transcription.subtitle_export_request`,
`transcription.plan_translation_handoff`,
`transcription.translation_handoff_request`, `transcription.inspect_job`, and
`transcription.get_artifact_handle` are descriptor-owned schema names. Result
statuses include success, paged, partial, streaming, asynchronous, denied,
unavailable, unsupported, conflict, stale-version, schema-mismatch,
format-unsupported, language-unsupported, model-unsupported,
diarization-unsupported, timestamp-unsupported, redaction-denied,
translation-denied, export-denied, write-denied, artifact-denied, quota,
timeout, cancellation, approval-required, and failure.

Streaming sessions are explicit state machines. Chunk append commands require
ordered chunk handles and idempotency keys. Transcript documents, segments, and
tokens expose refs and confidence metadata, not raw transcript text.

## Platform Comparison

Amazon Transcribe, Google Cloud Speech-to-Text, Azure AI Speech, OpenAI audio
transcription, Deepgram, AssemblyAI, Rev AI, Speechmatics, and local speech
models map to provider capability classes, batch/stream plans, language
profiles, vocabulary references, diarization, timestamp alignment, redaction,
subtitle export, and translation handoff. Provider model names, queues,
callbacks, raw transcripts, and native errors are intentionally not OS
semantics.

## App-Facing Examples

- Inspect batch, streaming, language, timestamp, diarization, redaction,
  subtitle, and translation support.
- Import or open a source by handle, then inspect media metadata.
- Plan batch or streaming transcription before submitting requests.
- Append stream chunks only through ordered chunk handles.
- Normalize, redact, export subtitles, and hand off translation through plans.
- Poll job status and consume artifacts by handle.
- Handle unavailable, language, model, diarization, timestamp, redaction,
  translation, quota, timeout, and artifact diagnostics generically.

## App-Facing Example Matrix

Generic examples cover provider inspection, source import/open, media
inspection, batch planning/request, stream planning/start/append/finish/cancel,
diarization planning/request, timestamp alignment, normalization, redaction
planning/request, subtitle export planning/request, translation handoff
planning/request, job inspection, and artifact handles with synthetic media,
stream, transcript, job, subtitle, translation, and artifact refs.

Diagnostic examples cover unavailable provider, missing source permission,
media redacted, unsupported format, unsupported language, unsupported model,
diarization unsupported, timestamp unsupported, redaction required,
approval required, provider quota, network denied, CPU/GPU unavailable, stream
timeout, chunk-order conflict, job timeout, translation denied, subtitle export
denied, and artifact denied. Diagnostics must not include provider names,
credentials, private conversations, voice biometric data, customer data, raw
transcripts containing PII, copyrighted recordings, raw provider payloads, or
workflow-specific conventions.

## Trace And Audit

Traces should record declaration, admission decision, command name, source id,
version hash, stream session id, chunk sequence, transcript id, job id, provider
class, capability hash, result status, and artifact id. They must not record raw
audio/video, raw chunks, voice biometrics, raw transcript text, subtitle PII,
credentials, provider payloads, or unbounded transcript data.

## Provider Authors

Conformance requires descriptor completeness, source/transcript/session/job and
artifact scope validation, format/language/model support, streaming state
machine enforcement, chunk ordering, diarization validation, timestamp
validation, redaction validation, subtitle export validation, translation
handoff validation, artifact redaction, bounded resources, policy hooks,
trace/audit events, unavailable behavior, snapshot/replay metadata, and
redaction tests.
