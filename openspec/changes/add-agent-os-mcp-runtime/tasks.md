## 1. Framework MCP Protocol Layer

- [x] 1.1 Add `McpTransportConfig` for `stdio`, `sse`, and `streamable_http`.
- [x] 1.2 Add `McpSessionMode` for `stateful` and `stateless` clients.
- [x] 1.3 Refactor existing `StdioMcpClient` behind the new transport abstraction without changing current stdio behavior.
- [x] 1.4 Implement HTTP MCP client support for SSE transport.
- [x] 1.5 Implement HTTP MCP client support for streamable HTTP transport.
- [x] 1.6 Add connect/list/call/read timeout configuration.
- [x] 1.7 Expand MCP content conversion for text, image, audio, embedded resources, and unknown fallback.
- [x] 1.8 Add namespace and collision policy to MCP toolkit registration.
- [x] 1.9 Add framework unit tests for stdio list/call/close behavior.
- [x] 1.10 Add framework integration tests for SSE and streamable HTTP test MCP servers.

## 2. Agent OS MCP Registry

- [x] 2.1 Add OS-level MCP server config models.
- [x] 2.2 Add global MCP config loader.
- [x] 2.3 Add optional application-level MCP config overlay.
- [x] 2.4 Add agent/application MCP policy models for allow/deny servers and tools.
- [x] 2.5 Add registry resolution from explicit config.
- [x] 2.6 Add registry discovery import from `metadata.macaca.mcpServers`.
- [x] 2.7 Move Playwright compatibility mapping from skill runtime into registry discovery.
- [x] 2.8 Add dependency checks and secret/env redaction.
- [x] 2.9 Add status API for resolved MCP servers and exposed tools.
- [x] 2.10 Add tests for config parsing, policy filtering, dependency failure, and redacted status.

## 3. Agent OS MCP Runtime Manager

- [x] 3.1 Add `McpRuntimeManager` owned by web/app state.
- [x] 3.2 Add scoped runtime keys for `global`, `app`, `session`, `agent_session`, and `call`.
- [x] 3.3 Implement instance startup and connection reuse by scope.
- [x] 3.4 Implement health check and failure caching.
- [x] 3.5 Implement reference counting or ownership tracking for toolkit/session use.
- [x] 3.6 Implement idle TTL cleanup.
- [x] 3.7 Implement session/app/backend shutdown cleanup.
- [x] 3.8 Add concurrency isolation for stateful MCP servers.
- [x] 3.9 Add Playwright default isolation policy using `--isolated` or unique `--user-data-dir`.
- [x] 3.10 Add leak regression tests that verify MCP subprocesses are closed.

## 4. Toolkit and Trace Integration

- [x] 4.1 Update `framework_toolkit::build_toolkit` to request eligible MCP tools from the OS MCP runtime.
- [x] 4.2 Ensure coordinator, planner, reviewer, and worker agents all use the same MCP injection path.
- [x] 4.3 Ensure MCP tools continue through existing `ToolMiddleware` and produce normal `tool_call` / `tool_result`.
- [x] 4.4 Emit and persist `mcp_server_resolved`.
- [x] 4.5 Emit and persist `mcp_server_starting`.
- [x] 4.6 Emit and persist `mcp_server_ready`.
- [x] 4.7 Emit and persist `mcp_server_failed`.
- [x] 4.8 Emit and persist `mcp_tools_registered`.
- [x] 4.9 Emit and persist `mcp_server_closed`.
- [x] 4.10 Verify browser refresh restores MCP lifecycle and tool events without duplication.

## 5. Skill MCP Migration

- [x] 5.1 Keep `SKILL.md` knowledge prompt injection unchanged.
- [x] 5.2 Change `skill_mcp.rs` to resolve MCP servers through the OS MCP registry/runtime instead of directly spawning clients.
- [x] 5.3 Preserve existing `skill_mcp_*` events as backward-compatible aliases or map them to the new `mcp_*` events.
- [x] 5.4 Preserve installed `playwright-mcp` skill behavior.
- [x] 5.5 Update skill status API to show both knowledge skill visibility and MCP runtime readiness.
- [x] 5.6 Update `agent-skills-runtime.md` with final Skill vs MCP responsibility boundary.

## 6. `macaca-mcp` Consolidation

- [ ] 6.1 Audit current `macaca-mcp` crate call sites.
- [ ] 6.2 Decide whether `macaca-mcp` becomes a thin wrapper/re-export or is marked deprecated.
- [ ] 6.3 Remove or replace stub protocol behavior so there is only one real MCP protocol implementation.
- [ ] 6.4 Add compile-time deprecation comments/docs for any retained compatibility API.
- [ ] 6.5 Add tests proving callers use the framework MCP implementation.

## 7. End-to-End Validation

- [ ] 7.1 Run `cargo check -p macaca-framework -p macaca-web -p macaca-mcp`.
- [ ] 7.2 Run framework MCP unit/integration tests.
- [ ] 7.3 Run web MCP registry/runtime tests.
- [ ] 7.4 Start backend and verify MCP status API for a globally configured Playwright server.
- [ ] 7.5 Send a direct task that uses `browser_navigate` and `browser_snapshot` against `https://example.com`.
- [ ] 7.6 Run two concurrent sessions using Playwright and verify no browser profile contention.
- [ ] 7.7 Verify session end releases MCP resources.
- [ ] 7.8 Verify refresh reloads historical MCP events and continues live incremental events.
- [ ] 7.9 Run GitNexus `detect_changes` before commit.
