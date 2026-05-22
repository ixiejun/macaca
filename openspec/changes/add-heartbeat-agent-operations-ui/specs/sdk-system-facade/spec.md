## ADDED Requirements

### Requirement: SDK SHALL expose focused Heartbeat profile operations

Macaca SDK SHALL expose focused Heartbeat client operations for querying sanitized heartbeat snapshots, listing run mementos, and updating native profile policy through typed commands.

#### Scenario: Web updates a heartbeat profile

- **WHEN** Web updates a native heartbeat profile for an application
- **THEN** it SHALL call the focused Heartbeat SDK client with a trace-bearing typed command
- **AND** the SDK client SHALL delegate through the service runtime
- **AND** the SDK SHALL NOT construct concrete Heartbeat providers

#### Scenario: Heartbeat service is unavailable

- **WHEN** the focused Heartbeat client has no backing service
- **THEN** snapshot reads SHALL return an unavailable snapshot or empty bounded history
- **AND** mutating commands SHALL fail structurally instead of silently succeeding
