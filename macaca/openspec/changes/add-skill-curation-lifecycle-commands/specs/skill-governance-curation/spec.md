## ADDED Requirements

### Requirement: Governed Skill Lifecycle Mutation Commands

The system SHALL expose Skill lifecycle pin, unpin, archive, and restore
operations only as traced Skill service commands that mutate governance metadata
without modifying skill instruction files, package bytes, aliases, executable
scripts, prompts, or provider payloads.

#### Scenario: Pin and unpin mutate only governance metadata
- **GIVEN** a governed skill identity and sanitized evidence references
- **WHEN** a caller invokes Skill curation pin or unpin through the SDK facade
- **THEN** the Skill service updates the governance record pinned state
- **AND** the result includes the trace id, lifecycle state, mutation flag,
  reason, and evidence ids
- **AND** no `SKILL.md` body, support file, alias, or executable script is
  changed

#### Scenario: Archive and restore update lifecycle state
- **GIVEN** a governed skill identity and sanitized evidence references
- **WHEN** a caller invokes Skill curation archive or restore through the SDK
  facade
- **THEN** the Skill service updates the lifecycle state to `Archived` or
  `Active`
- **AND** governance snapshots exclude archived records unless
  `include_archived` is requested
- **AND** no skill package content is deleted or rewritten

#### Scenario: Pinned skill cannot be archived by default
- **GIVEN** a governed skill record is pinned
- **WHEN** a caller invokes Skill curation archive without a future
  approval-gated override contract
- **THEN** the Skill service rejects the command with a structured error
- **AND** the skill remains pinned and non-archived

### Requirement: Lifecycle Mutation Boundary Ownership

The system SHALL keep Skill lifecycle mutation semantics inside replaceable Skill
service providers and expose them to Web, CLI, applications, and autonomy
callers only through typed SDK or service facade calls.

#### Scenario: Shell remains an adapter
- **GIVEN** a shell needs to trigger or display a lifecycle operation
- **WHEN** it interacts with Skill curation state
- **THEN** it calls the SDK Skill facade
- **AND** it does not classify lifecycle, protection, archive, restore, merge,
  or deletion rules locally
