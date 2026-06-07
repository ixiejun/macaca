## ADDED Requirements

### Requirement: Agent memory service uses facade primitives

Upper-crate agent memory services SHALL expose facade-first memory operations backed by `macaca-memory` request/result types.

#### Scenario: Agent recalls through facade

- **GIVEN** an agent service bundle has a configured memory service
- **WHEN** upper code calls `recall` with a `RecallQuery`
- **THEN** the service returns a `RecallResult`
- **AND** callers do not need to construct raw store queries.

#### Scenario: Agent remembers text through facade

- **GIVEN** an agent service bundle has a configured memory service
- **WHEN** upper code calls `remember_text` with a `RememberText`
- **THEN** the service stores memory through the configured backend
- **AND** returns the created `MemoryId`.

### Requirement: Deprecated memory service compatibility remains callable

The old agent memory service `store` and `retrieve` methods SHALL remain callable as deprecated compatibility helpers until all external consumers can migrate.

#### Scenario: Deprecated store remains callable

- **GIVEN** legacy code still calls `MemoryService::store`
- **WHEN** the crate is compiled
- **THEN** the method remains available
- **AND** it is marked deprecated for migration discovery.

#### Scenario: Deprecated retrieve remains callable

- **GIVEN** legacy code still calls `MemoryService::retrieve`
- **WHEN** the crate is compiled
- **THEN** the method remains available
- **AND** it is marked deprecated for migration discovery.

### Requirement: Kernel memory adapter uses facade-capable backends

`macaca-kernel` SHALL adapt `macaca-memory` manager backends to `macaca-agent::MemoryService` through facade-first methods.

#### Scenario: Kernel adapter preserves recall behavior

- **GIVEN** a kernel memory adapter wrapping a memory manager
- **WHEN** it remembers text and recalls by query
- **THEN** recalled entries match the stored memory behavior from the underlying manager.

#### Scenario: Kernel adapter supports isolated memory manager

- **GIVEN** a kernel memory adapter wrapping an isolated memory manager
- **WHEN** it remembers text and recalls by query
- **THEN** recalled entries remain scoped to that isolated manager.

### Requirement: No-op memory service remains side-effect free

No-op memory SHALL preserve existing no-service behavior while implementing facade-first methods.

#### Scenario: No-op recall returns empty result

- **GIVEN** an agent service bundle without configured memory
- **WHEN** upper code calls `recall`
- **THEN** the result contains no entries.

#### Scenario: No-op remember returns synthetic id

- **GIVEN** an agent service bundle without configured memory
- **WHEN** upper code calls `remember_text`
- **THEN** the call succeeds with a non-default `MemoryId`
- **AND** no persistent memory is written.
