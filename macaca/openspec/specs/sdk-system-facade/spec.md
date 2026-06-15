# sdk-system-facade Specification

## Purpose
TBD - created by archiving change update-sdk-system-facade-convergence. Update Purpose after archive.
## Requirements
### Requirement: Macaca SHALL expose SDK SystemFacade as the upper-layer system boundary

Macaca SHALL expose an SDK `SystemFacade` that upper layers use for service, task, trace, package, approval, and status operations instead of directly owning lower-layer system semantics.

#### Scenario: Upper layer calls the system through SDK facade

- **WHEN** Web, CLI, gateway, application runtime, or future plugins need a migrated system operation
- **THEN** they SHALL construct a typed SDK command or call a typed SDK facade method
- **AND** they SHALL NOT introduce a new direct provider construction path for that operation

#### Scenario: Stable contracts remain intact

- **WHEN** SDK facade convergence is implemented
- **THEN** existing YAML application loading, `/api/chat/v2`, trace viewer, task board, resume, driver, skill/MCP, Web UI, and CLI behavior SHALL keep stable external contracts
- **AND** the SDK SHALL route through focused clients, service facades, or proto DTOs without constructing providers

### Requirement: SDK clients SHALL be split by capability family

Macaca SHALL split SDK system clients into focused capability-family modules for service, task, trace, package, and status operations.

#### Scenario: Client module owns focused commands

- **WHEN** a system operation belongs to service, task, trace, package, or status scope
- **THEN** the corresponding SDK client module SHALL own typed command and result types for that operation
- **AND** the module SHALL expose a replaceable client trait or adapter boundary

#### Scenario: Facade avoids generic type explosion

- **WHEN** additional SDK system clients are added
- **THEN** `SystemFacade` SHALL compose focused clients or a small client bundle
- **AND** it SHALL NOT grow into an unreadable generic parameter list for every capability family

### Requirement: SDK commands SHALL be typed, scoped, and validated

SDK system operations SHALL be represented as typed command objects with explicit scope, bounded cursor or limit fields where applicable, and trace/policy-ready metadata.

#### Scenario: Invalid command scope is rejected

- **WHEN** a command is constructed with missing required scope such as blank session id, service id, package reference, or trace cursor scope
- **THEN** command construction or facade execution SHALL reject it with a structured error
- **AND** the rejection SHALL be logged with operation and scope fields when available

#### Scenario: Command remains provider-neutral

- **WHEN** a command is used by Web, CLI, gateway, or application code
- **THEN** it SHALL NOT require concrete provider, driver, gateway, model, workflow, chain, or business-specific identifiers beyond provider-neutral system scope

### Requirement: SystemFacade SHALL delegate to clients and preserve stable response contracts

`SystemFacade` SHALL validate/log facade calls, delegate to focused clients, and preserve existing response shapes for current task-board and status operations.

#### Scenario: Task board query keeps stable contract

- **WHEN** a session-scoped task board query is executed through `SystemFacade`
- **THEN** it SHALL return todos sorted by sequence number
- **AND** it SHALL preserve the existing task board result shape
- **AND** it SHALL log start and completion with application id, session id, and count

#### Scenario: Status snapshot keeps stable contract

- **WHEN** a status snapshot is requested through `SystemFacade`
- **THEN** it SHALL return the existing status snapshot fields
- **AND** it SHALL log the request and structured warnings for invalid snapshot data such as zero max agents

### Requirement: Unsupported service operations SHALL fail structurally

SDK system clients SHALL return structured unavailable or unsupported errors for operations whose concrete service providers are not migrated yet.

#### Scenario: Service call has no backing service

- **WHEN** a service call command targets a capability that has no S3 backing client
- **THEN** the SDK client SHALL return a structured unavailable or unsupported error
- **AND** it SHALL NOT panic, hang, silently succeed, or construct a concrete provider

### Requirement: SDK clients SHALL emit audit-friendly logs

SDK system clients and `SystemFacade` SHALL emit structured logs at command validation, facade entry, client delegation, success, rejection, and failure boundaries.

#### Scenario: Client rejection is auditable

- **WHEN** an SDK client rejects a command
- **THEN** logs SHALL include operation, command kind, relevant app/session/task/service/package scope when available, structured error, and timestamp
- **AND** logs SHALL NOT include secrets, provider credentials, raw encrypted package contents, private keys, or unbounded user input

### Requirement: SDK clients SHALL not become provider factories

SDK system clients SHALL adapt existing local state, kernel/service facades, or future `ServiceRuntime`/`ServiceBus` handles, but SHALL NOT construct concrete providers.

#### Scenario: Client implementation is audited for provider construction

- **WHEN** maintainers inspect SDK service, task, trace, package, and status clients
- **THEN** the clients SHALL be traits/adapters over existing boundaries
- **AND** they SHALL NOT instantiate concrete LLM, memory, task planner, driver, skill, MCP, gateway, payment, Web3, EVM, or package provider implementations

### Requirement: SDK/SystemFacade Governance SHALL Be Documented

Macaca SHALL document that SDK/SystemFacade is the upper-layer system API boundary and that concrete providers are owned by service/runtime-host boundaries.

#### Scenario: Governance explains SDK ownership

- **WHEN** maintainers read Macaca OS architecture governance
- **THEN** it SHALL state that SDK/SystemFacade is command-driven and client-composed
- **AND** it SHALL state that SDK clients are adapters, not provider factories
- **AND** it SHALL state that service/runtime-host owners are responsible for concrete service/provider lifecycle
