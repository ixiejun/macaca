# execution-control-service Specification

## Purpose
TBD - created by archiving change add-execution-control-service-v1. Update Purpose after archive.
## Requirements
### Requirement: Macaca SHALL provide optional execution-control capability

Macaca SHALL model pause, resume, checkpoint identity, execution-control state, and resume signals as an optional execution capability that can be enabled or disabled per application and per execution.

#### Scenario: Execution runs without execution control

- **GIVEN** an application does not declare execution-control capability
- **AND** an `AgentExecutionCommand` does not request execution-control override
- **WHEN** `service.agent_execution` starts the run
- **THEN** no pause/resume adapter SHALL be installed
- **AND** the run SHALL preserve ordinary agent execution behavior
- **AND** trace output SHALL record that execution control was disabled by policy

#### Scenario: Execution runs with app-selected execution control

- **GIVEN** an application declares execution-control capability in its manifest or application metadata
- **WHEN** `service.agent_execution` starts a matching run
- **THEN** the runtime SHALL install execution-control adapters according to the resolved policy
- **AND** pause/resume behavior SHALL be driven by declared trigger and resume strategies
- **AND** no application name, agent name, workflow name, provider name, or driver name branch SHALL be required

### Requirement: Macaca SHALL merge application defaults and command overrides deterministically

Macaca SHALL support execution-control policy from both application defaults and `AgentExecutionCommand` overrides. The resolver SHALL produce deterministic enabled, disabled, denied, or unsupported outcomes.

#### Scenario: Command override narrows app default

- **GIVEN** an application declares execution-control defaults with multiple allowed triggers and resume sources
- **AND** an `AgentExecutionCommand` override selects a subset of those triggers and resume sources
- **WHEN** the policy resolver evaluates the run
- **THEN** the resolver SHALL enable execution control with the narrower command policy
- **AND** trace/audit evidence SHALL identify both the app default source and the command override source

#### Scenario: Command override exceeds app permission

- **GIVEN** an application does not allow dynamic execution-control opt-in for a trigger
- **AND** an `AgentExecutionCommand` override requests that trigger
- **WHEN** the policy resolver evaluates the run
- **THEN** the resolver SHALL return a structured denied or unsupported result before side effects
- **AND** the runtime SHALL NOT silently fall back to another trigger

### Requirement: Macaca SHALL use strategy-driven pause triggers and resume sources

Execution-control policies SHALL express pause triggers and resume sources through typed strategies, not through hardcoded application, agent, workflow, provider, or driver names.

#### Scenario: Tool-call barrier pauses an execution

- **GIVEN** a resolved execution-control policy declares a tool-call barrier trigger
- **WHEN** the target tool call completes with a pause-worthy result
- **THEN** the execution-control adapter SHALL request a pause with a reason code and trace context
- **AND** the execution state SHALL transition through a typed pause state
- **AND** the event stream SHALL include sanitized pause evidence

#### Scenario: Goal lifecycle resumes an execution

- **GIVEN** a paused execution is waiting for a goal lifecycle resume source
- **WHEN** the goal reaches a terminal resume-worthy state
- **THEN** execution control SHALL accept the resume signal if policy allows it
- **AND** the waiting execution SHALL resume exactly once
- **AND** duplicate or stale resume signals SHALL be ignored with traceable diagnostics

### Requirement: Macaca SHALL emit traceable and auditable execution-control events

Macaca SHALL emit sanitized trace, audit, EventLog, RunTrace, and service diagnostic evidence for all execution-control state transitions and command outcomes.

#### Scenario: Pause and resume are replayable

- **GIVEN** an execution-control-enabled run pauses and later resumes
- **WHEN** a trace or audit replay is requested
- **THEN** replay SHALL show policy resolution, pause requested, pause entered, checkpoint recorded when configured, resume requested, resume accepted, and resume delivered
- **AND** replay SHALL include stable ids, reason codes, trace id, session id, execution id, and bounded metadata
- **AND** replay SHALL NOT include raw prompts, raw manifests, credentials, package bytes, WASM bytes, private keys, raw provider payloads, or unbounded output

### Requirement: Macaca SHALL expose execution control as a system service

Macaca SHALL provide `service.execution_control` after the built-in runtime capability is established. The service SHALL expose typed commands, descriptor, lifecycle, health, snapshot, unavailable provider behavior, trace-required calls, policy checks, structured errors, and sanitized audit events.

#### Scenario: Execution control service is available

- **GIVEN** `service.execution_control` is registered and running
- **WHEN** `service.agent_execution` needs to register an execution, request pause, await resume, request resume, query state, or create a snapshot
- **THEN** it SHALL call `service.execution_control` through `ServiceRuntime`
- **AND** each call SHALL carry application id, session id, execution id, source, command name, and trace context
- **AND** policy SHALL be checked before any side effect

#### Scenario: Execution control service is unavailable

- **GIVEN** `service.execution_control` is absent or unavailable
- **WHEN** a run requires execution-control capability
- **THEN** the system SHALL return a structured unavailable state
- **AND** the system SHALL NOT hang, fake success, silently disable required pause/resume behavior, or bypass policy

