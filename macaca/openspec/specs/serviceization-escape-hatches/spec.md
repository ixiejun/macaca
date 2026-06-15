# serviceization-escape-hatches Specification

## Purpose
Terminal governance for removing serviceization bypass paths: production reference scans, reconciliation-marker absence, and zero-debt baselines that prove coordination patches and direct capability paths have been deleted. Baseline aligned to the protocol microkernel terminal state.
## Requirements
### Requirement: Static Escape-Hatch Rejection
The system SHALL reject production Rust references to known serviceization bypass
paths outside explicit fixtures, tests, or examples. Production code SHALL not
define approved bypass modules or provider bridges that skip the canonical
service path.

#### Scenario: New Web direct runtime read is introduced
- **GIVEN** a production Web source file
- **WHEN** the file references `state.driver_runtime`, `state.mcp_runtime`, `state.runtime`, or `state.registry`
- **THEN** the serviceization escape-hatch gate fails with the file, line, token, and replacement guidance

#### Scenario: New direct application runtime start is introduced
- **GIVEN** a production source file outside the canonical application service provider
- **WHEN** the file references `AppRuntime::start_app` or `start_app_from_file`
- **THEN** the serviceization escape-hatch gate fails with the file, line, token, and service-command guidance

#### Scenario: New hardcoded agent role is introduced
- **GIVEN** a production OS-layer source file outside manifest interpretation surfaces
- **WHEN** the file hardcodes generic role names such as `coordinator`, `planner`, `worker`, `backend`, `frontend`, or `architect`
- **THEN** the gate fails and requires the role to come from a manifest, descriptor, or policy-owned contract

### Requirement: Dependency Exception Inventory SHALL Be Empty
Every dependency boundary exception inventory SHALL contain zero rows at terminal
state. A forbidden dependency edge SHALL be removed or the architecture boundary
SHALL be changed by OpenSpec before code lands.

#### Scenario: Forbidden dependency edge exists
- **GIVEN** a forbidden dependency edge exists
- **WHEN** the dependency gate visits the edge
- **THEN** the gate fails with the rule, source crate, target crate, rationale, and canonical replacement boundary

### Requirement: Behavior Is Preserved Through Canonical Paths
Removal of bypass paths SHALL preserve user-visible behavior by routing the same
capabilities through canonical service, facade, application ABI, or runtime-host
owner boundaries.

#### Scenario: Direct path is replaced
- **GIVEN** a direct reference was replaced by a canonical service path
- **WHEN** the static gate scans production sources
- **THEN** the direct reference is absent
- **AND** the canonical path preserves the externally visible result shape unless a later OpenSpec change explicitly changes it

### Requirement: Kernel Execution Port Boundary
The production kernel SHALL execute registered agents through a provider-neutral
execution port instead of storing concrete provider bundles.

#### Scenario: Kernel executes a registered agent
- **GIVEN** a registered agent and a configured agent execution port
- **WHEN** the kernel is asked to execute the agent by identifier
- **THEN** the kernel delegates to the execution port, records start/finish logs, preserves status transitions, and does not read LLM or tool provider handles directly

#### Scenario: Execution service is unavailable
- **GIVEN** a kernel built from the canonical service-client wiring
- **WHEN** agent execution is requested before the execution service is available
- **THEN** the execution port returns a structured unavailable error and logs the missing service without fabricating a successful agent output

### Requirement: Kernel Persistence Port Boundary
The production kernel SHALL use provider-neutral persistence ports for audit,
execution queue recovery, fork recovery, and payment evidence instead of
depending on concrete persistence provider crates.

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
- **THEN** Web records a structured unavailable diagnostic, emits a session-visible audit/trace event when a session exists, and does not call the retired driver runtime fallback

#### Scenario: MCP definitions are needed for toolkit registration
- **GIVEN** Web is building an agent toolkit for a session
- **WHEN** MCP server definitions are needed before registration and probing
- **THEN** Web obtains serialized definition payloads through the MCP service snapshot command and does not read `state.mcp_runtime.definitions()` directly

### Requirement: Bypass Paths SHALL Be Removed

After each serviceized capability has a complete service-client replacement, Macaca SHALL remove the corresponding bypass path so that any production reference fails the static serviceization gate. The terminal state SHALL contain zero bypass references in production code outside explicit fixtures and tests.

#### Scenario: Replaced bypass path becomes a hard failure
- **WHEN** a capability's service-client replacement is complete and the old direct path is removed
- **THEN** the serviceization gate SHALL fail on any remaining production reference to the old direct field, provider, or runtime
- **AND** the diagnostic SHALL name the file, line, token, and the service-client replacement

#### Scenario: Terminal scan reports zero bypass paths
- **WHEN** the serviceization gate runs at terminal state
- **THEN** the production-code occurrence count for forbidden bypass tokens SHALL be zero outside fixtures and tests
- **AND** the terminal debt inventory SHALL report zero raw hits

### Requirement: Kernel Provider Bridge SHALL Be Deleted

The kernel provider bridge module and the retired `Kernel::new(config, llm, tools)` constructor SHALL be deleted. The only kernel construction path SHALL build the kernel from a service-client `AgentExecutionPort` implementation.

#### Scenario: Kernel has no provider bridge surface
- **WHEN** `macaca-kernel` is inspected
- **THEN** there SHALL be no direct provider bridge module, no provider bridge re-exports, and no `Kernel::new(config, llm, tools)`
- **AND** `cargo check` SHALL produce no retired-item warnings within the kernel crate

### Requirement: Reconciliation Markers Are Removed From Production

Production code SHALL NOT contain the multi-path reconciliation markers used to coordinate old direct and serviceized execution paths.

#### Scenario: Reconciliation markers scan is clean
- **WHEN** the serviceization gate scans production sources
- **THEN** it SHALL report zero occurrences of old unmarked-path, non-authoritative-path, executor-lifecycle-suppression, and retired chat-pause switch tokens
- **AND** any `graph_owner` usage SHALL exist only as pure audit metadata, never as a path-discrimination switch
