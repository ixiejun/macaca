## ADDED Requirements

### Requirement: EventLog stores canonical session events with secondary indexes

The system SHALL persist every new session event under a canonical `session_id` primary key and SHALL write secondary indexes for generic source, agent, and event type scopes when those values are available.

#### Scenario: Event is appended with agent metadata

- **WHEN** a session event is appended with an agent name in explicit metadata or payload
- **THEN** the canonical row SHALL be stored under `events/{session_id}/{seq}`
- **AND** an agent index row SHALL be stored under that same `session_id`
- **AND** the event timestamp SHALL be assigned at append time

#### Scenario: Event is appended without agent metadata

- **WHEN** a session event has no explicit agent name and no generic agent payload field
- **THEN** the canonical row SHALL still be stored
- **AND** no application-specific or hardcoded agent fallback SHALL be invented

### Requirement: Session event reads use indexed selected scopes

The system SHALL serve selected session event reads through an EventLog query facade that uses a session-scoped secondary index when a source, agent, or event type filter is requested.

#### Scenario: Delegated tab events are fetched

- **WHEN** a client fetches `/api/sessions/{session_id}/events` with an `agent` filter
- **THEN** the backend SHALL read events by `session_id + agent`
- **AND** it SHALL NOT scan application-wide events
- **AND** it SHALL honor `since` and `limit`

#### Scenario: Main thread events are fetched

- **WHEN** a client fetches `/api/sessions/{session_id}/events` with a `source` filter
- **THEN** the backend SHALL read events by `session_id + source`
- **AND** it SHALL return only matching events up to the requested limit

#### Scenario: Run trace events are fetched

- **WHEN** a client fetches `/api/sessions/{session_id}/run-trace`
- **THEN** the backend SHALL read `run_trace` events by `session_id + event_type`
- **AND** it SHALL NOT replay the full session before filtering

### Requirement: Development history is reset instead of migrated

The system SHALL not backfill indexes for existing development session history in this change; local historical sessions SHALL be cleared before validating the new indexed storage behavior.

#### Scenario: Existing development database is present

- **WHEN** the indexed EventLog implementation is ready for validation
- **THEN** backend processes SHALL be stopped
- **AND** the local development `sessions.db` SHALL be deleted
- **AND** new sessions SHALL generate indexed event rows on append
