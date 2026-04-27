## 1. Metadata and Config

- [x] 1.1 Add `SkillMcpServerConfig` metadata model under `macaca-skill`.
- [x] 1.2 Parse `metadata.macaca.mcpServers` from `SKILL.md`.
- [ ] 1.3 Add app/global MCP config fallback for skill-backed servers.
- [x] 1.4 Add compatibility registry mapping `@playwright/mcp` / `playwright-mcp` to a Playwright MCP stdio server.

## 2. MCP Runtime

- [x] 2.1 Implement a small MCP stdio client wrapper or reuse existing framework/tooling MCP support if present.
- [x] 2.2 Start/connect MCP server with timeout and cancellation.
- [x] 2.3 Discover MCP tools and expose them as `macaca_framework::tool::Tool`.
- [x] 2.4 Handle subprocess cleanup when app/session stops.
- [x] 2.5 Reject tool name collisions or apply deterministic namespace policy.

## 3. Framework Integration

- [x] 3.1 Extend framework toolkit build path to accept visible `SkillSnapshot`.
- [x] 3.2 Register eligible skill-backed MCP tools for coordinator, planner, reviewer, and workers.
- [x] 3.3 Ensure denied/filtered skills do not register MCP tools.
- [x] 3.4 Ensure tool calls are traced through existing `ToolMiddleware`.

## 4. Status and Trace

- [x] 4.1 Add skill-backed MCP status to app/agent skill status API.
- [x] 4.2 Emit `skill_mcp_resolved`, `skill_mcp_ready`, `skill_mcp_failed`, and `skill_mcp_tools_registered`.
- [x] 4.3 Persist events to EventLog so browser refresh reloads them.
- [x] 4.4 Add frontend rendering if needed for skill MCP lifecycle events.

## 5. Validation

- [x] 5.1 Unit test metadata parsing for `metadata.macaca.mcpServers`.
- [x] 5.2 Unit test compatibility registry resolves `playwright-mcp`.
- [x] 5.3 Integration test status shows Playwright MCP tools when `playwright-mcp` is installed.
- [ ] 5.4 E2E task: model reads `playwright-mcp` skill and successfully calls `browser_navigate`/`browser_snapshot` against `https://example.com`.
- [x] 5.5 Regression test: when `playwright-mcp` binary is missing, skill status reports dependency failure and tools are not registered.
