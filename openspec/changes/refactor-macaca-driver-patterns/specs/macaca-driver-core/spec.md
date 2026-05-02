## ADDED Requirements

### Requirement: Driver creation SHALL use a factory primitive

`macaca-driver` SHALL provide a canonical factory primitive for constructing software drivers without changing existing driver ABI behavior.

#### Scenario: Loader creates a dynamic driver

- **GIVEN** a discovered driver manifest and library path
- **WHEN** the loader needs to instantiate the driver
- **THEN** it SHALL create the driver through a `DriverFactory` implementation
- **AND** existing dynamic driver loading behavior SHALL remain compatible

#### Scenario: Legacy dynamic loading entrypoint remains during migration

- **GIVEN** a caller still uses the old direct dynamic loading entrypoint
- **WHEN** the code compiles during migration
- **THEN** the entrypoint SHALL still exist
- **AND** it SHALL be marked deprecated with canonical replacement guidance

### Requirement: Driver tool execution SHALL use command primitives internally

`macaca-driver` SHALL represent driver tool execution as typed commands before crossing dynamic execution boundaries.

#### Scenario: Dynamic tool executes without streaming

- **GIVEN** a dynamic tool receives JSON input
- **WHEN** it executes without a trace sender
- **THEN** it SHALL use a driver command for the execution intent
- **AND** the output and error behavior SHALL match the previous non-streaming path

#### Scenario: Dynamic tool executes with streaming

- **GIVEN** a dynamic tool receives JSON input and a trace sender
- **WHEN** the dynamic driver supports streaming execution
- **THEN** it SHALL use a driver command for the streaming execution intent
- **AND** the output, error, and fallback behavior SHALL match the previous streaming path

### Requirement: Driver trace enrichment SHALL be centralized

`macaca-driver` SHALL provide a trace adapter that enriches driver trace events consistently.

#### Scenario: Driver trace lacks identity and timestamp

- **GIVEN** a driver emits a trace event without `driver_id` or timestamp
- **WHEN** the event is converted for Macaca trace forwarding
- **THEN** the adapter SHALL fill the driver identity
- **AND** the adapter SHALL fill a timestamp when one is missing

#### Scenario: Driver trace already contains metadata

- **GIVEN** a driver emits a trace event with `driver_id` or timestamp
- **WHEN** the event is converted for Macaca trace forwarding
- **THEN** the adapter SHALL preserve existing metadata

### Requirement: Dynamic ABI calls SHALL be isolated behind a proxy primitive

`macaca-driver` SHALL isolate dynamic C-ABI calls behind a proxy primitive while preserving public `DynamicDriver` behavior.

#### Scenario: Dynamic driver exposes tools

- **GIVEN** a loaded dynamic driver
- **WHEN** tools are requested
- **THEN** dynamic ABI tool-definition lookup SHALL be performed through the proxy
- **AND** returned tools SHALL behave as before

#### Scenario: Dynamic driver lifecycle is checked

- **GIVEN** a loaded dynamic driver
- **WHEN** health check, shutdown, or destroy behavior is invoked
- **THEN** dynamic ABI calls SHALL be routed through the proxy
- **AND** dynamic library drop-order safety SHALL be preserved

### Requirement: Streaming callback state SHALL be explicit

`macaca-driver` SHALL represent streaming callback state with an explicit session state primitive.

#### Scenario: Streaming callback receives an event

- **GIVEN** a dynamic driver streaming callback receives a serialized trace event
- **WHEN** the callback forwards the event
- **THEN** it SHALL access the event sender and driver identity through `DriverSessionState`
- **AND** the state SHALL remain scoped to the active blocking FFI call

### Requirement: Legacy driver APIs SHALL remain as deprecated compatibility wrappers

Existing direct entrypoints replaced by new primitives SHALL remain available during migration but SHALL be marked deprecated.

#### Scenario: Deprecated driver entrypoint is still present

- **GIVEN** a downstream consumer has not yet migrated
- **WHEN** it references a legacy direct driver entrypoint
- **THEN** the entrypoint SHALL still exist
- **AND** deprecation guidance SHALL identify the canonical replacement
