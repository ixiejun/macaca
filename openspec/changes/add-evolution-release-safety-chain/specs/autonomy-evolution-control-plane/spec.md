## ADDED Requirements

### Requirement: Release Safety Commands

The Autonomy Evolution Control Plane SHALL expose a typed release safety command
that evaluates quarantine, canary, promotion, active monitoring, rollback,
supersedence, rejection, inconclusive, and dry-run release actions without
embedding target-specific mutation logic.

#### Scenario: Canary release is accepted

- **GIVEN** an admitted candidate with passing benchmark evidence, scoped policy
  refs, capability diff evidence, ownership refs, resource permission refs, and
  rollback memento refs
- **WHEN** the release safety command requests canary start
- **THEN** the service SHALL return an accepted result for `CanaryRunning`
- **AND** the result SHALL include bounded policy, evidence, and rollback refs.

### Requirement: Release Policy Gate

The Autonomy Evolution Control Plane SHALL evaluate capability diff, package
ownership, tenant/application scope, trust level, resource permissions,
executable change flags, blast-radius score, benchmark decision, and rollback
memento readiness before side-effecting release actions.

#### Scenario: Unsafe release is denied

- **GIVEN** a candidate with an executable change and a high blast-radius score
- **WHEN** the release safety command requests promotion
- **THEN** the service SHALL deny the release action
- **AND** the denial SHALL include sanitized reason codes rather than raw target
  payloads.

### Requirement: Rollback Memento Enforcement

Rollback release actions SHALL require replayable rollback memento refs that
match the command scope, and SHALL fail closed when memento evidence is missing.

#### Scenario: Canary failure rolls back

- **GIVEN** a canary candidate with rollback memento refs and policy approval
- **WHEN** canary failure is reported through a rollback release command
- **THEN** the service SHALL return `RolledBack`
- **AND** the result SHALL preserve rollback refs for later Store/EventLog audit
  replay.

### Requirement: Release Safety Unavailable Behavior

SDK and runtime-host consumers SHALL receive structured unavailable release
results when the Autonomy Evolution release safety provider is absent.

#### Scenario: Missing provider does not fake success

- **GIVEN** no installed release safety provider
- **WHEN** an SDK caller requests a release safety decision
- **THEN** the SDK SHALL return a denied release result with an unavailable
  reason
- **AND** it SHALL NOT report quarantine, canary, promotion, or rollback success.
