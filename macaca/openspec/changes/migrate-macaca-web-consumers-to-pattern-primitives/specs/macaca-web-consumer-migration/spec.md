## ADDED Requirements

### Requirement: Rust consumers use builder startup

Upper-layer Rust consumers SHALL use `WebServerBuilder` for web server startup and SHALL NOT call deprecated `start_server` directly.

#### Scenario: CLI starts web server

- **WHEN** the CLI handles the web command
- **THEN** it SHALL start the server through `WebServerBuilder`
- **AND** deprecated `start_server` SHALL remain only as a compatibility definition

### Requirement: Frontend consumes web APIs through a facade

The frontend SHALL centralize Macaca web API URL construction and JSON/SSE request helpers behind a lightweight facade while preserving existing exported API functions.

#### Scenario: Existing UI calls status and apps APIs

- **WHEN** the launcher calls `fetchStatus` and `fetchApps`
- **THEN** those functions SHALL delegate through the facade
- **AND** they SHALL keep requesting `/api/status` and `/api/apps`

#### Scenario: Existing UI opens SSE streams

- **WHEN** the workspace subscribes to agent or session streams
- **THEN** EventSource URLs SHALL be built through the facade
- **AND** the stream endpoint paths SHALL remain unchanged

### Requirement: Active consumers avoid legacy chat endpoint

Active consumer code and current usage docs SHALL use `/api/chat/v2` for chat requests and SHALL NOT recommend legacy `/api/chat`.

#### Scenario: Chat request is sent

- **WHEN** the frontend sends a chat prompt
- **THEN** it SHALL request `/api/chat/v2`
- **AND** active docs SHALL point new consumers to `/api/chat/v2`

### Requirement: E2E web API consumers are application-generic

Active E2E scripts SHALL default to discovering applications and agents through web APIs rather than relying on hardcoded app IDs or concrete agent names.

#### Scenario: E2E runs without explicit app id

- **WHEN** `APP_ID` is not provided
- **THEN** the script SHALL discover an application from `/api/apps`
- **AND** agent checks SHALL use agents returned by `/api/apps/{id}/agents`

#### Scenario: Fixture run needs a specific app

- **WHEN** `APP_ID` is provided
- **THEN** the script SHALL use that app id
- **AND** it SHALL still avoid hardcoded agent role assumptions unless explicitly overridden
