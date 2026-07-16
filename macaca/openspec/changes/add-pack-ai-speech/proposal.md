# Change: Add AI Speech Pack

## Why

Developers need `pack.ai.speech.v1` as a real industrial capability for speech recognition, synthesis, voice metadata, translation, and timing alignment. The pack must not be a catalog label only: if an application declares it and the provider is installed, the SDK must expose callable typed commands; if policy, entitlement, provider, or host support is absent, Macaca must return explicit unavailable or denied diagnostics.

This child proposal intentionally narrows the broad industrial catalog into one implementable service-backed pack. It records the research basis, capability boundary, command surface, permission model, observability model, and acceptance gates needed before this pack can be called production-ready.

## Research And Borrowed Platform Patterns

The proposal borrows stable ideas from mature application platforms without copying their product-specific APIs:

- Platform privacy models: raw user data must not enter logs or diagnostics when processed by intelligent services.
- Android runtime permission pattern: AI operations that touch private data inherit source permissions.
- Windows capability declaration pattern: model-backed operations must be visible in app capability metadata.
- Apple entitlement/privacy pattern: sensitive processing requires policy and developer-declared purpose.

The Macaca interpretation is: app manifests declare pack intent; admission validates capability and permission metadata; runtime permission and approval gates happen before sensitive side effects; concrete providers stay behind service descriptors; traces and audits prove the execution path.

## What Changes

- Add the provider-neutral `pack.ai.speech.v1` contract under the `ai` family.
- Define a concrete command namespace, initial industrial command set, result/error DTO requirements, permission scopes, policy defaults, resource/entitlement behavior, and unavailable diagnostics.
- Bind implementation ownership to speech service provider; the kernel, SDK, shells, and generic application framework remain provider-neutral.
- Add SDK discovery metadata and examples for speech to text, text to speech, list voices, translate speech, align timing.
- Add trace, audit, health, snapshot, replay, and boundary gates proving the pack uses the canonical service path.

## Impact

- Affected specs: `pack-ai-speech`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs in protocol crates, descriptor/admission validators, SDK discovery/command helpers, speech service provider, optional provider registration, trace/audit tests, and dependency-boundary gates.
- Non-goals: no application-specific workflow, no provider-name routing in OS layers, no concrete provider construction in SDK/shell/kernel, and no fake success for absent providers.

## Supplier/API Baseline To Match

- Azure Speech, Google Speech-to-Text, and AWS Transcribe style APIs:
  streaming/batch transcription, diarization, word timing, language detection,
  translation, and confidence metadata.
- Cloud and OS TTS frameworks: voice catalogs, structured synthesis controls,
  output formats, voice/style availability, and quota accounting.
- Web Speech and native OS speech APIs: local/remote availability, permission
  declaration, foreground/background limits, and audio privacy.

Macaca's speech contract normalizes audio references, transcription segments,
timing alignment, voice descriptors, synthesis jobs, output formats, and privacy
policy without hardcoding provider voices or languages in OS code.

## Industrial Acceptance Bar

This proposal is not complete until implementation tasks produce a developer
guide at `docs/developer-packs/ai/speech.md`, typed audio/transcript/segment/
word-timing/voice/synthesis/alignment DTOs, deterministic streaming and voice
compatibility tests, and replay evidence proving raw audio and generated speech
bytes never enter observability surfaces.
