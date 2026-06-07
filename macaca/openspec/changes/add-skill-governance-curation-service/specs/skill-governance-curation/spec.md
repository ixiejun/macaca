## ADDED Requirements

### Requirement: Skill Governance Telemetry

The system SHALL expose provider-neutral Skill governance telemetry through the Skill service boundary without storing raw prompts, raw task outputs, secrets, manifests, package bytes, or provider payloads.

#### Scenario: Record usage through traced service command
- **GIVEN** a framework tool observes a skill view, use, patch, or lifecycle event
- **WHEN** it calls the Skill service usage recording command with trace context
- **THEN** the Skill service records sanitized counters and evidence references
- **AND** the result includes the updated sanitized telemetry record

#### Scenario: Missing trace is rejected
- **GIVEN** a usage recording command does not carry trace context
- **WHEN** the command reaches the Skill service provider
- **THEN** the provider rejects it with a structured missing-trace error

### Requirement: Skill Lifecycle Governance

The system SHALL model skill lifecycle state, provenance, pinned status, and source scope separately from `SKILL.md` instruction content.

#### Scenario: Governance snapshot separates metadata from instructions
- **GIVEN** governance telemetry exists for one or more skills
- **WHEN** a governance snapshot command is called
- **THEN** the result includes lifecycle state, provenance, counters, timestamps, and evidence ids
- **AND** it does not include full `SKILL.md` instruction bodies

#### Scenario: Pinned skill is protected in recommendations
- **GIVEN** a skill governance record is pinned
- **WHEN** curation dry-run evaluates the skill
- **THEN** the report marks the skill as protected
- **AND** it does not recommend archive, merge, or deletion actions for that skill

### Requirement: Non-Destructive Skill Curation Dry Run

The system SHALL provide a deterministic curation dry-run command that emits recommendations and a report without mutating skill files or governance state.

#### Scenario: Dry-run returns recommendations only
- **GIVEN** governance records contain stale or narrow agent-created skills
- **WHEN** curation dry-run is executed
- **THEN** the result contains `would_*` recommendations and rationale
- **AND** no skill files are patched, merged, archived, deleted, or restored

#### Scenario: Optional intelligence provider absence is explicit
- **GIVEN** no LLM or similarity provider is configured
- **WHEN** curation dry-run is executed
- **THEN** deterministic heuristic recommendations are returned
- **AND** the report records that semantic merge analysis is unavailable rather than faking an LLM review

### Requirement: Service Boundary Ownership

The system SHALL keep skill governance and curation behavior inside replaceable Skill service providers and expose it to shells and applications only through SDK/facade clients.

#### Scenario: Shell consumes facade instead of owning curation semantics
- **GIVEN** Web or CLI needs to display a curation report
- **WHEN** it requests that report
- **THEN** it calls the SDK Skill client
- **AND** it does not implement lifecycle, merge, archive, or stale-skill classification rules locally
