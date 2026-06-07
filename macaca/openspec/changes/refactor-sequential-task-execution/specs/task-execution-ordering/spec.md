## ADDED Requirements

### Requirement: Sequential Task Execution per Agent per Session

The system SHALL execute tasks assigned to a given agent within a session strictly in ascending `sequence_number` order. A task with `sequence_number = N` MUST NOT begin execution until all tasks with `sequence_number < N` for the same agent+session have reached a terminal state (Completed, Failed, or Cancelled).

#### Scenario: Normal sequential execution
- **GIVEN** agent "worker-1" has 3 tasks with sequence_number 1, 2, 3 in session "s1"
- **WHEN** `TaskBoard.claim_next()` is called
- **THEN** it returns the task with sequence_number 1
- **AND** after task 1 completes, the next `claim_next()` returns task with sequence_number 2

#### Scenario: Blocked task halts subsequent tasks
- **GIVEN** agent "worker-1" has tasks with sequence_number 1 (Blocked), 2 (Pending), 3 (Pending)
- **WHEN** `TaskBoard.claim_next()` is called
- **THEN** it returns None (no task available)
- **AND** tasks 2 and 3 remain Pending until task 1 is unblocked and completed

#### Scenario: Failed task does not auto-cancel subsequent tasks
- **GIVEN** agent "worker-1" has tasks with sequence_number 1 (Failed), 2 (Pending)
- **WHEN** `TaskBoard.claim_next()` is called
- **THEN** it returns None
- **AND** PlanLoop emits AnomalyDetected for the failed task
- **AND** subsequent tasks remain Pending until PlanLoop decides (retry, cancel, or skip)

### Requirement: Automatic Sequence Number Assignment

The system SHALL automatically assign monotonically increasing `sequence_number` values to tasks created within the same agent+session scope. Tasks created in a batch (e.g., from goal decomposition) SHALL be assigned sequence numbers based on their declared order. Tasks appended later SHALL receive sequence numbers continuing from the current maximum.

#### Scenario: Batch task creation from goal decomposition
- **GIVEN** LLM decomposes a goal into 3 tasks for agent "frontend" with sequence [1, 2, 3]
- **WHEN** `TaskSpace.create_and_assign()` processes the batch
- **THEN** tasks are assigned sequence_number 1, 2, 3 within that agent+session

#### Scenario: Appending tasks to existing queue
- **GIVEN** agent "frontend" already has tasks with sequence_number 1, 2, 3
- **WHEN** a new task is created for "frontend" in the same session
- **THEN** the new task receives sequence_number 4

#### Scenario: Migration of legacy data without sequence_number
- **GIVEN** existing TodoItems do not have a `sequence_number` field
- **WHEN** the system loads these items
- **THEN** sequence numbers are assigned based on `created_at` ascending order
- **AND** the assigned numbers are persisted

### Requirement: TodoItem Sequence Number Field

The `TodoItem` struct SHALL include a `sequence_number: u32` field representing the execution order within an agent+session scope. The `priority` field SHALL be retained but SHALL NOT be used for determining execution order within a single agent's task queue.

#### Scenario: TodoItem serialization includes sequence_number
- **GIVEN** a TodoItem with sequence_number 5
- **WHEN** the item is serialized to storage
- **THEN** the `sequence_number` field is persisted
- **AND** deserialization restores the correct value

#### Scenario: Priority field preserved for backward compatibility
- **GIVEN** a TodoItem with priority 8 and sequence_number 2
- **WHEN** `TaskBoard.claim_next()` evaluates ordering
- **THEN** sequence_number 2 is used for ordering, not priority 8

## MODIFIED Requirements

### Requirement: Task Queue Ordering

The `TaskQueue` SHALL order tasks by `sequence_number` in ascending order (lowest first) instead of by priority in descending order. For tasks from different agents, the queue SHALL use `sequence_number` as primary sort and `created_at` as tiebreaker.

#### Scenario: Queue dequeues by sequence_number
- **GIVEN** the queue contains tasks with sequence_number [3, 1, 2]
- **WHEN** `pop()` is called
- **THEN** the task with sequence_number 1 is returned first

#### Scenario: Same sequence_number uses created_at as tiebreaker
- **GIVEN** two tasks from different agents both have sequence_number 1
- **WHEN** `pop()` is called
- **THEN** the task with earlier `created_at` is returned first
