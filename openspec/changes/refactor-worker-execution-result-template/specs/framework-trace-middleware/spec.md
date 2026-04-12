## ADDED Requirements

### Requirement: Worker execution result template extraction preserves task behavior

The system SHALL preserve existing worker task execution behavior while unifying duplicated result handling for normal and retry worker execution paths.

#### Scenario: Normal worker success remains unchanged

- **GIVEN** a worker handles `WorkerEvent::TaskClaimed`
- **WHEN** the worker agent reply succeeds
- **THEN** the task SHALL be submitted for review with the same summary behavior as before
- **AND** empty reply output SHALL still produce `Task '<title>' completed`
- **AND** the system SHALL emit the same `ExecutorEvent::TaskCompleted`
- **AND** the system SHALL emit the same `WORKER_TASK_SUCCESS` and `WORKER_SUBMIT_REVIEW` run trace events
- **AND** the plan loop waker SHALL be invoked under the same condition as before

#### Scenario: Retry worker success remains unchanged

- **GIVEN** a worker handles `WorkerEvent::RetryTask`
- **WHEN** the retry worker agent reply succeeds
- **THEN** the task SHALL be submitted for review with the same summary behavior as before
- **AND** empty reply output SHALL still produce `Task '<title>' completed on retry`
- **AND** the system SHALL emit the same `ExecutorEvent::TaskCompleted`
- **AND** the system SHALL emit the same `WORKER_SUBMIT_REVIEW` run trace event with `retry_success`
- **AND** the plan loop waker SHALL be invoked under the same condition as before

#### Scenario: Worker failures remain unchanged

- **GIVEN** normal or retry worker execution returns an agent error, panics, or times out
- **WHEN** worker result handling is refactored
- **THEN** the task SHALL be marked failed in the same cases as before
- **AND** the system SHALL emit the same `ExecutorEvent::TaskFailed`
- **AND** the system SHALL emit the same `WORKER_TASK_FAILED` run trace event in the same cases as before
- **AND** normal execution SHALL keep the `Task execution panicked` and `Execution timeout (30 min)` messages
- **AND** retry execution SHALL keep the `Retry task execution panicked` and `Retry execution timeout (30 min)` messages

### Requirement: Worker execution result template extraction remains local and non-behavioral

The system SHALL limit this refactor to worker result handling helper extraction without changing orchestration semantics.

#### Scenario: Worker orchestration is not changed

- **GIVEN** workers claim or retry tasks
- **WHEN** worker result handling is refactored
- **THEN** worker agent construction SHALL still use the same traced entry point
- **AND** task claim, retry, dependency gating, review, and coordinator resume behavior SHALL remain unchanged
- **AND** SSE, EventLog, and browser refresh history consumers SHALL continue to observe the same event names and payload fields
