## ADDED Requirements

### Requirement: Web Uses Application Service First
The Web shell SHALL prefer `SystemApplicationClient` for application discovery, startup, status, route views, reload, chat preflight, and GenUI surface lookup.

#### Scenario: Web startup uses service
- **WHEN** the Web server boots
- **THEN** it SHALL register/start Application Service and use it to discover and start YAML applications before falling back to deprecated direct runtime compatibility paths.

#### Scenario: Web state keeps compatibility anchors
- **WHEN** Web state is built during S7 migration
- **THEN** direct `AppRegistry` and `AppRuntime` fields MAY remain but SHALL be marked deprecated and SHALL NOT be the preferred new call path.

### Requirement: Web Preserves Existing User Behavior
The Web shell SHALL preserve existing YAML auto-start behavior, app list/detail/agents routes, `/api/chat/v2` SSE response shape, session persistence, task/goal resume, trace streaming, and S6 Driver/Skill/MCP service-backed toolkit behavior.

#### Scenario: Chat preflight is service-backed
- **WHEN** `/api/chat/v2` receives a request
- **THEN** Web SHALL use Application Service for entry-agent resolution, app/session lifecycle preflight, executor readiness metadata, and session envelope, while keeping coordinator execution in the existing framework runner for S7.

#### Scenario: SSE response shape is unchanged
- **WHEN** `/api/chat/v2` creates a new session
- **THEN** the first SSE event SHALL still provide the session id using the existing response shape.

### Requirement: Web GenUI Surface Lookup
The Web shell SHALL query Application Service for application-owned GenUI surface data and preserve the current unavailable/no-surface fallback when no service surface exists.

#### Scenario: No surface remains safe
- **WHEN** a GenUI surface is requested and Application Service reports no application-provided surface
- **THEN** Web SHALL return the existing structured fallback rather than panic or block.

#### Scenario: GenUI logs are safe
- **WHEN** Web sends a GenUI surface or event command through Application Service
- **THEN** logs SHALL include app id, session id, surface id, event id, and trace id, and SHALL NOT include raw unsafe UI payloads beyond validated metadata.

