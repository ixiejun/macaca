# web-cli-thin-shell-v0 Specification

## Purpose
TBD - created by archiving change add-web-cli-thin-shell-v0. Update Purpose after archive.
## Requirements
### Requirement: Macaca SHALL provide a typed SDK system facade for presentation shells

Macaca SHALL provide a typed SDK system facade for shell-facing session, task, trace, package, service, and approval commands so Web, frontend, and CLI do not own core system semantics.

#### Scenario: Shell-facing commands remain typed and provider-neutral

- **WHEN** Web or CLI needs to inspect task board, session events, trace tail, service registry, package metadata, or approval decisions
- **THEN** it SHALL construct typed SDK facade commands or call typed facade methods
- **AND** it SHALL NOT call a generic untyped RPC dumping ground
- **AND** the facade contracts SHALL NOT depend on `macaca-web`, frontend implementation details, concrete application names, workflow names, provider names, driver names, gateway names, chain names, package names, or business-specific routes

#### Scenario: SDK facade can be used without Web server state

- **WHEN** CLI or another non-Web shell uses the SDK system facade
- **THEN** the facade SHALL operate through kernel/service/application/store adapters
- **AND** it SHALL NOT require a running `macaca-web` server or Web-only session state

### Requirement: Macaca SHALL keep Web routes as command adapters

Macaca Web routes SHALL validate transport/request scope, convert HTTP input into typed system commands, call SDK/Application/Kernel facades, and map results back to stable response shapes without owning system semantics.

#### Scenario: Migrated task board route preserves response shape

- **WHEN** a task board route is migrated through the Web Shell command adapter
- **THEN** the route SHALL still require session scope where required
- **AND** it SHALL preserve the existing JSON response shape expected by frontend callers
- **AND** route code SHALL log scope validation, command construction, facade execution, success, and structured rejection

#### Scenario: Web route does not define core task semantics

- **WHEN** Web handles a task/session/trace/package/service route
- **THEN** Web SHALL only adapt HTTP request/response concerns
- **AND** task/session/trace/package/service semantics SHALL come from SDK/Application/Kernel/service facades

### Requirement: Macaca SHALL keep trace and SSE as Observer presentation behavior

Macaca Web SHALL subscribe to trace/event sources, forward or render bounded events, and preserve replay cursors without redefining core trace semantics.

#### Scenario: Real-time trace remains live

- **WHEN** agent, task, service, driver, skill, MCP, plugin, payment, Web3, EVM, or UI events occur during an active session
- **THEN** Web SHALL forward live trace events through SSE or equivalent shell transport
- **AND** `RC-TRACE-001` SHALL remain valid

#### Scenario: Historical trace replay remains complete and non-duplicated

- **WHEN** a user refreshes or reloads a session
- **THEN** Web SHALL replay historical trace from EventLog or equivalent trace source using session-scoped cursors
- **AND** replay SHALL avoid duplicate historical/live events
- **AND** `RC-TRACE-002` SHALL remain valid

### Requirement: Macaca SHALL keep frontend as generic Shell and GenUI renderer

Macaca frontend SHALL render chat, trace, task board, package metadata, approvals, and GenUI surfaces as generic shell views rather than application-specific UI branches.

#### Scenario: No GenUI surface falls back to chat and trace shell

- **WHEN** an application has no GenUI surface
- **THEN** frontend SHALL keep the existing chat, trace, session logs, and task board shell behavior
- **AND** no application-specific fallback branch SHALL be required

#### Scenario: GenUI surface renders by schema kind

- **WHEN** an application provides a valid GenUI surface
- **THEN** frontend SHALL render it by schema/component/event kind
- **AND** frontend SHALL NOT dispatch by application name, workflow name, provider name, driver name, gateway name, chain name, package name, or business-specific route

### Requirement: Macaca SHALL keep CLI as facade-backed command shell

Macaca CLI SHALL parse terminal input, format terminal output, and start shell processes, but SHALL delegate system inspection and operation semantics to SDK facades or lower service/application facades.

#### Scenario: CLI system inspection uses SDK facade

- **WHEN** CLI inspects agents, services, sessions, traces, applications, packages, or approvals through migrated commands
- **THEN** CLI SHALL call SDK facade commands or typed lower-layer facades
- **AND** CLI SHALL NOT depend on `macaca-web` internals for those semantics

#### Scenario: CLI has no replaced direct helpers

- **WHEN** a CLI command has a facade-backed path
- **THEN** no replaced direct helper SHALL remain in production source
- **AND** command handlers SHALL use facade-backed paths

### Requirement: Macaca SHALL protect chat, session, trace, and task board regressions during thin shell operation

Macaca SHALL preserve Web/CLI thin shell behavior without regressing chat session creation, chat session resume, real-time trace, historical trace replay, or session-scoped task board behavior.

#### Scenario: Terminal thin-shell regression checks pass

- **WHEN** terminal thin-shell verification runs
- **THEN** `RC-CHAT-001`, `RC-CHAT-002`, `RC-TRACE-001`, `RC-TRACE-002`, and `RC-TASK-001` SHALL remain valid
- **AND** existing YAML applications, `/api/chat/v2`, task board, trace, resume, session replay, GenUI fallback, Web UI, and CLI behavior SHALL continue to compile and run through canonical protocol/service paths

### Requirement: Macaca SHALL delete replaced direct presentation-owned semantic paths

After a direct Web/CLI semantic path is replaced by a facade-backed command path, Macaca SHALL remove the old direct path and guard against reintroduction.

#### Scenario: Replaced presentation semantic helpers are absent

- **WHEN** a terminal guard or test scans callers after a helper is replaced
- **THEN** replaced direct Web/CLI semantic helpers SHALL be absent from production source
- **AND** upper-layer callers SHALL NOT use replaced direct Web/CLI semantic helpers

### Requirement: Macaca SHALL log and audit Web/CLI shell command boundaries

Macaca SHALL emit structured logs and replayable trace/audit records for shell command scope validation, command construction, facade execution, success, structured rejection, and failure.

#### Scenario: Rejected shell command is auditable

- **WHEN** a Web route or CLI command rejects a request due to missing scope, permission, policy, unavailable service, malformed input, or facade failure
- **THEN** logs or trace/audit records SHALL include shell kind, operation, route or command name when available, app id when available, session id when available, task id when available, request id or cursor when available, structured error code, and timestamp
- **AND** logs SHALL NOT include secrets, private keys, provider credentials, raw payment credentials, raw encrypted package contents, raw unbounded user input, or provider secrets

### Requirement: Macaca SHALL document new Web/CLI thin shell code with detailed English comments

All new Web/CLI thin-shell Rust and frontend code SHALL include detailed English comments explaining shell boundaries, command adapters, facade delegation, trace/audit behavior, replay cursors, GenUI rendering guardrails, and explicit non-goals.

#### Scenario: Maintainer can understand shell boundaries from comments

- **WHEN** a maintainer reads new Web/CLI thin shell modules
- **THEN** comments SHALL explain which layer owns transport, command construction, facade delegation, system semantics, trace/audit emission, and non-goals
- **AND** comments SHALL explain why Web/CLI must not define session, task, trace, package, service, payment, Web3, EVM, plugin, entitlement, or application UI semantics
