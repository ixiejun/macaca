## ADDED Requirements

### Requirement: Driver Service Contract

The system SHALL expose a provider-neutral Driver Service with operations for driver load, reload, inventory, tool catalog, tool invocation, status, service snapshot, and cleanup.

#### Scenario: Service exposes driver lifecycle and inventory

- **WHEN** an upper-layer adapter requests driver inventory through the Driver Service
- **THEN** the service SHALL return structured provider-neutral driver inventory without requiring the caller to access `DriverRuntime` directly
- **AND** the response SHALL include enough status metadata for existing driver status routes to preserve their public response semantics.

#### Scenario: Service reloads drivers

- **WHEN** a driver reload command is submitted with trace context
- **THEN** the Driver Service SHALL delegate reload to the configured driver runtime/provider strategy
- **AND** the result SHALL report loaded, failed, and skipped entries as structured metadata.

### Requirement: Driver Tool Catalog

The Driver Service SHALL expose sanitized driver tool descriptors through the shared capability tool DTO while retaining Driver Service ownership of invocation.

#### Scenario: Driver tools are cataloged

- **WHEN** the framework toolkit requests driver tools
- **THEN** the Driver Service SHALL return sanitized tool descriptors with origin kind `driver`
- **AND** descriptors SHALL NOT include env, headers, credentials, provider secrets, or full command lines with secrets.

#### Scenario: Tool name compatibility is preserved

- **WHEN** a driver tool descriptor is converted into a framework-visible tool
- **THEN** the generated tool schema and name SHALL remain backward compatible unless a separate OpenSpec delta explicitly changes the public tool contract.

### Requirement: Driver Tool Invocation

The Driver Service SHALL invoke driver tools only through typed, traced, policy-checkable commands that include explicit application/session/agent scope where applicable.

#### Scenario: Driver tool is invoked through service client

- **WHEN** a Web toolkit tool adapter invokes a driver tool
- **THEN** the adapter SHALL call the Driver Service client instead of calling `DriverRuntime` directly
- **AND** the service SHALL emit structured logs/events for command accepted, policy checked, dispatch started, completion or failure.

#### Scenario: Driver provider is unavailable

- **WHEN** no driver provider/runtime is configured
- **THEN** the Driver Service SHALL return structured unavailable
- **AND** Web startup SHALL NOT fail solely because the driver service is unavailable.

### Requirement: Deprecated Driver Compatibility Anchors

Existing direct driver runtime APIs SHALL remain available as deprecated, searchable compatibility anchors during S6 migration.

#### Scenario: Legacy path remains searchable

- **WHEN** a developer searches for deprecated direct driver runtime usage
- **THEN** the codebase SHALL retain explicit deprecated markers or compatibility wrappers
- **AND** new production call paths SHALL prefer the Driver Service client.
