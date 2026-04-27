## ADDED Requirements

### Requirement: Skill-Backed MCP Server Metadata

The system SHALL support Macaca-owned skill metadata that declares MCP servers associated with an AgentSkills-compatible `SKILL.md`.

#### Scenario: Parse MCP server metadata
- **GIVEN** a `SKILL.md` contains `metadata.macaca.mcpServers`
- **WHEN** the skill runtime parses the skill
- **THEN** the runtime records the MCP server command, args, transport, and tool namespace policy

#### Scenario: Instruction-only skill remains valid
- **GIVEN** a `SKILL.md` has no MCP server metadata
- **WHEN** the skill runtime parses the skill
- **THEN** the skill remains available as a knowledge skill
- **AND** no MCP server is started solely because the metadata is absent

### Requirement: Compatibility Registry for Installed Skill Packages

The system SHALL provide a generic compatibility registry for known skill-backed MCP packages when a standalone skill lacks machine-readable MCP metadata.

#### Scenario: Resolve Playwright MCP package
- **GIVEN** a visible skill declares install metadata for `@playwright/mcp`
- **AND** the `playwright-mcp` binary is available
- **WHEN** the skill-backed MCP runtime resolves servers
- **THEN** it resolves a Playwright MCP stdio server definition
- **AND** marks the server eligible for startup

#### Scenario: Missing dependency blocks tool registration
- **GIVEN** a visible skill maps to an MCP server
- **AND** the required command is missing
- **WHEN** the skill-backed MCP runtime resolves servers
- **THEN** it reports dependency failure
- **AND** it does not register that server's tools

### Requirement: MCP Tools Registered for Traced Agents

The system SHALL register eligible skill-backed MCP tools into the same framework toolkit used by all traced agents.

#### Scenario: Coordinator can use skill-backed browser tools
- **GIVEN** `playwright-mcp` is visible and its MCP server is ready
- **WHEN** the coordinator agent is built
- **THEN** the framework toolkit includes Playwright browser tools such as `browser_navigate` and `browser_snapshot`

#### Scenario: Worker can use skill-backed browser tools
- **GIVEN** `playwright-mcp` is visible to a worker agent
- **WHEN** the worker agent is built
- **THEN** the framework toolkit includes eligible Playwright browser tools

### Requirement: Skill Policy Controls MCP Tools

The system SHALL apply skill visibility policy to both knowledge prompt injection and MCP tool registration.

#### Scenario: Denied skill hides tools
- **GIVEN** an agent policy denies `playwright-mcp`
- **WHEN** the agent toolkit is built
- **THEN** the `playwright-mcp` knowledge skill is not visible
- **AND** its MCP tools are not registered

#### Scenario: Filtered skill hides tools
- **GIVEN** `playwright-mcp` is filtered because dependencies are missing
- **WHEN** the agent toolkit is built
- **THEN** its MCP tools are not registered

### Requirement: MCP Lifecycle Observability

The system SHALL persist and stream lifecycle events for skill-backed MCP servers.

#### Scenario: Server startup is traced
- **WHEN** the runtime starts a skill-backed MCP server
- **THEN** EventLog records `skill_mcp_starting`
- **AND** records either `skill_mcp_ready` or `skill_mcp_failed`

#### Scenario: Registered tools are visible
- **WHEN** a skill-backed MCP server exposes tools
- **THEN** EventLog records `skill_mcp_tools_registered`
- **AND** the status API lists exposed tool names

### Requirement: Playwright MCP End-to-End Validation

The system SHALL support the installed `playwright-mcp` skill as an end-to-end compatibility target.

#### Scenario: Agent uses Playwright MCP skill to navigate
- **GIVEN** `/Users/quantum/.macaca/skills/playwright-mcp/SKILL.md` is installed
- **AND** `playwright-mcp` is available on PATH
- **WHEN** a user asks an agent to use the Playwright MCP skill to open `https://example.com`
- **THEN** the agent reads the skill instructions
- **AND** calls `browser_navigate`
- **AND** obtains page content or title through a Playwright MCP tool
