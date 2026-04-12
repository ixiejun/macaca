## ADDED Requirements

### Requirement: Tool policy and toolkit module extraction preserves tool behavior

The system SHALL preserve existing framework toolkit behavior while moving tool policy and toolkit construction code into a dedicated web-internal module.

#### Scenario: FrameworkRunner toolkit callers remain unchanged

- **GIVEN** traced agents, runtime agents, and coordinator agents are built through `FrameworkRunner`
- **WHEN** tool policy and toolkit construction are moved into a dedicated module
- **THEN** those agent builders SHALL still construct a `Toolkit` with the same inputs: app id, agent name, session id, and optional goal id
- **AND** the same middlewares SHALL still be added by the caller after toolkit construction
- **AND** planner, worker, coordinator, EventLog, SSE, and run_trace behavior SHALL remain unchanged

#### Scenario: Base tool policy remains unchanged

- **GIVEN** an agent manifest defines allowed base tools
- **WHEN** toolkit construction is refactored
- **THEN** base tool allowlist filtering SHALL remain unchanged
- **AND** global `file_read`, `file_write`, and `shell` tools SHALL still be unregistered before workspace-scoped replacements are added
- **AND** workspace-scoped `file_read`, `file_write`, and `shell` tools SHALL keep the same names, schemas, path resolution, default timeout, and error messages

#### Scenario: Todo tool policy remains unchanged

- **GIVEN** an agent is classified by capabilities and entry-agent fallback
- **WHEN** tool policy is moved into a dedicated module
- **THEN** `todo_goal_management` SHALL still map to goal manager behavior
- **AND** `task_planning` or `todo_planning` SHALL still map to planner behavior
- **AND** `todo_execution` SHALL still map to worker behavior
- **AND** entry-agent fallback SHALL still map to goal manager behavior
- **AND** non-planner/non-entry fallback SHALL still map to worker behavior
- **AND** supervisor-like agents SHALL remain disallowed task assignees as before

#### Scenario: Todo tool registration remains unchanged

- **GIVEN** the toolkit registers per-agent todo tools
- **WHEN** registration logic is moved into a dedicated module
- **THEN** goal manager, planner, and worker policies SHALL receive the same todo tools as before
- **AND** planner `create_todo` SHALL keep the same coordinator name, disallowed assignees, assignee capabilities, and active goal id
- **AND** `create_goal` callbacks SHALL keep the same goal-to-session, ExecutionContext pause, and run_trace behavior
