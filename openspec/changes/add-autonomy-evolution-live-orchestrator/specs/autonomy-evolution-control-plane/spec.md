## ADDED Requirements

### Requirement: Live Orchestrator Loop

The Autonomy Evolution capability SHALL provide a service-owned live
orchestrator that connects bounded observer evidence to candidate discovery,
control-plane transitions, admission, benchmark workload collection, normalized
benchmark scoring, release safety, target adapter dispatch, governance ledger
persistence, and API-first audit reconstruction without moving semantics into
the kernel, Web, CLI, frontend, or application-specific code.

#### Scenario: Observer evidence advances through one governed tick

- **GIVEN** a completed app-scoped agent task has emitted bounded observer
  evidence refs, policy refs, audit refs, trace id, and scope
- **WHEN** the live orchestrator receives a tick command with a valid lease id
  and idempotency key
- **THEN** it SHALL discover bounded candidate descriptors
- **AND** it SHALL advance the control-plane lifecycle through typed transition
  commands
- **AND** it SHALL invoke admission, benchmark, release safety, target adapter,
  and ledger commands through service boundaries
- **AND** it SHALL NOT write Skill files, mutate OS source files, execute shell
  commands, or hardcode application-specific workflow logic.

### Requirement: Live Tick Idempotency And Lease Enforcement

The live orchestrator SHALL require lease and idempotency evidence before any
side-effecting phase and SHALL return existing checkpoints for duplicate live
ticks instead of repeating promotion, rollback, or ledger append side effects.

#### Scenario: Duplicate tick does not duplicate side effects

- **GIVEN** a live tick with idempotency key `K` has already appended a
  checkpoint and reached a terminal or waiting phase
- **WHEN** the same tick key `K` is submitted again for the same scope and run
- **THEN** the orchestrator SHALL return the prior checkpoint result
- **AND** it SHALL NOT dispatch target apply, target rollback, release promote,
  or duplicate ledger append side effects.

#### Scenario: Missing lease fails closed

- **GIVEN** observer evidence exists for a candidate
- **WHEN** a live tick command omits lease evidence
- **THEN** the orchestrator SHALL return a structured denied result
- **AND** it SHALL NOT invoke admission, benchmark, release, target adapter, or
  mutation-capable commands.

### Requirement: Candidate Discovery Strategy

Candidate discovery SHALL be a replaceable Strategy that emits bounded
candidate descriptors and evidence refs without reading raw prompts, raw
provider payloads, manifests, package bytes, secrets, credentials, or
application-specific business content.

#### Scenario: Discovery output is bounded and generic

- **GIVEN** observer evidence refs and a replay cursor
- **WHEN** candidate discovery runs
- **THEN** it SHALL return candidate id, target type, scope, evidence refs,
  reason codes, and bounded metadata only
- **AND** it SHALL NOT include raw task prompts, raw provider outputs,
  manifests, package bytes, or application-specific workflow names.

### Requirement: Benchmark Workload Runner Strategy

The live orchestrator SHALL use a replaceable benchmark workload runner
Strategy to collect baseline and candidate measurements for the same generic
task family before invoking normalized paired benchmark scoring.

#### Scenario: Comparable measurements are scored

- **GIVEN** an admitted quarantined candidate has a workload plan for task family
  `F`
- **WHEN** the workload runner returns baseline and candidate measurements for
  the same task family `F`
- **THEN** the orchestrator SHALL invoke normalized paired benchmark scoring
- **AND** it SHALL record the benchmark decision, score delta, evidence refs,
  artifact refs, and reason codes in the governance ledger.

#### Scenario: Non-comparable measurements are inconclusive

- **GIVEN** baseline and candidate measurements use different task families or
  lack required metrics
- **WHEN** benchmark scoring is requested
- **THEN** the orchestrator SHALL record an inconclusive benchmark result
- **AND** it SHALL NOT promote the candidate.

### Requirement: Release Safety And Target Dispatch

The live orchestrator SHALL evaluate release safety before any canary,
promotion, supersedence, active monitoring, or rollback dispatch, and SHALL
delegate target-specific apply or rollback mechanics only to Target Adapter
Strategies.

#### Scenario: Safe Skill candidate dispatches through Skill adapter

- **GIVEN** a Skill candidate has accepted admission, passing benchmark evidence,
  release safety approval, capability diff refs, ownership refs, resource
  permission refs, and rollback memento refs
- **WHEN** the orchestrator reaches target dispatch
- **THEN** it SHALL dispatch apply or canary intent through the Skill target
  adapter
- **AND** it SHALL NOT write Skill package files directly
- **AND** it SHALL record target adapter evidence refs and rollback refs.

#### Scenario: Unsupported target fails closed

- **GIVEN** a candidate target type has no installed target adapter
- **WHEN** the orchestrator reaches target dispatch
- **THEN** it SHALL return a structured unavailable result
- **AND** it SHALL record the missing adapter reason in bounded diagnostics
- **AND** it SHALL NOT fake promotion, rollback, or active monitoring success.

### Requirement: Governance Ledger Audit Reconstruction

The live orchestrator SHALL append sanitized records for every discovery,
transition, admission, benchmark, release, target dispatch, rollback, and audit
checkpoint, and SHALL reconstruct the full evolution chain from governance
ledger replay after restart.

#### Scenario: Audit reconstructs a restarted run

- **GIVEN** an evolution run has appended live orchestrator records and the
  process restarts
- **WHEN** an API-first audit reconstruction command is issued for the run
- **THEN** the orchestrator SHALL replay governance ledger records in sequence
- **AND** it SHALL return bounded phase statuses, decisions, evidence refs,
  policy refs, audit refs, rollback refs, and diagnostics
- **AND** it SHALL NOT depend on process memory or local package recovery as the
  source of truth.

### Requirement: OS-Code Targets Remain Non-Mutating

The live orchestrator SHALL route OS-code evolution targets only to the
non-mutating OS-code proposal adapter unless a separate approved source-mutation
capability is present.

#### Scenario: OS-code mutation request is denied

- **GIVEN** a live tick discovers an OS-code evolution target
- **WHEN** the target requests file writes, shell command execution, patch
  application, test execution, or commit creation
- **THEN** the orchestrator SHALL deny the mutation request
- **AND** it SHALL route only governed OpenSpec/Superpowers/GitNexus proposal
  evidence to the OS-code proposal adapter
- **AND** it SHALL record that no source mutation was performed.
