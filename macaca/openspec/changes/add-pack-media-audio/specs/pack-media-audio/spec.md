## ADDED Requirements

### Requirement: Macaca SHALL expose Media Audio as a serviceized industrial pack

Macaca SHALL expose `pack.media.audio.v1` as a provider-neutral pack for audio
provider inspection, audio import/open, metadata inspection, waveform and
loudness inspection, transcode planning, transcode requests, segment planning,
segment requests, filter planning, filter requests, mix planning, mix requests,
text-to-speech synthesis planning, synthesis requests, export planning, export
requests, artifact handles, health, snapshots, and replay diagnostics. The pack
SHALL be declared by applications, resolved by catalog/admission services, and
invoked only through descriptor-owned `audio.*` service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.media.audio.v1` as required and an audio media provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy template hash, resource limits, approval rules, health metadata, compatibility metadata, and replay metadata
- **AND** SDK discovery SHALL expose callable `audio.*` commands without leaking credentials, raw prompts, private recordings, speaker biometric data, generated audio bytes, raw exports, raw provider payloads, or provider secrets

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.media.audio.v1` as required but provider registration, host support, credential reference, permission, entitlement, resource, voice/model, codec/container, policy, or approval prerequisites are absent
- **THEN** admission SHALL block readiness with typed unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact a concrete provider, read private recordings, mutate audio, synthesize outputs, export artifacts, strip metadata, publish assets, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.media.audio.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability memento
- **AND** SDK helpers and WASM ABI descriptors SHALL mark unavailable commands as non-callable while preserving structured diagnostics for application recovery

### Requirement: Media Audio commands SHALL use typed canonical service calls

Every `pack.media.audio.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace context, policy, resource, entitlement, approval, lifecycle, health,
snapshot, structured error, and audit behavior. SDK helpers, WASM ABI handlers,
application admission, web, CLI, and frontend code SHALL only build or submit
canonical service calls and SHALL NOT call audio providers directly.

#### Scenario: Provider capability is inspected
- **WHEN** `audio.inspect_provider` is invoked with declared scope and trace context
- **THEN** Macaca SHALL return sanitized provider capability metadata for import/open, metadata, waveform/loudness, transcode, segment, filter, mix, synthesis, export, codecs, containers, sample-rate/channel support, auth, quota, lifecycle, health, and compatibility support
- **AND** the result SHALL include typed unavailable, unsupported, degraded, retired, format-limited, transcode-limited, segment-limited, filter-limited, mix-limited, synthesis-limited, export-limited, network-limited, CPU/GPU-limited, and quota-limited states when applicable

#### Scenario: Audio reads use bounded projections
- **WHEN** `audio.open_audio`, `audio.inspect_metadata`, `audio.inspect_waveform`, or `audio.get_artifact_handle` is invoked
- **THEN** Macaca SHALL enforce audio, segment, artifact, prompt, voice, permission, resource, and redaction scopes before provider access
- **AND** results SHALL be bounded, paged, partial, or asynchronous when needed, redacted according to policy, and represented by handles and summaries rather than raw audio samples, private recordings, speaker biometric features, generated audio bytes, or unbounded PCM/sample data

#### Scenario: Unsupported command is requested
- **WHEN** a descriptor exists but the active provider does not support the requested `audio.*` command, audio format, codec, container, sample rate, channel layout, waveform projection, filter operation, mix mode, voice/model, synthesis mode, export format, or artifact mode
- **THEN** Macaca SHALL return a typed unsupported, format-unsupported, codec-unsupported, voice-denied, or synthesis-denied result with descriptor and capability diagnostics
- **AND** SDK discovery SHALL report the command or feature as non-callable for the current effective capability set

### Requirement: Media Audio DTOs SHALL be provider-neutral and hash-stable

`pack.media.audio.v1` SHALL define provider-neutral DTOs for `AudioScope`,
`AudioProviderCapability`, `AudioHandle`, `AudioMetadata`,
`AudioWaveformSummary`, `AudioSegment`, `AudioFilterOperation`,
`AudioMixSource`, `AudioMixGraph`, `AudioVoiceCapability`,
`AudioSynthesisPlan`, `AudioExportPlan`, and `AudioArtifactHandle`. DTOs SHALL
use stable handles, version hashes, compatibility hashes, capability hashes,
redaction classes, sensitivity classes, provenance classes, event cursors, and
artifact handles rather than provider object references as OS-layer semantics.

#### Scenario: Provider-specific concepts are mapped
- **WHEN** a provider exposes FFmpeg commands, GStreamer pipeline graphs, Web Audio node graphs, libsndfile format/subtype data, OpenAI/ElevenLabs/Google TTS/Polly synthesis concepts, or storage artifact objects
- **THEN** the provider adapter SHALL map those concepts into Macaca provider-neutral DTOs
- **AND** provider-specific extensions SHALL appear only as bounded `adapter_metadata` protected by capability hashes and SHALL NOT drive OS-layer routing

#### Scenario: Hashes preserve compatibility and replay
- **WHEN** Macaca serializes descriptors, provider capabilities, codec/container support, audio versions, waveform summaries, segments, filter plans, mix graphs, voice capabilities, synthesis plans, export plans, artifact handles, event cursors, and redaction profiles
- **THEN** it SHALL produce stable hashes suitable for compatibility checks, stale-version detection, voice/synthesis diagnostics, audit correlation, and replay diagnostics
- **AND** schema evolution tests SHALL prove older compatible snapshots remain readable or return typed schema-mismatch diagnostics

### Requirement: Media Audio side effects SHALL use plan/request separation

Macaca SHALL split transcoding, segmenting, filtering, mixing, synthesizing,
exporting, and other side-effecting audio operations into non-mutating plan
commands and side-effecting request commands. Plan commands SHALL validate
audio versions, artifact scopes, format/codec/container support, prompts,
voice/model handles, consent/copyright metadata, resource budgets, approvals,
and idempotency before request commands can perform side effects.

#### Scenario: Transcode or filter plan validates before mutation
- **WHEN** `audio.plan_transcode` or `audio.plan_filter` receives codec, container, sample-rate, channel, normalize, resample, gain, fade, equalizer, denoise, or channel operations
- **THEN** Macaca SHALL validate operation schema, target handles, audio version hash, codec/container support, sample-rate/channel compatibility, metadata retention policy, provider support, resource budget, and required approvals
- **AND** it SHALL return validation diagnostics without mutating the audio, stripping metadata, exporting artifacts, or contacting external delivery systems for side effects

#### Scenario: Segment or mix plan validates before mutation
- **WHEN** `audio.plan_segment` or `audio.plan_mix` receives trim, split, concatenate, silence-based segmentation, multi-source timing, gain, pan, or fade operations
- **THEN** Macaca SHALL validate source handles, segment time ranges, source version hashes, mix source rights and consent metadata, sample-rate compatibility, filter graph compatibility, resource budget, and approvals
- **AND** it SHALL return plan diagnostics without publishing or modifying source artifacts

#### Scenario: Synthesis request executes idempotently
- **WHEN** `audio.synthesis_request` is invoked with a valid plan handle, prompt handle, voice capability hash, safety state, idempotency key, trace context, and sufficient permissions
- **THEN** Macaca SHALL execute through the audio media service provider and return typed success, voice-denied, prompt-denied, synthesis-denied, conflict, approval-required, quota, timeout, cancellation, or failure results
- **AND** repeated requests with the same idempotency key SHALL NOT duplicate generated speech outputs

#### Scenario: Export request returns artifact handles only
- **WHEN** `audio.export_request`, `audio.transcode_request`, `audio.segment_request`, `audio.filter_request`, `audio.mix_request`, or `audio.synthesis_request` produces a derived output
- **THEN** Macaca SHALL return bounded `AudioArtifactHandle` results with provenance and redaction metadata
- **AND** raw audio bytes, raw generated speech, waveform samples, and raw exports SHALL remain in artifact boundaries and SHALL NOT enter trace, audit, snapshots, SDK diagnostics, or examples

### Requirement: Media Audio SHALL enforce permission, policy, resource, entitlement, and approval gates

Every `audio.*` command SHALL be scoped to application id, tenant id, session
id, task id, trace id, provider scope, audio handle, artifact handle when
applicable, actor handle when available, credential reference, network policy,
artifact policy, voice/safety policy, and permission state. Side-effecting
commands SHALL run policy, resource, entitlement, approval, version, voice
safety, consent/copyright, metadata, and idempotency checks before concrete
provider calls.

#### Scenario: Permission is denied before provider access
- **WHEN** an application lacks `audio.provider.inspect`, `audio.import`, `audio.open`, `audio.metadata.read`, `audio.waveform.read`, `audio.transcode`, `audio.segment`, `audio.filter`, `audio.mix`, `audio.synthesize`, `audio.export`, or `audio.artifact.read`
- **THEN** Macaca SHALL return a typed denied result before invoking any provider
- **AND** audit evidence SHALL include bounded reason codes and sanitized scope handles only

#### Scenario: Sensitive operation requires approval
- **WHEN** a command touches private recordings, human voices, speaker identity, minors, legal/medical/financial calls, copyrighted music, raw prompts, generated voices, external delivery, metadata stripping, destructive edits, or operations that publish artifacts
- **THEN** Macaca SHALL require approval when policy marks the operation approval-gated
- **AND** denial, expiration, or missing approval SHALL return typed approval-required diagnostics without side effects

#### Scenario: Resource or entitlement is unavailable
- **WHEN** audio size, duration, sample count, channel count, segment count, filter count, mix source count, prompt size, generated output duration, render/export size, artifact size, provider quota, network transfer, timeout, CPU/GPU class, memory, storage, streaming output, retained snapshots, entitlement, voice/model access, codec/container support, or host support is insufficient
- **THEN** Macaca SHALL return typed quota, unavailable, denied, timeout, cancellation, CPU/GPU-unavailable, or host-resource diagnostics
- **AND** the provider SHALL NOT be called for side-effecting operations after a failed gate

### Requirement: Media Audio artifacts, waveforms, prompts, and voice outputs SHALL be bounded and redacted

`pack.media.audio.v1` SHALL treat raw recordings, waveforms, speaker identity,
voice biometric features, private prompts, generated voices, exported audio,
and derived artifacts as sensitive data. The pack SHALL expose handles, bounded
summaries, cursors, redaction classes, provenance classes, retention metadata,
and replay pointers rather than raw sensitive payloads in observability
surfaces.

#### Scenario: Metadata or waveform is inspected
- **WHEN** `audio.inspect_metadata` or `audio.inspect_waveform` is invoked with sufficient permission
- **THEN** Macaca SHALL return bounded metadata classes for duration, sample rate, channels, bit depth, bitrate, codec/container, tags presence, loudness summary handle, waveform summary handle, provenance handles, and checksum handle
- **AND** raw samples, full waveforms, private tags, speaker identity, and unbounded metadata blobs SHALL NOT enter traces, audits, snapshots, or SDK diagnostics

#### Scenario: Synthesis output is produced
- **WHEN** `audio.synthesis_request` produces generated speech
- **THEN** Macaca SHALL return artifact kind, voice capability hash, output profile, duration class, checksum handle, retention state, generated-voice provenance, sensitivity class, and redaction class
- **AND** raw prompts and generated audio bytes SHALL remain behind prompt and artifact boundaries

#### Scenario: Artifact metadata is inspected
- **WHEN** `audio.get_artifact_handle` resolves an audio, segment, mixed, synthesized, transcoded, filtered, or exported artifact
- **THEN** Macaca SHALL return artifact kind, source operation handle, content type, duration class, codec/container, size class, checksum handle, retention state, provenance, sensitivity class, and redaction class
- **AND** raw artifact bytes SHALL remain behind artifact boundaries

### Requirement: Media Audio SHALL preserve Macaca architecture boundaries

The Media Audio pack implementation SHALL preserve the microkernel, service
runtime, SDK/SystemFacade, application framework, runtime-host, plugin, and
shell boundaries defined by Macaca governance. Concrete audio providers SHALL
be replaceable Strategy adapters created only in approved runtime-host or
plugin composition roots.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, serviceization, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete FFmpeg, GStreamer, Web Audio, libsndfile, OpenAI, ElevenLabs, Google TTS, Polly, storage, moderation, credential, or artifact provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.media.audio.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract, permission model, trace/audit schema, snapshot shape, and structured unavailable behavior
- **AND** OS layers SHALL NOT branch on provider names, model names, voice names, codec names, file names, application names, or workflow names

### Requirement: Media Audio SHALL emit sanitized trace, audit, health, snapshot, and replay evidence

`pack.media.audio.v1` SHALL emit sanitized declaration, admission,
provider-inspection, import/open, metadata-inspection, waveform-inspection,
transcode, segment, filter, mix, synthesis, export, artifact-handle, policy,
entitlement, resource, approval, health, snapshot, unavailable, and failure
events. Snapshots SHALL contain enough bounded metadata to diagnose and replay
service behavior without storing raw sensitive content.

#### Scenario: Service call evidence is recorded
- **WHEN** any `audio.*` command is submitted
- **THEN** Macaca SHALL record trace-required service-call evidence with command name, descriptor version, sanitized scope handles, policy decision, resource decision, provider capability hash, result class, and replay pointer
- **AND** the evidence SHALL exclude raw credentials, raw prompts, private recordings, speaker biometric data, generated audio bytes, raw exports, raw provider payloads, manifests, package bytes, private keys, signatures, and unbounded PCM/sample data

#### Scenario: Snapshot supports recovery diagnostics
- **WHEN** the service runtime records an audio snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, audio codec/container and version hashes, command availability, provider health, policy template hash, resource counters, bounded metadata/waveform/operation/artifact summaries, event cursors, and sanitized replay pointers
- **AND** replay tests SHALL prove every `audio.*` command can be correlated through the canonical service path after restart

### Requirement: Media Audio SHALL provide industrial developer documentation

The implementation SHALL include a detailed developer guide at
`docs/developer-packs/media/audio.md` before `pack.media.audio.v1` is marked
complete. The guide SHALL be linked from SDK discovery metadata and the
industrial pack catalog index.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/media/audio.md`
- **THEN** the guide SHALL explain purpose, manifest declaration, required versus optional behavior, permissions, provider scopes, audio handles, metadata, codecs, containers, sample rates, channels, waveform summaries, loudness reports, segments, filter operations, mix graphs, voice capabilities, synthesis plans, export plans, artifacts, unavailable diagnostics, provider replacement, operational limits, and conformance expectations
- **AND** it SHALL document every command DTO and result DTO with field-level behavior, idempotency, redaction, pagination, streaming/asynchronous artifact behavior, timeout, cancellation, approval, artifact retention, audio version preconditions, format/codec/container compatibility, metadata stripping, prompt safety, voice consent/safety, generated-voice provenance, structured errors, and trace/audit interpretation

#### Scenario: Supplier mapping is documented
- **WHEN** the documentation describes supplier/API mapping
- **THEN** it SHALL map FFmpeg, GStreamer, Web Audio, libsndfile, OpenAI TTS, ElevenLabs, Google TTS, Amazon Polly, storage, safety, and export concepts to Macaca abstractions
- **AND** it SHALL explicitly document what is intentionally not exposed as OS semantics

#### Scenario: Examples are provided
- **WHEN** the guide provides examples
- **THEN** examples SHALL use only synthetic audio, prompts, voices, waveform summaries, generated artifacts, exported artifacts, and unavailable diagnostics
- **AND** examples SHALL NOT include provider names, real credentials, private recordings, speaker biometric data, copyrighted audio, raw prompts, raw generated audio, raw exports, or workflow-specific conventions
