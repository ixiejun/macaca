## 1. Specification

- [x] 1.1 Review governance docs and existing MCP service/runtime specs.
- [x] 1.2 Create `service-back-mcp-tool-invocation-v1` proposal, design, tasks, and MCP Service delta spec.
- [x] 1.3 Validate with `openspec validate service-back-mcp-tool-invocation-v1 --strict`.

## 2. Service Contracts

- [x] 2.1 Extend MCP invocation DTOs so a tool invocation carries server/provider routing metadata without parsing visible names.
- [x] 2.2 Extend MCP tool catalog descriptors with backend MCP tool name, lifecycle scope, resource scope, and conflict namespace metadata.
- [x] 2.3 Add tests proving descriptors remain sanitized and exclude env, headers, credentials, and raw provider payloads.

## 3. Runtime-Host Invocation Registry

- [x] 3.1 Add `McpInvocationSessionRegistry` for scoped MCP client lease ownership.
- [x] 3.2 Add `McpToolDescriptorIndex` mapping exposed tool names to backend server/tool metadata.
- [x] 3.3 Implement global, app, session, agent-session, and call lifecycle behavior.
- [x] 3.4 Add cleanup and idle-expiry behavior for registry-owned leases.
- [x] 3.5 Add unit tests for keying, reuse, isolation, cleanup, and missing dependency states.

## 4. MCP Service Invocation

- [x] 4.1 Replace the unsupported `mcp.tool.invoke` branch with real service-backed dispatch.
- [x] 4.2 Enforce trace, scope, policy, resource, and descriptor validation before side effects.
- [x] 4.3 Convert MCP protocol results into `CapabilityToolInvocationResult`.
- [x] 4.4 Emit sanitized trace/audit events for accepted, denied, dispatch started, completed, failed, and cleanup stages.
- [x] 4.5 Add tests for success, policy denial, unknown tool, missing server, missing binary, client failure, timeout, and unavailable provider.

## 5. Framework Toolkit Migration

- [x] 5.1 Add `ServiceMcpToolAdapter` in the Web/framework shell adapter layer.
- [x] 5.2 Migrate framework toolkit MCP assembly from direct MCP client registration to MCP service catalog descriptors.
- [x] 5.3 Preserve global, app overlay, and skill-backed MCP visibility.
- [x] 5.4 Preserve existing MCP lifecycle EventLog/SSE events while moving invocation through `service.mcp`.
- [x] 5.5 Add boundary tests proving production toolkit assembly no longer registers direct MCP client handlers.

## 6. WASM And SDK Verification

- [x] 6.1 Add SDK tests for `SystemMcpClient::invoke_tool` against a service-backed provider.
- [x] 6.2 Add WASM host-import/service-call regression coverage for `service.mcp/mcp.tool.invoke`.
- [x] 6.3 Prove framework agent and WASM app calls share the same service-backed invocation path.

## 7. Compatibility And Deprecation

- [x] 7.1 Keep `macaca-framework::mcp` protocol primitives as the single low-level MCP implementation.
- [x] 7.2 Mark direct toolkit MCP client registration APIs as deprecated compatibility anchors for Macaca OS production use.
- [x] 7.3 Add grep or boundary tests preventing new production use of direct MCP client toolkit registration.

## 8. Validation

- [x] 8.1 Run `openspec validate service-back-mcp-tool-invocation-v1 --strict`.
- [x] 8.2 Run targeted MCP runtime-host tests.
- [x] 8.3 Run targeted SDK MCP client tests.
- [x] 8.4 Run targeted Web framework toolkit and service adapter tests.
- [x] 8.5 Run WASM service-call regression tests.
- [x] 8.6 Run dependency-boundary checks required by Macaca OS governance.
