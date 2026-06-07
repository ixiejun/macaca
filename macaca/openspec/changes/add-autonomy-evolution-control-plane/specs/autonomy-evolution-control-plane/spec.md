## ADDED Requirements

### Requirement: Macaca SHALL provide a service-owned Autonomy Evolution Control Plane

The system SHALL provide a provider-neutral Autonomy Evolution Control Plane
service capability that models agent self-evolution as typed lifecycle commands
and bounded evidence records without placing orchestration semantics in the
kernel, Web, CLI, frontend, or application-specific code.

#### Scenario: Evolution run is created from bounded observation evidence
- **GIVEN** a real agent task completes and emits bounded observer evidence refs
- **WHEN** the control plane receives an evolution transition command with app,
  tenant, session, task, actor, trace, target type, and evidence refs
- **THEN** it SHALL record an evolution run in `Observed` or
  `CandidateQueued` state
- **AND** it SHALL persist only bounded refs, counts, states, reasons, and
  sanitized diagnostics
- **AND** it SHALL NOT store raw prompts, raw provider payloads, manifests,
  package bytes, secrets, credentials, private keys, raw signatures, or
  unbounded output

#### Scenario: Shells remain thin adapters
- **GIVEN** Web, CLI, or frontend requests evolution status or transition
- **WHEN** the shell handles the request
- **THEN** it SHALL call SDK/SystemFacade commands
- **AND** it SHALL NOT classify candidates, score benchmarks, promote targets,
  roll back targets, or infer lifecycle state from filesystem paths

### Requirement: Evolution runs SHALL use an explicit lifecycle state machine

The control plane SHALL validate all evolution run transitions through an
explicit lifecycle state machine and SHALL deny invalid transitions with
structured evidence.

#### Scenario: Valid transition advances lifecycle with traceable evidence
- **GIVEN** an evolution run is in `CandidateQueued`
- **WHEN** a transition command requests `CandidateClassified` with trace id,
  actor id, target type, scope, and bounded evidence refs
- **THEN** the control plane SHALL advance the run to `CandidateClassified`
- **AND** it SHALL record the previous state, next state, trace id, actor id,
  evidence refs, and audit refs in the transition evidence

#### Scenario: Invalid transition is denied without mutation
- **GIVEN** an evolution run is in `Observed`
- **WHEN** a transition command requests `Promoted`
- **THEN** the control plane SHALL return a structured denial result
- **AND** it SHALL keep the run in `Observed`
- **AND** it SHALL record a bounded denial reason and trace id

### Requirement: Side-effecting transitions SHALL require policy and audit refs

The control plane SHALL fail closed when a transition can mutate target state,
expose a candidate to a benchmark or canary, promote a candidate, supersede an
active target, or roll back target state and the command lacks policy and audit
evidence.

#### Scenario: Promotion without policy evidence is denied
- **GIVEN** an evolution run is in `CanaryRunning`
- **WHEN** a transition command requests `Promoted` without policy decision refs
  or audit refs
- **THEN** the control plane SHALL return a structured denial result
- **AND** it SHALL NOT invoke any target adapter
- **AND** it SHALL keep the run in `CanaryRunning`

#### Scenario: Rollback with policy evidence is accepted
- **GIVEN** an evolution run is in `ActiveMonitoring`
- **WHEN** a transition command requests `RolledBack` with policy decision refs,
  audit refs, rollback refs, trace id, and bounded evidence refs
- **THEN** the control plane SHALL accept the transition
- **AND** it SHALL dispatch rollback intent only through the target adapter
  Strategy
- **AND** it SHALL record rollback refs without storing target package bytes

### Requirement: Target-specific mutation SHALL be delegated to Target Adapter Strategies

The control plane SHALL use replaceable Target Adapter Strategies for target
specific proposal, materialization, benchmark, canary, promotion, monitoring,
and rollback mechanics.

#### Scenario: Skill target adapter delegates to Skill service
- **GIVEN** an evolution run targets a Skill package
- **WHEN** the control plane reaches a target-specific phase
- **THEN** the Skill adapter SHALL call existing Skill service commands for
  proposal processing, materialization, registry/load-path, usage telemetry,
  and audit evidence
- **AND** the control plane SHALL NOT write Skill files directly
- **AND** the control plane SHALL NOT infer Skill lifecycle from filesystem
  paths

#### Scenario: Unsupported target adapter returns structured unavailable
- **GIVEN** an evolution run targets a capability whose adapter is not installed
- **WHEN** the control plane reaches an adapter-required phase
- **THEN** it SHALL return a structured unavailable or unsupported result
- **AND** it SHALL NOT fake success
- **AND** it SHALL record the missing adapter id and trace id as bounded
  diagnostics

### Requirement: SDK/SystemFacade SHALL expose unavailable behavior

The SDK/SystemFacade boundary SHALL expose provider-neutral transition and
snapshot commands and SHALL return structured unavailable results when the
control plane provider is absent.

#### Scenario: Missing provider returns unavailable result
- **GIVEN** the Autonomy Evolution Control Plane provider is absent
- **WHEN** a shell or application invokes the SDK transition command
- **THEN** the SDK SHALL return an unavailable result with service id, command
  name, trace id, and bounded reason
- **AND** it SHALL NOT panic, hang, silently fall back, or return fake success
