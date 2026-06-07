# S6 Driver / Skill / MCP 服务化与模块化实施计划

## Scope

Implement S6 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`: move Driver, Skill, and MCP lifecycle / inventory / tool exposure / tool invocation behind service boundaries compatible with `ServiceRuntime` and `SystemFacade`.

S6 covers:

- Driver Service: load/reload, inventory, provider status, tool catalog, tool invocation, cleanup.
- Skill Service: skill snapshot, executable skill loading, skill tool catalog, skill invocation, status, sanitized snapshot.
- MCP Service: definition registration, probe/status, toolkit attach metadata, lifecycle cleanup, skill-backed MCP integration through provider-neutral definitions.
- SDK focused clients for upper consumers.
- Web toolkit migration from direct `DriverRuntime`, `SkillRuntimeFacade`, and `McpRuntimeFacade` calls to service-backed clients.
- Deprecated compatibility anchors for old direct runtime/registry APIs.

S6 does not cover:

- Application Service lifecycle. That belongs to S7.
- Gateway Service and external platform adapters. That belongs to S8.
- Store / Entitlement full serviceization. That belongs to S9.
- Payment / Web3 / EVM phases.
- Removing legacy wrappers before dependency gates prove all callers migrated.

## Required Governance Inputs

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/route-c-regression-matrix.md`
- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-08-s1-service-runtime-v1-plan.md`
- `docs/superpowers/plans/2026-05-08-s3-sdk-system-facade-convergence-plan.md`
- `docs/superpowers/plans/2026-05-08-s5-llm-memory-context-serviceization-plan.md`
- `docs/superpowers/plans/2026-05-09-s6-driver-skill-mcp-serviceization-brainstorm.md`

## Architecture Decision

Use three provider-neutral services plus a shared tool capability DTO:

- `DriverService`: owns driver runtime lifecycle, driver inventory, driver tool catalog, and driver tool invocation.
- `SkillService`: owns skill discovery/snapshot, executable skill catalog, skill invocation, and skill capability status.
- `McpService`: owns MCP definition registration, dependency probe, session/lifecycle status, and MCP tool attachment/invocation.
- `CapabilityToolDescriptor`: common DTO for model-visible tool metadata, but ownership and invocation remain with the service that produced it.

Design patterns:

- Facade: each capability exposes one focused service boundary.
- Adapter / Bridge: existing `DriverRuntime`, `SkillRuntimeFacade`, `ExecutableSkillToolSet`, and `McpRuntimeFacade` are adapted into provider-neutral services.
- Abstract Factory: built-in, plugin, package-installed, and future remote providers are created through `ServiceProviderFactory`.
- Command: all lifecycle, inventory, status, attach, and invocation actions are typed commands before `ServiceCommand` payload conversion.
- Strategy: driver provider selection, skill source selection, MCP definition source selection, tool conflict policy, and resource scope policy remain replaceable.
- Resource Manager / Mediator: command scope and lifecycle leases coordinate driver process, workspace, browser, and MCP session resources.
- State: driver/MCP lifecycle and skill snapshot readiness are explicit status values.
- Null Object: missing provider or disabled service returns structured unavailable.
- Observer: service call, provider lifecycle, tool invocation, dependency missing, and policy denial emit structured logs/events.
- Memento: status/snapshot commands expose sanitized inventories without dumping secrets, tool payloads, env, headers, or full files.
- Specification: command constructors and provider admission validate trace, scope, permission hints, tool allowlists, dependency readiness, and resource scope.

Rejected alternatives:

- Descriptor-only S6: rejected because Route C requires real serviceization and Web/Kernel dependency debt reduction.
- Single generic Tool Service: rejected because it hides Driver/Skill/MCP lifecycle differences and would become a new macro-service.
- MCP-only migration: rejected because skill-backed MCP and driver tools still leave Web as the aggregation hub.
- Move provider logic into kernel: rejected by microkernel boundaries.

## Proposed OpenSpec Change

Expected change id:

- `add-driver-skill-mcp-services-v1`

Expected artifacts:

- `openspec/changes/add-driver-skill-mcp-services-v1/proposal.md`
- `openspec/changes/add-driver-skill-mcp-services-v1/design.md`
- `openspec/changes/add-driver-skill-mcp-services-v1/tasks.md`
- `openspec/changes/add-driver-skill-mcp-services-v1/specs/driver-service/spec.md`
- `openspec/changes/add-driver-skill-mcp-services-v1/specs/skill-service/spec.md`
- `openspec/changes/add-driver-skill-mcp-services-v1/specs/mcp-service/spec.md`
- `openspec/changes/add-driver-skill-mcp-services-v1/specs/capability-tool-service/spec.md`

The proposal should state:

- S6 is additive-first and preserves existing YAML applications, `/api/chat/v2`, framework toolkit, skill snapshots, skill-backed MCP, driver reload/status APIs, trace viewer, and existing no-network tests.
- Driver/Skill/MCP direct runtime APIs remain as deprecated or provider-internal compatibility anchors until all consumers migrate.
- Service calls require trace context and policy admission through `ServiceRuntime` or equivalent SDK client boundary.
- Tool descriptors and snapshots must be sanitized by default and must not expose env, headers, API keys, workspace secrets, or full tool payloads.
- No provider/app/driver/MCP server/skill name can be hardcoded into service control flow.

## Implementation Slices

### Slice S6.1: Impact And Boundary Audit

Files to inspect before editing:

- `macaca/crates/macaca-driver/src/service_adapter.rs`
- `macaca/crates/macaca-driver/src/runtime.rs`
- `macaca/crates/macaca-driver/src/registry.rs`
- `macaca/crates/macaca-driver/src/command.rs`
- `macaca/crates/macaca-skill/src/service_adapter.rs`
- `macaca/crates/macaca-skill/src/runtime.rs`
- `macaca/crates/macaca-skill/src/facade.rs`
- `macaca/crates/macaca-skill/src/request.rs`
- `macaca/crates/macaca-runtime-host/src/mcp_runtime.rs`
- `macaca/crates/macaca-runtime-host/src/service_runtime.rs`
- `macaca/crates/macaca-sdk/src/service_client.rs`
- `macaca/crates/macaca-web/src/lib.rs`
- `macaca/crates/macaca-web/src/state.rs`
- `macaca/crates/macaca-web/src/framework_toolkit.rs`
- `macaca/crates/macaca-web/src/skill_mcp.rs`
- `macaca/crates/macaca-web/src/capability_catalog.rs`
- `macaca/crates/macaca-web/src/routes.rs`
- `macaca/crates/macaca-cli/src/commands.rs`
- `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs`

Required actions:

1. Run GitNexus impact before modifying existing structs/functions/traits.
2. Classify every direct Driver/Skill/MCP path as service provider, SDK client, Web adapter, CLI adapter, kernel compat, test, or provider-internal.
3. Identify which allowlist rows can be removed in S6 and which remain until S8/S12.
4. Confirm no file touched by S6 would exceed 500 lines; split before adding large logic.
5. Warn before editing HIGH or CRITICAL impact symbols.

### Slice S6.2: Common Capability Tool DTO

Files:

- Add: `macaca/crates/macaca-proto/src/capability_tool.rs` or a domain-local equivalent if proto ownership is too broad.
- Update: `macaca/crates/macaca-proto/src/lib.rs` if proto DTO is used.

Behavior:

- Define sanitized tool metadata:
  - service id
  - provider id
  - capability id
  - tool name
  - description
  - JSON schema
  - origin kind: driver / skill / mcp
  - required permission hints
  - resource scope hints
  - conflict namespace / display name
- Define invocation DTO:
  - trace
  - application id
  - session id
  - agent name
  - tool name
  - JSON input
  - policy hints
  - resource scope

Rules:

- No env, headers, secret values, full command lines with secrets, or provider-specific credentials.
- Detailed English comments explaining why metadata is safe and provider-neutral.
- This DTO is common metadata only; service ownership remains separate.

### Slice S6.3: Driver Service Contract

Files:

- Add or update: `macaca/crates/macaca-driver/src/service_contract.rs`
- Update: `macaca/crates/macaca-driver/src/service_adapter.rs`
- Update: `macaca/crates/macaca-driver/src/lib.rs`

Behavior:

- Define constants:
  - `DRIVER_SERVICE_ID`
  - `driver.load`
  - `driver.reload`
  - `driver.inventory`
  - `driver.tool.catalog`
  - `driver.tool.invoke`
  - `driver.status`
  - `driver.cleanup`
- Define typed commands and results:
  - `DriverLoadServiceCommand`
  - `DriverInventoryCommand`
  - `DriverToolCatalogCommand`
  - `DriverToolInvokeCommand`
  - `DriverStatusCommand`
  - `DriverServiceSnapshotCommand`
- Extend descriptor capabilities:
  - execute
  - inventory
  - reload
  - status
  - cleanup

Rules:

- Commands require explicit trace and app/session/agent scope where applicable.
- Tool invoke must carry tool name and input but not assume a specific driver implementation.
- Load/reload must report structured loaded/failed entries.
- Logs must include trace id, command, counts, and status, not raw secrets.

### Slice S6.4: Skill Service Contract

Files:

- Add or update: `macaca/crates/macaca-skill/src/service_contract.rs`
- Update: `macaca/crates/macaca-skill/src/service_adapter.rs`
- Update: `macaca/crates/macaca-skill/src/lib.rs`

Behavior:

- Define constants:
  - `SKILL_SERVICE_ID`
  - `skill.snapshot`
  - `skill.executable.load`
  - `skill.tool.catalog`
  - `skill.tool.invoke`
  - `skill.status`
  - `skill.service.snapshot`
- Define typed commands and results:
  - `SkillSnapshotServiceCommand`
  - `SkillExecutableLoadCommand`
  - `SkillToolCatalogCommand`
  - `SkillToolInvokeCommand`
  - `SkillStatusCommand`
  - `SkillServiceSnapshotCommand`
- Reuse `SkillSnapshotRequest`, `SkillSnapshot`, `SkillRegistrySnapshot`, and executable skill definition types where provider-neutral.

Rules:

- Snapshot command must carry app/session/agent scope and source hints, not direct Web state.
- Skill tool invocation must be traced and policy-checkable.
- Encrypted skill / paid package paths only expose entitlement readiness hooks in S6; full entitlement service belongs to S9.
- Snapshot/result payloads must not dump full `SKILL.md` bodies unless explicitly requested by trusted context-provider flows.

### Slice S6.5: MCP Service Contract

Files:

- Add: `macaca/crates/macaca-runtime-host/src/mcp_service_contract.rs` or split under `mcp_service/`.
- Update: `macaca/crates/macaca-runtime-host/src/lib.rs`

Behavior:

- Define constants:
  - `MCP_SERVICE_ID`
  - `mcp.register`
  - `mcp.probe`
  - `mcp.tool.catalog`
  - `mcp.tool.attach`
  - `mcp.tool.invoke`
  - `mcp.status`
  - `mcp.cleanup`
- Define typed commands and results around existing `McpServerDefinition`, `McpRuntimeContext`, `McpToolPolicy`, and `McpRuntimeStatus`.
- Define service snapshot:
  - registered definitions count
  - ready/failed/dependency_missing counts
  - exposed tool counts
  - lifecycle scopes
  - sanitized failure reasons

Rules:

- MCP service owns protocol lifecycle; Skill service only provides skill snapshot and MCP definition source.
- Resource scope must distinguish global/app/session/agent-session/call.
- Tool attachment must report conflicts and applied prefixes.
- No headers/env secrets in status/snapshot/event payloads.

### Slice S6.6: Runtime-Host Service Providers

Files:

- Add: `macaca/crates/macaca-runtime-host/src/driver_service_provider.rs`
- Add: `macaca/crates/macaca-runtime-host/src/skill_service_provider.rs`
- Add: `macaca/crates/macaca-runtime-host/src/mcp_service_provider.rs`
- Update: `macaca/crates/macaca-runtime-host/src/lib.rs`

Behavior:

- Implement `SystemService` wrappers:
  - `DriverSystemServiceProvider`
  - `SkillSystemServiceProvider`
  - `McpSystemServiceProvider`
- Translate `ServiceCommand` payloads into typed commands and results.
- Delegate to injected `DriverRuntime`, `SkillRuntimeFacade` / executable skill facade, and `McpRuntimeFacade`.
- Return structured unavailable if a provider is not configured.
- Emit structured logs at start/call/complete/fail/stop/cleanup.

Rules:

- Runtime-host provider wrappers must not encode concrete driver names, skill names, MCP server names, application names, or workflow names.
- Service providers use existing `ServiceRuntime` trace and policy decorators.
- File size under 500 lines; split helpers if necessary.

### Slice S6.7: SDK Focused Clients

Files:

- Add: `macaca/crates/macaca-sdk/src/driver_client.rs`
- Add: `macaca/crates/macaca-sdk/src/skill_client.rs`
- Add: `macaca/crates/macaca-sdk/src/mcp_client.rs`
- Update: `macaca/crates/macaca-sdk/src/lib.rs`
- Update: `macaca/crates/macaca-sdk/src/system_facade.rs`

Behavior:

- Define traits:
  - `SystemDriverClient`
  - `SystemSkillClient`
  - `SystemMcpClient`
- Define service-backed clients over `SystemServiceClient`.
- Define unavailable clients returning structured unavailable or empty inventory.
- Add `SystemFacade` methods where useful:
  - driver inventory/catalog/status
  - skill snapshot/catalog/status
  - MCP probe/catalog/status
  - service snapshots

Rules:

- SDK must not construct provider, registry, runtime, MCP process, or skill loader.
- Clients only translate typed commands into generic service calls.
- Logs include trace id and command; no raw tool payload unless already sanitized.

### Slice S6.8: Web Service Registration And State

Files:

- Update: `macaca/crates/macaca-web/src/lib.rs`
- Update: `macaca/crates/macaca-web/src/state.rs`
- Update: `macaca/crates/macaca-web/src/service_runtime_client.rs`

Behavior:

- Register Driver, Skill, and MCP service providers during Web startup.
- Start services with explicit trace contexts.
- Add service-backed clients to `AppState`.
- Keep direct `driver_runtime`, `mcp_runtime`, and direct skill loader paths as deprecated compatibility fields until all call sites migrate.

Rules:

- Do not remove current `/api/drivers/reload`, `/api/mcp/*`, `/api/apps/{id}/skills` behavior.
- No provider/app/server hardcode.
- Missing service returns structured unavailable and does not block Web startup.

### Slice S6.9: Web Toolkit Migration

Files:

- Update: `macaca/crates/macaca-web/src/framework_toolkit.rs`
- Update: `macaca/crates/macaca-web/src/skill_mcp.rs`
- Add optional helper: `macaca/crates/macaca-web/src/service_tool_adapter.rs`

Behavior:

- Replace direct `state.driver_runtime.collect_tools()` with Driver Service tool catalog + service-backed tool adapter.
- Replace direct skill executable tool registration with Skill Service catalog + service-backed tool adapter.
- Replace direct global MCP and skill-backed MCP registration with MCP Service attach/catalog flow where feasible.
- Preserve existing policy filtering and tool conflict behavior.
- Emit same or richer trace events for driver/MCP/skill calls.

Rules:

- Service-backed tool adapters must use `ToolCommandExecutor` pattern and call focused SDK clients.
- Existing direct runtime registration can remain as deprecated fallback only if service unavailable.
- Tool schema and names must remain backward compatible unless OpenSpec marks a deliberate behavior change.

### Slice S6.10: Routes / Capability Catalog / CLI Migration

Files:

- Update: `macaca/crates/macaca-web/src/routes.rs`
- Update: `macaca/crates/macaca-web/src/capability_catalog.rs`
- Update: `macaca/crates/macaca-cli/src/commands.rs`

Behavior:

- Driver status/reload routes call Driver Service client.
- Skill status/snapshot routes call Skill Service client.
- MCP probe/status routes call MCP Service client.
- CLI tool/service status paths inspect services through `SystemFacade`.

Rules:

- Web/CLI remain presentation adapters.
- Existing JSON response shapes should be preserved or mapped through compatibility serializers.
- Deprecated direct route implementations remain searchable until all frontend/API consumers are verified.

### Slice S6.11: Allowlist And Dependency Gate Update

Files:

- Update: `macaca/docs/route-c-serviceization-allowlist.md`
- Update: `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs`
- Update: `macaca/docs/route-c-architecture-governance.md`

Behavior:

- Add S6 ownership rules for Driver / Skill / MCP Service.
- Remove allowlist rows only when `cargo metadata` proves the direct dependency edge is gone.
- If direct Cargo edges remain for DTO/compat fields, document exact remaining debt and expiry condition.

Expected candidates:

- `macaca-kernel -> macaca-driver`
- `macaca-kernel -> macaca-skill`
- `macaca-kernel -> macaca-tools`
- `macaca-web -> macaca-driver`
- `macaca-web -> macaca-skill`
- `macaca-web -> macaca-tools`
- `macaca-cli -> macaca-tools`

Rules:

- Never remove an allowlist row just because code paths migrated; remove only when dependency gate passes without it.
- Any new exception must include replacement service/facade path and target phase.

### Slice S6.12: Tests And Regression

Required tests:

```bash
openspec validate add-driver-skill-mcp-services-v1 --strict
cargo fmt --all --check
cargo test -p macaca-driver
cargo test -p macaca-skill
cargo test -p macaca-runtime-host mcp
cargo test -p macaca-runtime-host service_runtime
cargo test -p macaca-sdk driver_client
cargo test -p macaca-sdk skill_client
cargo test -p macaca-sdk mcp_client
cargo test -p macaca-web framework_toolkit
cargo test -p macaca-web capability_catalog
cargo test -p macaca-integration-tests package_certification
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
cargo check --workspace
npx gitnexus detect-changes -r agent --scope staged
```

Regression matrix:

- RC-DRIVER-001: driver execution trace still shows driver/tool/action details.
- RC-SKILL-001: skill-backed MCP still exposes callable tools and trace.
- RC-TRACE-001: service/tool events still appear without refresh.
- RC-APP-001: existing YAML applications still load.
- RC-GOAL-001: goal/planner/worker/review path does not regress.

## Migration Order

1. OpenSpec change and spec deltas.
2. Common capability tool DTO.
3. Driver service contract and provider.
4. Skill service contract and provider.
5. MCP service contract and provider.
6. SDK clients.
7. Web service registration and state.
8. Web toolkit migration.
9. Routes/capability catalog/CLI migration.
10. Allowlist/governance update.
11. Full verification and GitNexus staged detect.

## Rollback Plan

- Keep existing direct `DriverRuntime`, `SkillRuntimeFacade`, `ExecutableSkillToolSet`, `McpRuntimeFacade`, and direct toolkit registration paths as deprecated compatibility anchors.
- If service-backed toolkit registration fails in runtime, fallback may temporarily use deprecated direct runtime path with a warning and trace event.
- Do not delete existing APIs in S6.
- Revert Web registration/state migration independently from domain service contracts if startup regressions occur.

## Definition Of Done

- OpenSpec change validates strictly.
- Driver/Skill/MCP service contracts exist and are provider-neutral.
- Runtime-host can register/start/call/snapshot Driver, Skill, and MCP providers through `ServiceRuntime`.
- SDK exposes focused clients and unavailable clients.
- Web primary Driver/Skill/MCP paths use service clients; direct runtime fields are deprecated compatibility anchors.
- Driver/Skill/MCP tool calls emit trace/log at key execution nodes.
- Dependency allowlist reflects actual `cargo metadata` edges after migration.
- Required tests pass.
