# Design: Service-backed MCP tool invocation

## Context

Macaca OS has converged on microkernel primitives plus serviceized non-kernel capabilities. MCP belongs to the service layer: it is replaceable, provider-dependent, transport-dependent, and application/tenant policy dependent.

The current runtime already includes:

- global and app MCP registry loading
- skill-backed MCP definition discovery
- stdio, SSE, and streamable HTTP protocol clients
- MCP lifecycle status and cleanup
- `SystemMcpClient` and `McpSystemServiceProvider`
- provider-neutral capability tool descriptors

The missing piece is that production tool invocation can still bypass `service.mcp`. This design closes that gap.

## Goals

- Make `service.mcp/mcp.tool.invoke` the only production MCP invocation path.
- Let framework agents, YAML apps, WASM apps, SDK callers, and future plugins share one invocation path.
- Bind MCP runtime clients to explicit lifecycle scope through a service-owned session registry.
- Preserve MCP tool visibility and skill-backed MCP behavior while changing invocation ownership.
- Emit replayable, sanitized trace/audit evidence.
- Keep kernel, SDK, Web, CLI, and applications free of concrete MCP runtime ownership.

## Non-Goals

- Do not rewrite the MCP JSON-RPC protocol implementation.
- Do not introduce a generic capability-tool super-service in this change.
- Do not add provider-specific branches for particular MCP servers.
- Do not expose raw MCP resources, prompts, inputs, outputs, env, headers, or credentials in observability surfaces.
- Do not create a WASM-specific MCP path.

## Decisions

### Decision: Use a service-owned invocation session registry

Runtime-host will introduce `McpInvocationSessionRegistry`, an internal service-provider component that owns scoped MCP clients and leases.

The registry key includes `server_id`, lifecycle, application id, session id, and agent name. It supports `global`, `app`, `session`, `agent_session`, and `call` semantics. `call` scope closes immediately after invocation; longer scopes are released by explicit cleanup or idle expiry.

This keeps lifecycle ownership in `service.mcp` instead of in Web toolkit assembly.

### Decision: Make catalog descriptors invocation-capable

`mcp.tool.catalog` must return descriptors that can be used for invocation routing. The descriptor or its metadata must carry the provider/server id, visible tool name, backend MCP tool name, lifecycle scope, conflict namespace, and resource scope hints.

The service must not infer routing by parsing visible tool names. Prefixing and collision handling are descriptor metadata, not routing logic.

### Decision: Replace direct toolkit MCP handlers with service-backed adapters

Framework toolkit assembly will create `ServiceMcpToolAdapter` instances from MCP service descriptors. The adapter holds no MCP client. It only converts a model tool call into `McpToolInvokeCommand` with explicit scope and trace.

This matches the Driver and Skill service-backed adapter pattern and keeps Web as a shell adapter.

### Decision: Keep framework MCP as protocol primitives

`macaca-framework::mcp` remains the low-level protocol implementation for stdio, SSE, streamable HTTP, JSON-RPC initialization, tool listing, tool calls, content conversion, and client close behavior.

It is not the Agent OS production invocation owner.

## Data Flow

### Framework agent invocation

1. Toolkit assembly calls `SystemMcpClient::tool_catalog` for the current application/session/agent scope.
2. The MCP Service resolves eligible global, app, and skill-backed definitions.
3. The service probes or refreshes the descriptor index and returns sanitized descriptors.
4. Toolkit assembly wraps descriptors with `ServiceMcpToolAdapter`.
5. The LLM calls the visible tool.
6. The adapter submits `mcp.tool.invoke`.
7. `McpSystemServiceProvider` validates trace, scope, descriptor metadata, policy, and resource gates.
8. `McpInvocationSessionRegistry` obtains the scoped client.
9. Runtime-host calls backend MCP `call_tool`.
10. The service returns `CapabilityToolInvocationResult` and emits sanitized trace/audit.

### WASM application invocation

1. A WASM guest calls `macaca:service/call` for `service.mcp/mcp.tool.invoke`.
2. The WASM host import bridge preserves app id, session id, trace, capability metadata, and payload bounds.
3. ServiceRuntime dispatches to the same MCP service provider.
4. The service uses the same descriptor index, registry, policy, lease, and audit path as framework agents.

## Audit And Diagnostics

MCP invocation audit records include:

- trace id
- service id and command
- server id and tool names
- application id, session id, and agent name
- lifecycle scope
- policy decision
- latency or timestamps
- input hash and output hash
- status and reason code
- sanitized error summary

They exclude raw inputs, raw outputs, secrets, prompts, env values, headers, credentials, raw provider payloads, and unbounded text.

## Migration Plan

1. Extend MCP invocation DTOs and descriptors with server/tool routing metadata.
2. Add runtime-host registry and descriptor index tests.
3. Implement `mcp.tool.invoke` in `McpSystemServiceProvider`.
4. Add `ServiceMcpToolAdapter`.
5. Migrate framework toolkit MCP assembly to service descriptors.
6. Preserve current global/app/skill MCP visibility and lifecycle events.
7. Add session cleanup and idle cleanup tests for registry-owned leases.
8. Add boundary tests rejecting production direct MCP client toolkit registration.
9. Mark direct toolkit MCP registration APIs as deprecated compatibility anchors.

## Risks And Mitigations

- Risk: broad toolkit path migration can regress existing skill-backed MCP behavior.
  Mitigation: keep protocol primitives unchanged and add regression tests for visible skill-backed MCP descriptors plus real service-backed invocation.

- Risk: descriptor metadata can leak sensitive config.
  Mitigation: descriptors include only server id, tool names, lifecycle, and bounded metadata; env, headers, cwd secrets, and raw provider data are never serialized.

- Risk: stateful MCP clients can leak state across sessions.
  Mitigation: lifecycle keying is explicit and `agent_session` / `call` scopes are available for isolation-sensitive tools.

- Risk: service-backed invocation adds latency.
  Mitigation: use scoped client reuse for stateful session/app/global lifecycles and reserve per-call clients for explicit `call` lifecycle.

## Open Questions

- Whether descriptor routing metadata should be first-class fields in `CapabilityToolDescriptor` or reserved `metadata` keys in the initial slice.
- Whether MCP invocation audit should be stored through the existing service-call audit sink only, or also mirrored into session EventLog through the observable runtime events bridge.
- Whether direct `McpToolHandler` should remain public for third-party framework users outside Macaca OS, or be hidden behind compatibility docs.
