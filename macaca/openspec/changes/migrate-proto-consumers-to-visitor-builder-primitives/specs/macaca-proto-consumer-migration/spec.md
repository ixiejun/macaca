## ADDED Requirements

### Requirement: Upper crates SHALL prefer proto visitor for high-frequency event consumption

The system SHALL migrate high-frequency upper-layer `AgentExecutionEvent` consumption paths to the visitor-based access pattern provided by `macaca-proto`.

#### Scenario: Repeated event translation uses visitor

- **GIVEN** an upper crate contains a repeated or central event translation path for `AgentExecutionEvent`
- **WHEN** this migration is applied
- **THEN** that path SHALL consume the event through the visitor-style interface
- **AND** the produced payload, trace step, or display output SHALL remain behaviorally equivalent

#### Scenario: Event schema remains unchanged

- **GIVEN** an upper crate migrated to the visitor path
- **WHEN** it emits SSE, trace, event log, or runtime bridge payloads
- **THEN** the event names and payload schema SHALL remain unchanged

### Requirement: Upper crates SHALL prefer proto builders for high-frequency config construction

The system SHALL migrate high-frequency upper-layer proto config construction paths to builder-based construction where the old code repeatedly hand-constructs large configs.

#### Scenario: Builder replaces repeated config boilerplate

- **GIVEN** an upper crate repeatedly hand-constructs a proto config DTO with many defaulted fields
- **WHEN** this migration is applied
- **THEN** the construction path SHALL prefer the proto builder
- **AND** the resulting config SHALL remain equivalent to the previous hand-written construction

#### Scenario: Direct construction remains compatible

- **GIVEN** `macaca-proto` still keeps direct struct construction available
- **WHEN** upper crates are migrated
- **THEN** the migration SHALL NOT require removal of the old proto API
- **AND** compatibility for unchanged callers SHALL remain intact

### Requirement: Upper crates SHALL use ProtoErrorAdapter for user-visible proto errors

The system SHALL migrate user-visible proto error display in upper crates to the unified proto error adapter.

#### Scenario: User-visible error text uses adapter

- **GIVEN** an upper crate surfaces a proto-layer error to users through CLI, API, trace, or UI-oriented payloads
- **WHEN** this migration is applied
- **THEN** the display text and error code SHALL be derived through `ProtoErrorAdapter`
- **AND** the semantic meaning of the error SHALL remain unchanged

#### Scenario: Runtime policy stays outside proto

- **GIVEN** an upper crate consumes a proto error
- **WHEN** it uses `ProtoErrorAdapter`
- **THEN** retry policy, HTTP status policy, and recovery policy SHALL remain implemented outside `macaca-proto`

### Requirement: Migration SHALL remain additive-first at the proto boundary

The system SHALL migrate upper crates without turning the proto-layer additive refactor into a breaking change.

#### Scenario: Proto compatibility remains in place

- **GIVEN** `macaca-proto` keeps its legacy enum and struct construction APIs
- **WHEN** upper crates migrate to visitor / builder / adapter usage
- **THEN** the old proto API SHALL remain available as a compatibility layer
- **AND** the migration SHALL be achieved by changing upper-layer consumption, not by deleting proto compatibility

#### Scenario: Business semantics are unchanged

- **GIVEN** an upper crate is migrated to the new proto primitives
- **WHEN** the system executes coordinator, planner, worker, runtime, CLI, or web flows
- **THEN** task semantics, session semantics, trace semantics, and wire schema SHALL remain behaviorally unchanged
