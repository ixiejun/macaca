## ADDED Requirements

### Requirement: Web UI Task Board SHALL require session scope

The Web UI Task Board todos endpoint SHALL require both application id and session id before reading persisted todos.

#### Scenario: Missing session id is rejected

- **GIVEN** a client calls `GET /api/apps/{app_id}/todos` without `session_id`
- **WHEN** the route handles the request
- **THEN** it SHALL return `400`
- **AND** it SHALL NOT call the application-wide todo scan path

#### Scenario: Blank session id is rejected

- **GIVEN** a client calls `GET /api/apps/{app_id}/todos?session_id=`
- **WHEN** the route handles the request
- **THEN** it SHALL return `400`
- **AND** it SHALL NOT call the application-wide todo scan path

#### Scenario: Current session todos are loaded lazily

- **GIVEN** the user has not opened the Task Board modal
- **WHEN** the chat page renders or receives trace events
- **THEN** the Web UI SHALL NOT request todos

- **GIVEN** the user opens the Task Board modal for a session
- **WHEN** the modal loads data
- **THEN** the Web UI SHALL request todos with `app_id` and `session_id`
- **AND** the backend SHALL use the session-scoped todo prefix
