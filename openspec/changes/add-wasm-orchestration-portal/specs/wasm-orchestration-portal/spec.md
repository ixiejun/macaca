## ADDED Requirements

### Requirement: WASM applications SHALL access OS orchestration through a generic portal

Macaca SHALL expose a WASM Orchestration Portal that maps guest task and agent orchestration imports to provider-neutral OS commands without exposing Web state, Kernel internals, concrete providers, or business-specific code.

#### Scenario: WASM guest creates a task goal

- **GIVEN** a WASM application session has app id, session id, trace context, and task capability metadata
- **WHEN** the guest invokes `macaca:task/create_goal`
- **THEN** Macaca SHALL route the request through ServiceRuntime to the Task Service boundary
- **AND** the result SHALL be returned as a bounded provider-neutral host command result
- **AND** the host SHALL log admission, dispatch, completion, and failure with trace id, app id, session id, import name, service id, operation, and reason code.

#### Scenario: WASM guest queries the task board

- **GIVEN** a WASM application session has app id, session id, trace context, and task capability metadata
- **WHEN** the guest invokes `macaca:task/query`
- **THEN** Macaca SHALL return only the session-scoped task board view
- **AND** it SHALL preserve task ordering by sequence number
- **AND** it SHALL NOT silently fall back to cross-session task scanning.

### Requirement: WASM applications SHALL be able to delegate to app-scoped agents

Macaca SHALL support a generic `macaca:agent/delegate` import that requests work from agents declared by the same application and session.

#### Scenario: WASM guest delegates to an app-scoped agent

- **GIVEN** a WASM application declares agents in its manifest
- **AND** the requested agent is part of the app-scoped executor
- **WHEN** the guest invokes `macaca:agent/delegate` with trace, app id, session id, target agent, prompt, and bounded context
- **THEN** Macaca SHALL dispatch the request through the Application Service orchestration backend
- **AND** the delegated work SHALL use the same executor isolation, trace, task, and session surfaces as YAML application delegation
- **AND** the host SHALL NOT route to global fallback agents.

#### Scenario: WASM guest requests an undeclared agent

- **GIVEN** a WASM application does not declare the requested agent
- **WHEN** the guest invokes `macaca:agent/delegate`
- **THEN** Macaca SHALL fail closed with a structured policy denial
- **AND** no worker execution SHALL start.

### Requirement: WASM orchestration SHALL remain policy-governed and auditable

Macaca SHALL require trace context, app/session scope, payload bounds, capability metadata, service-contract admission, and app-scoped agent validation for WASM orchestration imports.

#### Scenario: Orchestration import lacks trace or session scope

- **WHEN** a WASM orchestration import lacks trace context or a non-empty session id
- **THEN** Macaca SHALL reject the import before dispatch
- **AND** it SHALL return a stable reason code such as `missing_trace` or `scope_missing`
- **AND** it SHALL log the denial without raw payload bodies or secrets.

#### Scenario: Skill or MCP is invoked from WASM

- **GIVEN** a WASM application declares permission to use Skill or MCP service capabilities
- **WHEN** the guest invokes `macaca:service/call` for `service.skill` or `service.mcp`
- **THEN** Macaca SHALL use the existing ServiceRuntime-backed service call path
- **AND** it SHALL preserve Skill/MCP service ownership and policy checks
- **AND** it SHALL NOT add WASM-specific Skill or MCP business branches.

### Requirement: WASM applications with declared agents SHALL not lose WASM runtime flexibility

Macaca SHALL continue to execute WASM applications through their WASM runtime even when they declare app-scoped agents, while preparing executor and loop support as OS capabilities for the session.

#### Scenario: WASM app declares agents and receives chat input

- **GIVEN** an L2Wasm application declares runtime ability and application agents
- **WHEN** `/api/chat/v2` starts a new session for the app
- **THEN** Macaca SHALL persist the chat session, start the Application Service session envelope, prepare the app-scoped executor and PlanLoop/WorkerLoop, and dispatch the WASM export
- **AND** it SHALL NOT replace the WASM guest with a YAML-style coordinator path unless the application explicitly uses a non-WASM runtime.

### Requirement: WASM orchestration SHALL be application-agnostic

Macaca SHALL implement the portal without hardcoding application names, workflow names, driver names, provider names, symbols, domain-specific payloads, or business logic.

#### Scenario: A new WASM app uses the portal

- **GIVEN** a new WASM application declares the required service contracts and permissions
- **WHEN** it invokes task, agent, service, skill, MCP, or UI imports through the portal
- **THEN** Macaca SHALL route by generic service ids, operations, capabilities, app/session scope, and manifest declarations
- **AND** no OS crate SHALL require application-specific code to support the new app.
