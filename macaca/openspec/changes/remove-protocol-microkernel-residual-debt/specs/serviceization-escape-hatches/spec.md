## ADDED Requirements

### Requirement: Escape Hatches SHALL Be Deleted At Terminal State

Every known escape hatch SHALL be removed after its service/facade replacement exists. Terminal state SHALL contain no production modules, public functions, attributes, bridge aliases, old routes, or test callers whose only purpose is to keep the old path callable.

#### Scenario: Replacement exists for an escape hatch
- **WHEN** a service/facade replacement exists for an old direct path
- **THEN** implementation SHALL migrate all callers to the replacement
- **AND** delete the old path, tests, public exports, and static-gate exemption in the same phase

#### Scenario: Escape hatch remains after migration
- **WHEN** a terminal gate scans production or integration-test Rust and finds a remaining escape hatch token
- **THEN** the gate SHALL fail with canonical replacement guidance

### Requirement: Runtime And Application Public Old Surfaces SHALL Be Removed

Runtime-host, application framework, context services, and agent framework/application crates SHALL remove public old surfaces rather than keeping deprecated searchable wrappers.

#### Scenario: Runtime-host old facade remains public
- **WHEN** runtime-host public API exposes an old MCP manager, entitlement facade, optional bootstrap, or provider bridge surface
- **THEN** the escape-hatch gate SHALL fail and require a typed service provider/client surface

#### Scenario: Application old helper remains public
- **WHEN** application/framework/context/agent production API exposes old prompt, task planning, entry agent, context engine, or capability conversion helper
- **THEN** the escape-hatch gate SHALL fail and require canonical manifest projection, context composer, service command, or builder/value-object replacement

## REMOVED Requirements

### Requirement: Static Escape-Hatch Freeze

**Reason**: Freeze mode only prevents new debt. Terminal cleanup requires deleting existing debt.

**Migration**: Replace freeze-only checks with zero-occurrence terminal checks.

### Requirement: Freeze Without Behavior Removal

**Reason**: Preserving existing behavior without removal is a migration-stage rule and conflicts with this terminal change.

**Migration**: Preserve stable external behavior through canonical service/facade paths, then remove old paths.
