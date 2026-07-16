## ADDED Requirements

### Requirement: Macaca SHALL provide the Media Transcription Pack as a serviceized capability

Macaca SHALL provide `pack.media.transcription.v1` as a provider-neutral
industrial pack for provider inspection, audio/video source import/opening,
media inspection, batch transcription, streaming transcription, diarization,
channel labeling, timestamp alignment, transcript normalization, redaction,
subtitle/caption export, translation handoff, artifact management, job
inspection, snapshot, and replay. The pack SHALL be declared by applications,
resolved by application admission and catalog services, and invoked only through
typed service commands owned by the transcription service provider.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.media.transcription.v1` as required and the transcription service provider is registered, healthy, entitled, permissioned, resource-admissible, consent-admissible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy templates, resource limits, approval rules, health, compatibility, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider credentials, private media, raw audio chunks, voice biometric features, raw transcripts, raw provider payloads, or application-specific workflow metadata

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.media.transcription.v1` as required but provider registration, entitlement, permission, consent, credential reference, resource budget, language/model support, network policy, host capability, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact a concrete provider, stream chunks, generate transcripts, export artifacts, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.media.transcription.v1` as optional and the pack is unavailable or partially available
- **THEN** admission SHALL produce a degraded effective capability memento with unavailable commands, reason codes, provider capability hashes when safe, and remediation metadata
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands while still allowing discovery and diagnostics

### Requirement: Media Transcription Pack commands SHALL use typed canonical service calls

Every `pack.media.transcription.v1` operation SHALL be represented as a typed
`transcription.*` command/result DTO and SHALL traverse the canonical service
runtime path with trace context, policy, entitlement, resource reservation,
approval, consent, metering, health, snapshot, structured errors, and sanitized
audit behavior.

#### Scenario: Provider inspection succeeds through service runtime
- **WHEN** a declared caller invokes `transcription.inspect_provider`
- **THEN** Macaca SHALL route the typed command through SDK/facade helpers into the service runtime and transcription service provider
- **AND** the result SHALL include bounded provider capability, command availability, language/model classes, batch support, streaming support, diarization support, timestamp granularities, redaction support, subtitle export support, translation handoff support, quota class, lifecycle, health, and compatibility diagnostics
- **AND** trace and audit events SHALL contain stable trace identifiers and sanitized descriptor metadata only

#### Scenario: Command is denied before provider invocation
- **WHEN** policy, permission, entitlement, consent, approval, resource, version, language, model, diarization, timestamp, redaction, translation, export, or artifact checks reject a `transcription.*` command
- **THEN** Macaca SHALL return a typed denied, quota, stale-version, approval-required, unsupported, or unavailable result before invoking any concrete provider
- **AND** the audit trail SHALL include bounded reason codes without raw audio, raw transcript text, subtitle PII, voice biometric data, credentials, or provider payloads

#### Scenario: Provider does not support a command
- **WHEN** the active provider descriptor does not support a requested command such as `transcription.plan_stream` or `transcription.plan_redaction`
- **THEN** Macaca SHALL return a typed unsupported result with descriptor hash, provider capability hash, command name, and safe remediation diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: Media Transcription Pack SHALL expose provider-neutral DTOs and stable hashes

`pack.media.transcription.v1` SHALL define provider-neutral DTOs and
deterministic hashing for `TranscriptionScope`,
`TranscriptionProviderCapability`, `TranscriptionSourceHandle`,
`TranscriptionMediaMetadata`, `TranscriptionPlan`,
`TranscriptionStreamingSession`, `TranscriptionAudioChunkHandle`,
`TranscriptDocument`, `TranscriptSegment`, `TranscriptToken`, `SpeakerLabel`,
`ChannelLabel`, `LanguageProfile`, `VocabularyReference`,
`TranscriptionRedactionProfile`, `TranscriptionSubtitleExportPlan`,
`TranscriptionTranslationHandoffPlan`, `TranscriptionJobStatus`, and
`TranscriptionArtifactHandle`. Provider-specific extensions SHALL be bounded as
adapter metadata and SHALL NOT drive OS-layer routing.

#### Scenario: Handles and hashes remain replayable
- **WHEN** Macaca records a transcription plan, stream cursor, transcript document, job status, artifact handle, or service snapshot
- **THEN** it SHALL include stable descriptor, capability, source version, language profile, vocabulary reference, plan, streaming session cursor, transcript, segment/token projection, redaction profile, subtitle export plan, translation handoff plan, job, artifact, event cursor, and redaction hashes
- **AND** replay diagnostics SHALL be able to correlate the bounded evidence chain without reconstructing private audio/video, raw transcripts, or raw provider payloads

#### Scenario: Provider metadata is bounded
- **WHEN** a provider returns language, model, timestamp, diarization, redaction, subtitle, translation, confidence, logprob, streaming, webhook, or job metadata
- **THEN** the transcription service provider SHALL normalize it into provider-neutral DTO fields or bounded `adapter_metadata`
- **AND** the microkernel, SDK, shell, and generic application framework SHALL NOT branch on provider names, model names, vocabulary names, queue names, webhook names, speaker names, channel names, file names, or application workflow names

### Requirement: Media Transcription Pack SHALL separate planning from side-effecting requests

Macaca SHALL require batch transcription, streaming transcription, diarization,
redaction, subtitle export, and translation handoff operations to use
non-mutating plan commands before side-effecting request commands. Side-effecting
request commands SHALL require a validated plan handle, idempotency key, source
version preconditions, consent policy, resource reservation, approval state when
required, artifact retention policy, and audit reason.

#### Scenario: Batch transcription uses a validated plan
- **WHEN** a caller needs asynchronous transcription for an audio or video source
- **THEN** it SHALL call `transcription.plan_batch` before `transcription.batch_request`
- **AND** the plan SHALL validate source format, duration, channel count, language/model support, timestamp granularity, vocabulary references, redaction profile, consent, resource budget, and approval requirements before any transcript job starts

#### Scenario: Streaming transcription uses a validated session plan
- **WHEN** a caller needs live or incremental transcription
- **THEN** it SHALL call `transcription.plan_stream` before `transcription.start_stream` and SHALL append chunks only through `transcription.append_stream_chunk`
- **AND** the plan SHALL validate chunk format, chunk ordering, interim result policy, endpointing class, retention, network permission, consent, quota, timeout, and resource budget before any streaming provider session starts

#### Scenario: Diarization and alignment use typed commands
- **WHEN** a caller needs speaker labels, channel labels, or token/word/segment timestamps
- **THEN** it SHALL use `transcription.plan_diarization`, `transcription.diarization_request`, or `transcription.align_timestamps`
- **AND** Macaca SHALL validate speaker count hints, channel mapping, timestamp granularity, source/transcript version preconditions, consent, and provider capability before exposing bounded labels or timing metadata

#### Scenario: Redaction, subtitles, and translation handoff use validated plans
- **WHEN** a caller needs transcript redaction, SRT/VTT/TTML-like subtitle export, or translation handoff
- **THEN** it SHALL call `transcription.plan_redaction`, `transcription.plan_subtitle_export`, or `transcription.plan_translation_handoff` before the corresponding request command
- **AND** the plan SHALL validate entity classes, locale, timing constraints, line length, target languages, redaction policy, artifact retention, approval, and resource budget before producing artifacts or contacting another service

### Requirement: Media Transcription Pack SHALL model streaming sessions, asynchronous jobs, and artifacts explicitly

Streaming and long-running transcription operations SHALL return explicit
session, job, and artifact handles rather than blocking indefinitely or exposing
provider-native payloads. Session, job, and artifact state SHALL be inspectable,
cancellable where supported, resumable through snapshots, and replayable through
sanitized evidence.

#### Scenario: Streaming session advances through explicit states
- **WHEN** `transcription.start_stream`, `transcription.append_stream_chunk`, `transcription.finish_stream`, or `transcription.cancel_stream` is invoked
- **THEN** Macaca SHALL validate the `TranscriptionStreamingSession` state, sequence cursor, chunk bounds, cancellation state, retention policy, and redaction policy
- **AND** it SHALL emit bounded streaming events and partial-result cursors without exposing raw audio chunks or raw transcript text in observability

#### Scenario: Batch request returns an asynchronous job
- **WHEN** `transcription.batch_request`, `transcription.diarization_request`, `transcription.redaction_request`, `transcription.subtitle_export_request`, or `transcription.translation_handoff_request` is accepted
- **THEN** Macaca SHALL return `TranscriptionJobStatus` with job handle, command name, provider capability hash, state, progress class, queue class, cancellation state, result artifact handles when available, and redaction class
- **AND** the caller SHALL inspect progress through `transcription.inspect_job` rather than polling provider-specific APIs directly

#### Scenario: Artifact handle is resolved safely
- **WHEN** a caller invokes `transcription.get_artifact_handle`
- **THEN** Macaca SHALL enforce artifact permission, retention policy, scope, redaction, consent, entitlement, and export policy before returning bounded artifact metadata
- **AND** the result SHALL NOT include raw transcript text, subtitle text containing PII, raw audio chunks, signed provider URLs beyond policy, or unbounded payloads

### Requirement: Media Transcription Pack SHALL enforce permissions, consent, policy, resource, entitlement, and approval gates

Macaca SHALL gate `pack.media.transcription.v1` with explicit permission scopes:
`transcription.provider.inspect`, `transcription.source.import`,
`transcription.source.open`, `transcription.media.read`,
`transcription.batch`, `transcription.stream`,
`transcription.stream.append`, `transcription.stream.cancel`,
`transcription.diarization`, `transcription.timestamp.align`,
`transcription.normalize`, `transcription.redaction`,
`transcription.subtitle.export`, `transcription.translation.handoff`,
`transcription.job.read`, and `transcription.artifact.read`. Side effects SHALL
also pass consent, resource, entitlement, approval, redaction, artifact, network,
and retention policy checks.

#### Scenario: Media and transcript data are redacted by policy
- **WHEN** a caller invokes `transcription.inspect_media`, `transcription.normalize_transcript`, or artifact inspection for private recordings, regulated conversations, restricted transcripts, or subtitle text containing PII
- **THEN** Macaca SHALL return only bounded, redacted fields permitted by policy
- **AND** it SHALL include redaction class and reason metadata without exposing raw audio, raw transcript text, subtitle PII, voice biometric features, location metadata, or provider payloads

#### Scenario: Approval is required for sensitive side effects
- **WHEN** a request involves private conversations, voice biometric risk, minors, medical/legal/financial recordings, customer data, regulated calls, persistent transcripts, subtitle PII, external delivery, or translation handoff
- **THEN** Macaca SHALL return `approval_required` or use an approved approval state before invoking the provider
- **AND** the audit evidence SHALL identify the bounded approval reason and operation hash

#### Scenario: Resource budget is insufficient
- **WHEN** source duration, channel count, chunk count, streaming session time, token count, segment count, speaker/channel label count, vocabulary size, redaction entity count, subtitle artifact count, CPU/GPU class, memory, storage, network transfer, timeout, provider quota, or retained snapshot budget exceeds policy
- **THEN** Macaca SHALL reject the plan or request with a typed quota/resource result
- **AND** the concrete provider SHALL NOT be invoked for rejected side effects

### Requirement: Media Transcription Pack SHALL preserve source, voice, transcript, subtitle, and artifact boundaries

The pack SHALL treat source media, raw audio chunks, speaker labels, channel
labels, transcript documents, transcript tokens, subtitle exports, translation
handoff artifacts, and derived artifacts as scoped resources. Operations SHALL
use handles and bounded metadata across boundaries, while raw content access
remains behind provider, artifact, redaction, consent, and policy controls.

#### Scenario: Import and open create scoped source handles
- **WHEN** a caller invokes `transcription.import_source_request` or `transcription.open_source`
- **THEN** Macaca SHALL validate source artifact permission, format class, duration policy, credential reference, artifact policy, consent policy, and redaction policy
- **AND** it SHALL return a `TranscriptionSourceHandle` with provider scope, source artifact handle, media kind, format class, duration/channel/sample-rate classes, version hash, sensitivity class, provenance class, redaction class, and freshness

#### Scenario: Speaker labels do not imply identity
- **WHEN** diarization returns `SpeakerLabel` or `ChannelLabel` values
- **THEN** Macaca SHALL represent them as bounded labels and confidence classes
- **AND** it SHALL NOT treat them as biometric identity, speaker verification, account identity, or profile linkage unless a separate approved identity capability and policy path is declared

#### Scenario: Translation handoff remains a service boundary
- **WHEN** a translation handoff is requested
- **THEN** Macaca SHALL call the typed translation-capable service boundary declared by policy rather than embedding translation provider logic in the transcription pack
- **AND** the handoff SHALL preserve redaction, consent, artifact retention, trace, and audit metadata

### Requirement: Media Transcription Pack SHALL provide sanitized trace, audit, health, snapshot, and replay evidence

`pack.media.transcription.v1` SHALL emit sanitized declaration, admission,
provider-inspection, source import/open, media inspection, batch, stream,
diarization, timestamp alignment, normalization, redaction, subtitle export,
translation handoff, job, artifact, policy, entitlement, resource, approval,
health, unavailable, failure, snapshot, and replay events. Snapshots SHALL be
bounded and replayable.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.media.transcription.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, command availability, provider health, policy template hash, resource counters, bounded source/transcript/session/job/artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, private audio/video, raw audio chunks, voice biometric features, raw transcripts containing PII, subtitle text containing PII, raw provider payloads, manifests, package bytes, private keys, signatures, and unbounded transcript/audio data

#### Scenario: Replay follows the canonical path
- **WHEN** audit replay reconstructs a `transcription.*` command chain
- **THEN** it SHALL show descriptor admission, SDK/facade service call, policy decision, resource/entitlement decision, consent and approval state when applicable, provider dispatch, session/job/artifact state, and result evidence
- **AND** replay SHALL NOT require direct provider APIs, raw media content, raw transcript text, provider-native payloads, or shell-owned state

### Requirement: Media Transcription Pack SHALL preserve Macaca architecture boundaries

The `pack.media.transcription.v1` implementation SHALL preserve Macaca's
microkernel, service runtime, application framework, SDK, runtime-host, plugin,
and shell boundaries. Concrete transcription providers SHALL be replaceable
Strategy adapters created only by approved runtime-host composition roots. SDK
helpers SHALL only build typed service commands and SHALL NOT create providers,
stream chunks outside the service runtime, or access private media/transcripts
directly.

#### Scenario: Dependency gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and shell-boundary gates scan the implementation
- **THEN** they SHALL find no concrete Amazon, Google, Azure, OpenAI, Deepgram, AssemblyAI, Rev, Speechmatics, local model, storage, moderation, credential-manager, artifact-provider, translation, or export adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed `transcription.*` service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable transcription provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract, unavailable behavior, health semantics, trace shape, and audit semantics
- **AND** provider-specific details SHALL appear only as sanitized descriptor/capability data, not as OS-layer routing branches

### Requirement: Media Transcription Pack SHALL include industrial developer documentation

Macaca SHALL include detailed developer documentation for
`pack.media.transcription.v1` at
`docs/developer-packs/media/transcription.md` before implementation completion.
The documentation SHALL describe capability declaration, required versus
optional behavior, DTOs, commands, permissions, consent, policy, streaming
lifecycle, asynchronous jobs, artifacts, provider replacement, unavailable
states, redaction, trace/audit/replay, conformance tests, and supplier/API
mapping.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/media/transcription.md`
- **THEN** the guide SHALL explain provider scopes, source handles, media metadata, batch plans, streaming sessions, chunk handles, transcript documents, segments, tokens, speaker labels, channel labels, language profiles, vocabulary references, redaction profiles, subtitle export plans, translation handoff plans, job status, artifacts, diagnostics, and operational limits
- **AND** examples SHALL use synthetic media sources, speakers, channels, chunks, transcripts, jobs, subtitles, and artifacts only

#### Scenario: Provider author checks conformance
- **WHEN** a provider author uses the documentation to implement a provider
- **THEN** the guide SHALL include conformance checks for descriptor completeness, DTO compatibility, command support, stable hashing, scope validation, streaming state machine, chunk ordering, consent policy, redaction, resource bounds, approval behavior, trace/audit events, unavailable behavior, snapshot/replay, and redaction
- **AND** the guide SHALL map Amazon Transcribe, Google Cloud Speech-to-Text, Azure AI Speech, OpenAI audio transcription, Deepgram, AssemblyAI, Rev AI, Speechmatics, local speech model, storage, subtitle, redaction, and translation concepts to Macaca abstractions without making supplier-specific behavior OS semantics
