## ADDED Requirements

### Requirement: Framework MCP Transport Support

The system SHALL provide `macaca-framework` MCP clients for stdio, SSE, and streamable HTTP transports.

#### Scenario: Stdio MCP server exposes tools
- **GIVEN** a stdio MCP server command is configured
- **WHEN** a framework MCP client connects
- **THEN** it initializes the MCP session
- **AND** lists available tools
- **AND** can call a listed tool
- **AND** closes the subprocess when the client is closed

#### Scenario: SSE MCP server exposes tools
- **GIVEN** an SSE MCP server URL is configured
- **WHEN** a framework MCP client connects
- **THEN** it initializes the MCP session
- **AND** lists available tools
- **AND** can call a listed tool

#### Scenario: Streamable HTTP MCP server exposes tools
- **GIVEN** a streamable HTTP MCP server URL is configured
- **WHEN** a framework MCP client connects
- **THEN** it initializes the MCP session
- **AND** lists available tools
- **AND** can call a listed tool

### Requirement: Stateful and Stateless MCP Lifecycle

The framework SHALL support both stateful MCP sessions and stateless per-call MCP execution.

#### Scenario: Stateful client reuses session
- **GIVEN** a stateful MCP client is connected
- **WHEN** multiple tools are called
- **THEN** the calls reuse the same initialized MCP session until close

#### Scenario: Stateless client closes after call
- **GIVEN** a stateless MCP tool is called
- **WHEN** the tool call finishes
- **THEN** the MCP connection used by that call is closed

### Requirement: MCP Content Conversion

The framework SHALL convert MCP tool result content into framework `ToolResponse` content without losing textual observability.

#### Scenario: Text content is preserved
- **GIVEN** an MCP tool returns text content
- **WHEN** the framework converts the result
- **THEN** the resulting `ToolResponse` contains the same text

#### Scenario: Embedded resource is visible
- **GIVEN** an MCP tool returns an embedded text resource
- **WHEN** the framework converts the result
- **THEN** the resource content is represented as visible text or JSON text

#### Scenario: Unknown content falls back to JSON
- **GIVEN** an MCP tool returns an unsupported content block
- **WHEN** the framework converts the result
- **THEN** the result includes a JSON text fallback rather than silently dropping it

### Requirement: MCP Tool Registration Policy

The framework SHALL register MCP tools into `Toolkit` with deterministic namespace and conflict handling.

#### Scenario: Name collision is rejected
- **GIVEN** a toolkit already has a tool named `browser_navigate`
- **AND** an MCP server exposes `browser_navigate`
- **WHEN** registration uses the raise policy
- **THEN** registration fails with a clear collision error

#### Scenario: Name collision is prefixed
- **GIVEN** an MCP server exposes `search`
- **WHEN** registration uses prefix `mcp_web_`
- **THEN** the toolkit exposes the tool as `mcp_web_search`

### Requirement: Agent OS MCP Registry

The system SHALL maintain an Agent OS level MCP registry that resolves globally installed/configured MCP servers for all applications.

#### Scenario: Global MCP server is visible to an application
- **GIVEN** a global MCP server is configured
- **WHEN** an application toolkit is built
- **THEN** the MCP server is considered for that application
- **AND** app/agent policy determines whether its tools are registered

#### Scenario: Application policy denies a server
- **GIVEN** a global MCP server is configured
- **AND** an application or agent policy denies that server
- **WHEN** the agent toolkit is built
- **THEN** tools from that server are not registered

#### Scenario: Skill metadata imports server definition
- **GIVEN** a visible skill declares `metadata.macaca.mcpServers`
- **WHEN** the MCP registry resolves discovery sources
- **THEN** the declared MCP server can be imported as a registry definition

### Requirement: Agent OS MCP Runtime Manager

The system SHALL manage MCP server instances through an Agent OS runtime manager.

#### Scenario: Session scoped server is reused in one session
- **GIVEN** an MCP server has `session` lifecycle
- **WHEN** two agents in the same session require that server
- **THEN** the runtime reuses the session-scoped instance according to policy

#### Scenario: Agent session scoped server is isolated per agent
- **GIVEN** an MCP server has `agent_session` lifecycle
- **WHEN** two agents in the same session require that server
- **THEN** the runtime creates separate instances per agent

#### Scenario: Session cleanup releases resources
- **GIVEN** a session has active MCP instances
- **WHEN** the session ends or is cleaned up
- **THEN** the runtime closes those MCP instances
- **AND** releases child processes or HTTP sessions

### Requirement: Stateful MCP Concurrency Isolation

The runtime SHALL prevent stateful MCP servers from sharing unsafe state across concurrent sessions.

#### Scenario: Playwright browser profiles do not collide
- **GIVEN** two sessions concurrently use Playwright MCP
- **WHEN** both sessions call browser tools
- **THEN** the runtime starts isolated Playwright instances or profile directories
- **AND** neither session fails with a browser profile already in use error

#### Scenario: Max instances is enforced
- **GIVEN** an MCP server has a configured maximum instance count
- **WHEN** more sessions request the server than allowed
- **THEN** the runtime queues, rejects, or reports capacity failure according to policy
- **AND** the failure is visible in status and trace

### Requirement: MCP Toolkit Injection For All Traced Agents

The system SHALL inject eligible MCP tools into every traced framework agent type through the same toolkit build path.

#### Scenario: Coordinator receives eligible MCP tools
- **GIVEN** an MCP server is eligible for a coordinator
- **WHEN** the coordinator is built
- **THEN** its toolkit includes the server's allowed MCP tools

#### Scenario: Planner receives eligible MCP tools
- **GIVEN** an MCP server is eligible for a planner
- **WHEN** the planner is built
- **THEN** its toolkit includes the server's allowed MCP tools

#### Scenario: Worker receives eligible MCP tools
- **GIVEN** an MCP server is eligible for a worker
- **WHEN** the worker is built
- **THEN** its toolkit includes the server's allowed MCP tools

### Requirement: MCP Lifecycle Trace

The system SHALL persist and stream MCP lifecycle events for sessions that use MCP tools.

#### Scenario: MCP startup is visible live
- **WHEN** an MCP server is started for a session
- **THEN** live SSE includes an MCP lifecycle event
- **AND** EventLog persists the same lifecycle event

#### Scenario: MCP tool registration is visible
- **WHEN** MCP tools are registered into an agent toolkit
- **THEN** EventLog records the server id and exposed tool names

#### Scenario: MCP failure is visible
- **WHEN** MCP server startup or tool discovery fails
- **THEN** EventLog records the failure reason
- **AND** the live UI can show the failure without requiring browser refresh

### Requirement: Skill-Backed MCP Migration

The system SHALL preserve skill-backed MCP behavior while migrating it onto the Agent OS MCP registry and runtime.

#### Scenario: Playwright skill remains functional
- **GIVEN** the installed `playwright-mcp` skill is visible to an agent
- **AND** Playwright MCP is installed
- **WHEN** the agent is built
- **THEN** browser MCP tools remain available
- **AND** tool calls continue to be traced

#### Scenario: Denied skill does not grant MCP tools
- **GIVEN** an agent policy denies a skill that declares MCP metadata
- **WHEN** the MCP registry resolves discovery sources
- **THEN** that skill does not grant tools to the agent

### Requirement: Single MCP Protocol Implementation

The system SHALL avoid multiple competing real MCP protocol implementations.

#### Scenario: Legacy MCP crate does not own protocol behavior
- **GIVEN** `macaca-mcp` remains in the workspace
- **WHEN** it exposes MCP behavior
- **THEN** it delegates to or wraps the framework MCP implementation
- **AND** does not keep a separate stub protocol client as the primary path

### Requirement: MCP Status API

The system SHALL expose MCP registry and runtime status without leaking secrets.

#### Scenario: Status shows ready server
- **GIVEN** an MCP server is configured and reachable
- **WHEN** the status API is requested
- **THEN** the response includes server id, transport type, lifecycle, state, and exposed tools
- **AND** secrets or sensitive environment values are redacted

#### Scenario: Status shows dependency failure
- **GIVEN** an MCP server command is missing
- **WHEN** the status API is requested
- **THEN** the response reports dependency failure
- **AND** no tools are reported as ready
