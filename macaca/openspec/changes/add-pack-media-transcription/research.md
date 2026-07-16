# Media Transcription Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.media.transcription.v1`. Transcription support must expose batch,
streaming, word timing, diarization, channel identification, custom vocabulary,
redaction, subtitles, confidence, translation, callback/webhook, and diagnostics
through typed service commands. It must not become a meeting, call-center,
courtroom, medical, surveillance, subtitle-editor, voice-ID, or provider-model
workflow.

## Source Baseline

- Amazon Transcribe features and diarization:
  <https://aws.amazon.com/transcribe/features/>
  and <https://docs.aws.amazon.com/transcribe/latest/dg/diarization.html>
- Google Cloud Speech-to-Text:
  <https://cloud.google.com/speech-to-text>
- Azure AI Speech batch and fast transcription:
  <https://learn.microsoft.com/en-us/azure/ai-services/speech-service/batch-transcription-create>
  and
  <https://learn.microsoft.com/en-us/azure/ai-services/speech-service/fast-transcription-create>
- OpenAI speech-to-text:
  <https://developers.openai.com/api/docs/guides/speech-to-text>
  and
  <https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create/>
- Deepgram, AssemblyAI, Rev AI, and Speechmatics:
  <https://developers.deepgram.com/docs/live-streaming-audio>
  <https://developers.deepgram.com/docs/interim-results>
  <https://docs.rev.ai/api/features>
  <https://docs.speechmatics.com/speech-to-text/features/diarization>

## Supplier API Notes

- Amazon Transcribe contributes batch/streaming jobs, speaker partitioning,
  channel identification, custom vocabulary, language support, PII redaction,
  subtitles, word timing, confidence metadata, and job status.
- Google Cloud Speech-to-Text contributes synchronous, long-running, and
  streaming recognition, word offsets, diarization/channel behavior, adaptation,
  recognition config, operation metadata, and quota/error behavior.
- Azure AI Speech contributes batch transcription, fast transcription,
  diarization, word-level timestamps, multi-file jobs, storage-backed inputs and
  outputs, status polling, and failure behavior.
- OpenAI contributes file transcription, response formats, word/segment
  timestamps, optional streaming, log probability diagnostics, diarized output
  where supported, model capability variation, and structured errors.
- Deepgram, AssemblyAI, Rev AI, and Speechmatics contribute live streaming,
  interim/final results, endpointing, entity/redaction, webhook/callback jobs,
  subtitles, custom vocabulary/dictionary, and provider-specific model controls
  that must be normalized into capability reports.

## Macaca-Owned Abstractions

`pack.media.transcription.v1` should define `TranscriptionSource`,
`TranscriptionJob`, `TranscriptionStream`, `RecognitionConfig`,
`VocabularyHint`, `TranscriptSegment`, `TranscriptToken`,
`SpeakerSegment`, `ChannelSegment`, `TranscriptRedaction`,
`SubtitleExport`, `TranscriptionCallback`, `TranscriptionQualityReport`, and
`TranscriptionProviderCapability`.

The DTOs must carry audio/video source handles, mode, language, channel count,
timing granularity, diarization capability, vocabulary policy, interim/final
state, confidence, redaction, subtitle formats, callback metadata, provider
capability hashes, resource budgets, and replay pointers. Raw transcript text,
raw audio, provider payloads, credentials, speaker biometrics, and unbounded
segments are rejected where policy requires sanitization.

## Explicit Non-Goals

- Do not implement concrete Amazon, Google, Azure, OpenAI, Deepgram,
  AssemblyAI, Rev AI, Speechmatics, local, browser, storage, or subtitle
  providers in this research phase.
- Do not define meeting, call-center, courtroom, medical, surveillance,
  subtitle-editor, translation workflow, speaker-identification, or
  application-specific transcript semantics in OS layers.
- Do not expose raw provider model/vocabulary/queue controls or provider
  transcript payloads as stable SDK contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, file handles, notification/webhook patterns, media audio, media
  video, and AI speech proposals provide reusable substrate.
- Current evidence does not prove transcription DTOs, providers, SDK helpers,
  WASM ABI, tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
