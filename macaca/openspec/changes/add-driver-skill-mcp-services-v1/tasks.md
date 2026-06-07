## 1. Preparation and Impact Audit

- [x] 1.1 Read the S6 plan, Route C overview, microkernel boundary, serviceization allowlist, architecture governance, and regression matrix.
- [x] 1.2 Inspect current Driver, Skill, MCP, runtime-host, SDK, Web, CLI, capability catalog, toolkit, and integration-test call paths before editing.
- [x] 1.3 Run GitNexus impact before modifying existing structs, functions, traits, or methods, and report direct callers, affected processes, and risk level.
- [x] 1.4 Classify every direct Driver/Skill/MCP path as service provider, SDK client, Web adapter, CLI adapter, kernel compat, test, or provider-internal.
- [x] 1.5 Confirm files touched by S6 remain under 500 LOC or split them before adding logic.

## 2. OpenSpec

- [x] 2.1 Create `add-driver-skill-mcp-services-v1` proposal, design, tasks, and delta specs.
- [x] 2.2 Validate with `openspec validate add-driver-skill-mcp-services-v1 --strict`.
- [x] 2.3 Confirm scope stays on Driver/Skill/MCP serviceization and does not absorb Application, Gateway, Store/Entitlement, Payment, Web3, or EVM phases.

## 3. Common Capability Tool DTO

- [x] 3.1 Add sanitized `CapabilityToolDescriptor` metadata with service id, provider id, capability id, tool name, description, JSON schema, origin kind, permission hints, resource scope hints, conflict namespace, and display name.
- [x] 3.2 Add invocation DTOs carrying trace, application id, session id, agent name, tool name, JSON input, policy hints, and resource scope.
- [x] 3.3 Ensure DTOs cannot expose env, headers, secret values, provider credentials, or full command lines with secrets.
- [x] 3.4 Add detailed English comments explaining why the metadata is provider-neutral and safe to expose.

## 4. Driver Service Contract

- [x] 4.1 Add `DRIVER_SERVICE_ID` and operation constants for load, reload, inventory, tool catalog, tool invoke, status, service snapshot, and cleanup.
- [x] 4.2 Add typed Driver Service commands/results for load, inventory, catalog, invoke, status, snapshot, and cleanup.
- [x] 4.3 Update driver service adapter and exports without introducing kernel/runtime-host/Web/CLI dependencies.
- [x] 4.4 Require trace and explicit scope for applicable commands.
- [x] 4.5 Add structured logs for driver command start, provider selection, completion, failure, and snapshot emission.

## 5. Skill Service Contract

- [x] 5.1 Add `SKILL_SERVICE_ID` and operation constants for snapshot, executable load, tool catalog, tool invoke, status, service snapshot, and cleanup.
- [x] 5.2 Add typed Skill Service commands/results reusing provider-neutral skill snapshot and executable skill types where possible.
- [x] 5.3 Ensure snapshots and service snapshots are sanitized and do not dump full `SKILL.md` bodies by default.
- [x] 5.4 Expose entitlement/package readiness hooks without implementing full entitlement service behavior in S6.
- [x] 5.5 Add structured logs for skill snapshot, executable loading, invocation, status, failure, and snapshot emission.

## 6. MCP Service Contract

- [x] 6.1 Add `MCP_SERVICE_ID` and operation constants for register, probe, tool catalog, tool attach, tool invoke, status, service snapshot, and cleanup.
- [x] 6.2 Add typed MCP Service commands/results around provider-neutral JSON definition, context, policy, and status view types.
- [x] 6.3 Model skill-backed MCP through provider-neutral definition payloads while MCP Service owns protocol lifecycle; host-local Toolkit attach remains a documented compatibility debt.
- [x] 6.4 Add lifecycle scopes for global, app, session, agent-session, and call resources.
- [x] 6.5 Add structured logs for registration, probe, attach, invocation rejection, cleanup, dependency missing, failure, and snapshot emission.

## 7. Runtime-Host Service Providers

- [x] 7.1 Add `DriverSystemServiceProvider`, `SkillSystemServiceProvider`, and `McpSystemServiceProvider`.
- [x] 7.2 Translate `ServiceCommand` payloads into typed service commands and structured results.
- [x] 7.3 Delegate to injected `DriverRuntime`, `SkillRuntimeFacade` / executable skill facade, and `McpRuntimeFacade`.
- [x] 7.4 Return structured unavailable when a provider is not configured.
- [x] 7.5 Ensure runtime-host providers do not encode concrete driver names, skill names, MCP server names, application names, or workflow names.

## 8. SDK Focused Clients

- [x] 8.1 Add `SystemDriverClient`, `SystemSkillClient`, and `SystemMcpClient` traits.
- [x] 8.2 Add service-backed clients over `SystemServiceClient`.
- [x] 8.3 Add unavailable/null-object clients for shells without configured runtime.
- [x] 8.4 Add thin `SystemFacade` accessors for Driver, Skill, and MCP focused clients; command methods live on focused clients.
- [x] 8.5 Validate SDK remains a client/facade layer and not a provider factory.

## 9. Web Service Registration and State

- [x] 9.1 Register Driver, Skill, and MCP service providers during Web startup with explicit trace contexts.
- [x] 9.2 Add service-backed clients to `AppState` as the primary path.
- [x] 9.3 Keep direct `driver_runtime`, `mcp_runtime`, and direct skill loader paths only as deprecated compatibility fields.
- [x] 9.4 Ensure missing services return structured unavailable and do not block Web startup.

## 10. Web Toolkit Migration

- [x] 10.1 Replace direct driver tool collection with Driver Service tool catalog and service-backed tool adapter.
- [x] 10.2 Replace direct executable skill tool registration with Skill Service catalog and service-backed tool adapter.
- [x] 10.3 Replace direct global MCP status/catalog routes with MCP Service; host-local Toolkit registration remains on deprecated `McpRuntimeFacade` until MCP attach can carry a host-owned Toolkit handle safely.
- [x] 10.4 Preserve current policy filtering, tool conflict behavior, tool schemas, and tool names unless a spec explicitly changes them.
- [x] 10.5 Emit equivalent or richer trace events for driver and skill service tool calls; MCP runtime event emission remains in the host-local compatibility path.

## 11. Routes, Capability Catalog, and CLI Migration

- [x] 11.1 Migrate driver status/reload routes to Driver Service client with compatibility serializers.
- [x] 11.2 Migrate skill status/snapshot cache paths to Skill Service client with compatibility fallback.
- [x] 11.3 Migrate MCP probe/status routes to MCP Service client with compatibility serializers.
- [x] 11.4 Migrate Web capability catalog paths to SDK/SystemFacade service clients; CLI has no production Driver/Skill/MCP call sites in this slice.
- [x] 11.5 Keep deprecated direct route/runtime implementations searchable until frontend/API consumers are verified.

## 12. Allowlist and Governance

- [x] 12.1 Update Route C governance with Driver Service, Skill Service, MCP Service, and Capability Tool DTO ownership rules.
- [x] 12.2 Remove allowlist rows only when dependency gates prove the direct Cargo edge is gone.
- [x] 12.3 Document exact remaining debt and expiry condition for DTO/compat dependency edges that cannot be removed in S6.
- [x] 12.4 Update dependency boundary tests for any allowlist changes; no allowlist row removed in this slice because direct Cargo edges remain for compatibility/runtime adapter ownership.

## 13. Verification

- [x] 13.1 Run `openspec validate add-driver-skill-mcp-services-v1 --strict`.
- [x] 13.2 Run `cargo fmt --all --check`.
- [x] 13.3 Run `cargo test -p macaca-driver`.
- [x] 13.4 Run `cargo test -p macaca-skill`.
- [x] 13.5 Run `cargo test -p macaca-runtime-host mcp`.
- [x] 13.6 Run `cargo test -p macaca-runtime-host service_runtime`.
- [x] 13.7 Run `cargo test -p macaca-sdk driver_client`.
- [x] 13.8 Run `cargo test -p macaca-sdk skill_client`.
- [x] 13.9 Run `cargo test -p macaca-sdk mcp_client`.
- [x] 13.10 Run `cargo test -p macaca-web framework_toolkit`.
- [x] 13.11 Run `cargo test -p macaca-web capability_catalog`.
- [x] 13.12 Run `cargo test -p macaca-integration-tests package_certification`.
- [x] 13.13 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 13.14 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 13.15 Run `cargo check --workspace`.
- [x] 13.16 Run `npx gitnexus detect-changes -r agent --scope staged`.
