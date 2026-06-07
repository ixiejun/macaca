## ADDED Requirements

### Requirement: Macaca SHALL provide one production agent execution service

Macaca SHALL provide `service.agent_execution` as the only production boundary that starts agent work for chat, YAML, WASM, task, goal, worker, SDK, gateway, and future application adapters.

#### Scenario: WASM delegates to an app-scoped agent

- **GIVEN** a WASM application invokes `macaca:agent/delegate` with app, session, trace, target agent, user prompt, and bounded context
- **WHEN** Macaca admits the request
- **THEN** the request SHALL be converted into `AgentExecutionCommand`
- **AND** it SHALL dispatch through `ServiceRuntime` to `service.agent_execution`
- **AND** it SHALL NOT start agent work through an executor fast path that bypasses the service.

#### Scenario: YAML workflow starts an agent step

- **GIVEN** a YAML application workflow step targets an agent
- **WHEN** the step starts
- **THEN** the YAML adapter SHALL produce `AgentExecutionCommand`
- **AND** the agent SHALL start through `service.agent_execution`
- **AND** the YAML adapter SHALL NOT own separate runtime execution semantics.

### Requirement: Agent execution SHALL separate trusted system context from user prompt

Macaca SHALL prevent application adapters, WASM guests, YAML workflows, task loops, and upstream agents from supplying system prompts to production agent execution. They may supply only user prompts and bounded delegated context.

#### Scenario: Delegated prompt is executed

- **GIVEN** an upstream application or agent delegates work with a prompt
- **WHEN** `service.agent_execution` invokes the target agent runtime
- **THEN** the delegated prompt SHALL be sent as user input
- **AND** trusted system context SHALL come only from `service.agent_context`
- **AND** no production path SHALL treat the delegated prompt as a system prompt.

### Requirement: Agent execution SHALL be trace-required and auditable

Macaca SHALL require trace, app, session, target agent, execution intent, policy context, and capability scope before starting agent work.

#### Scenario: Execution request is missing trace

- **WHEN** an agent execution request lacks trace context
- **THEN** `service.agent_execution` SHALL reject it before side effects
- **AND** return a structured denied or rejected result
- **AND** emit sanitized denial evidence without raw prompt or raw provider payloads.

#### Scenario: Agent execution completes

- **WHEN** an agent execution completes
- **THEN** Macaca SHALL persist lifecycle, context reference, runtime events, result metadata, and sanitized diagnostics before streaming completion to shells.
