## ADDED Requirements

### Requirement: IPC SHALL expose a transport bridge contract

The `macaca-ipc` crate SHALL provide a transport-level abstraction that lets callers obtain message sender and receiver capabilities without depending on a concrete transport implementation.

#### Scenario: Create local transport through bridge contract
- **WHEN** a caller constructs a local IPC transport through the new contract
- **THEN** the caller can obtain a sender and a receiver
- **AND** publish, direct-send, subscribe, unsubscribe, and receive semantics remain unchanged from the previous local bus behavior

#### Scenario: Create NATS transport through bridge contract
- **WHEN** a caller constructs a NATS IPC transport through the new contract
- **THEN** the caller can obtain a sender and a receiver
- **AND** publish, direct-send, subscribe, unsubscribe, and receive semantics remain unchanged from the previous NATS bus behavior

### Requirement: IPC SHALL provide a factory-based transport selection entry

The `macaca-ipc` crate SHALL provide a unified factory entry for selecting and constructing a transport from explicit transport configuration, so upper-layer crates do not need to branch on transport implementation details.

#### Scenario: Select local transport from configuration
- **WHEN** a caller passes local transport configuration to the factory
- **THEN** the factory returns a local transport implementation through the common bridge contract

#### Scenario: Select NATS transport from configuration
- **WHEN** a caller passes NATS transport configuration to the factory
- **THEN** the factory returns a NATS transport implementation through the common bridge contract

### Requirement: Legacy bus constructors SHALL remain available but deprecated

Existing `LocalBus` and `NatsBus` compatibility APIs SHALL remain available during the migration period, but the legacy sender/receiver creation methods MUST be marked deprecated so callers can be located and migrated later.

#### Scenario: Existing local bus callers continue to compile
- **WHEN** an existing caller uses `LocalBus::new()` and then calls `sender()` or `receiver()`
- **THEN** the code still compiles and runs
- **AND** the legacy methods are marked deprecated

#### Scenario: Existing NATS bus callers continue to compile
- **WHEN** an existing caller uses `NatsBus::connect()` and then calls `sender()` or `receiver()`
- **THEN** the code still compiles and runs
- **AND** the legacy methods are marked deprecated
