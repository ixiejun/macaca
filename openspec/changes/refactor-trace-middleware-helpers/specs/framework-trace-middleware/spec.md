## ADDED Requirements

### Requirement: Trace middleware refactor preserves event behavior

The system SHALL preserve existing trace event behavior while unifying duplicated trace middleware helper logic.

#### Scenario: SSE tool result events remain unchanged

- **GIVEN** `SseToolMiddleware` receives a successful tool response
- **WHEN** the middleware emits a `tool_result` SSE event
- **THEN** the event name SHALL remain `tool_result`
- **AND** the payload SHALL keep the existing `tool_name` and `output` fields
- **AND** the `output` value SHALL use the same UTF-8-safe truncation behavior and truncation limit as before

#### Scenario: Channel tool result events remain unchanged

- **GIVEN** `ChannelToolMiddleware` receives a successful tool response
- **WHEN** the middleware sends an `AgentExecutionEvent`
- **THEN** the event SHALL remain `AgentExecutionEvent::ToolResult`
- **AND** the event SHALL keep the existing `tool_name`, `output`, and `is_error` values
- **AND** the `output` value SHALL use the same UTF-8-safe truncation behavior and truncation limit as before

#### Scenario: Executor tool result events remain unchanged

- **GIVEN** `ExecutorToolMiddleware` receives a successful tool response
- **WHEN** the middleware broadcasts an executor event
- **THEN** the event SHALL remain `ExecutorEvent::AgentEvent`
- **AND** the nested agent event SHALL remain `AgentExecutionEvent::ToolResult`
- **AND** the event SHALL keep the existing `task_id`, `agent`, `tool_name`, `output`, and `is_error` values
- **AND** the `output` value SHALL use the same UTF-8-safe truncation behavior and truncation limit as before

### Requirement: Trace middleware helper extraction remains local and non-behavioral

The system SHALL limit this change to local helper extraction in the web framework runner without changing orchestration or task lifecycle semantics.

#### Scenario: Orchestration behavior is not changed

- **GIVEN** coordinator, planner, and worker agents execute through existing traced builder entry points
- **WHEN** trace middleware helper code is refactored
- **THEN** the traced builder entry points SHALL remain the same
- **AND** PlanLoop and WorkerLoop scheduling behavior SHALL remain unchanged
- **AND** task review, dependency gating, and coordinator resume behavior SHALL remain unchanged

#### Scenario: Browser refresh history remains compatible

- **GIVEN** trace events were persisted to EventLog before the refactor
- **WHEN** the browser refreshes and reloads a session history
- **THEN** the frontend SHALL continue to receive the same event names and payload shapes required to reconstruct historical trace steps
