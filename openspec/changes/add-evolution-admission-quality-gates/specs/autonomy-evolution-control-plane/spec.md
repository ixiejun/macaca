## ADDED Requirements

### Requirement: Evolution candidates SHALL pass executable admission quality gates

The Autonomy Evolution Control Plane SHALL expose a provider-neutral admission
command that evaluates candidate metadata through service-owned executable
Specifications before a candidate can be treated as admissible for quarantine,
benchmarking, canary, or promotion.

#### Scenario: High quality Skill candidate is accepted
- **GIVEN** a Skill package candidate has a semantic package name, bounded
  trigger/frontmatter descriptions, focused `SKILL.md` summary metadata,
  declared resource categories, quick validation evidence refs, forward-test
  evidence refs, fresh metadata, and no duplicate candidate refs
- **WHEN** the admission command is evaluated
- **THEN** the result SHALL be `Accepted`
- **AND** the result SHALL include bounded gate findings and the trace id
- **AND** the service SHALL NOT read or store raw package bytes or raw Skill body

#### Scenario: Meaningless generated Skill name is denied
- **GIVEN** a Skill package candidate name uses a meaningless generated pattern
  such as `skill-exp-*`
- **WHEN** admission is evaluated
- **THEN** the result SHALL be `Denied`
- **AND** the denial reason SHALL be sanitized and bounded

#### Scenario: Missing trigger quality needs evidence
- **GIVEN** a Skill package candidate has no meaningful trigger/frontmatter
  descriptions
- **WHEN** admission is evaluated
- **THEN** the result SHALL be `NeedsEvidence`
- **AND** the missing evidence SHALL identify trigger quality without exposing
  raw prompt or package content

#### Scenario: Duplicate candidate is denied
- **GIVEN** admission input declares a duplicate candidate reference
- **WHEN** admission is evaluated
- **THEN** the result SHALL be `Denied`
- **AND** the duplicate reference SHALL be represented as a bounded evidence ref

#### Scenario: Stale metadata requires regeneration evidence
- **GIVEN** candidate metadata is stale or generated from an outdated source
- **WHEN** admission is evaluated
- **THEN** the result SHALL be `NeedsEvidence`
- **AND** the result SHALL require metadata regeneration evidence before the
  candidate can be accepted
