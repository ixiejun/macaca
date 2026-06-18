## ADDED Requirements

### Requirement: Runtime-host provider families SHALL be split by ownership before extraction

Runtime-host provider modules approaching governance limits SHALL be split internally by clear ownership boundaries before any service-family crate extraction is attempted. Valid split boundaries include descriptor construction, command routing, command handlers, state, adapter bridges, lifecycle helpers, diagnostics, and test fixtures.

#### Scenario: Near-limit provider file receives new behavior
- **WHEN** a production runtime-host provider file is at or above the advisory line threshold
- **THEN** new behavior SHALL be implemented in an ownership-specific module
- **AND** comments SHALL explain ownership, design pattern intent, trace/audit behavior, and non-goals
- **AND** the public service descriptor and command surface SHALL remain provider-neutral

#### Scenario: Provider split preserves lifecycle
- **WHEN** a provider family is split into descriptor, state, command, handler, adapter, or fixture modules
- **THEN** service lifecycle, health, snapshot, structured errors, trace context, policy admission, and sanitized audit behavior SHALL remain unchanged

### Requirement: Mature provider-family extraction SHALL require readiness proof

Macaca SHALL NOT extract a runtime-host provider family into a dedicated service crate until an extraction-readiness checklist proves contract stability, test coverage, replacement mechanics, trace/audit behavior, optional unavailable behavior, and rollback strategy.

#### Scenario: Provider family is proposed for extraction
- **WHEN** maintainers propose extracting a mature provider family out of runtime-host
- **THEN** the proposal SHALL include extraction-readiness evidence
- **AND** the evidence SHALL include public contract stability, service replacement mechanics, trace/audit behavior, dependency impact, targeted tests, and rollback plan
- **AND** no new application-specific, provider-specific, or business-specific dependency edge SHALL be introduced

### Requirement: ServiceRuntime SHALL support architecture-smell diagnostics as report-only observations

Macaca SHALL provide a deterministic architecture-smell diagnostic lane for complexity, coupling, file-size headroom, static state, and DTO-density trends. The initial lane SHALL report findings without failing hard-boundary CI.

#### Scenario: Smell diagnostics run in report-only mode
- **WHEN** the architecture-smell diagnostic lane runs
- **THEN** it SHALL emit deterministic sanitized findings with rule identifiers, file paths, line numbers when available, and suggested owner boundaries
- **AND** report-only complexity/coupling trends SHALL NOT fail the build
- **AND** hard architecture boundary violations SHALL continue to fail through their existing gates
