## ADDED Requirements

### Requirement: Macaca SHALL provide the AI Speech Pack as a serviceized capability

Macaca SHALL provide `pack.ai.speech.v1` as a provider-neutral industrial pack for speech recognition, synthesis, voice metadata, translation, and timing alignment. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.ai.speech.v1` as required and speech service provider is registered, healthy, entitled, and policy-admissible
- **THEN** admission SHALL expose `pack.ai.speech.v1` in the effective capability set with command schemas, permission scopes, policy template, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets or raw provider payloads

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.ai.speech.v1` as required but provider, permission, entitlement, resource, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.ai.speech.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: AI Speech Pack commands SHALL use typed canonical service calls

Every `pack.ai.speech.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `speech.speech_to_text` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and speech service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, or resource checks reject a `pack.ai.speech.v1` command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** the audit trail SHALL include the bounded reason code without raw user data or provider payloads

#### Scenario: Command is unsupported by the active provider
- **WHEN** a descriptor exists but the active provider does not support a requested command
- **THEN** Macaca SHALL return a typed unsupported result with descriptor and provider capability diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: AI Speech Pack SHALL expose concrete industrial metadata

`pack.ai.speech.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots, and unavailable diagnostics.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.ai.speech.v1`
- **THEN** it SHALL return the command namespace `speech.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, and compatibility metadata
- **AND** examples SHALL use generic handles or synthetic data rather than application-specific workflows

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.ai.speech.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

### Requirement: AI Speech Pack implementation SHALL preserve Macaca boundaries

The `pack.ai.speech.v1` implementation SHALL remain owned by speech service provider; the microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.ai.speech.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class and descriptor metadata rather than provider-specific business branches

### Requirement: AI Speech Pack SHALL model transcription, timing, and diarization

`pack.ai.speech.v1` SHALL expose typed audio input, streaming frames, transcript segments, word timing, diarization, language metadata, and alignment DTOs.

#### Scenario: Streaming transcript frames are ordered
- **WHEN** streaming speech recognition emits partial and final transcript frames
- **THEN** every frame SHALL include sequence number, time range, frame kind, confidence band, and trace id
- **AND** replay SHALL reproduce the frame order without raw audio or provider payloads

#### Scenario: Word timing is aligned
- **WHEN** `speech.align_timing` aligns transcript text to audio timing
- **THEN** Macaca SHALL return segment and word timing references with start/end times and confidence bands
- **AND** it SHALL preserve channel and speaker references when available

#### Scenario: Diarization is policy visible
- **WHEN** transcription includes speaker diarization
- **THEN** Macaca SHALL return provider-neutral speaker references and segment mapping only when policy permits
- **AND** hidden identity or biometric details SHALL NOT be exposed

#### Scenario: Unsupported language is explicit
- **WHEN** speech recognition, translation, or synthesis is requested for an unsupported language or locale
- **THEN** Macaca SHALL return a typed unsupported result before provider invocation when discoverable
- **AND** SDK discovery SHALL report language or locale availability without hardcoded provider branching

### Requirement: AI Speech Pack SHALL model voice catalogs and synthesis artifacts

`pack.ai.speech.v1` SHALL expose provider-neutral voice descriptors, synthesis requests, output artifacts, and validation commands.

#### Scenario: Voice descriptor is provider neutral
- **WHEN** `speech.list_voices` is invoked
- **THEN** Macaca SHALL return voice descriptor ids, locale, style tags, format support, lifecycle, and availability
- **AND** raw provider credentials, provider-native secrets, and hidden routing data SHALL NOT be exposed

#### Scenario: Voice validation blocks incompatible synthesis
- **WHEN** `speech.text_to_speech` requests an incompatible voice, style, locale, or output format
- **THEN** Macaca SHALL return unsupported or denied before provider side effects
- **AND** no generated audio artifact SHALL be created

#### Scenario: Synthesis output uses artifact references
- **WHEN** speech synthesis succeeds
- **THEN** Macaca SHALL return an audio artifact reference, duration band, format, voice descriptor id, and usage counters
- **AND** generated audio bytes SHALL NOT be stored in trace, audit, or snapshot records
