# Service-Backed MCP Tool Invocation Design

Date: 2026-05-15

## Context

Macaca OS already has a real MCP protocol client, an Agent OS MCP registry, skill-backed MCP discovery, runtime lifecycle status, and a Route C `service.mcp` boundary. The remaining architectural gap is invocation ownership: production MCP tool calls can still be attached directly to a framework `Toolkit` through host-local MCP clients, while `service.mcp/mcp.tool.invoke` is only a placeholder.

This violates the stable direction defined by:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

MCP protocol runtime, tool catalog, tool invocation, lifecycle cleanup, policy, trace, and audit are non-kernel capabilities. They must be service-owned, replaceable, scoped, and auditable. Web and framework toolkit assembly may adapt model-visible tools, but they must not own MCP invocation semantics.

## Decision

Macaca OS shall make `service.mcp/mcp.tool.invoke` the only production MCP tool invocation path.

All callers, including framework agents, YAML application adapters, WASM applications, SDK callers, and future plugin or gateway adapters, must invoke MCP tools through:

```text
Caller
  -> SystemMcpClient or generic service.call
  -> ServiceRuntime
  -> service.mcp / mcp.tool.invoke
  -> McpInvocationSessionRegistry
  -> scoped MCP lease/client
  -> MCP call_tool
  -> sanitized result, trace, and audit
```

The current direct toolkit registration path remains only as a deprecated compatibility anchor for low-level protocol tests or temporary migration. New production code must not attach host-local MCP clients directly to agent toolkits.

## Ownership

The kernel owns trace-required service dispatch, service identity, policy facade hooks, and audit bus invariants. It does not own MCP protocol clients, server definitions, tool mapping, or invocation behavior.

`macaca-proto` owns provider-neutral MCP commands, results, capability tool descriptors, invocation scope, and audit-safe result shapes.

`macaca-sdk` owns `SystemMcpClient` as a focused facade over the generic service client. It must not construct MCP runtime providers, clients, registries, sessions, or toolkits.

`macaca-runtime-host` owns MCP server definition resolution, protocol transport adaptation, lifecycle leases, session-scoped invocation registry, service provider implementation, policy and resource admission, cleanup, and sanitized diagnostics.

`macaca-framework::mcp` owns protocol primitives such as stdio, SSE, streamable HTTP clients, MCP content conversion, and JSON-RPC behavior. It does not own Agent OS service semantics.

`macaca-web`, CLI, and gateways are shells. They may convert sanitized descriptors into model-visible tools and render diagnostics, but they must call `SystemMcpClient` for invocation.

Applications own orchestration intent and UI behavior. They may request MCP service use only through declared capabilities and service boundaries.

## Components

### McpInvocationSessionRegistry

`McpInvocationSessionRegistry` is a runtime-host internal component. It owns active MCP invocation sessions keyed by:

```text
server_id
lifecycle
application_id
session_id
agent_name
```

The registry creates or reuses MCP clients according to lifecycle:

- `global`: one shared runtime instance.
- `app`: one instance per application.
- `session`: one instance per application session.
- `agent_session`: one instance per application session and agent.
- `call`: one short-lived instance per invocation, closed immediately after completion.

The registry stores only bounded runtime metadata: lease key, state, ref count or last-used timestamp, exposed backend tool names, sanitized failure reason, and cleanup status. It must not store raw env values, headers, credentials, tool input, or raw tool output.

### McpToolDescriptorIndex

`McpToolDescriptorIndex` is the service-owned catalog index that maps model-visible tools to backend MCP calls.

It records:

- `server_id`
- exposed tool name after prefix/conflict policy
- backend MCP tool name before prefixing
- lifecycle and resource scope
- descriptor metadata safe for model visibility
- source metadata such as global, app, skill, or compatibility definition source

This index prevents callers from guessing a server or backend tool from the visible name. Descriptor metadata, not string parsing, is the authority for invocation routing.

### ServiceMcpToolAdapter

`ServiceMcpToolAdapter` is a shell/framework adapter analogous to existing Driver and Skill service-backed tool adapters.

It owns no MCP client and no lifecycle state. It holds a `CapabilityToolDescriptor`, `SystemMcpClient`, and explicit application/session/agent scope. On execution it builds `McpToolInvokeCommand` and calls `SystemMcpClient::invoke_tool`.

### McpInvocationAudit

MCP invocation audit records are sanitized mementos. They contain:

- trace id
- service id
- server id
- exposed and backend tool names
- application id, session id, agent name
- lifecycle scope
- policy decision
- start/completion/failure timestamps or latency
- input hash and output hash
- status and stable reason code
- sanitized error summary

They never contain raw input, raw output, prompts, secrets, env, headers, credentials, raw provider payloads, or unbounded text.

## Data Flow

### Framework Agent Tool Call

1. Toolkit assembly asks the MCP Service for a scoped tool catalog.
2. The MCP Service resolves global, app, and skill-backed definitions, probes eligible servers, and returns sanitized `CapabilityToolDescriptor` entries.
3. Web/framework assembly wraps each descriptor with `ServiceMcpToolAdapter`.
4. The LLM calls the visible tool name.
5. `ServiceMcpToolAdapter` submits `mcp.tool.invoke` with trace and explicit scope.
6. The MCP Service validates policy and descriptor metadata.
7. `McpInvocationSessionRegistry` obtains the scoped lease/client.
8. Runtime-host calls MCP `call_tool` through the protocol client.
9. The service returns `CapabilityToolInvocationResult` and emits sanitized trace/audit.

### WASM Application Service Call

1. A WASM guest invokes `macaca:service/call` for `service.mcp/mcp.tool.invoke`.
2. The WASM host import bridge preserves app id, session id, trace, service id, operation, payload bounds, and capability metadata.
3. ServiceRuntime dispatches to the same MCP Service provider used by framework tools.
4. Invocation uses the same descriptor index, registry, lease, policy, and audit path.

WASM must not receive a special MCP path.

## Policy, Resource, And Entitlement Gates

Every `mcp.tool.invoke` call must require:

- non-empty trace context
- explicit application id, session id, and agent name
- declared MCP capability or permission hints
- server and tool allow/deny policy
- payload size limits before side effects
- resource and lifecycle admission before client creation
- entitlement and metering hooks where available

Denials return structured `denied`, `unavailable`, `unsupported`, or `failed` states. They must not crash, hang, silently fall back, or report fake success.

## Migration

1. Extend MCP service DTOs so invocation can identify the server and backend tool without name guessing.
2. Add `McpInvocationSessionRegistry` and `McpToolDescriptorIndex` inside runtime-host.
3. Implement real `McpSystemServiceProvider::mcp.tool.invoke`.
4. Add `ServiceMcpToolAdapter` in Web/framework adapter code.
5. Change framework toolkit assembly to use MCP service descriptors instead of direct MCP client registration.
6. Keep `macaca-framework::mcp` protocol primitives as the single low-level protocol implementation.
7. Mark direct `McpRuntimeFacade::register_definitions(... Toolkit ...)` production use as deprecated compatibility-only.
8. Preserve skill-backed MCP and global/app MCP visibility while moving invocation through the service path.
9. Add boundary tests that fail if production toolkit assembly registers MCP clients directly.

## Rejections

The following designs are rejected:

- Keeping framework MCP invocation as direct host-local client calls.
- Making Web the owner of MCP invocation routing, policy, or lifecycle.
- Adding a WASM-specific MCP invocation branch.
- Letting `mcp.tool.invoke` infer server identity by parsing visible tool names.
- Storing raw MCP inputs or outputs in audit/event surfaces.
- Moving MCP protocol runtime into the kernel.
- Creating provider-specific or application-specific branches for known MCP servers.
- Building a generic capability tool super-service before MCP invocation itself is service-owned.

## Acceptance Gates

- Framework agents invoke MCP tools through `service.mcp/mcp.tool.invoke`.
- WASM applications invoke MCP tools through the same `service.mcp/mcp.tool.invoke` path.
- MCP invocation trace/audit can replay catalog resolution, policy decision, lease acquisition, dispatch, result, and cleanup.
- Session cleanup releases session and agent-session scoped MCP resources.
- Missing server, missing binary, denied policy, unknown tool, client crash, and timeout all return structured states.
- Web and CLI contain only shell adapters and diagnostics for MCP invocation.
- No OS-layer code hardcodes app names, workflow names, provider names, business domains, or specific MCP server product names in control flow.
- Logs, snapshots, EventLog rows, SSE payloads, and audit records remain bounded and sanitized.
