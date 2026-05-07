## ADDED Requirements

### Requirement: Macaca SHALL define Plugin Manifest v0 contracts

Macaca SHALL define provider-neutral Plugin Manifest v0 contracts for plugin identity, version, developer identity, runtime declaration, provided services, provided capabilities, required services, permissions, resources, entry declaration, signature metadata, lifecycle state, health, and structured plugin errors.

#### Scenario: Plugin manifest round trips through serde

- **WHEN** a plugin manifest containing runtime declaration, provided services, capabilities, required services, permissions, resources, entry, signature metadata, and custom metadata is serialized and deserialized
- **THEN** the decoded manifest SHALL preserve plugin id, version, developer id, runtime kind, provided services, capabilities, required services, permissions, resources, entry declaration, signature metadata, and metadata
- **AND** the plugin contract SHALL NOT depend on `macaca-web`, concrete gateway implementations, concrete driver implementations, concrete memory providers, concrete skill packages, concrete MCP servers, Store implementation, payment implementation, chain implementation, or business workflows

#### Scenario: Unsupported runtime kind remains structured

- **WHEN** a manifest declares a future or unsupported runtime kind
- **THEN** parsing SHALL preserve the runtime declaration as structured data
- **AND** validation SHALL return a structured unsupported-runtime error instead of panicking, hanging, silently accepting arbitrary execution, or launching code

### Requirement: Macaca SHALL validate plugin permissions and resources before registration

Plugin Runtime v0 SHALL reject plugins that provide privileged services or capabilities without explicit permission and resource declarations.

#### Scenario: Missing permissions are rejected

- **WHEN** a plugin manifest provides a gateway, driver, memory, context, skill, MCP, payment, or compliance capability without declaring required permissions
- **THEN** Plugin Runtime SHALL reject registration with a structured missing-permission error
- **AND** no plugin-provided service or capability SHALL be registered
- **AND** the rejection SHALL be logged and traceable

#### Scenario: Missing resources are rejected

- **WHEN** a plugin manifest requires workspace, filesystem, network, browser, process, storage, secret, memory, driver, gateway, or external-service resources but omits resource declarations
- **THEN** Plugin Runtime SHALL reject registration with a structured missing-resource error
- **AND** no plugin-provided service or capability SHALL be registered

### Requirement: Macaca SHALL expose a runtime-host Plugin Runtime facade

Macaca SHALL expose a Plugin Runtime facade in `macaca-runtime-host` that validates manifests, selects plugin host factories, coordinates lifecycle operations, calls the kernel plugin registry, and emits logs/trace/audit records without exposing concrete plugin host internals.

#### Scenario: Descriptor-only plugin registers through facade

- **WHEN** a descriptor-only or built-in adapter plugin manifest passes validation
- **THEN** the runtime-host facade SHALL select a descriptor/in-process built-in host strategy
- **AND** it SHALL register the plugin-provided service descriptors through the kernel plugin registry
- **AND** it SHALL emit structured logs for validation, host selection, registration, and lifecycle state transition

#### Scenario: Third-party execution runtime is unavailable in v0

- **WHEN** a plugin manifest requests WASM, native, process, shell, or remote execution in Phase 07
- **THEN** the runtime-host facade SHALL return a structured runtime-unavailable result
- **AND** it SHALL NOT execute third-party code
- **AND** it SHALL emit an auditable rejection event

### Requirement: Macaca SHALL maintain a kernel plugin registry for invariants only

Macaca SHALL maintain a kernel plugin registry that tracks plugin identity, lifecycle state, provided service descriptors, provided capability descriptors, ownership, and cleanup while keeping plugin capability behavior outside the kernel.

#### Scenario: One plugin registers multiple service descriptors

- **WHEN** a plugin manifest provides multiple services and capabilities
- **THEN** the registry SHALL store the descriptor set under the owning plugin id
- **AND** consumers SHALL be able to query descriptor ownership without invoking plugin behavior

#### Scenario: Duplicate plugin id is rejected

- **WHEN** a second plugin registers with an already-registered plugin id and incompatible version or developer identity
- **THEN** the registry SHALL reject the registration with a structured duplicate-plugin error
- **AND** existing descriptor ownership SHALL remain unchanged

### Requirement: Macaca SHALL enforce plugin lifecycle state transitions

Plugin Runtime v0 SHALL enforce typed lifecycle transitions for installed, registered, starting, running, stopping, stopped, failed, and uninstalled states.

#### Scenario: Valid lifecycle path succeeds

- **WHEN** a plugin follows `installed -> registered -> starting -> running -> stopping -> stopped -> uninstalled`
- **THEN** each transition SHALL update lifecycle state atomically
- **AND** each transition SHALL emit structured logs and trace/audit records

#### Scenario: Invalid lifecycle transition is rejected

- **WHEN** a plugin attempts to transition from installed directly to running or from uninstalled back to running
- **THEN** the registry or lifecycle controller SHALL reject the transition with a structured invalid-transition error
- **AND** the previous lifecycle state SHALL remain unchanged

#### Scenario: Failed lifecycle transition is auditable

- **WHEN** validation, registration, start, stop, or uninstall fails
- **THEN** Plugin Runtime SHALL persist or emit a failure lifecycle event with plugin id, previous state, attempted next state, operation, error code, and timestamp
- **AND** failed service descriptors SHALL NOT remain active unless an explicit stopped descriptor policy applies

### Requirement: Macaca SHALL model built-in capabilities as plugin-provided service descriptors

Macaca SHALL model built-in gateway, driver, memory/context, skill, and MCP capabilities as plugin-provided service descriptors through Adapter boundaries without changing their current execution paths in Phase 07.

#### Scenario: Built-in gateway descriptor is queryable

- **WHEN** the built-in gateway adapter descriptor is registered
- **THEN** plugin registry queries SHALL expose a provider-neutral gateway service descriptor
- **AND** existing gateway execution paths SHALL continue to behave as before
- **AND** absence of a specific gateway SHALL return structured unavailable without affecting base OS behavior

#### Scenario: Built-in driver descriptor preserves driver trace behavior

- **WHEN** a built-in driver descriptor is registered
- **THEN** existing driver execution and trace payloads SHALL continue to satisfy `RC-DRIVER-001`
- **AND** plugin runtime SHALL NOT route by hardcoded driver names

#### Scenario: Skill-backed MCP descriptor preserves smoke path

- **WHEN** built-in skill and MCP descriptors are registered
- **THEN** existing skill/MCP runtime smoke behavior SHALL continue to satisfy `RC-SKILL-001`
- **AND** plugin runtime SHALL NOT bind to concrete skill package names or MCP server names

### Requirement: Macaca SHALL clean up plugin-provided services on uninstall

Plugin Runtime v0 SHALL remove every service descriptor and capability descriptor owned by a plugin during uninstall or failed registration cleanup.

#### Scenario: Uninstall removes descriptor ownership

- **WHEN** a plugin with multiple provided services is uninstalled
- **THEN** all service descriptors and capability descriptors owned by that plugin SHALL be removed from the registry
- **AND** later service queries SHALL NOT return stale descriptors
- **AND** uninstall SHALL emit structured trace/audit records

### Requirement: Macaca SHALL preserve Route C plugin regressions

Plugin Runtime v0 SHALL be implemented additively without regressing driver execution trace, skill/MCP smoke paths, real-time trace, historical trace replay, chat, task board, session recovery, Application ABI, GenUI, Web UI, or CLI behavior.

#### Scenario: Route C Phase 07 regression checks pass

- **WHEN** Phase 07 verification runs
- **THEN** the implementation SHALL preserve regression matrix scenarios `RC-DRIVER-001` and `RC-SKILL-001`
- **AND** plugin lifecycle trace SHALL continue to use the existing trace/event infrastructure without breaking `RC-TRACE-001`
- **AND** existing YAML applications, `/api/chat/v2`, task board, session logs, driver trace, skill/MCP trace, frontend, CLI, Application ABI, and GenUI behavior SHALL continue to compile and run through existing paths until explicitly migrated by later changes

### Requirement: Macaca SHALL log and audit plugin runtime decisions

Macaca SHALL emit structured logs and trace/audit records for manifest loading, validation start/pass/reject, permission/resource guard decisions, host factory selection, plugin install/register/start/run/stop/uninstall transitions, service descriptor registration/removal, health changes, and lifecycle failures.

#### Scenario: Rejected plugin operation is auditable

- **WHEN** manifest validation, permission guard, resource guard, host selection, lifecycle transition, registration, start, stop, or uninstall rejects an operation
- **THEN** trace/audit records SHALL include plugin id when available, developer id when available, version when available, runtime kind when available, previous state when available, attempted next state when available, service ids when available, capability ids when available, operation name, structured error code, and timestamp
- **AND** logs SHALL NOT include secrets, private keys, raw signatures beyond bounded fingerprints, provider credentials, payment credentials, encrypted package contents, or unbounded user input

### Requirement: Macaca SHALL document Plugin Runtime code with detailed English comments

Macaca SHALL include detailed English comments in new Phase 07 Rust code explaining plugin manifest invariants, validation rules, lifecycle state transitions, registry ownership, service descriptor cleanup, trace/audit behavior, adapter boundaries, future proxy extension points, and explicit non-goals.

#### Scenario: Maintainer can understand plugin runtime invariants from comments

- **WHEN** a maintainer reads new Plugin Runtime modules
- **THEN** comments SHALL explain what each public type, trait, facade, registry method, lifecycle transition, validation rule, adapter descriptor, and structured error represents
- **AND** comments SHALL explain why Phase 07 does not execute arbitrary third-party code
- **AND** comments SHALL explain how trace, audit, permissions, resources, service registry boundaries, and cleanup invariants are protected
