## ADDED Requirements

### Requirement: Memory manager facade APIs

`macaca-memory` SHALL provide additive facade APIs for common remember, recall, list, get, and forget operations while preserving existing manager methods as deprecated compatibility helpers where superseded.

#### Scenario: Remember and recall through facade

- **GIVEN** a memory manager with session and file stores
- **WHEN** a caller stores text through the facade and recalls it by query
- **THEN** the returned entries match the existing storage and retrieval behavior.

#### Scenario: Deprecated manager methods remain callable

- **GIVEN** existing code still calls a deprecated manager-level direct method
- **WHEN** the crate is compiled
- **THEN** the method remains available for compatibility
- **AND** the deprecation marker makes the call site grepable for future migration.

### Requirement: Cached embedding provider

`macaca-memory` SHALL provide a decorator that caches embedding vectors per text input without changing the `EmbeddingProvider` trait.

#### Scenario: Repeated text uses cached vector

- **GIVEN** a cached embedding provider wrapping a counting provider
- **WHEN** the same text is embedded twice
- **THEN** the wrapped provider is called once for that text
- **AND** both embedding responses are identical.

### Requirement: Memory backend factory

`macaca-memory` SHALL provide additive backend factory configuration for standard manager construction without requiring upper crates to hardcode all concrete stores.

#### Scenario: Standard test manager construction

- **GIVEN** a file path and session TTL
- **WHEN** a standard in-memory-vector manager is created through the factory
- **THEN** storing and retrieving memory behaves like direct `MemoryManager::new` construction.

### Requirement: Memory snapshot memento

`macaca-memory` SHALL provide serializable snapshot types and replay helpers for session/file memory debugging and resume support.

#### Scenario: Snapshot round-trip

- **GIVEN** a memory store with two entries
- **WHEN** a snapshot is captured and replayed into an empty store
- **THEN** listing the destination store returns the same entries by memory id and content.

### Requirement: Vector query strategy

`macaca-memory` SHALL provide vector query strategy primitives that preserve current similarity search behavior and allow metadata filtering.

#### Scenario: Similarity search remains default

- **GIVEN** an in-memory vector store with multiple vectors
- **WHEN** the default strategy searches with a query vector
- **THEN** result ordering matches the existing vector store similarity ordering.
