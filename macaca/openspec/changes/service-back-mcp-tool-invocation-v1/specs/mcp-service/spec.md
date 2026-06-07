## ADDED Requirements

### Requirement: MCP Service Shall Own Production Tool Invocation

Macaca SHALL route every production MCP tool invocation through `service.mcp/mcp.tool.invoke`.

#### Scenario: Framework agent invokes MCP tool

- **GIVEN** a framework agent toolkit exposes an MCP tool
- **WHEN** the model calls that tool
- **THEN** the toolkit adapter SHALL call `SystemMcpClient::invoke_tool`
- **AND** the invocation SHALL dispatch through ServiceRuntime to `service.mcp/mcp.tool.invoke`
- **AND** no production framework agent path SHALL call an MCP protocol client directly.

#### Scenario: WASM application invokes MCP tool

- **GIVEN** a WASM application has declared permission to use MCP service capabilities
- **WHEN** the guest invokes `macaca:service/call` for `service.mcp/mcp.tool.invoke`
- **THEN** the host SHALL route the call through the same MCP Service provider used by framework agents
- **AND** it SHALL NOT add a WASM-specific MCP invocation branch.

### Requirement: MCP Service Shall Maintain Invocation Session Registry

The MCP Service SHALL maintain a runtime-host owned invocation session registry for scoped MCP clients and leases.

#### Scenario: Session-scoped client is reused within session

- **GIVEN** an MCP server definition has `session` lifecycle
- **AND** a session invokes two tools from that server
- **WHEN** the MCP Service handles both invocations
- **THEN** it SHALL reuse the same session-scoped MCP client according to policy
- **AND** the registry key SHALL include server id, lifecycle, application id, and session id.

#### Scenario: Agent-session scoped client is isolated per agent

- **GIVEN** an MCP server definition has `agent_session` lifecycle
- **WHEN** two agents in the same application session invoke tools from that server
- **THEN** the MCP Service SHALL isolate the clients by agent name
- **AND** cleanup for one agent-session SHALL NOT close the other agent's client.

#### Scenario: Call-scoped client is closed after invocation

- **GIVEN** an MCP server definition has `call` lifecycle
- **WHEN** a tool invocation completes or fails
- **THEN** the MCP Service SHALL close the MCP client for that call
- **AND** it SHALL record sanitized cleanup status.

### Requirement: MCP Tool Catalog Shall Provide Invocation Routing Metadata

The MCP Service SHALL return sanitized tool descriptors that contain enough metadata for service-backed invocation routing without parsing visible tool names.

#### Scenario: Descriptor maps visible name to backend tool

- **GIVEN** an MCP server exposes backend tool `search`
- **AND** namespace policy exposes it as `mcp_web_search`
- **WHEN** `mcp.tool.catalog` returns descriptors
- **THEN** the descriptor SHALL identify the server id
- **AND** it SHALL identify both the visible tool name and backend MCP tool name
- **AND** it SHALL include lifecycle and resource scope hints.

#### Scenario: Descriptor redacts sensitive config

- **GIVEN** an MCP server definition contains env values, headers, cwd, command args, or credentials
- **WHEN** `mcp.tool.catalog` returns descriptors
- **THEN** descriptors SHALL NOT include raw env values, headers, credentials, or raw provider payloads
- **AND** descriptors SHALL remain safe for model-visible tool metadata.

### Requirement: MCP Tool Invocation Shall Be Policy-Governed And Scoped

The MCP Service SHALL validate trace, application/session/agent scope, descriptor routing metadata, policy, and resource admission before MCP side effects.

#### Scenario: Missing scope is rejected

- **WHEN** `mcp.tool.invoke` is submitted without application id, session id, agent name, or trace id
- **THEN** the MCP Service SHALL reject the call before creating or reusing an MCP client
- **AND** it SHALL return a structured failure reason such as `missing_trace` or `scope_missing`.

#### Scenario: Denied server or tool is rejected

- **GIVEN** policy denies an MCP server or tool
- **WHEN** `mcp.tool.invoke` requests that server or tool
- **THEN** the MCP Service SHALL reject the call before MCP protocol dispatch
- **AND** it SHALL return a structured denied result
- **AND** it SHALL emit sanitized audit evidence.

### Requirement: MCP Invocation Audit Shall Be Replayable And Sanitized

The MCP Service SHALL emit replayable sanitized trace and audit evidence for MCP tool invocation.

#### Scenario: Successful invocation is auditable

- **WHEN** an MCP tool invocation succeeds
- **THEN** audit evidence SHALL include trace id, service id, server id, tool name, lifecycle scope, application id, session id, agent name, policy decision, latency, input hash, output hash, and status
- **AND** it SHALL NOT include raw input, raw output, secrets, prompts, env values, headers, credentials, or unbounded provider payloads.

#### Scenario: Failed invocation is auditable

- **WHEN** MCP protocol dispatch fails, times out, or returns an error
- **THEN** the MCP Service SHALL return a structured failed result
- **AND** audit evidence SHALL include a sanitized error summary and stable reason code
- **AND** raw provider payloads SHALL NOT be copied into logs, snapshots, EventLog rows, or SSE payloads.

### Requirement: Direct MCP Toolkit Clients Shall Be Compatibility-Only

Direct MCP client registration into production framework toolkits SHALL be deprecated in favor of service-backed adapters.

#### Scenario: Production toolkit uses service-backed adapter

- **WHEN** framework toolkit assembly exposes MCP tools for an agent
- **THEN** it SHALL build service-backed tool adapters from MCP Service descriptors
- **AND** it SHALL NOT retain host-local MCP clients as the production invocation path.

#### Scenario: Protocol primitives remain available

- **GIVEN** low-level MCP protocol tests or compatibility code need stdio, SSE, or streamable HTTP clients
- **WHEN** they use `macaca-framework::mcp` protocol primitives
- **THEN** those primitives MAY remain available
- **AND** they SHALL NOT be treated as Agent OS production invocation ownership.
