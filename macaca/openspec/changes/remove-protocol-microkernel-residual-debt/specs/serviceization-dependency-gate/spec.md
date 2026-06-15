## ADDED Requirements

### Requirement: Terminal Debt Token Gate

Macaca SHALL provide an executable terminal debt gate that rejects old-path debt tokens in production and integration-test Rust source after migration. The gate SHALL cover deprecated attributes, allow-deprecated attributes, old route terminology, old helper names, provider bridge names, and public compatibility facade names.

#### Scenario: Debt token appears in production Rust
- **WHEN** production Rust source contains an old-path debt token after terminal migration
- **THEN** the gate SHALL fail with file, line, token, owning layer, and canonical replacement
- **AND** the token SHALL be removed by migrating behavior, not by adding an allowlist

#### Scenario: Deprecated attribute appears in tests
- **WHEN** integration-test Rust source contains `#[deprecated]` or `#[allow(deprecated)]`
- **THEN** the gate SHALL fail
- **AND** tests SHALL use canonical fixtures and APIs instead

### Requirement: Terminal Dependency Gate SHALL Have Zero Migration Exceptions

Dependency boundary gates SHALL reject all new and existing migration exceptions at terminal state. No direct workspace edge may be allowed because a bridge, route, or compatibility alias still exists.

#### Scenario: Migration exception is present
- **WHEN** dependency gate allowlist rows or migration exception rows are non-empty
- **THEN** the terminal dependency gate SHALL fail and identify the source crate, target crate, rule id, and service/facade replacement

### Requirement: SDK And Shell Ownership Gates SHALL Be Enforced

Dependency and static gates SHALL prove SDK and shells cannot hide provider/runtime ownership through re-exports, bridge modules, construction ports, or local execution state.

#### Scenario: SDK re-exports lower-layer provider
- **WHEN** SDK production source re-exports provider/runtime-host/application/framework crates
- **THEN** the gate SHALL fail and require a focused SDK client or proto DTO boundary

#### Scenario: Shell owns execution state
- **WHEN** shell production source owns task loops, wakers, framework construction, provider anchors, or old route wrappers
- **THEN** the gate SHALL fail and require service/runtime-host ownership

## REMOVED Requirements

### Requirement: Macaca SHALL represent current violations through a migration allowlist

**Reason**: Terminal debt cleanup cannot represent remaining violations as acceptable migration rows.

**Migration**: Remove each violating edge by moving callers to service/facade boundaries, then keep the allowlist empty.

### Requirement: Macaca SHALL keep S0 additive and non-migrating

**Reason**: This phase-specific rule no longer applies after terminal serviceization.

**Migration**: Replace with terminal zero-debt enforcement and canonical protocol path requirements.
