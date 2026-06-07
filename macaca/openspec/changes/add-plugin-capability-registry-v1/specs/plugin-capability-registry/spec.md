## ADDED Requirements

### Requirement: Macaca SHALL define provider-neutral plugin capability descriptors

Macaca SHALL define plugin capability descriptors for tool, hook, driver, gateway, skill, MCP, memory, context, LLM provider, observability, HTTP route, CLI command, and custom capabilities without binding descriptors to concrete providers, applications, workflows, drivers, gateways, models, chains, or business names.

#### Scenario: Capability descriptor round trips through serde

- **WHEN** a plugin capability descriptor with schema, visibility, permission hints, resource hints, trace schema, slot metadata, and custom metadata is serialized and deserialized
- **THEN** the decoded descriptor SHALL preserve the capability id, plugin id, capability kind, schema metadata, visibility, permission hints, resource hints, trace schema, and slot metadata

### Requirement: Macaca SHALL support contract-first capability discovery

Macaca SHALL discover plugin capability ownership from plugin manifests and repository snapshots without launching plugin runtime code.

#### Scenario: Capability owner is discovered before runtime start

- **WHEN** a plugin manifest declares a tool or gateway capability
- **THEN** Plugin Capability Registry SHALL resolve the owning plugin from manifest data
- **AND** it SHALL NOT start WASM, process, native, or remote plugin execution

### Requirement: Macaca SHALL enforce capability conflict policies

Macaca SHALL detect and report conflicts for active tool names, exclusive capability slots, gateway routes, HTTP routes, CLI commands, and custom conflict namespaces before exposing conflicting capabilities.

#### Scenario: Duplicate CLI command fails closed

- **WHEN** two active plugins provide the same CLI command name without an explicit conflict policy
- **THEN** Plugin Capability Registry SHALL reject the conflicting activation
- **AND** it SHALL return a structured conflict report naming the plugin ids and capability ids

### Requirement: Macaca SHALL canonicalize built-in services as plugin capabilities

Macaca SHALL expose built-in Driver, Skill, MCP, Gateway, Memory, Context, LLM Provider, and Observability capabilities through canonical plugin capability descriptors without changing existing execution behavior during the additive migration.

#### Scenario: Built-in driver capability is queryable

- **WHEN** built-in driver capability descriptors are registered
- **THEN** Plugin Capability Registry SHALL expose provider-neutral driver capability metadata
- **AND** existing driver execution behavior SHALL remain unchanged until explicitly migrated by a later change

### Requirement: Macaca SHALL clean capability ownership on deactivation and uninstall

Plugin Capability Registry SHALL remove every active capability descriptor owned by a plugin when the plugin is disabled, stopped, uninstalled, or fails activation.

#### Scenario: Disabled plugin removes active capabilities

- **WHEN** a running plugin with multiple capabilities is disabled
- **THEN** Plugin Capability Registry SHALL remove all active capability ownership for that plugin
- **AND** later capability queries SHALL NOT return stale active descriptors

### Requirement: Macaca SHALL require trace and admission for capability calls

Macaca SHALL require trace context and permission/resource admission before any plugin capability call is routed to a built-in adapter, descriptor-safe handler, WASM host, process host, or remote proxy host.

#### Scenario: Capability call without trace is rejected

- **WHEN** a caller attempts to invoke a plugin capability without trace context
- **THEN** the call SHALL be rejected before reaching the capability implementation
- **AND** the rejection SHALL be logged and auditable
