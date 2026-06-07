## ADDED Requirements

### Requirement: Macaca SHALL provide a service-owned autonomous materialization operator

The system SHALL provide a provider-neutral Skill service operator that can run
a bounded autonomous materialization cycle by composing proposal processing and
proposal materialization without moving eligibility or package-write semantics
into shells, applications, or the kernel.

#### Scenario: Dry-run operator produces body-free preview evidence
- **GIVEN** draft proposals exist in the Skill proposal backlog
- **WHEN** an approved caller invokes the autonomous materialization operator in
  dry-run mode with trace, scope, evidence refs, policy hints, and a batch limit
- **THEN** the Skill service SHALL run proposal processing without writing Skill
  package files
- **AND** it SHALL select only proposals marked `ReadyForMaterialization`
- **AND** it SHALL return aggregate body-free preview evidence including counts,
  selected proposal ids, denial reasons, content digests, audit refs, and trace id
- **AND** it SHALL NOT promote proposals or create active governance records

#### Scenario: Apply operator materializes only ready proposals
- **GIVEN** proposal processing marks one or more proposals
  `ReadyForMaterialization`
- **AND** the operator has package guard readiness, entitlement readiness,
  evidence refs, policy decision refs, audit refs, and a resolved package target
- **WHEN** the operator runs in apply mode
- **THEN** it SHALL invoke the existing proposal materialization command for each
  eligible proposal within the batch limit
- **AND** each file write SHALL go through the existing content-mutation Strategy
- **AND** each generated Skill package SHALL follow Skill Creator-compatible
  structure: a concise `SKILL.md`, trigger-oriented `name` and `description`,
  bounded provenance refs, and no auxiliary documentation clutter
- **AND** each proposal SHALL be promoted only after its materialization mutation
  succeeds
- **AND** the aggregate result SHALL include applied, previewed, denied, skipped,
  bytes-written, rollback, evidence, policy, audit, and trace refs
- **AND** the result SHALL NOT include full generated `SKILL.md` bodies

#### Scenario: Apply operator fails closed without policy approval
- **GIVEN** proposals exist in the backlog
- **WHEN** the operator is invoked in apply mode without policy decision refs,
  entitlement readiness, package guard readiness, or package target resolution
- **THEN** it SHALL return structured denial evidence
- **AND** it SHALL NOT write files
- **AND** it SHALL NOT promote proposal lifecycle
- **AND** it SHALL log bounded denial fields with trace and proposal identifiers

### Requirement: Autonomous materialization evidence SHALL stay separated from activation and optimization

The system SHALL report autonomous materialization as P3 materialization evidence
only, and SHALL NOT treat it as activation, reuse, or measurable optimization
without separate service-owned telemetry.

#### Scenario: Operations snapshot separates proof phases
- **GIVEN** an autonomous materialization operator run has completed
- **WHEN** Skill operations data is queried
- **THEN** operations output SHALL expose body-free operator counts and result refs
- **AND** it SHALL keep proposal capture, processing readiness, materialization,
  registry/load-path, usage telemetry, and optimization metrics as distinct
  fields or sections
- **AND** it SHALL NOT infer P4 activation/reuse or P5 optimization from P3
  materialization alone
