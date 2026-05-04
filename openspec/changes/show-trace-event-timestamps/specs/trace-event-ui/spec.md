## ADDED Requirements

### Requirement: Trace event blocks show occurrence time when available

The web UI SHALL display a compact event time on trace event blocks when the trace step has a timestamp.

#### Scenario: Main thread trace event is loaded from EventLog

- **WHEN** the frontend converts an EventLog event into a main thread trace step
- **THEN** it SHALL preserve the EventLog timestamp on the trace step
- **AND** the trace block SHALL display the event time

#### Scenario: Delegated trace event is loaded from EventLog

- **WHEN** the frontend converts an EventLog event into a delegated trace step
- **THEN** it SHALL preserve the EventLog timestamp on the delegated trace step
- **AND** the delegated trace block SHALL display the event time

#### Scenario: Trace step has no timestamp

- **WHEN** a trace step has no timestamp
- **THEN** the UI SHALL render the trace block without inventing a fake occurrence time
