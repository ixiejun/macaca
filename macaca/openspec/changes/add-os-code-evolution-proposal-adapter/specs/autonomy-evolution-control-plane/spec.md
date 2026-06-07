## ADDED Requirements

### Requirement: Non-Mutating OS-Code Proposal Adapter

The Autonomy Evolution Control Plane SHALL provide a non-mutating OS-code target
adapter that creates governed proposal metadata without editing source files,
running commands, applying patches, committing changes, or bypassing approval.

#### Scenario: Proposal bundle is created

- **GIVEN** OpenSpec, Superpowers, GitNexus impact, expected test, release-gate,
  and rollback evidence refs
- **WHEN** the OS-code proposal adapter evaluates the command
- **THEN** it SHALL return a proposal bundle marked ready for review
- **AND** it SHALL indicate that source mutation was not performed.

### Requirement: Mandatory Governance Evidence

OS-code proposal readiness SHALL require OpenSpec proposal/design/tasks refs,
Superpowers design/plan refs, GitNexus impact refs, expected test refs, release
gate refs, and rollback refs.

#### Scenario: Missing evidence blocks readiness

- **GIVEN** a command missing GitNexus impact evidence
- **WHEN** the adapter evaluates the command
- **THEN** it SHALL return `NeedsEvidence`
- **AND** it SHALL include sanitized missing-evidence reason codes.

### Requirement: Blast-Radius Quarantine

The OS-code proposal adapter SHALL quarantine high blast-radius proposals instead
of marking them ready for implementation.

#### Scenario: High blast-radius proposal is quarantined

- **GIVEN** a proposal command with a blast-radius score above the release
  threshold
- **WHEN** the adapter evaluates the command
- **THEN** it SHALL return `Quarantined`
- **AND** source mutation SHALL remain false.

### Requirement: Source Mutation Refusal

The OS-code proposal adapter SHALL deny commands that request direct source
mutation.

#### Scenario: Mutation request is denied

- **GIVEN** a command with source mutation requested
- **WHEN** the adapter evaluates the command
- **THEN** it SHALL return `Denied`
- **AND** it SHALL include `source_mutation_not_allowed`.
