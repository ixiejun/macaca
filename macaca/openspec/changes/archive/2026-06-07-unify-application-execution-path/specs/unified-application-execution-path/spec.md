## ADDED Requirements

### Requirement: Single application execution ingress
The system SHALL route every production application execution request through `service.application_execution`, regardless of whether the application is WASM, YAML, GenUI, headless, gateway-triggered, or app-owned UI.

#### Scenario: WASM application starts a task
- **WHEN** a WASM application requests agent work through a host import
- **THEN** the host import SHALL submit a typed application execution or agent execution command through `service.application_execution`
- **AND** it SHALL NOT create an independent authoritative task graph outside `service.task`.

#### Scenario: YAML application starts a workflow step
- **WHEN** a YAML application workflow step requests agent work
- **THEN** the YAML adapter SHALL submit the same provider-neutral command shape used by WASM applications
- **AND** the resulting trace phases SHALL include application execution, task graph admission, and agent execution.

### Requirement: Single authoritative task graph per execution run
The system SHALL allow at most one authoritative Task Service graph for a single application execution run.

#### Scenario: Compatibility fallback occurs during a run
- **WHEN** compatibility fallback decomposition is needed during an application execution session
- **THEN** the fallback tasks SHALL be marked as compatibility or diagnostic graph entries
- **AND** they SHALL NOT become authoritative terminal facts for the application execution run.

#### Scenario: Duplicate authoritative graph is requested
- **WHEN** a second authoritative graph is requested for the same application id, session id, and run id
- **THEN** Task Service SHALL reject it or return the existing graph according to idempotency
- **AND** the service SHALL emit bounded trace/audit evidence with a reason code.

### Requirement: Agent work uses the agent execution service
The system SHALL start production agent work only through `service.agent_execution`.

#### Scenario: Application adapter delegates to an agent
- **WHEN** an application adapter delegates work to an app-scoped agent
- **THEN** the adapter SHALL produce a typed command for `service.agent_execution`
- **AND** it SHALL NOT construct runtime agents, framework runners, model calls, or tool loops directly.

### Requirement: Terminal state is projected from authoritative service facts
The system SHALL compute application execution terminal state from the authoritative application execution run and its authoritative Task Service graph.

#### Scenario: Authoritative tasks complete and compatibility task fails
- **WHEN** all required authoritative execution tasks are completed
- **AND** a compatibility or diagnostic task has failed
- **THEN** the application execution current state SHALL NOT be `Failed` solely because of the compatibility or diagnostic task
- **AND** the failure SHALL remain visible as a diagnostic event.

#### Scenario: Required authoritative task fails
- **WHEN** a required authoritative execution task fails and bounded recovery is exhausted
- **THEN** the application execution current state SHALL be `Failed`
- **AND** the EventLog replay SHALL include the task failure, recovery state, and terminal reason code.

### Requirement: Shells and app-owned UI are projection adapters
The system SHALL prevent Web, CLI, frontend, and app-owned UI code from owning production execution semantics.

#### Scenario: Browser refreshes during or after execution
- **WHEN** the browser reloads an application session
- **THEN** the UI SHALL load persisted session state, replay EventLog rows, and query current-state projection
- **AND** it SHALL NOT infer authoritative terminal state from local cached event arrays.

#### Scenario: Web route receives an execution request
- **WHEN** a Web route receives a start, control, replay, current-state, or session query request
- **THEN** the route SHALL call SDK/SystemFacade or a focused service client
- **AND** it SHALL NOT directly run provider loops, create semantic task graphs, or persist authoritative execution events.

### Requirement: Execution evidence is traceable and sanitized
The system SHALL emit replayable, sanitized trace and audit evidence for every key execution node.

#### Scenario: Task graph admission is evaluated
- **WHEN** Task Service admits, rejects, or compatibility-scopes a task graph
- **THEN** it SHALL log application id, session id, run id, graph owner, trace id, lifecycle state, and reason code
- **AND** it SHALL NOT log raw prompts, secrets, package bytes, WASM bytes, raw provider payloads, or unbounded output.

#### Scenario: Hosted execution aggregates host command rows
- **WHEN** hosted application execution evaluates host command rows
- **THEN** it SHALL emit bounded diagnostic counts for non-authoritative failed rows
- **AND** it SHALL compute terminal state only from authoritative execution facts.

### Requirement: No application-specific execution branches
The system SHALL reject OS-layer and generic service implementation paths that branch on application name, workflow name, model name, provider name, driver name, gateway name, programming language, or business domain.

#### Scenario: CODEX-WASM-WORKBENCH validates the unified path
- **WHEN** CODEX-WASM-WORKBENCH runs a coding task
- **THEN** any successful behavior SHALL be explained by generic application execution, task, agent execution, file, process, and event services
- **AND** no OS-layer code SHALL contain a Workbench-specific branch.
