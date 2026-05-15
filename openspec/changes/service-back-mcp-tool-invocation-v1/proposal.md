# Change: Service-back MCP tool invocation

## Why

Macaca OS currently has MCP server discovery, lifecycle status, and toolkit attachment, but production MCP tool calls can still be executed through host-local framework toolkit clients. This leaves MCP invocation semantics outside the service boundary and conflicts with the microkernel, serviceization, and shell-boundary rules.

## What Changes

- Make `service.mcp/mcp.tool.invoke` the only production path for MCP tool invocation.
- Add a runtime-host owned invocation session registry that binds MCP clients to explicit global, app, session, agent-session, or call lifecycle scope.
- Make framework toolkit MCP tools service-backed adapters over `SystemMcpClient` instead of direct MCP client handlers.
- Extend MCP tool catalog metadata so service invocation can route by descriptor metadata rather than parsing visible tool names.
- Emit sanitized trace and audit evidence for policy, lease, dispatch, completion, failure, and cleanup.
- Preserve `macaca-framework::mcp` as the single low-level MCP protocol implementation while removing it as the Agent OS invocation owner.
- Mark direct toolkit MCP client registration as deprecated compatibility-only for production paths.

## Impact

- Affected specs: `mcp-service`
- Related pending changes: `add-agent-os-mcp-runtime`, `add-driver-skill-mcp-services-v1`, `add-wasm-orchestration-portal`, `add-observable-runtime-events`
- Affected code areas:
  - `macaca/crates/foundation/macaca-proto/src/mcp_service.rs`
  - `macaca/crates/foundation/macaca-proto/src/capability_tool.rs`
  - `macaca/crates/facade/macaca-sdk/src/mcp_client.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/mcp_service_provider.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/mcp_runtime.rs`
  - `macaca/crates/runtime/macaca-framework/src/mcp.rs`
  - `macaca/crates/shells/macaca-web/src/framework_toolkit.rs`
  - `macaca/crates/shells/macaca-web/src/service_tool_adapter.rs`
  - `macaca/crates/shells/macaca-web/src/skill_mcp.rs`

## Governance

This change enforces:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

MCP protocol runtime and tool invocation are non-kernel capabilities. They must be service-owned, policy-governed, traceable, auditable, replaceable, and absent-safe.
