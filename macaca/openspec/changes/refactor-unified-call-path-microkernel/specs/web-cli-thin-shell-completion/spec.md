## ADDED Requirements

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
- **THEN** internal workspace dependencies SHALL be limited to `macaca-sdk` (and proto DTO / necessary thin adapters)
- **AND** there SHALL be zero web and zero cli dependency-gate allowlist rows

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
