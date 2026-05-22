## ADDED Requirements

### Requirement: Skill Alias Resolution

The system SHALL expose skill redirect, superseded-by, and absorbed-into relationships through traced Skill service commands rather than rewriting task, scheduler, context, or skill files as a curation side effect.

#### Scenario: Resolve an existing skill alias
- **GIVEN** a Skill alias record maps an old skill id to a replacement skill id
- **WHEN** a consumer calls the Skill alias resolve command with trace context
- **THEN** the service returns `resolved = true`
- **AND** the result includes the replacement skill id, alias kind, rationale, and evidence ids

#### Scenario: Resolve a skill without alias
- **GIVEN** no alias record exists for a requested skill id
- **WHEN** a consumer calls the Skill alias resolve command with trace context
- **THEN** the service returns `resolved = false`
- **AND** it does not invent a fallback replacement

### Requirement: Skill Alias Governance

The system SHALL store Skill alias records as sanitized governance metadata with replayable evidence references.

#### Scenario: Upsert alias record
- **GIVEN** a curation provider determines that one skill has been absorbed into another skill
- **WHEN** it calls the alias upsert command with trace context
- **THEN** the Skill service stores a sanitized alias record
- **AND** the record contains ids, names, alias kind, rationale, timestamps, and evidence ids without raw prompts or raw task outputs

#### Scenario: Alias snapshot is diagnostic only
- **GIVEN** one or more alias records exist
- **WHEN** a diagnostic client calls the alias snapshot command
- **THEN** the service returns sorted alias records
- **AND** no skill files, scheduler jobs, task definitions, or context snapshots are mutated

### Requirement: Skill Provider File Boundary

The runtime-host Skill provider implementation SHALL keep mutable governance and alias state in focused helper modules so the public provider adapter remains below the repository file-size ceiling.

#### Scenario: Provider remains a thin adapter
- **WHEN** the Skill provider handles governance and alias commands
- **THEN** reusable state mutation logic lives outside the public provider adapter file
- **AND** existing public provider construction remains compatible
