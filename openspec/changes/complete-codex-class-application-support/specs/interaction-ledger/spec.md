## ADDED Requirements

### Requirement: Durable Interaction Ledger Service
The system SHALL provide `service.interaction` as the service-owned ledger for
generic Thread, Turn, and Item lifecycle state.

#### Scenario: Start and replay a thread
- **WHEN** an application starts a thread with trace, application, session, and
  scope metadata
- **THEN** `service.interaction` SHALL persist a thread record, emit a sanitized
  event, and return a replayable thread reference
- **AND** reading the thread later SHALL return bounded metadata and item refs
  without raw secrets or unbounded provider payloads

#### Scenario: Fork and rollback are replayable
- **WHEN** a loaded or persisted thread is forked or rolled back
- **THEN** the service SHALL persist a memento record that identifies the source
  thread and rollback boundary
- **AND** future resumes SHALL reconstruct the pruned or forked state from the
  ledger instead of relying on shell memory

### Requirement: Turn Lifecycle and Steering
The system SHALL model turn start, interruption, steering, completion, and
failure as explicit service-owned state transitions.

#### Scenario: Interrupt active turn
- **WHEN** a caller interrupts an active turn through `service.interaction`
- **THEN** the service SHALL record an interrupted state, emit a turn event, and
  propagate cancellation to downstream execution services
- **AND** later replay SHALL show the interruption boundary

#### Scenario: Steer active turn
- **WHEN** user input is steered into an active turn
- **THEN** the service SHALL append a bounded steering item and emit an item
  notification
- **AND** it SHALL reject steering for completed, failed, archived, or
  unsupported turn states with structured errors

### Requirement: Item Stream and Artifact Boundaries
The system SHALL represent user input, agent output, reasoning summaries, tool
calls, tool results, shell output, file edits, approvals, reviews, and
diagnostics as typed items.

#### Scenario: Oversized item payload
- **WHEN** an item payload exceeds the inline budget or contains sensitive
  provider output
- **THEN** the service SHALL store the payload as an artifact reference
- **AND** item streams and snapshots SHALL include only bounded summaries and
  artifact refs

#### Scenario: Subscribe to item events
- **WHEN** a shell or protocol gateway subscribes to a thread
- **THEN** the service SHALL emit typed item lifecycle notifications
- **AND** subscribers SHALL not become owners of item persistence semantics
