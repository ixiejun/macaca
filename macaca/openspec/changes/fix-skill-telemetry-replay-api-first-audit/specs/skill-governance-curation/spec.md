## ADDED Requirements

### Requirement: Durable Skill Usage Telemetry Replay

The Skill service SHALL durably replay sanitized usage telemetry events for governed Skills after a provider restart.

#### Scenario: Replays usage counters after restart

- **GIVEN** a Skill service provider records sanitized `Created`, `Activated`, and `SuccessfulTask` governance events for a governed Skill
- **AND** those events are written to the provider's configured durable governance event journal
- **WHEN** a new Skill service provider starts with the same journal path
- **THEN** its governance snapshot SHALL include the governed Skill record
- **AND** `activation_count`, `use_count`, and `successful_task_count` SHALL reflect the replayed events
- **AND** the replay SHALL NOT read raw prompts, raw provider payloads, full Skill bodies, package bytes, credentials, or application-specific task content

#### Scenario: Package recovery does not reset replayed counters

- **GIVEN** the durable governance event journal contains usage events for a materialized Skill package
- **AND** the same materialized package also exists on disk with proposal-linked provenance refs
- **WHEN** the Skill service provider starts
- **THEN** it SHALL replay journal telemetry before materialized package recovery
- **AND** package recovery SHALL NOT overwrite or reset the replayed telemetry counters

### Requirement: Canonical API-First Self-Evolution Audit

The Web shell SHALL expose a canonical API-first self-evolution audit/trigger verification adapter that reports service-owned evidence before filesystem evidence.

#### Scenario: Reports complete canonical evidence

- **GIVEN** Skill operations contains an `Active` governance record for a target Skill
- **AND** the registry/load-path projection exposes the same Skill to the target agent
- **AND** session evidence contains bounded self-evolution observer or Skill snapshot events
- **WHEN** the API-first audit adapter is called for the app, agent, session, and target Skill
- **THEN** it SHALL report the canonical evidence status as passed
- **AND** it SHALL include bounded references to operations, registry/load-path, and observer evidence
- **AND** it SHALL NOT parse full Skill bodies, raw prompts, raw provider payloads, package bytes, credentials, or application-specific task content

#### Scenario: Reports missing canonical evidence without filesystem inference

- **GIVEN** any required canonical evidence source is missing
- **WHEN** the API-first audit adapter is called
- **THEN** it SHALL report the canonical evidence status as failed
- **AND** it SHALL identify the missing operations, registry/load-path, or observer evidence category
- **AND** it SHALL NOT claim self-evolution trigger success from filesystem artifacts alone
