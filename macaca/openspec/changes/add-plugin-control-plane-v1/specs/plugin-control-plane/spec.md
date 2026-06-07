## ADDED Requirements

### Requirement: Macaca SHALL provide a Plugin Control Plane service facade

Macaca SHALL expose a runtime-host-owned Plugin Control Plane facade that manages plugin repositories, install sources, manifest loading, admission, activation state, lifecycle handoff, health snapshots, diagnostics, and trace/audit events without exposing concrete repository or host internals to Web, CLI, or SDK callers.

#### Scenario: Shell lists plugins through the control plane

- **WHEN** Web or CLI requests the plugin list
- **THEN** it SHALL call the Plugin Control Plane through an SDK or service client
- **AND** it SHALL NOT read plugin directories, parse plugin manifests directly, or access plugin host internals
- **AND** the returned list SHALL include deterministic plugin identity, source kind, activation state, health state, and diagnostics summary

### Requirement: Macaca SHALL model plugin install sources as replaceable strategies

Macaca SHALL support provider-neutral install source declarations for bundled, user-local, project-local, dev-link, archive, store-cache placeholder, and git placeholder sources through a replaceable Strategy boundary.

#### Scenario: Unsupported install source is unavailable-safe

- **WHEN** a caller requests an install source that is declared but not implemented
- **THEN** Plugin Control Plane SHALL return a structured unavailable result
- **AND** it SHALL NOT panic, hang, silently install, or bypass admission checks
- **AND** the rejection SHALL be logged and traceable

#### Scenario: Project-local source requires explicit opt-in

- **WHEN** a plugin exists in a project-local source and project plugins are not explicitly enabled
- **THEN** Plugin Control Plane SHALL report the plugin as disabled by policy or unavailable
- **AND** it SHALL NOT register active capabilities for that plugin

### Requirement: Macaca SHALL validate plugin admission before activation

Macaca SHALL run a Chain of Responsibility admission process before enabling or starting a plugin, covering manifest parsing, schema validation, signature metadata, compatibility, permission declarations, resource declarations, config requirements, secret requirements, and entitlement placeholders.

#### Scenario: Missing required secret blocks activation

- **WHEN** a plugin declares a required secret and the secret is absent
- **THEN** Plugin Control Plane SHALL reject activation with a structured missing-secret diagnostic
- **AND** logs and trace/audit events SHALL include the secret name and status only, never the secret value

### Requirement: Macaca SHALL expose plugin control commands

Macaca SHALL expose typed commands for `plugin.list`, `plugin.inspect`, `plugin.install`, `plugin.enable`, `plugin.disable`, `plugin.start`, `plugin.stop`, `plugin.uninstall`, `plugin.health`, and `plugin.diagnostics`.

#### Scenario: Control command requires trace context

- **WHEN** a state-changing plugin control command is submitted without trace context
- **THEN** Plugin Control Plane SHALL reject the command before mutating state
- **AND** it SHALL emit a structured policy or trace-required diagnostic

### Requirement: Macaca SHALL preserve microkernel and thin-shell boundaries

Plugin Control Plane SHALL keep install-source, manifest loading, activation, health, and diagnostics orchestration outside the kernel and outside Web/CLI presentation shells.

#### Scenario: Kernel remains descriptor-invariant only

- **WHEN** a plugin is discovered, installed, enabled, disabled, or diagnosed
- **THEN** the kernel SHALL only observe registry/lifecycle/ownership invariants when invoked
- **AND** it SHALL NOT execute plugin code, read plugin storage, evaluate install sources, or implement plugin business behavior
