# web-cli-thin-shell-completion Specification

## Purpose
TBD - created by archiving change complete-web-cli-thin-shell-v1. Update Purpose after archive.
## Requirements
### Requirement: Macaca SHALL move service provider bootstrap out of presentation shell ownership

Macaca SHALL provide a runtime-host-owned bootstrap boundary for Route C service provider registration and startup so Web and CLI do not own provider lifecycle semantics.

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

#### Scenario: Web keeps high-risk compatibility state explicit

- **WHEN** chat, framework, session, toolkit, or resume paths still need deprecated Web state
- **THEN** the compatibility field or helper SHALL remain present as a documented deprecated anchor
- **AND** new low-risk paths SHALL prefer service clients or `SystemFacade`
- **AND** the remaining anchor SHALL document its future removal condition

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
- **AND** any remaining `macaca-cli -> macaca-web` direct dependency SHALL be documented as server-start-only compatibility debt with an expiry condition

### Requirement: Macaca SHALL guard deprecated presentation-owned semantic paths

Macaca SHALL prevent replaced Web/CLI direct semantic helpers from gaining new production callers while preserving deprecated compatibility anchors for migration search and rollback.

#### Scenario: Deprecated Web provider/runtime fields are classified

- **WHEN** implementation scans `AppState` deprecated provider/runtime fields
- **THEN** each production read SHALL be migrated to a service client when low-risk or documented as a compatibility anchor when high-risk
- **AND** new code SHALL NOT use those fields as the default path for serviceized capabilities

#### Scenario: Guard allows compatibility definitions but blocks new direct callers

- **WHEN** migration guards or tests scan presentation code
- **THEN** explicit deprecated compatibility definitions MAY remain
- **AND** new upper-layer callers of replaced direct helpers SHALL fail the guard or be documented in the OpenSpec tasks before approval

### Requirement: Macaca SHALL preserve Route C Web/CLI regression scenarios

Macaca SHALL complete S12 additively without regressing chat session creation, chat session resume, real-time trace, historical trace replay, session-scoped task board, service unavailable behavior, optional module unavailable behavior, Web UI, or CLI behavior.

#### Scenario: Mandatory regression gates pass

- **WHEN** S12 completion verification runs
- **THEN** `RC-CHAT-001`, `RC-CHAT-002`, `RC-TRACE-001`, `RC-TRACE-002`, and `RC-TASK-001` SHALL remain valid
- **AND** `cargo test -p macaca-integration-tests --test route_c_baseline` SHALL pass
- **AND** `cargo test -p macaca-integration-tests route_c_dependency_boundaries` SHALL pass

### Requirement: Macaca SHALL keep S12 dependency governance honest

Macaca SHALL update Route C allowlist and dependency-boundary tests only when implementation changes direct dependency edges or narrows an existing exception.

#### Scenario: Allowlist row is removed only after executable proof

- **WHEN** implementation claims a presentation dependency edge is removed
- **THEN** `cargo metadata` and the executable dependency boundary gate SHALL prove the direct edge is gone
- **AND** the allowlist SHALL be updated with the removed or narrowed status
- **AND** rows SHALL NOT be deleted merely because a route path now prefers a service client

### Requirement: Macaca SHALL log and audit S12 shell and bootstrap boundaries

Macaca SHALL emit structured logs at runtime-host bootstrap, Web route adapter, and CLI command adapter execution nodes.

#### Scenario: Bootstrap and shell failures are auditable

- **WHEN** bootstrap, Web route adapter, or CLI command adapter fails or rejects work
- **THEN** logs SHALL include layer kind, operation, service id when available, command name when available, trace id when available, app/session/task scope when available, sanitized status, and reason code
- **AND** logs SHALL NOT include secrets, private keys, provider credentials, raw payment credentials, raw encrypted package contents, raw prompt bodies, raw package bytes, wallet secrets, raw signed transactions, raw ABI/bytecode, raw tool payloads, unbounded user input, or provider secrets

### Requirement: Macaca SHALL document new S12 Rust code with detailed English comments

All new S12 Rust code SHALL include detailed English comments explaining ownership, design pattern intent, runtime behavior, trace/audit behavior, compatibility anchors, and non-goals.

#### Scenario: Maintainer can identify ownership from code comments

- **WHEN** a maintainer reads new runtime-host bootstrap, Web shell adapter, or CLI shell adapter code
- **THEN** comments SHALL explain which layer owns provider lifecycle, transport adaptation, command construction, facade delegation, system semantics, trace/audit emission, and compatibility fallback
- **AND** comments SHALL explain why Web/CLI must not define provider, service, session, task, trace, package, payment, Web3, EVM, plugin, entitlement, or application workflow semantics

