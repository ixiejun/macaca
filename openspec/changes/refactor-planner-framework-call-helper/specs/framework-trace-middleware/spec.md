## ADDED Requirements

### Requirement: Planner framework call helper extraction preserves planner behavior

The system SHALL preserve existing planner decomposition, review, and follow-up behavior while unifying duplicated planner framework call templates.

#### Scenario: Goal decomposition remains unchanged

- **GIVEN** the plan loop handles `PlanEvent::GoalReady`
- **WHEN** planner framework call handling is refactored
- **THEN** the decomposition prompt SHALL remain unchanged
- **AND** the planner SHALL still be marked `Working` before execution and `Idle` after execution
- **AND** the planner SHALL still emit `ExecutorEvent::TaskStarted` with the goal id before execution
- **AND** the planner SHALL still use `build_traced_agent_with_goal` with the same session id, task id, executor, and goal context
- **AND** the planner SHALL still emit `ExecutorEvent::TaskCompleted` or `ExecutorEvent::TaskFailed` with the same id and agent
- **AND** `PLAN_GOAL_DELEGATE`, SSE plan decision events, EventLog writes, and worker wake timing SHALL remain unchanged

#### Scenario: Task review remains unchanged

- **GIVEN** the plan loop handles `PlanEvent::ReviewNeeded`
- **WHEN** planner framework call handling is refactored
- **THEN** the review prompt SHALL remain unchanged
- **AND** the planner SHALL still be marked `Working` before execution and `Idle` after execution
- **AND** the planner SHALL still emit `ExecutorEvent::TaskStarted` with the task id before execution
- **AND** the planner SHALL still use the current `build_worker_agent` entry point with the same session id, task id, and executor
- **AND** the planner SHALL still emit `ExecutorEvent::TaskCompleted` or `ExecutorEvent::TaskFailed` with the same id and agent
- **AND** `PLAN_REVIEW_DELEGATE`, review result SSE plan decision events, EventLog writes, and worker wake timing SHALL remain unchanged

#### Scenario: Follow-up planning remains unchanged

- **GIVEN** goal evaluation returns `NeedsMoreWork`
- **WHEN** planner framework call handling is refactored
- **THEN** the follow-up planning prompt SHALL remain unchanged
- **AND** the planner SHALL still be marked `Working` before execution and `Idle` after execution
- **AND** the planner SHALL still emit `ExecutorEvent::TaskStarted` with the goal id before execution
- **AND** the planner SHALL still use `build_traced_agent_with_goal` with the same session id, task id, executor, and goal context
- **AND** the planner SHALL still emit `ExecutorEvent::TaskCompleted` or `ExecutorEvent::TaskFailed` with the same id and agent
- **AND** `goal_needs_work` plan decision emission and `PLAN_GOAL_NEEDS_WORK` trace behavior SHALL remain unchanged

### Requirement: Planner helper extraction remains local and non-behavioral

The system SHALL limit planner framework helper extraction to local web-layer glue code without changing orchestration semantics.

#### Scenario: Core orchestration is not changed

- **GIVEN** the plan loop and worker loops are running for an application
- **WHEN** planner framework call handling is refactored
- **THEN** planner selection SHALL remain capability-driven as before
- **AND** task dependency gating, review, retry, follow-up planning, and coordinator resume behavior SHALL remain unchanged
- **AND** SSE, EventLog, run_trace, and browser refresh history consumers SHALL continue to observe the same event names and payload fields
