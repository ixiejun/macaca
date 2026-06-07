## ADDED Requirements

### Requirement: Plugin Marketplace Lifecycle
The system SHALL provide service-owned plugin marketplace add, remove, upgrade,
install, uninstall, enable, disable, read, list, and auth-status operations.

#### Scenario: Install plugin through policy gates
- **WHEN** a plugin is installed from a marketplace
- **THEN** store, entitlement, signature, manifest, permission, resource, and
  policy gates SHALL run before registration
- **AND** bundled capabilities SHALL register through service descriptors

#### Scenario: Plugin unavailable by policy
- **WHEN** a plugin or marketplace is disabled by admin policy or entitlement
- **THEN** the service SHALL return structured unavailable or denied state
- **AND** no shell code SHALL hide or fake provider availability

### Requirement: MCP Operator Lifecycle
The system SHALL upgrade `service.mcp` with server status, reload, tool calls,
resource reads, OAuth login/status, diagnostics snapshots, watched changes, and
per-thread exposure.

#### Scenario: MCP OAuth login required
- **WHEN** a configured MCP server requires OAuth before tool/resource access
- **THEN** `service.mcp` SHALL expose auth-required status and login flow refs
- **AND** invocation SHALL return structured unavailable until auth completes

#### Scenario: MCP reload
- **WHEN** MCP configuration is reloaded
- **THEN** loaded threads SHALL refresh exposure on their next active turn
- **AND** status changes SHALL be observable and audited

### Requirement: Skill Operator Lifecycle
The system SHALL upgrade `service.skill` with catalog listing, markdown read,
config write, watch/unwatch, changed notifications, enablement changes, and
provenance audit.

#### Scenario: Skill file changed
- **WHEN** a watched skill changes
- **THEN** `service.skill` SHALL emit a bounded changed notification
- **AND** future planning SHALL use the updated skill lifecycle state

#### Scenario: Skill enablement policy
- **WHEN** a skill is enabled or disabled for a scope
- **THEN** the service SHALL persist the decision, emit audit evidence, and
  update tool/context visibility through service-owned catalogs
