# serviceization-escape-hatches Specification

## Purpose
TBD - created by archiving change freeze-serviceization-escape-hatches. Update Purpose after archive.
## Requirements
### Requirement: Static Escape-Hatch Freeze
The system SHALL reject new production Rust references to known serviceization
escape hatches outside approved migration modules, tests, fixtures, examples, or
service provider bridges.

#### Scenario: New Web direct runtime read is introduced
- **GIVEN** a production Web source file outside the approved migration surface
- **WHEN** the file references `state.driver_runtime`, `state.mcp_runtime`, `state.runtime`, or `state.registry`
- **THEN** the serviceization escape-hatch gate fails with the file, line, token, and replacement guidance

#### Scenario: New direct application runtime start is introduced
- **GIVEN** a production source file outside the application runtime or application service provider bridge
- **WHEN** the file references `AppRuntime::start_app` or `start_app_from_file`
- **THEN** the serviceization escape-hatch gate fails with the file, line, token, and service-command guidance

#### Scenario: New hardcoded agent role is introduced
- **GIVEN** a production OS-layer source file outside approved migration or manifest interpretation surfaces
- **WHEN** the file hardcodes generic role names such as `coordinator`, `planner`, `worker`, `backend`, `frontend`, or `architect`
- **THEN** the gate fails and requires the role to come from a manifest, descriptor, or explicit migration approval

### Requirement: Auditable Dependency Allowlist Rows
Every Route C dependency allowlist row SHALL include owner track, current caller
evidence, replacement boundary, expiry phase, and validation command metadata.

#### Scenario: Existing forbidden dependency edge is allowlisted
- **GIVEN** a forbidden dependency edge is still migration debt
- **WHEN** the Route C dependency gate visits the edge
- **THEN** the allowlist diagnostic includes the rule, source crate, target crate, owner track, current caller, expiry phase, replacement, and validation command

### Requirement: Freeze Without Behavior Removal
The escape-hatch freeze SHALL preserve existing behavior while preventing new
violations from entering untracked production paths.

#### Scenario: Existing migration surface remains during staged refactor
- **GIVEN** a direct reference exists in an approved migration file
- **WHEN** the static gate scans production sources
- **THEN** the gate permits the reference only as named migration debt and does not remove or change runtime behavior

### Requirement: Kernel Execution Port Boundary
The production kernel SHALL execute registered agents through a provider-neutral
execution port instead of storing concrete provider compatibility bundles.

#### Scenario: Kernel executes a registered agent
- **GIVEN** a registered agent and a configured agent execution port
- **WHEN** the kernel is asked to execute the agent by identifier
- **THEN** the kernel delegates to the execution port, records start/finish logs, preserves status transitions, and does not read LLM or tool provider handles directly

#### Scenario: Service-client construction lacks a legacy execution bridge
- **GIVEN** a kernel built from service-client-only compatibility wiring
- **WHEN** legacy agent execution is invoked before the execution service is available
- **THEN** the execution port returns a structured unavailable error and logs the missing bridge without fabricating a successful agent output

### Requirement: Kernel Persistence Port Boundary
The production kernel SHALL use provider-neutral persistence ports for audit,
execution queue recovery, fork recovery, and deprecated payment compatibility
instead of depending on concrete persistence provider crates.

#### Scenario: Kernel persists audit or recovery mementos
- **GIVEN** a configured kernel persistence port
- **WHEN** audit logging, queue checkpointing, or fork recovery writes a durable memento
- **THEN** the kernel calls the port, records key execution logs, and does not import concrete Redb or foundation persistence provider types

#### Scenario: Kernel is built without a durable persistence backend
- **GIVEN** a Null Object kernel persistence port
- **WHEN** a caller attempts to read, write, delete, or list persistence keys
- **THEN** the port returns an explicit non-durable result and logs that no durable backend is configured

### Requirement: Web Toolkit Focused Client Boundary
The Web toolkit assembly path SHALL use focused service clients for Driver and
MCP capability discovery instead of reading runtime internals from `AppState`.

#### Scenario: Driver catalog service is unavailable
- **GIVEN** Web is building an agent toolkit for a session
- **WHEN** the Driver focused client cannot return a tool catalog
- **THEN** Web records a structured unavailable diagnostic, emits a session-visible audit/trace event when a session exists, and does not call the deprecated driver runtime fallback

#### Scenario: MCP definitions are needed for toolkit registration
- **GIVEN** Web is building an agent toolkit for a session
- **WHEN** MCP server definitions are needed before registration and probing
- **THEN** Web obtains serialized definition payloads through the MCP service snapshot command and does not read `state.mcp_runtime.definitions()` directly

### Requirement: Escape Hatches SHALL Be Removed Not Only Frozen

After each serviceized capability has a complete service-client replacement, Macaca SHALL remove the corresponding migration-module exemption so that any reference (including pre-existing ones) fails the static escape-hatch gate. The terminal state SHALL contain zero escape-hatch references in production code outside explicit fixtures and tests.

#### Scenario: Replaced escape hatch becomes a hard failure
- **WHEN** a capability's service-client replacement is complete and its migration-module exemption is removed
- **THEN** the escape-hatch gate SHALL fail on any remaining production reference to the old direct field, provider, or runtime
- **AND** the diagnostic SHALL name the file, line, token, and the service-client replacement

#### Scenario: Terminal scan reports zero escape hatches
- **WHEN** the escape-hatch gate runs at terminal state
- **THEN** the production-code occurrence count for forbidden escape-hatch tokens SHALL be zero outside fixtures and tests
- **AND** migration debt inventory SHALL match the terminal baseline of zero raw hits

### Requirement: Kernel Provider Compatibility SHALL Be Deleted

The kernel `provider_compat` module and the deprecated `Kernel::new(config, llm, tools)` constructor SHALL be deleted. The only kernel construction path SHALL build the kernel from a service-client `AgentExecutionPort` implementation.

#### Scenario: Kernel has no provider compatibility surface
- **WHEN** `macaca-kernel` is inspected
- **THEN** there SHALL be no `provider_compat` module, no `KernelProviderCompat`, no `LegacyLlmProvider`/`LegacyToolCatalog` re-exports, and no `Kernel::new(config, llm, tools)`
- **AND** `cargo check` SHALL produce no deprecated-item warnings within the kernel crate

### Requirement: Reconciliation Markers Are Removed From Production

Production code SHALL NOT contain the multi-path reconciliation markers used to coordinate legacy and serviceized execution paths.

#### Scenario: Reconciliation markers scan is clean
- **WHEN** the escape-hatch gate scans production sources without migration exemptions
- **THEN** it SHALL report zero occurrences of `legacy_unmarked`, `non_authoritative`, `suppress_executor_lifecycle`, and `legacy_chat_main_thread_goal_pause`
- **AND** any `graph_owner` usage SHALL exist only as pure audit metadata, never as a path-discrimination switch

