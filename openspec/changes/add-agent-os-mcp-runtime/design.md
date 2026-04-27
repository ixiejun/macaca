# Design: Agent OS Level MCP Runtime

## Context

The current implementation proves that MCP tools can be bridged into framework agents, but it is scoped to visible skills:

- `macaca-framework/src/mcp.rs` contains a real stdio JSON-RPC MCP client and toolkit adapter.
- `macaca-web/src/skill_mcp.rs` resolves MCP servers from skill snapshots and starts them per toolkit.
- `macaca-mcp` contains an older driver-shaped MCP crate whose client is still stub-like.

This creates three risks:

- MCP capabilities are not a first-class Agent OS primitive.
- Stateful MCP servers such as Playwright can leak or conflict without central lifecycle control.
- MCP implementation can diverge across `macaca-framework`, `macaca-web`, and `macaca-mcp`.

AgentScope separates these concerns cleanly:

- Client abstractions own protocol transport.
- Stateful clients reuse one session across tool calls.
- Stateless clients connect per tool call.
- Toolkit registration lists MCP tools, applies filtering, and wraps each MCP tool as a normal callable tool.
- Cleanup is explicit.

Macaca should adopt these boundaries in Rust and keep all application logic generic.

## Goals

- Make MCP a `macaca-framework` toolkit primitive.
- Support stdio, SSE, and streamable HTTP MCP transports.
- Support stateful and stateless lifecycle modes.
- Provide Agent OS level MCP registry and runtime manager.
- Make installed MCP servers available to all applications through policy-controlled toolkit injection.
- Ensure all MCP lifecycle and tool activity is visible in live SSE and persisted EventLog.
- Preserve existing skill-backed MCP compatibility by migrating it onto the OS MCP runtime.

## Non-Goals

- Do not build a marketplace or package installer.
- Do not hardcode any application-specific MCP behavior.
- Do not bypass existing traced framework agent construction.
- Do not change task planning/review semantics.

## Architecture

### Layer 1: `macaca-framework` MCP Protocol Layer

`macaca-framework` SHALL own the real MCP protocol implementation.

Planned types:

```rust
pub enum McpTransportConfig {
    Stdio { command: String, args: Vec<String>, env: BTreeMap<String, String>, cwd: Option<PathBuf> },
    Sse { url: String, headers: BTreeMap<String, String> },
    StreamableHttp { url: String, headers: BTreeMap<String, String> },
}

pub enum McpSessionMode {
    Stateful,
    Stateless,
}

pub enum McpToolNameConflictPolicy {
    Raise,
    Skip,
    Prefix(String),
}
```

Protocol clients:

- `StdioMcpClient`: local subprocess stdio JSON-RPC.
- `HttpMcpClient`: SSE and streamable HTTP transport.
- `StatefulMcpClient`: explicit `connect -> list_tools -> call_tool -> close`.
- `StatelessMcpClient`: each tool call opens a scoped connection and closes it.

The framework layer SHALL convert MCP content into framework `ContentBlock` / `ToolResponse`:

- text -> `TextBlock`
- image -> image content block if available, otherwise JSON fallback
- audio -> audio content block if available, otherwise JSON fallback
- embedded text resource -> text/json block
- unknown content -> JSON text fallback

### Layer 2: Agent OS MCP Registry

Agent OS SHALL maintain MCP server definitions independently from applications.

Sources:

- Global OS config, e.g. `~/.macaca/mcp.yaml`.
- Optional application overlay config.
- Skill metadata discovery, e.g. `metadata.macaca.mcpServers`.
- Compatibility registry for known installed skill packages such as `@playwright/mcp`.

Registry output:

- server id
- transport config
- lifecycle scope
- stateful/stateless mode
- dependency requirements
- tool prefix/namespace
- default policy
- redacted status fields

Application/agent policy controls visibility:

```yaml
tools:
  mcp:
    allowServers: ["playwright"]
    denyServers: []
    allowTools: []
    denyTools: ["browser_install"]
```

### Layer 3: Agent OS MCP Runtime Manager

The runtime manager SHALL own MCP instances and cleanup.

Lifecycle scopes:

- `global`: one instance for the whole backend.
- `app`: one instance per application id.
- `session`: one instance per session id.
- `agent_session`: one instance per `(session_id, agent_name)`.
- `call`: transient instance per tool call.

Responsibilities:

- start/connect MCP server
- list tools
- health check
- reuse instances by scope
- enforce max instances and concurrency policy
- close idle instances
- close instances on session/app/backend shutdown
- record lifecycle status and failure reasons

Playwright policy:

- default lifecycle: `agent_session` or `session`
- stateful: true
- args MUST include `--isolated` or a generated unique `--user-data-dir`
- concurrent sessions MUST NOT share one disk profile

### Layer 4: Toolkit Injection and Trace

`framework_toolkit::build_toolkit` SHALL request eligible MCP tools from the OS MCP runtime and register them into the same framework `Toolkit` as built-in tools.

MCP tool calls SHALL continue through existing tool middleware, so normal events remain:

- `tool_call`
- `tool_result`

Additional lifecycle events:

- `mcp_server_resolved`
- `mcp_server_starting`
- `mcp_server_ready`
- `mcp_server_failed`
- `mcp_tools_registered`
- `mcp_server_closed`

Events SHALL be persisted to EventLog and sent to live SSE when session context exists.

## Migration Plan

### Stage 1: Framework MCP protocol

- Extend `macaca-framework/src/mcp.rs` to support transport config and HTTP MCP clients.
- Add stateful/stateless lifecycle abstractions.
- Add timeout and content conversion coverage.
- Add namespace/collision policy to toolkit registration.

### Stage 2: Agent OS MCP registry

- Add config model and loader for global/app MCP definitions.
- Add status model and status API.
- Add skill metadata and compatibility registry as discovery sources.

### Stage 3: Agent OS MCP runtime manager

- Add scoped instance manager.
- Add lifecycle, health, cleanup, idle TTL, and concurrency isolation.
- Move Playwright isolation handling into runtime policy.

### Stage 4: Framework toolkit integration

- Inject eligible OS MCP tools into all traced framework agents.
- Persist and stream lifecycle events.
- Ensure frontend can render lifecycle events and normal tool traces.

### Stage 5: Consolidation

- Migrate `skill_mcp.rs` to use the OS MCP registry/runtime.
- Deprecate or wrap `macaca-mcp` so it no longer owns a separate protocol implementation.
- Update documentation and tests.

## Compatibility

Existing `playwright-mcp` skill-backed behavior SHALL keep working throughout migration.

Existing application manifests SHALL not be required to declare MCP servers to use globally installed MCP servers. They only need policy if they want to restrict or opt into specific servers.

## Risks

- HTTP MCP compatibility may vary by MCP SDK/server implementation.
- Stateful MCP services can leak child processes without explicit runtime cleanup.
- Tool namespace collisions can confuse agents if conflict policy is unclear.
- Exposing too many global MCP tools can bloat prompts; policy and namespace filtering are required.
- Migrating `skill_mcp.rs` too early can regress the validated Playwright path, so migration must happen after framework and registry tests pass.
