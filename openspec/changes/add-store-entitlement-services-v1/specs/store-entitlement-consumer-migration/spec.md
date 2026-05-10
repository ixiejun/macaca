## ADDED Requirements

### Requirement: Macaca upper consumers SHALL use Store/Entitlement service-first paths

Macaca Web, CLI, app commercial guards, skill encrypted package hooks, and package-manager surfaces SHALL prefer Store/Entitlement service-backed clients or authorizers over direct runtime helper calls.

#### Scenario: Web registers Store and Entitlement services

- **WHEN** Web starts with `ServiceRuntime` available
- **THEN** it SHALL register and start Store Service and Entitlement Service providers where configured
- **AND** Web package/entitlement surfaces SHALL prefer `SystemFacade` clients

#### Scenario: CLI uses SystemFacade for package and entitlement operations

- **WHEN** CLI package inspect/install/status or entitlement inspection commands are invoked
- **THEN** CLI SHALL use SDK Store/Entitlement clients where available
- **AND** CLI SHALL remain a shell adapter that does not define commerce policy

### Requirement: Deprecated direct paths SHALL remain searchable but not be default production paths

Macaca SHALL retain Phase 08 direct helper APIs as deprecated compatibility anchors until all consumers migrate and dependency gates prove they can be removed.

#### Scenario: Direct entitlement facade remains for compatibility

- **WHEN** older code still references Phase 08 `EntitlementRuntimeFacade` or direct guard helpers
- **THEN** those APIs SHALL remain present and behavior-compatible
- **AND** new service-backed paths SHALL be documented as the default route
- **AND** deprecated annotations or comments SHALL make remaining direct calls searchable for later cleanup

### Requirement: Consumer migration SHALL preserve Route C regressions

S9 consumer migration SHALL preserve existing app, skill, and trace behavior.

#### Scenario: Route C S9 regression checks pass

- **WHEN** S9 verification runs
- **THEN** YAML application loading SHALL preserve `RC-APP-001`
- **AND** skill/MCP and encrypted skill hook behavior SHALL preserve `RC-SKILL-001`
- **AND** entitlement allow/deny/metering events SHALL preserve `RC-TRACE-001`

### Requirement: Consumer migration SHALL not introduce application-specific hardcode

S9 consumer migration SHALL not introduce control flow hardcoded to any application, workflow, Store vendor, payment provider, driver, gateway, model, chain, package business name, or tenant-specific name.

#### Scenario: Hardcode scan is clean

- **WHEN** new Store/Entitlement service, SDK, Web, CLI, app, and skill code is scanned
- **THEN** no service control flow SHALL branch on app/workflow/provider/driver/gateway/model/chain/business-specific names
- **AND** any unavoidable test fixture names SHALL remain isolated to tests
