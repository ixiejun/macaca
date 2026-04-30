## ADDED Requirements

### Requirement: Proto event visitor preserves existing wire behavior

The system SHALL provide visitor-style access to core proto event enums while preserving existing enum schema and serialization behavior.

#### Scenario: Existing event serialization remains unchanged

- **GIVEN** a core proto event value that can be serialized before this refactor
- **WHEN** the visitor interface is added
- **THEN** serializing the event SHALL produce the same wire representation as before
- **AND** existing code that pattern matches on the enum SHALL remain valid

#### Scenario: Visitor dispatch matches event variant semantics

- **GIVEN** a supported proto event variant
- **WHEN** the event is passed through the new visitor entrypoint
- **THEN** the matching visitor method SHALL be invoked
- **AND** the visitor SHALL receive the same semantic field values as direct enum matching

### Requirement: Proto config builders remain additive and equivalent

The system SHALL provide builder-based construction for selected high-frequency proto config DTOs without breaking direct struct construction.

#### Scenario: Builder output matches existing defaults

- **GIVEN** a selected proto config DTO with an existing default or commonly used hand-written construction pattern
- **WHEN** the builder constructs the DTO with equivalent inputs
- **THEN** the resulting DTO SHALL be equal to the existing construction result

#### Scenario: Direct struct construction remains supported

- **GIVEN** existing code that constructs a selected proto config DTO directly
- **WHEN** builder support is introduced
- **THEN** direct struct construction SHALL remain supported
- **AND** no field name or serialization contract SHALL change

### Requirement: Proto error display and code adaptation remain stable

The system SHALL provide a unified proto-layer error adaptation entry for display and code extraction while preserving existing error semantics.

#### Scenario: Error meaning is preserved

- **GIVEN** an existing proto error value
- **WHEN** the new error adaptation entry is used
- **THEN** the adapted display and code information SHALL represent the same underlying error meaning as before
- **AND** the original error type SHALL remain usable by existing callers

#### Scenario: Proto error adapter does not introduce runtime policy

- **GIVEN** a proto-layer error
- **WHEN** the error is adapted for display or code extraction
- **THEN** the adapter SHALL NOT decide HTTP policy, retry policy, or recovery policy
- **AND** those decisions SHALL remain in upper layers

### Requirement: Proto contracts remain data-only and strategy-free

The system SHALL keep `macaca-proto` focused on data contracts and SHALL NOT move runtime strategy into the proto layer.

#### Scenario: Runtime strategy remains outside proto

- **GIVEN** planning, review, session resume, tool policy, or traced execution behavior
- **WHEN** `macaca-proto` is refactored
- **THEN** those runtime strategies SHALL remain implemented in upper layers such as task, framework, kernel, or web
- **AND** proto SHALL only define the DTOs, events, configs, and errors required to represent those behaviors

#### Scenario: Refactor does not reduce observability

- **GIVEN** the Agent OS currently persists and displays trace, task, and session events using proto contracts
- **WHEN** visitor, builder, or error adaptation abstractions are added
- **THEN** event names, payload fields, and trace-related DTO schemas SHALL remain unchanged
- **AND** no fake or synthetic trace event SHALL be introduced by the proto refactor
