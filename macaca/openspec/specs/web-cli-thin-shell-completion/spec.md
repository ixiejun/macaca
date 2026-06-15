# web-cli-thin-shell-completion Specification

## Purpose
Terminal thin-shell presentation boundaries for Macaca Web and CLI: runtime-host-owned provider bootstrap, SDK-only workspace dependencies (`macaca-proto` + `macaca-sdk`), facade-backed capability access, and auditable shell adapters. Extended to P5 terminal state by `refactor-unified-call-path-microkernel` (iteration 125–126).
## Requirements
### Requirement: Macaca SHALL move service provider bootstrap out of presentation shell ownership

Macaca SHALL provide a runtime-host-owned bootstrap boundary for service provider registration and startup so Web and CLI do not own provider lifecycle semantics.

#### Scenario: Runtime-host bootstrap starts service families for Web shell consumption

- **WHEN** Web startup needs Store, Entitlement, Payment, Web3, or EVM services
- **THEN** Web SHALL use a runtime-host bootstrap boundary or bundle to register and start those providers
- **AND** Web SHALL NOT define provider lifecycle semantics in route handlers or presentation adapters
- **AND** the bootstrap SHALL return structured diagnostics and started service ids

#### Scenario: Bootstrap remains provider-neutral at the shell boundary

- **WHEN** the bootstrap exposes services to Web or CLI
- **THEN** it SHALL expose a `ServiceRuntime`, typed SDK clients, or typed factory inputs
- **AND** it SHALL NOT expose a generic untyped service locator
- **AND** it SHALL NOT hardcode application names, workflow names, provider names, driver names, gateway names, model names, chain names, package names, or business-specific routes

### Requirement: Macaca SHALL keep Web as an adapter over runtime-backed SDK clients

Macaca Web SHALL consume runtime-backed SDK focused clients or `SystemFacade` for migrated system surfaces while keeping HTTP, SSE, GenUI, response mapping, and approval UI as presentation responsibilities.

#### Scenario: Web builds shell clients from a host runtime bundle

- **WHEN** Web startup receives the host runtime bundle
- **THEN** Web SHALL build focused SDK clients or a `SystemFacade` from the bundle
- **AND** Web routes SHALL use those clients for migrated status, service inspection, optional module, package, entitlement, and payment surfaces
- **AND** Web SHALL preserve existing response shapes unless a future proposal explicitly changes them

#### Scenario: Web keeps runtime state outside presentation ownership

- **WHEN** chat, framework, session, toolkit, or resume paths need runtime state
- **THEN** Web SHALL keep only presentation state and SDK/runtime-host owner handles
- **AND** pause/resume, provider, toolkit, and agent execution semantics SHALL be owned by service/runtime-host boundaries
- **AND** no retired presentation-owned anchor SHALL remain in production source

### Requirement: Macaca SHALL keep CLI as terminal shell and narrow Web dependency

Macaca CLI SHALL remain responsible for terminal parsing, terminal formatting, process lifecycle, and explicit server launch only, while delegating system semantics to SDK/SystemFacade or focused clients.

#### Scenario: CLI inspection path avoids Web internals

- **WHEN** CLI handles a migrated read-only inspection command
- **THEN** it SHALL call `SystemFacade` or a focused SDK client
- **AND** it SHALL NOT depend on `macaca-web` internals for inspection semantics

#### Scenario: CLI server command is explicitly isolated

- **WHEN** CLI starts the Web server
- **THEN** the command SHALL use a narrow public server-start adapter seam
- **AND** it SHALL NOT duplicate Web runtime/provider/service composition semantics inside CLI
- **AND** CLI inspection and operation commands SHALL NOT depend on `macaca-web` internals

### Requirement: Macaca SHALL delete replaced presentation-owned semantic paths

Macaca SHALL remove replaced Web/CLI direct semantic helpers after facade-backed paths exist. Production code SHALL contain no retired presentation-owned semantic path, old-path anchor, or revert-only wrapper.

#### Scenario: Web provider/runtime fields are absent

- **WHEN** implementation scans `AppState`
- **THEN** direct provider/runtime fields SHALL be absent
- **AND** capability access SHALL go through SDK clients, `SystemFacade`, or runtime-host owner handles

#### Scenario: Guard rejects replaced definitions and direct callers

- **WHEN** terminal guards or tests scan presentation code
- **THEN** replaced direct helper definitions SHALL fail the guard
- **AND** upper-layer callers of replaced direct helpers SHALL fail the guard

### Requirement: Macaca SHALL preserve Web/CLI regression scenarios

Macaca SHALL preserve chat session creation, chat session resume, real-time trace, historical trace replay, session-scoped task board, service unavailable behavior, optional module unavailable behavior, Web UI, or CLI behavior while enforcing the terminal thin-shell boundary.

#### Scenario: Mandatory regression gates pass

- **WHEN** terminal thin-shell verification runs
- **THEN** `RC-CHAT-001`, `RC-CHAT-002`, `RC-TRACE-001`, `RC-TRACE-002`, and `RC-TASK-001` SHALL remain valid
- **AND** the protocol microkernel, shell dependency purity, and audit replay gates SHALL pass

### Requirement: Macaca SHALL keep dependency governance terminal

Macaca SHALL keep presentation dependency governance terminal: Web and CLI may depend only on `macaca-sdk` and `macaca-proto` among workspace crates, and dependency-boundary tests SHALL reject any exception row.

#### Scenario: Dependency purity is proven by executable gate

- **WHEN** implementation claims terminal shell purity
- **THEN** `cargo metadata` and the executable dependency boundary gate SHALL prove Web/CLI have no extra workspace dependency
- **AND** any non-empty shell dependency exception row SHALL fail the gate

### Requirement: Macaca SHALL log and audit S12 shell and bootstrap boundaries

Macaca SHALL emit structured logs at runtime-host bootstrap, Web route adapter, and CLI command adapter execution nodes.

#### Scenario: Bootstrap and shell failures are auditable

- **WHEN** bootstrap, Web route adapter, or CLI command adapter fails or rejects work
- **THEN** logs SHALL include layer kind, operation, service id when available, command name when available, trace id when available, app/session/task scope when available, sanitized status, and reason code
- **AND** logs SHALL NOT include secrets, private keys, provider credentials, raw payment credentials, raw encrypted package contents, raw prompt bodies, raw package bytes, wallet secrets, raw signed transactions, raw ABI/bytecode, raw tool payloads, unbounded user input, or provider secrets

### Requirement: Macaca SHALL document new S12 Rust code with detailed English comments

All new terminal thin-shell Rust code SHALL include detailed English comments explaining ownership, design pattern intent, runtime behavior, trace/audit behavior, and non-goals.

#### Scenario: Maintainer can identify ownership from code comments

- **WHEN** a maintainer reads new runtime-host bootstrap, Web shell adapter, or CLI shell adapter code
- **THEN** comments SHALL explain which layer owns provider lifecycle, transport adaptation, command construction, facade delegation, system semantics, and trace/audit emission
- **AND** comments SHALL explain why Web/CLI must not define provider, service, session, task, trace, package, payment, Web3, EVM, plugin, entitlement, or application workflow semantics

### Requirement: Web Shell Holds No Direct Provider State

`macaca-web` `AppState` SHALL NOT hold direct provider/runtime fields (application runtime, registry, LLM provider/router, memory runtime, MCP runtime, driver runtime/registry). The shell SHALL access all capabilities through `SystemFacade`/focused SDK clients held in a small composition bundle.

#### Scenario: AppState exposes only facade clients
- **WHEN** `macaca-web/src/state.rs` is inspected
- **THEN** it SHALL NOT contain direct provider/runtime fields
- **AND** capability access SHALL occur through SDK clients only

### Requirement: Web And CLI Depend Only On The SDK

`macaca-web` and `macaca-cli` SHALL depend only on `macaca-sdk` (plus provider-neutral proto DTOs). They SHALL NOT directly depend on the kernel, runtime-host internals, or concrete service-provider crates.

#### Scenario: Shell dependency trees are minimal
- **WHEN** `cargo tree -e normal -p macaca-web --depth 1` and `-p macaca-cli --depth 1` are evaluated
- **THEN** internal workspace dependencies SHALL be limited to `macaca-sdk` and `macaca-proto` at terminal state
- **AND** there SHALL be zero web and zero cli dependency-gate exception rows at terminal state

### Requirement: Session Loop Ownership Is Serviceized

Session loop control (plan/worker loop pull, wakeup, heartbeat, Fork-Join pause/resume orchestration) SHALL be owned by `service.execution_control` and the task service. The Web shell SHALL only expose SSE endpoints, map HTTP DTOs, and subscribe to events.

#### Scenario: Web subscribes instead of orchestrating
- **WHEN** a delegated or goal-driven execution pauses and resumes
- **THEN** the orchestration SHALL be driven by execution-control/task service events
- **AND** the Web shell SHALL only forward SSE events and map DTOs, not own the loop

### Requirement: CLI Does Not Depend On Web Internals

The CLI SHALL start the web server through a small public bootstrap facade or binary-only entrypoint, not by importing `macaca-web` internals, and SHALL obtain run/status data through SDK/runtime clients.

#### Scenario: CLI server start uses a bootstrap facade
- **WHEN** the CLI starts the web server or queries status
- **THEN** it SHALL use the public bootstrap facade / SDK clients
- **AND** there SHALL be no `macaca-cli -> macaca-web` direct dependency edge

### Requirement: Domain Packs Live Outside Base Runtime Host

Business-domain service packs (for example finance/crypto market/financials/news providers) SHALL NOT compile into base `macaca-runtime-host`. They SHALL register as plugin/package service providers declared by manifest with descriptor and policy metadata; base runtime-host SHALL retain only the generic `ServiceProviderFactory` and registration mechanics.

#### Scenario: Base runtime-host has no business-domain code
- **WHEN** base `macaca-runtime-host` is scanned
- **THEN** it SHALL contain no finance/crypto/exchange domain identifiers or hardcoded business endpoints
- **AND** absent domain packs SHALL return structured unavailable without affecting base OS semantics
