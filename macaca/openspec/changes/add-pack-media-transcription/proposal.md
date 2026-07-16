# Change: Add Media Transcription Pack

## Why

Developers need `pack.media.transcription.v1` as an industrial speech-to-text
capability for audio/video transcription, streaming transcription, asynchronous
batch jobs, speaker diarization, channel labeling, word/segment timestamps,
language detection, custom vocabulary and model adaptation hints, transcript
redaction, confidence diagnostics, subtitle/caption export, translation handoff,
artifact management, and replay diagnostics. It must not be a thin wrapper
around Amazon Transcribe, Google Cloud Speech-to-Text, Azure AI Speech, OpenAI
audio transcription, Deepgram, AssemblyAI, Rev AI, Speechmatics, or one local
speech model.

Transcription input can contain private voices, biometric identifiers, minors,
medical/legal/financial conversations, customer data, regulated recordings,
copyrighted media, location or device metadata, and sensitive captions. Macaca
must therefore expose transcription only through provider-neutral typed service
commands with declared permissions, consent policy, entitlement, resource
reservation, approval, redaction, artifact retention, trace, audit, health,
snapshot, replay, and structured unavailable behavior.

## Research And Supplier/API Baseline

Official and supplier references considered for this pack:

- Amazon Transcribe exposes batch and streaming transcription, speaker
  partitioning, language support, custom vocabulary, PII redaction, subtitle
  generation, and word-level output with timing/confidence metadata. References:
  https://docs.aws.amazon.com/cli/latest/reference/transcribe/start-transcription-job.html,
  https://docs.aws.amazon.com/transcribe/latest/dg/diarization.html,
  https://docs.aws.amazon.com/transcribe/latest/dg/pii-redaction.html, and
  https://docs.aws.amazon.com/transcribe/latest/dg/subtitles.html
- Google Cloud Speech-to-Text exposes synchronous, long-running, and streaming
  recognition, word time offsets, speaker diarization, adaptation, language
  configuration, and long-running operation metadata. References:
  https://docs.cloud.google.com/speech-to-text/docs/v1/speech-to-text-requests,
  https://docs.cloud.google.com/speech-to-text/docs/v1/async-time-offsets,
  https://docs.cloud.google.com/speech-to-text/docs/multiple-voices, and
  https://docs.cloud.google.com/speech-to-text/docs/reference/rest/v1/RecognitionConfig
- Azure AI Speech exposes batch transcription, fast transcription, diarization,
  word-level timestamps, multi-file job status, and storage-backed asynchronous
  processing. References:
  https://learn.microsoft.com/en-us/azure/ai-services/speech-service/batch-transcription,
  https://learn.microsoft.com/en-us/azure/ai-services/speech-service/batch-transcription-create,
  and https://learn.microsoft.com/en-us/azure/ai-services/speech-service/fast-transcription-create
- OpenAI audio transcription exposes response formats, word or segment
  timestamp granularities, optional streaming, log probability diagnostics for
  supported models, and diarized JSON for supported diarization models.
  Reference: https://platform.openai.com/docs/api-reference/audio/createTranscription
- Deepgram, AssemblyAI, Rev AI, and Speechmatics provide additional industrial
  baselines for live streaming, interim results, endpointing, diarization,
  entity detection/redaction, webhook/callback jobs, subtitle exports, and
  provider-specific model controls. These are provider baselines, not OS
  semantics.

Macaca maps these supplier concepts into provider-neutral transcription scope,
provider capability, media source handle, transcription plan, streaming session,
batch job, transcript segment, token/word timing, speaker label, channel label,
language profile, vocabulary/adaptation reference, redaction profile, subtitle
export plan, translation handoff plan, transcript artifact, provider
capability, event cursor, and diagnostics DTOs. Concrete cloud, local, browser,
streaming, storage, moderation, translation, and export providers stay behind
replaceable service providers.

## What Changes

- Add provider-neutral `pack.media.transcription.v1` under the `media` family.
- Define command namespace `transcription.*` for:
  - provider capability inspection
  - transcription source import/open and metadata inspection
  - batch transcription planning and requests
  - streaming transcription session planning, start, append, finish, and cancel
  - diarization/channel-label planning and requests
  - timestamp alignment and transcript normalization
  - redaction planning and requests
  - subtitle/caption export planning and requests
  - translation handoff planning and requests
  - transcript job/status inspection and artifact handle resolution
- Define DTOs for transcription scope, provider capability, source handle,
  media metadata, transcription plan, streaming session, audio chunk handle,
  transcript job status, transcript document, transcript segment, transcript
  token, speaker label, channel label, language profile, vocabulary reference,
  redaction profile, subtitle export plan, translation handoff plan, artifact
  handle, event cursor, and diagnostics.
- Define permission scopes, consent policy, voice/biometric handling, PII
  redaction, regulated-recording policy, streaming retention, callback/webhook
  boundaries, artifact retention, resource/entitlement behavior, SDK discovery,
  developer documentation, trace/audit events, snapshots, replay, and boundary
  gates.
- Require detailed developer documentation at
  `docs/developer-packs/media/transcription.md` before implementation
  completion.

## Impact

- Affected specs: `pack-media-transcription`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, transcription
  service provider or unavailable provider, runtime-host provider adapters,
  audio/video artifact bridges, redaction/moderation support, trace/audit
  schemas, replay tests, dependency-boundary gates, and developer
  documentation.
- Non-goals: no concrete Amazon/Google/Azure/OpenAI/Deepgram/AssemblyAI/Rev/
  Speechmatics/local model/storage/moderation/translation/export provider
  implementation in this proposal; no meeting app, call-center app, courtroom
  workflow, medical dictation workflow, subtitle editor, surveillance workflow,
  or application-specific transcription logic; no provider-name, model-name,
  queue-name, vocabulary-name, speaker-name, channel-name, or workflow-name
  routing in OS layers beyond declarative descriptor data; no raw credentials,
  private audio/video, raw audio chunks, voice biometric features, raw
  transcripts containing PII, raw provider payloads, manifests, package bytes,
  private keys, signatures, or unbounded transcript/audio data in observability;
  no SDK/shell/kernel provider construction; no fake success when provider,
  language, model, diarization, timestamp, redaction, subtitle, translation,
  permission, entitlement, approval, resource, or host support is absent.
