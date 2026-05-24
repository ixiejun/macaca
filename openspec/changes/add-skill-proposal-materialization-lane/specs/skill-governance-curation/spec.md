## ADDED Requirements

### Requirement: Macaca SHALL materialize ready Skill proposals through a service-owned lane

The system SHALL provide a provider-neutral Skill proposal materialization
command behind `service.skill` that can convert an eligible proposal into a
bounded AgentSkills-compatible `SKILL.md` draft only after proposal processing
marks it `ReadyForMaterialization`.

#### Scenario: Dry-run materialization does not mutate files or governance
- **GIVEN** a Draft proposal has a processing record in `ReadyForMaterialization`
- **WHEN** a caller invokes materialization in dry-run mode with trace, evidence,
  policy refs, package guard readiness, and entitlement readiness
- **THEN** the service SHALL return planned skill id, relative path, content
  digest, and planned byte count
- **AND** it SHALL NOT write `SKILL.md`
- **AND** it SHALL NOT promote the proposal or create active governance records

#### Scenario: Apply materialization writes a bounded SKILL draft
- **GIVEN** a Draft proposal has a processing record in `ReadyForMaterialization`
- **WHEN** a caller invokes materialization in apply mode with an approved
  package root, ownership class, trace, evidence refs, policy decision refs,
  package guard readiness, and entitlement readiness
- **THEN** the Skill service SHALL build bounded AgentSkills-compatible
  `SKILL.md` content through a service-owned Builder
- **AND** the runtime-host provider SHALL delegate the write to the existing
  content-mutation Strategy
- **AND** the result SHALL include mutation status, rollback ref, content digest,
  byte count, evidence refs, policy refs, audit refs, and trace id
- **AND** the result SHALL NOT include the full generated Skill body
- **AND** the proposal SHALL be promoted into active governance metadata only
  after the write succeeds

#### Scenario: Non-ready proposals are denied before mutation
- **GIVEN** a proposal has no processing record or the record is not
  `ReadyForMaterialization`
- **WHEN** a caller invokes materialization
- **THEN** the service SHALL deny the request without writing files
- **AND** it SHALL NOT change proposal lifecycle or active governance state
- **AND** the denial SHALL include a bounded reason suitable for audit surfaces

### Requirement: Proposal materialization SHALL preserve Macaca service boundaries

The system SHALL keep proposal materialization semantics inside replaceable
Skill service providers and SHALL prevent shells, applications, and the kernel
from owning eligibility, content construction, package mutation, or promotion
sequencing.

#### Scenario: Materialization remains provider-neutral and generic
- **GIVEN** a materialization command is handled by the built-in local provider
- **WHEN** it evaluates readiness, builds content, applies mutation, logs events,
  or promotes governance state
- **THEN** it SHALL NOT branch on application names, workflow names, provider
  names, model names, driver names, gateway names, chain names, payment names,
  or business-domain identifiers
- **AND** logs, snapshots, and results SHALL omit raw prompts, raw provider
  payloads, unbounded task output, full generated Skill bodies, package bytes,
  manifests, credentials, secrets, and executable scripts

#### Scenario: Materialization does not claim later optimization
- **GIVEN** materialization writes a `SKILL.md` draft and promotes governance
  metadata
- **WHEN** operations surfaces or reports display the result
- **THEN** they SHALL treat it as Skill materialization evidence only
- **AND** they SHALL NOT count it as later Skill activation, reuse, successful
  downstream optimization, or measured self-improvement without separate
  telemetry evidence
