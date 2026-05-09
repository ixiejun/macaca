## ADDED Requirements

### Requirement: MCP Service Contract

The system SHALL expose a provider-neutral MCP Service with operations for definition registration, dependency probe, tool catalog, toolkit attach metadata, tool invocation, status, service snapshot, and cleanup.

#### Scenario: MCP definition is registered

- **WHEN** an MCP definition is registered through the MCP Service
- **THEN** the service SHALL record provider-neutral definition metadata and lifecycle scope
- **AND** the caller SHALL NOT need direct access to `McpRuntimeFacade`.

#### Scenario: MCP dependency is probed

- **WHEN** a probe command is submitted
- **THEN** the MCP Service SHALL report ready, failed, or dependency-missing status as structured metadata
- **AND** failure summaries SHALL be sanitized and free of env, headers, credentials, and raw secrets.

### Requirement: MCP Tool Catalog and Attach

The MCP Service SHALL expose MCP tools through sanitized capability tool descriptors and SHALL own toolkit attachment metadata, conflict reporting, prefixes, and lifecycle scope.

#### Scenario: MCP tools are attached to a toolkit

- **WHEN** the framework toolkit attaches MCP tools
- **THEN** the MCP Service SHALL return sanitized descriptors with origin kind `mcp`
- **AND** the attach result SHALL report conflicts and applied prefixes without exposing raw server credentials.

#### Scenario: Skill-backed MCP is integrated

- **WHEN** skill-backed MCP definitions are available
- **THEN** the Skill Service SHALL provide provider-neutral definition/source metadata
- **AND** the MCP Service SHALL own protocol lifecycle, probing, attachment, invocation, and cleanup for those definitions.

### Requirement: MCP Tool Invocation

The MCP Service SHALL invoke MCP tools only through typed, traced, policy-checkable commands with explicit scope and resource lifecycle semantics.

#### Scenario: MCP tool is invoked through service client

- **WHEN** a Web toolkit tool adapter invokes an MCP tool
- **THEN** the adapter SHALL call the MCP Service client instead of direct global MCP or skill-backed MCP runtime invocation
- **AND** the service SHALL emit structured logs/events for command accepted, policy checked, dispatch started, completion or failure.

#### Scenario: MCP provider is unavailable

- **WHEN** no MCP runtime/facade is configured
- **THEN** the MCP Service SHALL return structured unavailable
- **AND** Web startup SHALL NOT fail solely because MCP is unavailable.

### Requirement: MCP Lifecycle Cleanup

The MCP Service SHALL support cleanup for global, application, session, agent-session, and call-scoped resources.

#### Scenario: Session-scoped MCP resources are cleaned

- **WHEN** a session-scoped cleanup command is submitted
- **THEN** the MCP Service SHALL release only resources belonging to the requested scope
- **AND** the service SHALL emit sanitized cleanup logs with trace id, scope, counts, and status.

### Requirement: Deprecated MCP Compatibility Anchors

Existing direct MCP runtime/facade APIs SHALL remain available as deprecated, searchable compatibility anchors during S6 migration.

#### Scenario: Legacy path remains searchable

- **WHEN** a developer searches for deprecated direct MCP runtime usage
- **THEN** the codebase SHALL retain explicit deprecated markers or compatibility wrappers
- **AND** new production call paths SHALL prefer the MCP Service client.
