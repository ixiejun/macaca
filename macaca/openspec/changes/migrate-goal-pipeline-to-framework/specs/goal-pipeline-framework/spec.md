## ADDED Requirements

### Requirement: Framework-Backed Goal Execution Path
Macaca SHALL support migrating goal-pipeline execution from ad-hoc delegation
and polling into framework-backed agent execution while preserving the existing
OS scheduling, task board, persistence, and HTTP contracts.

#### Scenario: Goal execution uses framework agent runner
- **GIVEN** a goal-pipeline consumer chooses the framework-backed execution path
- **WHEN** the consumer needs agent output for planning, work, review, or follow-up
- **THEN** it invokes a framework agent runner and maps the structured result back to existing task-board state transitions

### Requirement: Goal Pipeline Adapter Boundaries
Macaca SHALL bridge existing LLM providers, tool registries, shell event
streams, and trace stores into the framework through adapters rather than by
embedding concrete provider or shell behavior inside the framework.

#### Scenario: Existing tools are exposed through a toolkit adapter
- **GIVEN** an existing tool registry and a framework-backed agent
- **WHEN** the agent needs tool definitions or tool execution
- **THEN** an adapter exposes the tools through framework toolkit contracts without changing the original tool registry ownership

### Requirement: Scheduler and Execution Separation
Macaca SHALL keep goal scheduling semantics separate from framework execution
semantics during migration.

#### Scenario: Worker consumer executes a claimed task
- **GIVEN** a task has already been claimed by the OS scheduling layer
- **WHEN** the execution layer runs a framework-backed agent for that task
- **THEN** scheduling ownership remains with the existing loop and store while execution output is submitted through existing state-transition APIs

### Requirement: Traceable Framework Migration
Macaca SHALL emit structured logs, trace events, and sanitized diagnostics at
framework runner construction, adapter delegation, agent execution start,
agent execution completion, timeout, cancellation, and failure boundaries.

#### Scenario: Framework-backed execution fails
- **GIVEN** a framework-backed goal-pipeline execution fails before producing an acceptable result
- **WHEN** the failure is surfaced to the pipeline
- **THEN** logs and trace evidence include operation, session or task scope when available, sanitized error code, and timestamp without leaking secrets or raw unbounded user input

### Requirement: Additive Goal Pipeline Rollout
Macaca SHALL migrate the goal pipeline additively so existing chat, task-board,
trace, session recovery, and API response contracts remain compatible until a
future approved change removes legacy paths.

#### Scenario: Legacy execution remains available during migration
- **GIVEN** the framework-backed path is not selected or is not yet proven as the default
- **WHEN** a caller uses the existing public contract
- **THEN** the legacy-compatible behavior remains available and no public API shape changes without a separate approved proposal

