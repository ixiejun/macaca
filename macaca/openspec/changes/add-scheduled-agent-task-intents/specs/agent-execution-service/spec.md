## MODIFIED Requirements

### Requirement: Macaca SHALL provide one production agent execution service

Macaca SHALL provide `service.agent_execution` as the only production boundary
that starts agent work for chat, YAML, WASM, task, goal, worker, Scheduler due
runs, SDK, gateway, and future application adapters.

#### Scenario: Scheduled task run invokes an app-scoped agent

- **GIVEN** Runtime Host leases a Scheduler run whose target is `SchedulerTargetCommand::AgentExecution`
- **WHEN** Runtime Host resolves the target payload reference through the Scheduled Agent Task service
- **THEN** it SHALL produce an `AgentExecutionCommand`
- **AND** it SHALL dispatch through `ServiceRuntime` to `service.agent_execution`
- **AND** it SHALL NOT start agent work through a Scheduler provider, Web route, frontend component, or executor fast path that bypasses the service.

#### Scenario: WASM delegates to an app-scoped agent

- **GIVEN** a WASM application invokes `macaca:agent/delegate` with app, session, trace, target agent, user prompt, and bounded context
- **WHEN** Macaca admits the request
- **THEN** the request SHALL be converted into `AgentExecutionCommand`
- **AND** it SHALL dispatch through `ServiceRuntime` to `service.agent_execution`
- **AND** it SHALL NOT start agent work through an executor fast path that bypasses the service.

### Requirement: Agent execution SHALL separate trusted system context from user prompt

Macaca SHALL prevent application adapters, WASM guests, YAML workflows, task
loops, Scheduler due-run dispatchers, and upstream agents from supplying system
prompts to production agent execution. They may supply only user prompts and
bounded delegated context.

#### Scenario: Scheduled task prompt is executed

- **GIVEN** a scheduled agent task payload contains task requirements
- **WHEN** `service.agent_execution` invokes the target agent runtime
- **THEN** the scheduled task requirements SHALL be sent as user input
- **AND** trusted system context SHALL come only from `service.agent_context`
- **AND** no production path SHALL treat the scheduled task prompt as a system prompt.

#### Scenario: Agent execution completes

- **WHEN** scheduled agent task execution completes
- **THEN** Macaca SHALL persist lifecycle, context reference, runtime events, result metadata, and sanitized diagnostics
- **AND** the Scheduled Agent Task audit chain SHALL be able to correlate the result without exposing raw provider output.
