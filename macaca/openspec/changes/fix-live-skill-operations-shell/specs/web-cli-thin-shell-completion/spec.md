## MODIFIED Requirements

### Requirement: Macaca SHALL keep CLI as terminal shell and narrow Web dependency

Macaca CLI SHALL remain responsible for terminal parsing, terminal formatting, process lifecycle, and explicit server launch only, while delegating system semantics to SDK/SystemFacade, focused clients, or public live shell facades backed by those clients.

#### Scenario: CLI inspection path avoids Web internals
- **WHEN** CLI handles a migrated read-only inspection command
- **THEN** it SHALL call `SystemFacade`, a focused SDK client, or an app-scoped public Web API facade backed by the same service client
- **AND** it SHALL NOT depend on `macaca-web` internals for inspection semantics

#### Scenario: CLI app-scoped Skill operations use the live runtime facade
- **GIVEN** a live Web shell is serving app-scoped Skill operation routes
- **WHEN** CLI Skill operations are invoked with an application id and API base
- **THEN** CLI SHALL forward the operator command to the public Web API facade
- **AND** the Web route SHALL continue to translate that request into SDK/service-owned Skill commands
- **AND** CLI SHALL print the service response trace id without classifying lifecycle, curation, merge, proposal, alias, or mutation semantics locally

#### Scenario: CLI server command is explicitly isolated
- **WHEN** CLI starts the Web server
- **THEN** the command SHALL use a narrow public server-start adapter seam
- **AND** it SHALL NOT duplicate Web runtime/provider/service composition semantics inside CLI
- **AND** any remaining `macaca-cli -> macaca-web` direct dependency SHALL be documented as server-start-only compatibility debt with an expiry condition
