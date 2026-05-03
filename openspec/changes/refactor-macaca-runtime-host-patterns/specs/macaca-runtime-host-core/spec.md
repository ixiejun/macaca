## ADDED Requirements

### Requirement: Runtime Host MCP Facade

The system SHALL provide a stable runtime-host facade for MCP lifecycle orchestration so host consumers do not need to call low-level runtime manager details directly.

#### Scenario: Host consumer registers tools through facade
- **GIVEN** a host consumer needs to register MCP tools into a toolkit
- **WHEN** it uses the runtime-host facade
- **THEN** the facade performs definition resolution and tool registration
- **AND** the observable registration result remains compatible with the existing runtime behavior

#### Scenario: Host consumer probes MCP status through facade
- **GIVEN** a host consumer needs MCP readiness information
- **WHEN** it uses the runtime-host facade
- **THEN** the facade returns MCP runtime statuses compatible with the current status schema

### Requirement: Transport Bridge Separation

The runtime host SHALL separate MCP transport creation from host lifecycle orchestration through a transport bridge.

#### Scenario: Stdio transport is created through the bridge
- **GIVEN** an MCP server definition uses `stdio`
- **WHEN** the runtime host creates a client
- **THEN** transport-specific client creation is handled by the transport bridge
- **AND** host orchestration code does not require transport-specific branching beyond the bridge boundary

#### Scenario: HTTP-class transport is created through the bridge
- **GIVEN** an MCP server definition uses `sse` or `streamable_http`
- **WHEN** the runtime host creates a client
- **THEN** transport-specific setup is handled by the transport bridge
- **AND** the exposed MCP behavior remains compatible with current runtime behavior

### Requirement: Explicit MCP Session Lease

The runtime host SHALL represent MCP runtime ownership and cleanup through explicit lease semantics rather than ad hoc reference counting alone.

#### Scenario: Lease release cleans up session-scoped resources
- **GIVEN** a session-scoped MCP runtime has been acquired
- **WHEN** its lease is released
- **THEN** runtime ownership is decremented or closed according to lifecycle policy
- **AND** any attached cleanup commands are executed

#### Scenario: Failure path still releases lease
- **GIVEN** an MCP runtime is used by a task
- **WHEN** the task fails, times out, or is cancelled
- **THEN** the associated lease is released
- **AND** runtime cleanup remains visible to host status and trace flows

### Requirement: Runtime Host Factory And Env Builder

The runtime host SHALL construct MCP definitions, isolation options, and environment exports through explicit factory and builder boundaries.

#### Scenario: Definition construction preserves existing schema semantics
- **GIVEN** a definition comes from YAML config, skill snapshot, or compat registry
- **WHEN** the runtime host constructs the definition
- **THEN** lifecycle, session mode, required bins, tool prefix, and enabled semantics remain compatible

#### Scenario: Env builder preserves forwarding and placeholder rules
- **GIVEN** MCP env entries include literal values, forwarded env names, and placeholders
- **WHEN** the runtime host applies environment export rules
- **THEN** literal and forwarded values are applied as before
- **AND** placeholders and missing env references remain skipped according to current semantics

### Requirement: Deprecated Compatibility Path

The runtime host SHALL preserve legacy public APIs for migration, but those APIs MUST be marked deprecated and MUST delegate to the new implementation.

#### Scenario: Legacy interface remains available for migration lookup
- **GIVEN** a downstream consumer still references a legacy runtime-host API
- **WHEN** the code compiles
- **THEN** the legacy API remains present
- **AND** the API is marked deprecated with a migration note
- **AND** the API delegates to the new facade, bridge, lease, or factory path

#### Scenario: New logic does not enter deprecated path
- **GIVEN** the runtime host introduces new lifecycle or transport behavior
- **WHEN** the implementation is updated
- **THEN** new logic is added only to the new primary abstraction path
- **AND** deprecated APIs remain compatibility shims rather than active extension points

### Requirement: Incremental Slice Delivery

The runtime host refactor SHALL be delivered as one planned change with multiple small implementation slices rather than one large rewrite.

#### Scenario: Slices are implemented in order
- **GIVEN** the approved refactor plan includes facade, transport bridge, lease, factory, and deprecated compatibility slices
- **WHEN** implementation starts
- **THEN** each slice is implemented, compiled, and verified independently
- **AND** later slices do not bypass earlier compatibility constraints

#### Scenario: Rollback remains possible at slice granularity
- **GIVEN** a regression is discovered in one refactor slice
- **WHEN** the change is rolled back
- **THEN** the affected slice can be reverted without requiring a full runtime-host rewrite rollback
