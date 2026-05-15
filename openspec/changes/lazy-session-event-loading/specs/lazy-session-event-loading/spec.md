## ADDED Requirements

### Requirement: Application session loading is lightweight and paged

The system SHALL load application sessions as lightweight summaries and SHALL default application session listing to at most 20 sessions when no explicit limit is provided.

#### Scenario: Application opens session list

- **WHEN** the frontend opens an application
- **THEN** it SHALL request lightweight session summaries
- **AND** the backend SHALL return no more than 20 sessions by default

#### Scenario: Client requests a larger page

- **WHEN** a client provides an explicit session list `limit`
- **THEN** the backend SHALL use that limit up to the configured maximum

#### Scenario: User loads older sessions

- **WHEN** the sidebar has loaded a full page of session summaries
- **THEN** it SHALL expose a "load more" action
- **AND** activating it SHALL append the next page without replacing the visible page

### Requirement: Session detail is not an application-wide trace payload

The system SHALL return selected session detail without application-scoped plan decision history and without rebuilding all trace events in the session detail hot path.

#### Scenario: Session detail is fetched

- **WHEN** the frontend fetches `GET /api/sessions/detail/{session_id}`
- **THEN** the response SHALL include session messages, stored turns, metadata, event URL, and event count
- **AND** it SHALL NOT include app-scoped `plan_decisions`
- **AND** it SHALL NOT replay all EventLog rows to reconstruct delegated traces

### Requirement: Session events are fetched by selected scope

The system SHALL expose session-scoped EventLog reads that can be filtered by generic event source, agent, and event type.

#### Scenario: Main thread trace is loaded

- **WHEN** the frontend needs coordinator trace for a selected session
- **THEN** it SHALL fetch events for that session and render the manifest entry-agent source as the main thread
- **AND** executor-sourced delegated events for the manifest entry agent SHALL render in the main thread instead of being lost when that entry agent is hidden from delegated tabs
- **AND** entry-agent context, skill, and MCP lifecycle events SHALL render as bounded trace details
- **AND** it SHALL preserve legacy generic coordinator-source events for older sessions without hardcoding application-specific agent names

#### Scenario: Delegated tab trace is loaded

- **WHEN** the frontend opens a delegated agent tab
- **THEN** it SHALL fetch events for the selected session filtered to that agent

#### Scenario: Delegated tab trace is loading

- **WHEN** the frontend opens a delegated agent tab that has not completed its event fetch
- **THEN** it SHALL show a loading indicator instead of the empty trace message
- **AND** it SHALL show the empty trace message only after the event fetch completes with no trace data

### Requirement: Legacy app-scoped plan decisions remain discoverable but deprecated

The system SHALL keep legacy app-scoped plan decision storage helpers discoverable for migration, but SHALL NOT use them for session detail rendering.

#### Scenario: Session detail is built

- **WHEN** the backend builds session detail
- **THEN** it SHALL NOT call the app-scoped plan decision loader
