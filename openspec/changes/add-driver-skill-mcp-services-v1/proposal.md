# Change: Add Driver / Skill / MCP Services v1

## Why

Route C requires non-kernel capabilities to move behind provider-neutral System Service boundaries. Driver runtime, executable skills, and MCP lifecycle/tool exposure are still consumed through direct runtime/facade paths in Web, SDK, CLI, and toolkit assembly, which keeps presentation shells acting as capability hubs and makes trace, policy, and dependency gates harder to enforce.

S6 introduces additive-first Driver, Skill, and MCP services so upper layers can consume tool capability, status, lifecycle, and invocation through `ServiceRuntime` / `SystemFacade` while existing behavior remains compatible during migration.

## What Changes

- Add `DriverService` for driver load/reload, inventory, provider status, tool catalog, tool invocation, service snapshot, and cleanup.
- Add `SkillService` for skill snapshot, executable skill loading, skill tool catalog, skill invocation, status, sanitized service snapshot, and compatibility with existing skill snapshots.
- Add `McpService` for MCP definition registration, dependency probe, tool catalog, toolkit attach metadata, tool invocation, status, cleanup, and skill-backed MCP integration through provider-neutral definitions.
- Add common `CapabilityToolDescriptor` and invocation DTOs for sanitized model-visible tool metadata shared by Driver, Skill, and MCP services.
- Add runtime-host service provider wrappers that adapt existing `DriverRuntime`, `SkillRuntimeFacade` / executable skill loading, and `McpRuntimeFacade` into `SystemService`.
- Add SDK focused clients and SystemFacade accessors so Web, CLI, and framework/toolkit paths call services instead of direct runtimes.
- Migrate Web toolkit, capability catalog, routes, and CLI status paths to service-backed clients while preserving existing YAML applications, `/api/chat/v2`, framework toolkit behavior, skill snapshots, skill-backed MCP, driver reload/status APIs, trace viewer behavior, and no-network tests.
- Keep legacy direct runtime/facade APIs as deprecated, searchable compatibility anchors; do not delete them in S6.
- Require trace context, policy admission, structured logs, sanitized snapshots, and auditable service events for service lifecycle and tool invocation.
- Update Route C allowlist and architecture governance only after dependency gates prove direct edges are removed or explicitly documented as remaining DTO/compat debt.

## Impact

- Affected specs: `driver-service` (new), `skill-service` (new), `mcp-service` (new), `capability-tool-service` (new)
- Affected governance:
  - `macaca/docs/agent-os-microkernel-boundaries.md`
  - `macaca/docs/route-c-serviceization-allowlist.md`
  - `macaca/docs/route-c-architecture-governance.md`
  - `macaca/docs/route-c-regression-matrix.md`
  - `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs`
- Affected code:
  - `macaca/crates/macaca-proto/src/capability_tool.rs`
  - `macaca/crates/macaca-driver/src/service_contract.rs`
  - `macaca/crates/macaca-driver/src/service_adapter.rs`
  - `macaca/crates/macaca-skill/src/service_contract.rs`
  - `macaca/crates/macaca-skill/src/service_adapter.rs`
  - `macaca/crates/macaca-runtime-host/src/driver_service_provider.rs`
  - `macaca/crates/macaca-runtime-host/src/skill_service_provider.rs`
  - `macaca/crates/macaca-runtime-host/src/mcp_service_contract.rs`
  - `macaca/crates/macaca-runtime-host/src/mcp_service_provider.rs`
  - `macaca/crates/macaca-sdk/src/driver_client.rs`
  - `macaca/crates/macaca-sdk/src/skill_client.rs`
  - `macaca/crates/macaca-sdk/src/mcp_client.rs`
  - `macaca/crates/macaca-sdk/src/system_facade.rs`
  - `macaca/crates/macaca-web/src/lib.rs`
  - `macaca/crates/macaca-web/src/state.rs`
  - `macaca/crates/macaca-web/src/service_runtime_client.rs`
  - `macaca/crates/macaca-web/src/framework_toolkit.rs`
  - `macaca/crates/macaca-web/src/skill_mcp.rs`
  - `macaca/crates/macaca-web/src/capability_catalog.rs`
  - `macaca/crates/macaca-web/src/routes.rs`
  - `macaca/crates/macaca-cli/src/commands.rs`

## Non-Goals

- Do not serviceize Application lifecycle, Gateway, Store/Entitlement, Payment, Web3, EVM, or external platform adapters in S6.
- Do not remove existing direct `DriverRuntime`, `SkillRuntimeFacade`, `ExecutableSkillToolSet`, `McpRuntimeFacade`, or direct route/toolkit compatibility paths.
- Do not move driver, skill, or MCP provider logic into kernel.
- Do not create a single generic Tool Service that hides Driver/Skill/MCP lifecycle differences.
- Do not add application-specific, workflow-specific, driver-specific, skill-specific, MCP-server-specific, provider-specific, or business-specific hardcoding.
