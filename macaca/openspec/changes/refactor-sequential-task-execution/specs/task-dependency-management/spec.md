## ADDED Requirements

### Requirement: Cross-Agent Task Dependency Declaration

The system SHALL support declaring dependencies between tasks assigned to different agents. Dependencies SHALL be expressible as either "all tasks of agent X" or "specific task of agent X by title". Dependencies SHALL be resolved to concrete `TaskId` values at goal decomposition time.

#### Scenario: Agent-level dependency (all tasks)
- **GIVEN** a goal decomposition declares frontend tasks depend on all architect tasks
- **WHEN** `TaskSpace.create_and_assign()` processes the decomposition
- **THEN** each frontend task's `depends_on` includes all architect task IDs
- **AND** frontend tasks are marked Blocked until all architect tasks complete

#### Scenario: Specific task dependency by title
- **GIVEN** task "Implement login page" depends on architect task "Design auth flow"
- **WHEN** both tasks are created from the same goal decomposition
- **THEN** "Implement login page" has `depends_on` containing the TaskId of "Design auth flow"

#### Scenario: Dependency on non-existent agent warns but proceeds
- **GIVEN** a decomposition declares dependency on agent "qa" which has no tasks
- **WHEN** `TaskSpace.create_and_assign()` processes the decomposition
- **THEN** the system logs a warning
- **AND** the dependent task is created without that dependency (not blocked)

### Requirement: Cycle Detection in Task Dependencies

The system SHALL detect circular dependencies at task creation time and reject the creation with an error. Cycle detection SHALL use depth-first search on the dependency graph.

#### Scenario: Direct cycle detected
- **GIVEN** task A depends on task B, and task B depends on task A
- **WHEN** the system attempts to create these tasks
- **THEN** the creation fails with a cycle detection error
- **AND** no tasks are persisted

#### Scenario: Transitive cycle detected
- **GIVEN** task A depends on B, B depends on C, C depends on A
- **WHEN** the system attempts to create these tasks
- **THEN** the creation fails with a cycle detection error

#### Scenario: Valid DAG passes cycle check
- **GIVEN** task A has no dependencies, B depends on A, C depends on A and B
- **WHEN** the system creates these tasks
- **THEN** all tasks are created successfully (diamond DAG is valid)

### Requirement: Automatic Dependency Unblocking

When a task reaches a terminal state (Completed, Failed, Cancelled), the system SHALL re-evaluate all Blocked tasks that depend on it. A Blocked task SHALL transition to Pending if and only if ALL of its dependencies have reached Completed state. If any dependency is Failed or Cancelled, the Blocked task SHALL remain Blocked and PlanLoop SHALL emit an AnomalyDetected event.

#### Scenario: All dependencies completed unblocks task
- **GIVEN** task C depends on tasks A and B, both now Completed
- **WHEN** the unblocking check runs after B completes
- **THEN** task C transitions from Blocked to Pending
- **AND** WorkerLoop is woken to claim the newly available task

#### Scenario: One dependency failed keeps task blocked
- **GIVEN** task C depends on tasks A (Completed) and B (Failed)
- **WHEN** the unblocking check runs after B fails
- **THEN** task C remains Blocked
- **AND** PlanLoop emits AnomalyDetected with context about the failed dependency

#### Scenario: Cancelled dependency keeps task blocked
- **GIVEN** task C depends on task A which was Cancelled
- **WHEN** the unblocking check runs
- **THEN** task C remains Blocked
- **AND** PlanLoop emits AnomalyDetected

### Requirement: Enhanced LLM Decomposition Output

The `LlmDecomposer` SHALL request and parse an enhanced task format from the LLM that includes `sequence` (execution order within assigned agent) and `depends_on_agents` (cross-agent dependency declarations). The prompt SHALL guide the LLM to declare dependencies explicitly.

#### Scenario: LLM outputs tasks with sequence and cross-agent dependencies
- **GIVEN** a goal "Build a user management system" with agents [architect, frontend, backend]
- **WHEN** the LLM decomposes the goal
- **THEN** architect tasks have sequence [1, 2, ...] with no external dependencies
- **AND** frontend tasks have sequence [1, 2, ...] with `depends_on_agents: [{agent: "architect", type: "all"}]`
- **AND** backend tasks have sequence [1, 2, ...] with `depends_on_agents: [{agent: "architect", type: "all"}]`

#### Scenario: Decomposer resolves agent references to TaskIds
- **GIVEN** decomposed tasks include `depends_on_agents: [{agent: "architect", type: "all"}]`
- **WHEN** `TaskSpace.create_and_assign()` processes the batch
- **THEN** concrete architect TaskIds are resolved and stored in `depends_on`
- **AND** the `depends_on_agents` declarations are not persisted (resolved at creation time)

### Requirement: Task Skip API

The system SHALL provide a `skip_task(task_id)` operation that transitions a task from Pending or Blocked to Cancelled, triggers dependency re-evaluation for downstream tasks, and advances the agent's execution to the next sequence number.

#### Scenario: Skipping a pending task advances execution
- **GIVEN** agent has tasks seq 1 (Completed), seq 2 (Pending), seq 3 (Pending)
- **WHEN** `skip_task(seq_2_id)` is called
- **THEN** task seq 2 becomes Cancelled
- **AND** task seq 3 is now claimable by `claim_next()`

#### Scenario: Skipping a blocked task with dependents
- **GIVEN** task A (Blocked) has dependent task B (Blocked, depends on A)
- **WHEN** `skip_task(A)` is called
- **THEN** task A becomes Cancelled
- **AND** PlanLoop emits AnomalyDetected for task B (dependency cancelled)
