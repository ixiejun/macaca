## ADDED Requirements

### Requirement: Skill Snapshot Lifecycle Events

The Skill Service Web adapter SHALL emit session-scoped lifecycle events for skill snapshot cache and build outcomes.

#### Scenario: Skill snapshot is built for an agent session

- **WHEN** a skill snapshot is loaded or built for a session-scoped agent
- **THEN** the session EventLog SHALL include lifecycle events such as build started, ready, failed, cached, or cache hit
- **AND** each event SHALL identify the agent and include sanitized counts or error summaries
- **AND** full `SKILL.md` instruction bodies SHALL NOT be stored in the event payload

### Requirement: Skill-backed MCP Events Use The Shared Runtime Event Bridge

The Web adapter SHALL persist and stream skill-backed MCP lifecycle events through the same generic runtime event bridge used by other session runtime events.

#### Scenario: Skill-backed MCP tools are registered

- **WHEN** skill-backed MCP definitions are resolved and registered for a session
- **THEN** the adapter SHALL persist the event before SSE delivery
- **AND** the payload SHALL include skill name, server id, state, exposed tool names, and failure summary when present
