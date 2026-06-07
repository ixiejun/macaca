## ADDED Requirements
### Requirement: Materialized Skill package recovery SHALL restore active governance identity after restart
The Skill service SHALL recover an `Active` governance record for an already
materialized agent-created Skill package after process restart when the package
contains bounded frontmatter identity and proposal-linked provenance refs.

#### Scenario: Restart recovers materialized package governance record
- **GIVEN** a materialized Skill package exists on disk with `SKILL.md`
  frontmatter `name` and `description`
- **AND** the package body contains bounded provenance refs for proposal, task,
  and trace identity
- **AND** a new Skill service provider starts with empty in-memory governance
  state
- **WHEN** the provider rebuilds or returns its governance snapshot
- **THEN** the snapshot SHALL contain one `Active` governance record for the
  materialized Skill
- **AND** the record SHALL include the package identity and provenance refs as
  bounded evidence
- **AND** the recovery SHALL NOT read raw prompts, raw provider payloads, full
  task outputs, executable scripts, package bytes, credentials, or
  application-specific content

#### Scenario: Package without provenance is skipped
- **GIVEN** a Skill package exists on disk with no proposal-linked provenance
  refs
- **WHEN** governance package recovery scans the package
- **THEN** it SHALL log a bounded skip reason
- **AND** it SHALL NOT create or fake a governance record for that package
