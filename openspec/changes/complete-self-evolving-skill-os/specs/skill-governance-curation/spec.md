## ADDED Requirements

### Requirement: Durable Skill Governance Store

The system SHALL persist Skill governance state through a Store/EventLog-backed
Skill service boundary that is replayable, provider-neutral, and sanitized.

#### Scenario: Governance state survives provider restart
- **GIVEN** lifecycle, provenance, telemetry, alias, proposal, and curation run
  events were appended with trace context
- **WHEN** the built-in Skill governance provider rebuilds its read model
- **THEN** the rebuilt snapshot includes the same sanitized governance records
- **AND** the snapshot excludes full skill instruction bodies, raw prompts, raw
  task outputs, raw provider payloads, manifests, package bytes, secrets, and
  unbounded diagnostics

#### Scenario: Governance store is unavailable
- **GIVEN** the configured governance store provider is absent or unhealthy
- **WHEN** a caller invokes a governance mutation command
- **THEN** the Skill service returns a structured unavailable state
- **AND** no caller receives fake success or partially applied lifecycle state

### Requirement: Complete Skill Lifecycle State Machine

The system SHALL model Skill lifecycle transitions with explicit states for
`Draft`, `Active`, `Stale`, `Archived`, `Quarantined`, `Superseded`, and
`Rejected`.

#### Scenario: Draft promotion requires approval and evidence
- **GIVEN** a draft skill proposal has sanitized evidence refs
- **WHEN** a caller requests promotion through the Skill service
- **THEN** the service requires trace context, policy approval, package guard,
  and entitlement checks before changing the lifecycle to `Active`
- **AND** the promotion audit record includes the policy decision id and
  evidence refs

#### Scenario: Superseded skill requires alias mapping
- **GIVEN** a curation run proposes absorbing a source skill into an umbrella
  skill
- **WHEN** the proposal is applied
- **THEN** the source skill transitions to `Superseded`
- **AND** the Skill service creates a redirect, warn, or deny alias record
  before hiding the superseded skill from normal catalogs

#### Scenario: Pinned lifecycle protection is enforced
- **GIVEN** a skill governance record is pinned
- **WHEN** a curation command attempts archive, merge apply, supersede, or
  deletion without an explicit approval override contract
- **THEN** the Skill service rejects the command with a structured denied state
- **AND** the skill remains visible according to its prior lifecycle state

### Requirement: Rich Skill Provenance And Telemetry

The system SHALL record sanitized provenance and telemetry that distinguish
skill usage, activation, resource reads, patches, successful task outcomes, and
failed task outcomes.

#### Scenario: Successful task updates effectiveness telemetry
- **GIVEN** a verified task used a skill and completed successfully
- **WHEN** the task service records the outcome through the Skill service
- **THEN** the Skill governance store increments the successful task counter
- **AND** it updates the last success timestamp with trace and evidence refs

#### Scenario: Failed task updates effectiveness telemetry
- **GIVEN** a skill activation is associated with a verified failed task
- **WHEN** the task service records the outcome through the Skill service
- **THEN** the Skill governance store increments the failed task counter
- **AND** curation snapshots can use the failure signal without storing raw task
  output

### Requirement: Verified Task Experience Extraction

The system SHALL convert verified reusable task experience into draft-only Skill
or support-file proposals through the Skill Evolution service.

#### Scenario: Verified terminal success creates bounded candidate
- **GIVEN** a task has reached verified terminal success and has evidence refs
- **WHEN** Task/Autonomy calls Skill Evolution with a bounded summary and trace
  digest
- **THEN** Skill Evolution classifies the candidate as memory, knowledge,
  skill patch proposal, new skill draft, support-file draft, or no-op
- **AND** active skill files and active catalogs remain unchanged unless a later
  approval-gated promotion occurs

#### Scenario: Unverified task is rejected
- **GIVEN** a task has not passed the evidence gate
- **WHEN** Skill Evolution receives an experience extraction request
- **THEN** the service rejects the request with a structured validation error
- **AND** no proposal, skill file, alias, or lifecycle mutation is stored

### Requirement: Proposal Promotion And Rejection

The system SHALL expose traced proposal lifecycle commands for proposing
patches, promoting drafts, and rejecting drafts without shell-owned semantics.

#### Scenario: Proposal is rejected with durable rationale
- **GIVEN** a draft skill proposal exists
- **WHEN** a caller rejects the proposal through the SDK Skill facade
- **THEN** the proposal lifecycle becomes `Rejected`
- **AND** the governance store retains bounded rationale, evidence refs, trace
  id, policy decision id, and audit event id

#### Scenario: Duplicate promotion is denied
- **GIVEN** a draft proposal was already promoted or rejected
- **WHEN** a caller attempts to promote it again
- **THEN** the Skill service returns a structured denied or conflict state
- **AND** no second active skill mutation is applied

### Requirement: Safe Skill Content Mutation

The system SHALL allow skill content mutation only through policy-gated Skill
service commands that create rollback mementos before side effects.

#### Scenario: Support file is written safely
- **GIVEN** an approved proposal writes a support file under `references/`,
  `templates/`, `scripts/`, or `assets/`
- **WHEN** the Skill service applies the mutation
- **THEN** it validates the package root, path, size, encoding, executable
  policy, entitlement, and sensitive-content rules
- **AND** it creates a rollback memento before the atomic write

#### Scenario: Protected package mutation is denied
- **GIVEN** a skill is bundled, marketplace-managed, paid, encrypted,
  application-owned without application-scope approval, or cross-tenant
- **WHEN** an agent-initiated mutation attempts to patch active content
- **THEN** the Skill service denies the mutation with a structured policy state
- **AND** no package bytes or governance lifecycle state are changed

### Requirement: Curation Run, Report, And Rollback

The system SHALL provide curation status, run, snapshot, and rollback commands
that produce replayable reports and mementos.

#### Scenario: Dry-run is non-mutating
- **GIVEN** governance records contain stale, duplicate, narrow, invalid, or
  protected skill candidates
- **WHEN** curation dry-run executes
- **THEN** the result includes deterministic recommendations, optional semantic
  analysis status, bounded rationale, and report refs
- **AND** no governance state, alias map, scheduler ref, context snapshot, or
  skill file is mutated

#### Scenario: Approved curation run creates rollback ref
- **GIVEN** a curation apply run has policy-approved actions
- **WHEN** the run applies lifecycle, alias, merge, archive, or support-file
  mutations
- **THEN** the service records before and after snapshot refs, report refs,
  rollback refs, policy decision ids, and audit event ids
- **AND** rollback can restore lifecycle, telemetry, aliases, and package refs

### Requirement: Optional Semantic Review Provider

The system SHALL treat semantic curation providers as optional Strategy
providers that return typed proposals and never mutate skills directly.

#### Scenario: Semantic provider is absent
- **GIVEN** no LLM or similarity provider is configured
- **WHEN** a curation run reaches semantic analysis
- **THEN** deterministic curation phases still run
- **AND** the report records semantic review as unavailable instead of faking
  provider success

#### Scenario: Semantic provider output is typed and bounded
- **GIVEN** a semantic provider returns merge, duplicate, demotion, or patch
  suggestions
- **WHEN** the Skill service accepts the provider result
- **THEN** it validates typed proposal fields, bounds text payloads, strips raw
  provider payloads, and requires policy approval before any apply step

### Requirement: Umbrella Merge And Support-File Demotion

The system SHALL support approval-gated umbrella merge proposals that move
session-specific detail into support files and preserve redirects for absorbed
skills.

#### Scenario: Compatible skills are merged into an umbrella skill
- **GIVEN** multiple skills share compatible scope, ownership, permissions,
  trust level, package source, tenant, and capability semantics
- **WHEN** a policy-approved merge is applied
- **THEN** reusable generic flow remains in the umbrella `SKILL.md`
- **AND** session-specific detail, starter artifacts, and repeatable actions are
  demoted to `references/`, `templates/`, and `scripts/` as appropriate
- **AND** absorbed skills become `Superseded` with alias records

#### Scenario: Incompatible merge is rejected
- **GIVEN** two skills differ in package ownership, trust level, executable
  permissions, tenant boundary, or source scope
- **WHEN** curation proposes a merge
- **THEN** the Skill service rejects or marks the proposal as policy-denied
- **AND** no active skill content or lifecycle state is changed

### Requirement: Context Composer Lifecycle And Alias Integration

The system SHALL compose skill context from Skill service snapshots that apply
lifecycle filtering and alias resolution.

#### Scenario: Normal catalog includes active skills only
- **GIVEN** governance records include active, draft, stale, archived,
  quarantined, rejected, and superseded skills
- **WHEN** Context Composer builds a normal skill catalog
- **THEN** only active visible skills are included by default
- **AND** the context report lists filtered counts and reasons without loading
  full skill bodies

#### Scenario: Alias is resolved before activation
- **GIVEN** a task, scheduler entry, or context request references a superseded
  skill id
- **WHEN** the consumer resolves the skill through the Skill service
- **THEN** the service returns the target skill, warning, or denial according to
  the alias resolution policy
- **AND** the resolution emits trace and audit evidence

### Requirement: Package Ownership And Entitlement Enforcement

The system SHALL enforce package ownership, application scope, tenant scope, and
entitlement rules before curation or evolution mutates skills.

#### Scenario: Marketplace skill receives local overlay proposal
- **GIVEN** a marketplace-managed skill needs an agent-discovered improvement
- **WHEN** Skill Evolution creates a reusable proposal
- **THEN** the proposal targets a local overlay or draft
- **AND** the service does not mutate or impersonate the upstream package

#### Scenario: Paid or encrypted skill mutation is restricted
- **GIVEN** a paid or encrypted skill is present without mutation entitlement
- **WHEN** curation proposes patch, merge, archive materialization, or support
  file mutation
- **THEN** the Skill service restricts the operation to metadata, usage, alias,
  or no-op reporting
- **AND** it records a structured entitlement denial

### Requirement: Thin Operations Adapters

The system SHALL keep Web, CLI, and frontend operations surfaces as adapters
that call SDK/SystemFacade clients and do not implement self-evolution
semantics.

#### Scenario: Operator approves a proposal from UI
- **GIVEN** an operator clicks approve, reject, rollback, pin, archive, restore,
  quarantine, or run curation in a shell surface
- **WHEN** the shell submits the action
- **THEN** it sends a typed SDK/SystemFacade command with trace context
- **AND** it does not classify skills, merge content, write files, or implement
  lifecycle rules locally

### Requirement: Boundary, Audit, And Sanitization Gates

The system SHALL provide executable gates proving self-evolving skills preserve
microkernel, serviceization, shell, optional-provider, audit, and sanitization
boundaries.

#### Scenario: Boundary gate rejects kernel semantic dependency
- **GIVEN** a code change introduces a kernel dependency on skill curation,
  evolution, semantic review, mutation providers, Web, CLI, or frontend code
- **WHEN** dependency-boundary tests run
- **THEN** the tests fail as an architecture violation

#### Scenario: Reports and logs are sanitized
- **GIVEN** curation, evolution, promotion, mutation, rollback, or context
  integration commands execute
- **WHEN** logs, snapshots, reports, route payloads, or audit read models are
  inspected
- **THEN** they contain bounded metadata, ids, counts, refs, and structured
  states only
- **AND** they exclude raw prompts, secrets, raw provider payloads, manifests,
  package bytes, credentials, raw signatures, full skill bodies, and unbounded
  outputs
