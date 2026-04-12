## ADDED Requirements

### Requirement: Executor lifecycle helper extraction preserves event behavior

The system SHALL preserve existing executor lifecycle event behavior while unifying duplicated lifecycle event construction in the web loop manager.

#### Scenario: Task started events remain unchanged

- **GIVEN** planner or worker execution is about to start an agent task
- **WHEN** the loop manager emits an executor lifecycle start event
- **THEN** the event SHALL remain `ExecutorEvent::TaskStarted`
- **AND** the event SHALL keep the same `task_id` value as before
- **AND** the event SHALL keep the same `agent` value as before
- **AND** the event SHALL be broadcast at the same point in the control flow as before

#### Scenario: Task completed events remain unchanged

- **GIVEN** planner or worker execution completes successfully
- **WHEN** the loop manager emits an executor lifecycle completion event
- **THEN** the event SHALL remain `ExecutorEvent::TaskCompleted`
- **AND** the event SHALL keep the same `task_id` and `agent` values as before
- **AND** the nested `TaskResult` SHALL keep `success=true`
- **AND** the nested `TaskResult` SHALL keep the same `output`, `error`, `artifacts`, and `tokens_used` values as before
- **AND** the event SHALL be broadcast at the same point in the control flow as before

#### Scenario: Task failed events remain unchanged

- **GIVEN** planner or worker execution fails, panics, times out, or fails to build an agent after start
- **WHEN** the loop manager emits an executor lifecycle failure event
- **THEN** the event SHALL remain `ExecutorEvent::TaskFailed`
- **AND** the event SHALL keep the same `task_id`, `agent`, and `error` values as before
- **AND** the event SHALL be broadcast at the same point in the control flow as before

### Requirement: Executor lifecycle helper extraction remains local and non-behavioral

The system SHALL limit this refactor to local helper extraction without changing task lifecycle semantics.

#### Scenario: Task orchestration behavior is not changed

- **GIVEN** a goal is decomposed, reviewed, retried, or executed by worker agents
- **WHEN** executor lifecycle event construction is refactored
- **THEN** PlanLoop and WorkerLoop scheduling behavior SHALL remain unchanged
- **AND** task claim, submit-for-review, mark-failed, retry, dependency gating, and coordinator resume behavior SHALL remain unchanged
- **AND** SSE and EventLog consumers SHALL continue to receive the same event names and payload shapes

#### Scenario: Browser refresh history remains compatible

- **GIVEN** executor lifecycle events were persisted to EventLog before the refactor
- **WHEN** the browser refreshes and reloads a session history
- **THEN** the frontend SHALL continue to reconstruct historical agent trace state from the same event names and payload fields
