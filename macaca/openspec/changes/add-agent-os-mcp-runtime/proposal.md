# Change: Add Agent OS level MCP runtime

## Why

Macaca currently has a working skill-backed MCP bridge for cases such as `playwright-mcp`, but MCP is still treated as an agent skill side effect rather than an Agent OS primitive. This limits reuse: every application should be able to use installed MCP services through the same framework toolkit, policy, lifecycle, and trace pipeline.

AgentScope demonstrates the right boundary: MCP is a toolkit-level capability with client transports, stateful/stateless lifecycle, tool wrapping, and cleanup. Macaca needs the same capability at the `macaca-framework` layer, plus an Agent OS registry/runtime so installed MCP services are available to all applications without app-specific hardcoding.

## What Changes

- Add complete MCP protocol support to `macaca-framework` for stdio, SSE, and streamable HTTP transports.
- Add stateful/stateless MCP client lifecycle abstractions inspired by AgentScope, including explicit connect/list/call/close behavior.
- Add MCP content conversion for text, image, audio, embedded resources, and unknown fallback blocks.
- Add toolkit registration options for MCP tools, including namespace/collision policy and timeout handling.
- Add an Agent OS MCP registry for globally installed/configured MCP servers, with app/agent policy overlays.
- Add an Agent OS MCP runtime manager for startup, connection reuse, health checks, concurrency isolation, and cleanup.
- Integrate OS-level MCP tools into every traced framework agent toolkit, not only skill-visible agents.
- Preserve skill-backed MCP support as a discovery/import path, then migrate it to the OS MCP registry/runtime.
- Add persistent and live trace events for MCP lifecycle and ensure ordinary MCP tool calls continue through existing `tool_call` / `tool_result` trace.
- Deprecate or wrap the older `macaca-mcp` stub path so there is only one real MCP protocol implementation.

## Impact

- Affected specs: `agent-os-mcp-runtime`
- Related active changes:
  - `add-skill-backed-mcp-runtime`
  - `add-agent-skills-runtime`
- Affected code:
  - `macaca/crates/macaca-framework/src/mcp.rs`
  - `macaca/crates/macaca-framework/src/tool.rs`
  - `macaca/crates/macaca-web/src/framework_toolkit.rs`
  - `macaca/crates/macaca-web/src/skill_mcp.rs`
  - `macaca/crates/macaca-web/src/routes.rs`
  - `macaca/crates/macaca-web/src/state.rs`
  - `macaca/crates/macaca-mcp/*`
  - skill/app config structs under `macaca-app`, `macaca-skill`, and SDK crates
  - frontend trace/status UI if lifecycle event rendering requires additions

## Non-Goals

- No MCP marketplace, search, or installer in this change.
- No automatic npm/brew/go/uv package installation.
- No application-specific MCP behavior for FULLSTACK-AUTODEV, NEWSROOM-AUTOWRITER, or any other app.
- No rewrite of PlanLoop/WorkerLoop orchestration semantics.
- No removal of AgentSkills knowledge prompt injection.

## Rollout

This is intentionally a staged architecture change. The implementation SHALL preserve existing `playwright-mcp` behavior at each stage and add tests before each migration step.
