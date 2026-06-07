## ADDED Requirements

### Requirement: Todo Lifecycle Policy Boundary

The system SHALL centralize Todo lifecycle transition rules behind a dedicated lifecycle policy while preserving the current task status semantics.

#### Scenario: Claim start review flow remains unchanged

- **GIVEN** a pending task that is claimable for an agent
- **WHEN** the worker claims it, starts it, and submits it for review
- **THEN** the task SHALL still transition through `Assigned -> InProgress -> PendingReview`
- **AND** the transition decisions SHALL be produced through the lifecycle policy boundary rather than scattered inline checks

#### Scenario: Failed review still chooses retry or fail by attempt count

- **GIVEN** a task in `PendingReview`
- **WHEN** review feedback fails the task and the max attempt count has not been reached
- **THEN** the task SHALL transition to `NeedsOptimization`

- **GIVEN** a task in `PendingReview`
- **WHEN** review feedback fails the task and the max attempt count has been exhausted
- **THEN** the task SHALL transition to `Failed`

### Requirement: Dependency Resolution Strategy Boundary

The system SHALL centralize task dependency gating behind a dependency resolver while preserving current dependency behavior.

#### Scenario: Unmet dependencies still create blocked tasks

- **GIVEN** a new task with dependencies that are not all `Completed`
- **WHEN** the task is created in task space
- **THEN** the task SHALL be stored with status `Blocked`

#### Scenario: Completed dependency still unblocks dependents

- **GIVEN** a blocked task whose dependencies become fully `Completed`
- **WHEN** the completed dependency is processed
- **THEN** the blocked task SHALL transition to `Pending`

#### Scenario: Cancelled or failed dependency does not unblock

- **GIVEN** a blocked task whose dependency is `Cancelled` or `Failed`
- **WHEN** dependency reevaluation occurs
- **THEN** the blocked task SHALL remain blocked

### Requirement: Loop Template Refactor Preserves Event Contract

The system SHALL be allowed to refactor `PlanLoop` and `WorkerLoop` into explicit template steps without changing emitted event contracts.

#### Scenario: PlanLoop event semantics stay stable

- **GIVEN** a goal, pending reviews, and goal completion conditions
- **WHEN** `PlanLoop` runs after the refactor
- **THEN** it SHALL still emit the same `PlanEvent` variants with equivalent payload semantics

#### Scenario: WorkerLoop event semantics stay stable

- **GIVEN** a worker with claimable or retryable tasks
- **WHEN** `WorkerLoop` runs after the refactor
- **THEN** it SHALL still emit the same `WorkerEvent` variants with equivalent payload semantics

### Requirement: Deprecated Wrappers Must Remain Available During Migration

The system SHALL retain superseded public task APIs as deprecated wrappers during migration instead of deleting them immediately.

#### Scenario: Deprecated wrapper remains callable

- **GIVEN** an existing caller still invoking an old public task API
- **WHEN** the workspace compiles after the refactor
- **THEN** the old API SHALL still exist
- **AND** it SHALL be marked `deprecated`
- **AND** it SHALL delegate to the new canonical implementation

#### Scenario: Repository callers migrate to canonical APIs

- **GIVEN** superseded public task APIs have canonical replacements
- **WHEN** repository-owned callers are updated in this change
- **THEN** those callers SHALL use the new canonical APIs instead of continuing to introduce deprecated usage
