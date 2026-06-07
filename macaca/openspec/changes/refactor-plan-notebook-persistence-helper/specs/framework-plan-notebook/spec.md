## ADDED Requirements

### Requirement: PlanNotebook persistence helper extraction preserves notebook content

The system SHALL preserve existing planner notebook writes while unifying duplicated web-side PlanNotebook persistence logic.

#### Scenario: Goal decomposition notebook write remains unchanged

- **GIVEN** the plan loop records goal decomposition in `PlanNotebook`
- **WHEN** PlanNotebook persistence is refactored
- **THEN** the planner session scope SHALL be computed the same way as before
- **AND** the system SHALL still load module state before mutating the notebook
- **AND** the decomposition plan id SHALL remain `goal:<goal_id>`
- **AND** the decomposition plan summary SHALL remain the goal description
- **AND** the decomposition plan objective SHALL remain `Decompose goal into executable todos`
- **AND** the decomposition subtask name, description, expected result, start, finish detail, and plan finish message SHALL remain unchanged
- **AND** the system SHALL still save module state after mutation

#### Scenario: Task review notebook write remains unchanged

- **GIVEN** the plan loop records task review in `PlanNotebook`
- **WHEN** PlanNotebook persistence is refactored
- **THEN** the planner session scope SHALL be computed the same way as before
- **AND** the system SHALL still load module state before mutating the notebook
- **AND** the review plan id SHALL remain `review:<task_id>`
- **AND** the review plan summary SHALL remain `Review task '<task_title>'`
- **AND** the review plan objective SHALL remain `Task review decision persisted via review_todo`
- **AND** the review subtask name, description, expected result, start, finish detail, and plan finish message SHALL remain unchanged
- **AND** the system SHALL still save module state after mutation

### Requirement: PlanNotebook remains agent-local planning memory

The system SHALL keep the existing responsibility boundary between PlanNotebook and TodoBoard during this refactor.

#### Scenario: Responsibility boundary is unchanged

- **GIVEN** planner notebook persistence is refactored
- **WHEN** planner decomposition and review events are handled
- **THEN** `PlanNotebook` SHALL remain the agent-local “脑内计划本”
- **AND** `TodoBoard` SHALL remain the durable task source of truth
- **AND** task scheduling, review, retry, dependency gating, coordinator resume, SSE/EventLog, run_trace, and browser refresh restore behavior SHALL remain unchanged
