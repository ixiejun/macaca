# AI Speech Pack Research

## Purpose

This note records borrowed platform patterns, Macaca mapping, existing platform
inventory, and GitNexus memo evidence for `pack.ai.speech.v1`. The pack must
provide speech-to-text, text-to-speech, voice discovery, speech translation, and
timing alignment through provider-neutral service commands. It must not become
media storage, voice-provider business logic, or raw audio leakage.

## Source Baseline

- OpenAI speech-to-text and text-to-speech documentation:
  <https://platform.openai.com/docs/guides/speech-to-text>
  and <https://platform.openai.com/docs/guides/text-to-speech>
- Azure AI Speech documentation:
  <https://learn.microsoft.com/en-us/azure/ai-services/speech-service/>
- Google Cloud Speech-to-Text and Text-to-Speech documentation:
  <https://cloud.google.com/speech-to-text/docs>
  and <https://cloud.google.com/text-to-speech/docs>
- AWS Transcribe and Polly documentation:
  <https://docs.aws.amazon.com/transcribe/latest/dg/what-is-transcribe.html>
  and <https://docs.aws.amazon.com/polly/latest/dg/what-is.html>
- Platform microphone/speech permission models inform source-permission
  inheritance and declared purpose requirements.

## Borrowed Platform Patterns

- Speech APIs converge on audio references, streaming frames, batch jobs,
  transcript segments, timestamps, confidence, language detection, diarization,
  channel separation, voice catalogs, synthesis controls, output formats, and
  generated audio artifacts.
- Providers differ in voice ids, styles, locale coverage, pronunciation controls,
  diarization, word timing, and streaming behavior. Macaca should expose
  provider-neutral voice descriptors and capability reports.
- Streaming speech requires sequence numbers, partial/final frames, late-frame
  handling, cancellation, and finalization evidence.
- Speech translation and alignment cross language and timing boundaries, so
  policy must validate source permissions, output retention, and redaction.
- Generated speech bytes and raw audio are artifacts, not trace payloads.

## Macaca Mapping

- Descriptor: `pack.ai.speech.v1`, command namespace `speech.*`, scopes
  `ai.speech.recognize`, `ai.speech.synthesize`, and `ai.speech.translate`.
- Commands: `speech.speech_to_text`, `speech.text_to_speech`,
  `speech.list_voices`, `speech.translate_speech`, and
  `speech.align_timing`.
- DTOs: `SpeechAudioInput`, `SpeechStreamFrame`, `TranscriptSegment`,
  `VoiceDescriptor`, `SpeechSynthesisRequest`, `SpeechSynthesisResult`, and
  `SpeechAlignment`.
- Policy: validate microphone/source permission inheritance, consent/purpose,
  duration, format, language, voice compatibility, style/locale, output format,
  resource budget, entitlement, and provider capability before dispatch.
- Trace/audit: record audio hash, duration, format, language hints, voice id
  hash, segment counts, confidence bands, artifact refs, latency, and bounded
  errors only.

## Existing Macaca Platform Inventory

- Generic service descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object clients, scheduler/resource patterns, and persistence
  snapshots can back speech commands and long-running jobs.
- Media/file packs will own raw audio and generated speech artifacts; speech
  should consume and emit handles.
- No current evidence proves speech-specific command DTOs, providers, SDK/WASM
  ABI, redaction gates, voice catalog, or developer docs are complete.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
