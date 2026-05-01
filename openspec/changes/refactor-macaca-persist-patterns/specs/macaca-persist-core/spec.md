## ADDED Requirements

### Requirement: Event Replay Iterator Primitive

The `macaca-persist` crate SHALL provide an additive-first event replay iterator primitive for ordered session event restoration.

#### Scenario: Existing event readers remain compatible

- **WHEN** the crate introduces replay iterator support
- **THEN** existing event-reading entry points remain available
- **AND** their externally observable ordering behavior remains unchanged

#### Scenario: Replay preserves event order

- **WHEN** persisted events are replayed for a session
- **THEN** the iterator yields them in the same order they were appended
- **AND** callers can continue replay from a stable cursor boundary

### Requirement: Append Event Command Object

The `macaca-persist` crate SHALL provide an explicit command object for appending session events.

#### Scenario: Append parameters are passed as one command

- **WHEN** a caller appends an event
- **THEN** the session identifier, event type, source, and payload are represented as a single append command object
- **AND** the append operation preserves the existing stored payload semantics

### Requirement: Checkpoint Builder Compatibility

The `macaca-persist` crate SHALL provide a builder-style checkpoint construction entry while preserving existing checkpoint behavior.

#### Scenario: Builder does not change checkpoint meaning

- **WHEN** a checkpoint is created through the new builder entry
- **THEN** the resulting checkpoint is semantically equivalent to the existing construction path
- **AND** existing checkpoint recovery behavior remains unchanged

### Requirement: Backend Strategy Boundary

The `macaca-persist` crate SHALL separate persistence backend contract from the default Redb implementation.

#### Scenario: Redb remains default backend

- **WHEN** backend abstraction is introduced
- **THEN** `RedbStore` remains the default backend implementation
- **AND** no new backend is required for the refactor to be valid

### Requirement: Session Snapshot Memento Semantics

The `macaca-persist` crate SHALL define snapshot semantics for session persistence state without changing higher-level restore behavior.

#### Scenario: Snapshot captures restore-critical references

- **WHEN** a session persistence snapshot is created
- **THEN** it captures the restore-critical metadata needed for session recovery
- **AND** it does not alter existing refresh, trace restore, or resume semantics by itself
