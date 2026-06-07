## ADDED Requirements

### Requirement: Macaca SHALL move kernel provider-facing construction behind compatibility boundaries

Macaca SHALL ensure that provider-oriented kernel construction lives behind a temporary compatibility adapter boundary rather than as the kernel core ownership model.

#### Scenario: Kernel retains migration shims

- **WHEN** existing code calls a legacy provider-oriented kernel constructor or builder
- **THEN** the call SHALL remain possible during migration
- **AND** the constructor or builder SHALL be marked deprecated with explicit replacement guidance
- **AND** the legacy path SHALL be searchable for later removal

#### Scenario: New kernel composition is facade-oriented

- **WHEN** new kernel composition code is added
- **THEN** it SHALL use provider-neutral or facade-oriented entry points
- **AND** it SHALL NOT require new direct provider ownership in kernel core modules

### Requirement: Macaca SHALL isolate temporary provider compatibility code from kernel core logic

Macaca SHALL place any remaining provider-facing kernel compatibility logic into a dedicated adapter boundary so that the kernel core remains focused on invariants, registries, policy, trace, and facades.

#### Scenario: Compatibility code remains isolated

- **WHEN** kernel still needs provider compatibility during transition
- **THEN** that code SHALL live in a dedicated compat boundary
- **AND** kernel core modules SHALL not reintroduce provider-specific branching as the default architecture

### Requirement: Macaca SHALL reduce direct kernel dependence on provider crates

Macaca SHALL reduce direct `macaca-kernel` dependence on replaceable provider crates where those dependencies are only needed for migration compatibility.

#### Scenario: Unused provider dependency is pruned

- **WHEN** a provider crate is no longer needed by the kernel core or its compat path
- **THEN** the dependency SHALL be removed from kernel direct dependencies
- **AND** the removal SHALL preserve existing behavior through the compat or service path

#### Scenario: Remaining provider dependency is clearly marked as migration debt

- **WHEN** a provider dependency still remains for a temporary compatibility reason
- **THEN** the code and documentation SHALL make that reason explicit
- **AND** the dependency SHALL remain represented as migration debt rather than accepted architecture

### Requirement: Macaca SHALL keep kernel boundary cleanup additive and non-breaking

Macaca SHALL preserve current kernel behavior while the provider dependency boundary is cleaned up.

#### Scenario: Current flows continue to work

- **WHEN** this change is implemented
- **THEN** existing kernel behavior, current tests, and existing shell/application flows SHALL continue to work through the legacy compatibility path
- **AND** the change SHALL NOT rename or delete migration breadcrumbs needed by later phases

### Requirement: Macaca SHALL preserve trace and audit visibility for compatibility paths

Macaca SHALL keep kernel compatibility and boundary cleanup paths observable with structured logs and trace-friendly diagnostics.

#### Scenario: Compatibility path is auditable

- **WHEN** a deprecated kernel constructor or compatibility adapter is used
- **THEN** the code SHALL emit structured logs at key execution nodes
- **AND** the diagnostics SHALL identify the migration path without embedding app-specific or provider-specific hardcoding
