## ADDED Requirements

### Requirement: Skill Evolution Proposal Snapshot

The system SHALL expose a traced Skill service command that returns sanitized draft Skill experience proposals without creating, patching, archiving, deleting, activating, or otherwise mutating skill state.

#### Scenario: Proposal snapshot lists draft proposals
- **GIVEN** verified task evidence has created one or more draft Skill experience proposals
- **WHEN** a caller invokes the Skill evolution proposal snapshot command through the Skill service
- **THEN** the service returns sanitized proposal records sorted deterministically
- **AND** the result states that active skill state was not mutated

#### Scenario: Empty proposal snapshot is explicit
- **GIVEN** no Skill experience proposals have been created
- **WHEN** a caller invokes the Skill evolution proposal snapshot command
- **THEN** the service returns an empty proposal list
- **AND** the result still includes trace-backed captured metadata

### Requirement: Proposal Snapshot Boundary

The system SHALL expose Skill experience proposals only through service or SDK facade calls, so shells, applications, and task services do not scan provider-local storage or implement proposal read semantics locally.

#### Scenario: Facade caller reads proposals
- **GIVEN** a shell, task service, or application needs to inspect draft Skill experience proposals
- **WHEN** it reads proposal state
- **THEN** it uses the SDK Skill client or service facade
- **AND** it does not read provider-local files, scan skill directories, or mutate active skill files
