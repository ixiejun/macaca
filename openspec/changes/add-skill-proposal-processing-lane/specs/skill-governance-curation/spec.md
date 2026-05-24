## ADDED Requirements

### Requirement: Macaca SHALL process Skill evolution proposals through a service-owned lane

The system SHALL provide a provider-neutral Skill proposal processing lane that
scores, groups, suppresses, and snapshots evolution proposals after capture and
before any future materialization gate.

#### Scenario: Processing dry-run does not mutate proposal or package state
- **GIVEN** one or more Skill experience proposals exist
- **WHEN** a caller invokes proposal processing in dry-run mode through the SDK
  or Skill service facade
- **THEN** the result SHALL include proposed processing records, quality scores,
  duplicate group summaries, state counts, and bounded reasons
- **AND** no proposal lifecycle, Skill package file, active catalog entry,
  executable script, alias, registry entry, or usage telemetry SHALL be mutated

#### Scenario: Processing apply requires policy and evidence refs
- **GIVEN** one or more Skill experience proposals exist
- **WHEN** a caller invokes proposal processing in apply mode
- **THEN** the Skill service SHALL require trace context, sanitized evidence
  refs, and policy decision refs before processing state is mutated
- **AND** missing refs SHALL be rejected with a structured service error
- **AND** the rejection SHALL leave proposal lifecycle and processing state
  unchanged

#### Scenario: Duplicate low-information proposals are suppressed
- **GIVEN** multiple Draft proposals share the same bounded summary,
  classification, destination, recommended action, and target skill identity
- **WHEN** proposal processing apply runs
- **THEN** the Skill service SHALL retain all proposal records for audit replay
- **AND** the first eligible proposal MAY be marked `ReadyForMaterialization`
- **AND** later low-information duplicates SHALL be marked
  `SuppressedDuplicate` with duplicate group size and evidence refs
- **AND** no shell SHALL compute duplicate status locally

#### Scenario: Processing snapshots expose backlog pressure
- **GIVEN** proposal processing has run or proposals are waiting to be processed
- **WHEN** an operations surface requests a processing snapshot
- **THEN** the Skill service SHALL return processing records, state counts,
  duplicate group counts, ready counts, suppressed counts, rejected counts,
  mutated flag, and captured timestamp
- **AND** the snapshot SHALL NOT include raw prompts, raw provider payloads, raw
  task output, full Skill bodies, manifests, package bytes, credentials,
  secrets, private keys, raw signatures, or unbounded diagnostics

### Requirement: Proposal processing SHALL preserve Macaca service boundaries

The system SHALL keep proposal processing semantics inside replaceable Skill
service providers and expose them to Web, CLI, frontend, applications, and
autonomy callers only through typed SDK or service facade calls.

#### Scenario: Shell remains a thin adapter
- **GIVEN** Web, CLI, or frontend displays or triggers proposal processing
- **WHEN** it interacts with proposal processing state
- **THEN** it SHALL call SDK or Skill service commands
- **AND** it SHALL NOT compute quality scores, duplicate signatures,
  suppression decisions, materialization readiness, rejection decisions, or
  lifecycle transitions locally

#### Scenario: Processing does not claim closed-loop optimization
- **GIVEN** proposal processing marks a proposal `ReadyForMaterialization`
- **WHEN** self-evolution evaluation scores the platform
- **THEN** readiness SHALL count only as post-capture processing evidence
- **AND** it SHALL NOT count as Skill package materialization, activation,
  reuse, or measurable optimization until separate bounded evidence exists
