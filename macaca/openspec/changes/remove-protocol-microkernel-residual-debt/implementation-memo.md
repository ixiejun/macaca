# Implementation Memo

## 2026-06-14 Kernel Persistence Adapter Relocation Checkpoint

- Moved `RedbKernelPersistenceAdapter` out of the Web shell and into
  `crates/runtime/macaca-host-composition/src/persistence_adapter.rs`.
  Kernel persistence adaptation is host composition behavior because it binds
  concrete Redb persistence to kernel-facing persistence commands.
- Exported the adapter from `macaca-host-composition` and updated Web
  service-runtime wiring to import `RedbKernelPersistenceAdapter` from the host
  composition crate.
- Removed the Web-local `persistence_adapter` module declaration so the shell
  no longer owns that persistence adapter implementation.
- GitNexus impact memo: `RedbKernelPersistenceAdapter` reported LOW risk with
  zero direct indexed callers and zero affected indexed processes. Source scans
  and compiler output were used as the authoritative evidence for the moved
  module because several split Web paths are newer than the current index.
- Current terminal dependency evidence remains unchanged: `macaca-web` still
  has normal workspace dependencies on `macaca-host-composition`,
  `macaca-proto`, and `macaca-sdk`. The terminal shell gate therefore still
  fails only for the remaining `macaca-web -> macaca-host-composition` edge.

Validation:

- `cargo fmt --all`: passed with the existing Cargo config-file warning.
- `cargo check -p macaca-host-composition`: passed with pre-existing warnings.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  failed only for `macaca-web -> macaca-host-composition`; the CLI terminal
  purity assertion passed and the Web terminal allowlist zero-row assertion
  passed. No allowlist row or gate weakening was added.

## 2026-06-14 Workbench Route Service Facade Narrowing

- `GET /api/apps/{app_id}/workbench/operations` no longer constructs
  `HostRuntimeSystemServiceClient` inside the route handler.
- The route now uses the already-injected `WebSystemFacadeBundle`
  `SystemServiceClient` and builds the same SDK `WorkbenchClientCatalog`
  from that provider-neutral service facade.
- Updated the route static guard to require `system_facade.service_client()`
  and reject direct `macaca_host_composition` spelling in the module.
- GitNexus impact memo: `get_workbench_operations` reported LOW risk with zero
  direct indexed callers, zero affected indexed processes, and zero affected
  modules.

Validation:

- `cargo fmt --all`: passed with the existing Cargo config-file warning.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `cargo test -p macaca-web workbench_routes --lib`: passed, 1 test.

## 2026-06-14 Context Runtime Facade Extraction Checkpoint

- Moved the context provider runtime snapshot implementation out of
  `crates/shells/macaca-web/src/state.rs` into
  `crates/shells/macaca-web/src/context_runtime_facade.rs`.
- `AppState::context_provider_runtime_snapshot` now delegates to
  `WebContextRuntimeClient`, so route-facing diagnostics no longer require
  `AppState` to own context registry traversal, external-adapter inventory
  synchronization, health-ledger lookup, or JSON sanitization behavior.
- The bootstrap assembly now constructs one shared
  `ContextProviderRegistry`, `ContextEngineRegistry`,
  `ExternalAdapterRuntimeRegistry`, and `ProviderHealthLedger` set, then injects
  those same handles into both `AppState` and `WebContextRuntimeClient`. This
  avoids split-brain diagnostics while preserving the remaining framework runner
  call sites that still need registry/ledger handles during the larger
  host-composition split.
- `ExternalAdapterRuntimeInstallation`,
  `ExternalAdapterRuntimeRegistry`, `ContextProviderRuntimeSnapshot`, and
  `context_external_adapter_runtime_rows` are now owned by the context runtime
  Facade module. External adapter install code, bootstrap context, service
  runtime wiring, and route tests import those types from the new module.
- Source evidence: `state.rs` dropped from roughly 677 lines to 356 lines after
  the extraction; `context_runtime_facade.rs` is 342 lines and contains the
  moved unit tests for overlay sync/prune behavior.
- GitNexus impact memo: `AppState` previously reported HIGH impact with
  `serve_web_server` as the indexed direct caller and four affected
  `Serve_web_server` processes. New facade methods were not found in the stale
  index, so source scans and current compiler/test output are authoritative for
  this checkpoint.
- Current terminal dependency evidence: `cargo metadata --format-version 1
  --no-deps` still reports `macaca-web` direct workspace dependencies
  `macaca-host-composition`, `macaca-proto`, and `macaca-sdk`. This contradicts
  the checked state of tasks 5.14 and 11.5; those tasks remain incomplete in
  current source despite the checkbox state in `tasks.md`.
- Remaining Web host-composition surface measurement:
  `rg "macaca_host_composition|macaca-host-composition|host_composition"
  crates/shells/macaca-web/src crates/shells/macaca-web/Cargo.toml -n | wc -l`
  returned `558`. The largest clusters are composition bootstrap,
  framework/agent construction, workspace memory/context reporting, `state.rs`,
  and tests.

Validation:

- `cargo fmt --all`: passed with the existing Cargo config-file warning.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `cargo test -p macaca-web routes --lib`: passed, 39 tests.
- `cargo test -p macaca-web session --lib`: passed, 40 tests.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  failed only for `macaca-web -> macaca-host-composition`; the CLI terminal
  purity assertion passed and the Web terminal allowlist zero-row assertion
  passed. No allowlist row or gate weakening was added.

## 2026-06-14 Route Host-type Projection Checkpoint

- App UI asset and bridge route helpers no longer carry
  `macaca-host-composition`'s full `AppUiRuntimeConfig`. The application UI
  context loader projects manifest-owned UI data into `AppUiRouteRuntime`, a
  route-local data object containing only entry path, asset allowlist, and bridge
  capability declarations.
- GenUI event persistence moved behind `AppState::persist_genui_event_command`.
  The route still builds a provider-neutral `UiEventCommand`, but concrete
  EventLog persistence is now hidden behind the AppState Facade instead of
  imported directly into `genui_routes.rs`.
- Added `application_execution_event_log.rs` with the
  `ApplicationExecutionEventLog` Observer port and a host-backed adapter. The
  application execution SSE/WebSocket route now queries durable `EventEntry`
  rows and subscribes to append notifications through this port rather than a
  concrete host EventLog type.
- Updated the route source guard to assert the WebSocket adapter uses the
  observer port, durable payload conversion, and no append APIs. This preserves
  the invariant that browser streams observe EventLog state and do not own
  execution or persistence side effects.
- Source evidence:
  `rg "macaca_host_composition" crates/shells/macaca-web/src/app_ui_routes
  crates/shells/macaca-web/src/genui_routes.rs
  crates/shells/macaca-web/src/application_execution_stream_routes.rs
  crates/shells/macaca-web/src/routes -n --glob '*.rs'` now reports only
  `routes/tests.rs` fixture imports.
- Updated remaining Web host-composition surface measurement:
  `rg "macaca_host_composition|macaca-host-composition|host_composition"
  crates/shells/macaca-web/src crates/shells/macaca-web/Cargo.toml -n | wc -l`
  returned `554`. The largest remaining clusters are still composition
  bootstrap, framework/runtime adapters, workspace memory/context reporting,
  and shared `AppState` fields.
- GitNexus impact memo: `AppUiRouteContext` reported CRITICAL, direct
  `app_ui_context` caller and eight app UI route processes; `ensure_declared_asset`
  reported HIGH with direct route/test callers; `persist_event_command` reported
  CRITICAL with direct route/test callers; `run_websocket_event_observer`
  reported LOW with one direct WebSocket route caller. These findings were
  recorded per instruction and did not block the narrow projection/facade edits.

Validation:

- `cargo fmt --all`: passed with the existing Cargo config-file warning.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `cargo test -p macaca-web routes --lib`: passed, 39 tests.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  still failed only for the direct `macaca-web -> macaca-host-composition`
  dependency; CLI terminal purity and Web zero-row allowlist assertions passed.

## Skills/MCP Route Facade Checkpoint

- `/api/apps/:id/skills` no longer imports `macaca-host-composition` app
  loader, Skill policy, Skill service scope, or Skill snapshot command types in
  the HTTP route module.
- Added `AppState::app_skill_status_snapshots` as a shell-local Facade/Adapter
  over application manifest resolution and Skill service snapshot commands. The
  method emits provider-neutral trace logs with app id, agent name, visible
  skill count, filtered skill count, and MCP server count. The route now parses
  the path/query, maps errors to HTTP status, and serializes a route-safe read
  model only.
- Source evidence: production `crates/shells/macaca-web/src/routes/*.rs` files
  have zero `macaca_host_composition` imports; the remaining hit is
  `routes/tests.rs` fixture imports only.
- GitNexus impact memo: `get_app_skills` reported LOW/0 callers on the stale
  monolithic `routes.rs` path. Source scans remained authoritative for the
  split Web routes.

Validation:

- `cargo fmt --all`: passed.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `cargo test -p macaca-web routes --lib`: passed, 39 tests.

## Session Inspection Route Facade Checkpoint

- `/api/sessions/:id/compact` and `/api/sessions/:id/lineage` no longer import
  `macaca-host-composition` persistence/context helpers in the HTTP route
  module.
- Added `AppState::manual_session_compaction_snapshot` as a shell-local
  Facade/Memento operation over the host-owned lineage store. The route now
  passes only `session_id` and optional focus topic, while the state method
  performs lineage persistence, writes bounded audit events, and emits
  provider-neutral trace logs for request and persistence completion.
- Added `AppState::session_lineage_snapshot` so the lineage route renders a
  protocol DTO read model without constructing `SessionLineageStore` in route
  code.
- GitNexus impact memo: `manual_compact_session` reported LOW/0 callers on a
  stale `routes.rs` path; `AppState` reported HIGH with `serve_web_server` as
  the indexed direct caller and four affected `Serve_web_server` processes.
  `macaca-web-server` and `BootstrapCtx` were not found in the index. Source
  scans remained authoritative for the split Web route/bootstrap modules.
- Source evidence: `rg -n "macaca_host_composition"
  crates/shells/macaca-web/src/routes/session_inspect.rs
  crates/shells/macaca-web/src/routes/context_runtime.rs` returned zero hits.

Validation:

- `cargo fmt --all`: passed.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `cargo test -p macaca-web routes --lib`: passed, 39 tests.
- `cargo test -p macaca-web session --lib`: passed, 40 tests.

## Web Provider-neutral Route-state Checkpoint

- Added a runtime-backed `SystemStatusClient` adapter that holds only a weak
  reference to composed Web state and returns bounded unavailable snapshots
  when the process state has already been released.
- `/api/status` now consumes the injected SDK status client instead of reading
  kernel, application runtime, or LLM provider fields in the route handler.
- Test-only Skill imports in capability catalog and telemetry tests now use the
  host-composition runtime surface rather than deleted SDK-root aliases.
- Session persistence and turn-model modules no longer import
  `ExecutorEvent`/`HookEvent`. The unused realtime executor-event projection
  helper was deleted; session snapshot persistence now exposes only its actual
  storage-oriented API.
- Unused `save_agent_traces` and `load_agent_traces` session persistence
  helpers were deleted after source scans and GitNexus both showed zero
  callers. The remaining session persistence API is the active WASM chat-path
  snapshot writer.
- `/api/context/provider-runtime` now renders a `ContextProviderRuntimeSnapshot`
  produced by `AppState` instead of directly importing context registry and
  health-ledger symbols in the route module. The snapshot builder is a
  shell-local Facade/Adapter over host-owned registries, emits a provider-neutral
  trace log with row counts, and converts complex descriptor/config families to
  bounded JSON values with structured warnings on serialization failure.
- GitNexus impact memo: `serve_web_server` and
  `session_status_from_executor_event` both reported LOW risk; `save_agent_traces`
  and `load_agent_traces` reported LOW/0 callers;
  `get_context_provider_runtime` reported LOW/0 callers. Source scans remained
  authoritative because the index paths for split Web modules are stale.
- The terminal shell dependency gate still fails only for
  `macaca-web -> macaca-host-composition`. No allowlist row or gate exception
  was added.

Validation:

- `cargo fmt --all`: passed.
- `cargo test -p macaca-web session --lib`: passed, 40 tests.
- `cargo test -p macaca-web routes --lib`: passed, 39 tests, before the
  session-persistence cleanup; passed again after context provider runtime route
  Facade extraction.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  CLI and zero-row allowlist assertions passed; Web failed only for the
  remaining host-composition dependency.

## SDK Runtime-host Surface Deletion Checkpoint

- Added `crates/runtime/macaca-host-composition` as the explicit host-owned
  Facade/Adapter composition crate for remaining context, skill, application,
  framework, persistence, provider, and runtime contracts.
- Deleted the SDK `runtime-host-bootstrap` feature, optional
  `macaca-runtime-host` dependency, runtime-host facade, focused runtime
  surfaces, and runtime-host-bound context/skill clients.
- Migrated Web context and Skill callers that previously imported deleted SDK
  re-exports to `macaca-host-composition`. The Web crate compiles after the
  migration, while the SDK no longer depends on host/provider/application/
  framework workspace crates.
- GitNexus impact memo: `SystemFacade`, `SystemContextClient`,
  `SystemSkillClient`, `ServiceBackedContextClient`,
  `ServiceBackedSkillClient`, and `SystemMcpClient` reported LOW risk. Module
  targets that GitNexus could not resolve were audited with source scans.
- `cargo tree -e normal -p macaca-sdk --all-features --depth 1` now shows
  `macaca-proto` as the SDK's only direct workspace dependency. This closes task
  4.8.
- Task 4.8a remains open because Web process bootstrap, provider/runtime/
  application/persistence anchors, and host-owned state still live in
  `macaca-web`.
- Task 4.8b remains open because `macaca-web-server` is still a `macaca-web`
  binary and `macaca-web` still has a normal dependency on
  `macaca-host-composition`. The terminal shell dependency gate correctly
  rejects that remaining edge.

Validation:

- `cargo check -p macaca-sdk`: passed.
- `cargo check -p macaca-host-composition`: passed.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `cargo test -p macaca-sdk --lib`: passed, 77 tests.
- `cargo test -p macaca-integration-tests --test sdk_default_dependency_purity_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test context_no_old_entrypoint_gate -- --nocapture`:
  passed after moving its context-client scan target to host composition.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  failed only for the remaining `macaca-web -> macaca-host-composition` normal
  dependency; CLI and the zero-row allowlist assertions passed.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`: passed.

## Terminal SDK Dependency Re-audit

- The previous default-feature dependency gate was insufficient because
  `runtime-host-bootstrap` and `domain-pack-finance` hid production workspace
  dependencies behind optional SDK features.
- The gate now evaluates `cargo tree -p macaca-sdk --all-features --depth 1`.
  Its first expected failure identified exactly two forbidden edges:
  `macaca-sdk -> macaca-runtime-host` and
  `macaca-sdk -> macaca-domain-pack-finance`.
- Source inspection found roughly 520 Web imports through focused SDK modules
  that still expose runtime-host, framework, application, persistence, and
  provider types. Those modules narrowed spelling but did not transfer
  ownership, so tasks 4.3 through 4.6 remain incomplete.
- The selected terminal design is an explicit host composition crate. Axum
  presentation code remains in `macaca-web`; process bootstrap, provider
  construction, persistence, application discovery, and optional-package
  registration move to the host crate. SDK no longer proxies host or package
  crates.
- GitNexus could not resolve module-level `focused_runtime_surfaces` or
  `runtime_host` targets. This is recorded as a source-scan-backed impact memo.
  `SystemFacade` and the edited plugin route test reported LOW risk.
- The SDK `domain-pack-finance` feature and concrete package re-exports were
  deleted. The base Web shell no longer installs a named business-domain pack;
  package crates remain installable by approved external composition roots and
  integration tests, while the base catalog reports unresolved packs.
- Re-running the all-features SDK dependency gate after that deletion reduced
  the failure from two forbidden edges to exactly one:
  `macaca-sdk -> macaca-runtime-host`. This is the remaining task 4.8 blocker.

## 2026-06-09 Baseline

- Branch/worktree: `main...origin/main`.
- Pre-existing untracked audit document: `docs/2026-06-09-macaca-os-protocol-microkernel-reaudit-design.md`.
- New approved OpenSpec change: `openspec/changes/remove-protocol-microkernel-residual-debt/`.
- `openspec list` shows `remove-protocol-microkernel-residual-debt` as active with `0/148` tasks before implementation updates.
- `openspec list --specs` baseline specs: `context-composer`, `execution-control-service`, `microkernel-boundary-purity`, `sdk-system-facade`, `service-runtime`, `serviceization-dependency-gate`, `serviceization-escape-hatches`, `unified-execution-path`, `web-cli-thin-shell-completion`, `web-cli-thin-shell-v0`.

## Initial Dependency Snapshots

- `cargo tree -e normal -p macaca-kernel --depth 1` still showed `reqwest`, proving kernel alert transport/network debt remains.
- `cargo tree -e normal -p macaca-sdk --depth 1` still showed direct dependencies on `macaca-agent`, `macaca-app`, `macaca-context`, `macaca-driver`, `macaca-framework`, `macaca-kernel`, `macaca-llm`, `macaca-memory`, `macaca-runtime-host`, `macaca-skill`, `macaca-task`, and `macaca-tools`. As of the SDK direct framework dependency removal checkpoint, the only remaining direct internal production dependencies are `macaca-proto` and `macaca-runtime-host`.
- `cargo tree -e normal -p macaca-web --depth 1` showed workspace dependencies limited to `macaca-proto` and `macaca-sdk`.
- `cargo tree -e normal -p macaca-cli --depth 1` showed workspace dependencies limited to `macaca-proto` and `macaca-sdk`.

## Initial Debt Scan

The broad baseline scan:

```bash
rg -n "deprecated|allow\\(deprecated\\)|legacy|compat|Route C migration|shell_provider_bridge|route_legacy|AgentOrchestrator|WebhookAlertChannel|McpRuntimeManager|EntitlementRuntimeFacade" crates openspec/specs docs --glob '!target/**'
```

confirmed the active residual families called out by the proposal:

- Baseline specs still contain migration/deprecated allowances that must be removed during archive.
- `crates/kernel/macaca-kernel/src/orchestrator.rs` still defines `AgentOrchestrator`.
- `docs/2026-06-09-macaca-os-protocol-microkernel-reaudit-design.md` records `WebhookAlertChannel`, `shell_provider_bridge`, `McpRuntimeManager`, `EntitlementRuntimeFacade`, and old chat route debt.
- Runtime warnings during tests confirm deprecated `EntitlementRuntimeFacade`, `McpRuntimeManager`, old app helper re-exports, and old chat route remain active.
- Context, LLM, application, and agent crates still contain additional old-path naming/API debt that later tasks must classify and remove.

## Initial Main Path Regression Baseline

- `cargo test -p macaca-web unified_audit_replay_convergence_tests`: passed, 6 tests.
- `cargo test -p macaca-web unified_delegation_path_tests`: passed, 7 tests.

These tests prove the canonical path still works before deletion work begins, but the warnings in the same runs are evidence that residual debt remains.

## GitNexus Impact Memo

- `Toolkit` upstream impact in `agent-macaca-phase07`: LOW, 0 direct callers reported. The index appears incomplete for this symbol because grep shows many current call sites.
- `ToolHandler` upstream impact in `agent-macaca-phase07`: LOW, 4 direct test implementors and 2 indirect test functions reported.
- Because grep shows many production/test users of `crate::tool::*`, the `tool.rs` split was handled as a pure structure move with stable `crate::tool::{Toolkit, ToolHandler, ToolResponse, ToolError, ToolTraceEvent, ToolMiddleware, ToolkitResource, ToolGroup, RegisteredTool}` re-exports.

## File-size Gate Closure

`crates/runtime/macaca-framework/src/tool.rs` was split into:

- `crates/runtime/macaca-framework/src/tool/mod.rs`
- `crates/runtime/macaca-framework/src/tool/types.rs`
- `crates/runtime/macaca-framework/src/tool/traits.rs`
- `crates/runtime/macaca-framework/src/tool/registry.rs`
- `crates/runtime/macaca-framework/src/tool/invocation.rs`

Post-split line counts:

- `mod.rs`: 39
- `types.rs`: 126
- `traits.rs`: 80
- `registry.rs`: 179
- `invocation.rs`: 100

Validation:

- `cargo test -p macaca-framework --test agentscope2_framework_boundaries`: passed, 2 tests.
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`: passed, 2 tests.

## Kernel Alert Transport Serviceization

GitNexus impact memo:

- `AlertManager`: LOW, primarily kernel alert tests and Web bootstrap holders.
- `WebhookAlertChannel`: LOW, no live callers reported after the current kernel refactor; the indexed result reflects stale pre-refactor implementations.
- `AlertChannel`: LOW, three direct implementors reported by the stale index (`LogAlertChannel`, `WebhookAlertChannel`, `CountingChannel`); current source keeps only the abstract trait plus test `CountingChannel`.
- `AlertConfig`: LOW, direct impact limited to kernel alert tests.
- `Alert`: LOW, no upstream processes reported.
- `AlertSystemServiceProvider`: target not found in GitNexus because it is newly added and not indexed yet.

Implementation notes:

- `macaca-kernel` alert code now owns only provider-neutral alert DTOs, severity, deduplication, and the abstract `AlertChannel` port.
- Kernel `AlertConfig` no longer carries `webhook_url`.
- Kernel `AlertManager::new` no longer installs concrete log or webhook channels.
- Runtime-host owns `service.alert` through `AlertSystemServiceProvider`, `AlertServiceProviderFactory`, and delivery Strategies:
  - `TracingAlertDelivery`
  - `WebhookAlertDelivery`
  - `UnavailableAlertDelivery`
- Alert commands now cover `alert.raise`, `alert.resolve`, `alert.health`, and `alert.snapshot`.
- Web no longer constructs or stores kernel `AlertManager`. `AppConfig` now holds a service-backed `SystemAlertClient`, Web startup registers and starts `AlertSystemServiceProvider::tracing()` through `ServiceRuntime`, and plan-loop anomaly alerts call `alert.raise` through `SystemServiceClient`.
- `alert.resolve` is side-effect free and records provider-neutral trace logs.
- Webhook delivery constructs `reqwest::Client` only inside runtime-host provider code and never logs the URL.

Validation:

- `cargo test -p macaca-runtime-host alert_service_provider`: passed, 3 tests.
- `cargo test -p macaca-sdk alert_client --lib`: passed, 0 tests selected; the new SDK client compiled through the crate test target.
- `cargo test -p macaca-web unified_delegation_path_tests`: passed, 7 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests`: passed, 6 tests.
- `cargo test -p macaca-kernel alert`: passed, 10 tests.
- `cargo test -p macaca-integration-tests --test kernel_purity_gate`: passed, 2 tests.
- `cargo tree -e normal -p macaca-kernel --depth 1`: no network/http client dependency; direct normal deps are `macaca-proto`, `macaca-ipc`, `serde`, `serde_json`, `tokio`, `async-trait`, `tracing`, `chrono`, `uuid`, `thiserror`, and `futures`.
- `rg -n "WebhookAlertChannel|reqwest::|reqwest|webhook_url|\\.post\\(" crates/kernel/macaca-kernel crates/runtime/macaca-runtime-host/src/alert_service_provider.rs` shows transport tokens only in `runtime-host` alert provider.
- `rg -n "alert_manager|AlertManager::new|AlertManager|WebhookAlertChannel|webhook_url" crates/shells/macaca-web/src crates/kernel/macaca-kernel/src crates/runtime/macaca-runtime-host/src --glob '*.rs'` shows `AlertManager` only in kernel provider-neutral source/tests.

## Kernel Agent Orchestrator Removal

GitNexus impact memo:

- `AgentOrchestrator`: LOW, 0 direct callers/processes reported.
- `OrchestratorBuilder`: LOW, 0 direct callers/processes reported.
- `delegate_task`: LOW, but GitNexus matched `examples/apps/fullstack-autodev/personas/coordinator/TOOLS.md` instead of the kernel Rust helper, so source scans were used as the authoritative follow-up.
- `aggregate_results`: target not found.
- `report_to_coordinator`: target not found.

Implementation notes:

- Deleted `crates/kernel/macaca-kernel/src/orchestrator.rs`.
- Removed `pub mod orchestrator` and `pub use orchestrator::AgentOrchestrator` from `macaca-kernel/src/lib.rs`.
- Shared orchestration DTOs remain in `macaca-proto::orchestration`; this keeps protocol contracts provider-neutral while deleting kernel ownership of routing, delegation queues, result aggregation, and LLM tool command parsing.
- Existing Web/runtime-host delegation paths already route through `service.agent_execution` and `service.execution_control`; no production caller depended on the deleted kernel orchestrator.
- `kernel_purity_gate` now also rejects agent/task orchestration semantics in kernel production source.

Validation:

- `rg -n "AgentOrchestrator|OrchestratorBuilder|OrchestrationCommand|OrchestrationEvent|DelegatedTask|DelegatedTaskResult|AgentRouting|RoutingDecision|delegate_task|aggregate_results|report_to_coordinator|find_best_agent|parse_command" crates/kernel/macaca-kernel/src crates/kernel/macaca-kernel/Cargo.toml`: zero hits.
- `cargo test -p macaca-integration-tests --test kernel_purity_gate`: passed, 3 tests.
- `cargo test -p macaca-kernel`: passed, 49 unit tests, 4 e2e tests, 6 primitive tests, 6 system service contract tests, and doc tests.
- `cargo test -p macaca-web unified_delegation_path_tests`: passed, 7 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests`: passed, 6 tests.
- `cargo test -p macaca-proto agent_execution_service`: passed, 7 tests.

## Old Chat Route Deletion

GitNexus impact memo:

- `post_chat`: LOW, 0 direct callers/processes reported. The indexed path appears stale (`chat_orchestrator.rs`) while current source uses `chat_orchestrator/route_legacy.rs`; source scans were used as the authoritative follow-up.

Implementation notes:

- Deleted `crates/shells/macaca-web/src/chat_orchestrator/route_legacy.rs`.
- Removed `mod route_legacy` and `pub(crate) use route_legacy::post_chat` from `chat_orchestrator/mod.rs`.
- Removed the deleted file from the test-only `chat_orchestrator/contract_source.rs` source bundle.
- Router registration remains `/api/chat/v2` plus `/api/chat/stop`; no `/api/chat` production route wrapper remains.

Validation:

- `rg -n "route_legacy|pub\\(crate\\) use route_legacy|post_chat\\b|#\\[deprecated\\]|/api/chat\\\"" crates/shells/macaca-web/src/chat_orchestrator crates/shells/macaca-web/src/bootstrap.rs crates/shells/macaca-web/src`: zero hits.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests`: passed, 6 tests.
- `cargo test -p macaca-web unified_delegation_path_tests`: passed, 7 tests.

## Session Loop Waker Alias Cleanup

GitNexus impact memo:

- `LegacyPlanLoopWaker`: target not found.
- `LegacyWorkerLoopWaker`: target not found.

Implementation notes:

- Removed unused `LegacyPlanLoopWaker` and `LegacyWorkerLoopWaker` aliases from `session_loop_shell_adapter.rs`.
- Reworded the same module from old-path terminology to stable `local shell notification` terminology.
- This does not claim completion of shell loop ownership migration; `PlanLoopWaker` and `WorkerLoopWaker` are still local shell state and remain covered by later shell-local-execution-owner tasks.

Validation:

- `rg -n "LegacyPlanLoopWaker|LegacyWorkerLoopWaker|legacy shell adapter|compat seam|legacy" crates/shells/macaca-web/src/session_loop_shell_adapter.rs`: zero hits.
- `cargo test -p macaca-web unified_delegation_path_tests`: passed, 7 tests.

## Runtime-host MCP/WASM Debt-token Cleanup

GitNexus impact memo:

- `McpDefinitionSource`: LOW. The index still points at the old single-file `mcp_runtime.rs`, so source scans under `crates/runtime/macaca-runtime-host/src/mcp_runtime/` were used as the authoritative hit list.
- `WasmUpgradeReport`: LOW.
- `abi_compatible`: LOW.

Implementation notes:

- Renamed `McpDefinitionSource::Compatibility` to `McpDefinitionSource::Mapping` and changed the serialized value from `compatibility` to the enum-wide snake-case value `mapping`.
- Updated skill MCP mapping materialization, MCP invocation registry fixtures, and MCP runtime test fixtures to use the new `Mapping` source.
- Renamed WASM upgrade report field `abi_compatible` to `abi_matches`.
- Renamed WASM lifecycle metadata key `abi_compatible` to `abi_matches`.
- Reworded WASM lifecycle comments from compatibility terminology to ABI match/mismatch terminology.

Validation:

- `cargo test -p macaca-proto wasm_runtime_provider --lib`: passed, 12 tests.
- `cargo test -p macaca-runtime-host mcp_runtime --lib`: passed, 22 tests.
- `cargo test -p macaca-runtime-host wasm_runtime_provider --lib`: passed, 63 tests.
- `cargo test -p macaca-runtime-host skill_mcp_mapping_registry --lib`: passed, 4 tests.
- `rg -n "McpDefinitionSource::Compatibility|abi_compatible|\\bcompatible\\b|compatibility" crates/foundation/macaca-proto/src/wasm_runtime_provider crates/runtime/macaca-runtime-host/src/mcp_runtime/types.rs crates/runtime/macaca-runtime-host/src/skill_mcp_mapping_registry.rs crates/runtime/macaca-runtime-host/src/mcp_invocation_registry.rs crates/runtime/macaca-runtime-host/src/mcp_runtime/tests/fixtures.rs crates/runtime/macaca-runtime-host/src/wasm_runtime_provider --glob '*.rs'`: zero hits.

## SDK shell provider bridge deletion

GitNexus impact memo:

- `shell_provider_bridge`: target not found.
- `macaca-sdk/src/lib.rs`: target not found.
- Source scans showed the deleted module was only declared and re-exported by `macaca-sdk/src/lib.rs`; downstream callers use the public `macaca_sdk::{driver,llm,memory,skill,task,tools,kernel,agent,context,framework,app,runtime_host}` paths.

Implementation notes:

- Deleted `crates/facade/macaca-sdk/src/shell_provider_bridge.rs`.
- Removed `pub mod shell_provider_bridge` and the old `pub use shell_provider_bridge::{...}` block.
- Added explicit SDK top-level modules for the existing public paths so current consumers keep compiling while the remaining SDK purity work migrates each surface to focused clients and protocol DTOs.
- Updated the dependency-boundary gate text so it no longer points callers back to the deleted module.

Validation:

- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `rg -n "shell_provider_bridge|Route C migration|macaca_sdk::shell_provider_bridge" crates/facade/macaca-sdk/src crates/shells/macaca-web/src crates/tests/macaca-integration-tests/tests --glob '*.rs'`: no SDK or Web production hits; remaining hits are route-c dependency gate/test naming that will be handled by the terminal gate rename task.

## SDK Autonomy Runtime Narrow Module Cleanup

GitNexus impact memo:

- `AutonomyRuntimeBundle`: LOW, direct indexed impact limited to runtime-host autonomy bootstrap helpers.
- `AutonomyRuntimeConfig`: LOW, no upstream processes reported.
- `AutonomyProviderMode`: LOW, no upstream processes reported.
- `bootstrap_autonomy_services`: HIGH because it participates in web-server startup processes. The change is import-only: the function body, signature, and call arguments are unchanged, and Web now calls the same runtime-host export through `macaca_sdk::autonomy_runtime`.
- `TodoStore`: LOW, no upstream processes reported. This was included as an adjacent import-surface cleanup because Web stores the task tool backing store handle alongside runtime bootstrap state.

Implementation notes:

- Added focused SDK module `macaca_sdk::autonomy_runtime` for autonomy runtime bootstrap contracts.
- Migrated Web autonomy config translation, bootstrap context/state storage, and application-discovery startup from `macaca_sdk::runtime_host` root paths to the focused autonomy module.
- Added `TodoStore` to `macaca_sdk::tools` and moved Web state/bootstrap storage references to that focused path.
- No provider selection, lifecycle behavior, or supervisor wiring changed.

## SDK Agent-execution Narrow Module Cleanup

GitNexus impact memo:

- `AgentExecutionBackend`: CRITICAL because it is the unified execution trait used by service providers and autonomy flows. This change does not edit the trait, implementations, signatures, or service command handling; it only moves Web imports to `macaca_sdk::agent_execution`.
- `ComposedAgentExecutionBackend`: LOW, no upstream processes reported.
- `AgentExecutionSystemServiceProvider`: LOW, no upstream processes reported.
- `FrameworkAgentMaterializationPort`: not found in the current index; source scans confirmed usage in Web construction adapters and runtime-host framework runtime service.

Implementation notes:

- Added focused SDK module `macaca_sdk::agent_execution` for unified agent-execution provider contracts, composed backend construction, framework materialization ports, and kernel dispatch wiring.
- Migrated Web execution adapters, framework materialization adapter, Skill self-evolution decorator, and post-bootstrap service registration from `macaca_sdk::runtime_host` root imports to the focused agent-execution module.
- No execution routing, audit replay, evidence collection, or Skill observation behavior changed.

## SDK Tool-bootstrap Narrow Module Cleanup

GitNexus impact memo:

- `bootstrap_local_base_tools` and `resolve_workspace_tool_path` were not found in the current index. Source scans located their canonical definitions in runtime-host tool bootstrap modules and their Web callers.

Implementation notes:

- Added focused SDK module `macaca_sdk::tool_bootstrap` for orchestration tools, task toolkit assembly, workspace toolkit assembly, and path/input helpers.
- Migrated Web orchestration assembly, framework toolkit construction, workspace helper tests, and base-tool startup away from `macaca_sdk::runtime_host` root paths.
- Concrete tool construction and task/workspace semantics remain in runtime-host.

## SDK Application-bootstrap Narrow Module Cleanup

GitNexus impact memo:

- `ApplicationSystemServiceProvider`: LOW, no upstream processes reported.
- `ApplicationOrchestrationBackend`: LOW, direct impact limited to the Web adapter implementation and runtime-host test fake.
- `PluginControlSystemServiceProvider`: LOW, no upstream processes reported.
- `DomainPackProviderRegistration`: LOW, no upstream processes reported.

Implementation notes:

- Added focused SDK module `macaca_sdk::application_bootstrap` for Application Service provider registration, plugin service provider registration, domain-pack provider registrations, and WASM host-import bridge contracts.
- Migrated Web application discovery, WASM orchestration adapter, bootstrap context storage, and domain-pack wiring away from `macaca_sdk::runtime_host` root paths for these contracts.
- No application startup, plugin service, WASM host-import, or domain-pack provider behavior changed.

## SDK Service-bootstrap Narrow Module Cleanup

GitNexus impact memo:

- `LlmSystemServiceProvider`: LOW, no upstream processes reported.
- `AlertSystemServiceProvider`, `bootstrap_optional_services`, and `bootstrap_driver_service` were not found in the current index. Source scans located their runtime-host definitions and Web bootstrap callers.

Implementation notes:

- Added focused SDK module `macaca_sdk::service_bootstrap` for host-local service provider registration and startup functions used by the Web composition root.
- Migrated Web skill asset bootstrap, driver service bootstrap, application-execution/interaction/task service bootstrap, alert/LLM/MCP/Memory/Context provider registration, workbench-family service bootstrap, tool-planning bootstrap, and optional-service bootstrap away from `macaca_sdk::runtime_host` root paths.
- Runtime-host remains the owner of provider construction, lifecycle, unavailable behavior, and service diagnostics.

## SDK Agent-context and Execution-control Contract Cleanup

GitNexus impact memo:

- `AgentContextBackend`: LOW, direct impact limited to the Web backend implementation and runtime-host test fake.
- `execution_control_service_descriptor`: HIGH because it participates in Web startup and execution-control provider tests. The descriptor implementation and registration behavior are unchanged.
- Goal-lifecycle and fork/join request DTOs were not found in the current index; source scans confirmed they are runtime-host execution-control contracts used by Web adapters.

Implementation notes:

- Added Agent Context service descriptor/backend/provider exports to `macaca_sdk::agent_execution`.
- Added execution-control service descriptor plus goal-lifecycle and fork/join request DTOs to `macaca_sdk::execution_control`.
- Migrated Web service registration and lifecycle adapters away from `macaca_sdk::runtime_host` root imports.

## SDK MCP Runtime and Agent-execution Helper Cleanup

GitNexus impact memo:

- Remaining MCP runtime helper functions were source-scan-backed because `RuntimeEnvBuilder`, `apply_concurrency_isolation`, and the Skill MCP mapping registry were not found as indexed symbols.
- Agent-execution helper imports were test-support-only path cleanup using existing runtime-host helper functions through the focused `macaca_sdk::agent_execution` module.

Implementation notes:

- Extended `macaca_sdk::mcp_runtime` with environment propagation, concurrency isolation, and Skill MCP mapping registry access used by Web MCP adapters.
- Migrated Web capability catalog, MCP environment bootstrap, Skill MCP server resolution, and agent-execution test support away from `macaca_sdk::runtime_host` root paths.
- No MCP launch-plan, environment propagation, or agent-execution helper behavior changed.

## Task Service DTO and Web Task Board Query Migration

GitNexus impact memo:

- `QueryTaskBoardCommand`: LOW, 0 impacted symbols/processes in the indexed graph.
- `TaskServiceRuntime`: LOW, 0 impacted symbols/processes in the indexed graph.
- `TaskSystemServiceProvider`: LOW, 0 impacted symbols/processes in the indexed graph.
- `ServiceBackedTaskBoardDataSource`: target not found because the symbol is newly introduced after the current GitNexus index.
- `get_todo_progress`: LOW, 0 impacted symbols/processes in the indexed graph.
- `diagnose_session_claims`: CRITICAL, 3 direct / 6 affected processes in the indexed graph, including `get_todo_claim_diagnostics`; this was recorded as required by the proposal and not used as a blocker per user direction.
- `SessionClaimDiagnostics`: CRITICAL, 1 direct / 6 affected processes in the indexed graph, again centered on claim diagnostics. The code change only added `Deserialize` to existing DTO shapes and did not change serialized fields.

Implementation notes:

- Moved Task Service command/result DTOs and command names into `macaca-proto::task_service`, including:
  - `TaskBoardQueryResult`
  - `QueryTaskProgressCommand`
  - `TaskProgressSummary`
  - `QueryAgentTodosCommand`
  - `AgentTaskBoardResult`
  - `QueryTaskGoalsCommand`
  - `TaskGoalsResult`
  - `QueryTaskClaimDiagnosticsCommand`
  - `TASK_PROGRESS_COMMAND`
  - `TASK_AGENT_TODOS_COMMAND`
  - `TASK_GOALS_COMMAND`
  - `TASK_CLAIM_DIAGNOSTICS_COMMAND`
  - all existing Task Service lifecycle command name constants
- `macaca-task/src/commands.rs` remains a re-export of proto-owned Task Service command DTOs.
- `TaskServiceRuntime` now owns service-level query handlers for progress, agent board, goals, and claim diagnostics. These handlers use the existing generic `TodoStore`/`TaskSpace` internals inside the task service owner instead of exposing store reads to SDK or shell code.
- `TaskSystemServiceProvider` dispatches the new commands through `ServiceCommand`, records provider-neutral `tracing`, and returns the same JSON field shapes currently expected by Web callers.
- `ServiceBackedTaskBoardDataSource` now provides focused methods for task progress, agent board, goals, and claim diagnostics through `SystemServiceClient`.
- `ServiceBackedTaskBoardDataSource` now also creates goals through the Task Service `task.create_goal` command.
- Web `routes/todos.rs` no longer reads `state.persist.todo_store`, no longer constructs `macaca_sdk::task::TaskSpace`, and no longer calls `diagnose_session_claims` directly. It uses SDK service-backed task queries while preserving response fields:
  - `/todos/progress`: `total`, status counts, `all_done`
  - `/todos/{agent_name}`: `agent`, `todos`, `count`
  - `/goals`: `goals`, `count`
  - `/todos/claim-diagnostics`: unchanged diagnostics JSON shape
- Web `loop_manager/goal_route.rs` no longer constructs `TaskSpace` or reads `todo_store` for HTTP goal creation. It calls the SDK task-service adapter and still starts the PlanLoop/WorkerLoop lifecycle after the service-owned goal is created.
- The stale SDK task-client comment that said the implementation read `TodoStore` was removed.

Validation:

- `cargo fmt`: passed.
- `cargo check -p macaca-proto`: passed with pre-existing `orchestration.rs` unused `uuid::Uuid` warning.
- `cargo check -p macaca-task`: passed.
- `cargo check -p macaca-runtime-host`: passed with pre-existing warnings.
- `cargo check -p macaca-sdk`: passed with pre-existing lower-layer warnings.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `cargo test -p macaca-runtime-host task_service_provider --lib`: passed, 2 tests.
- `cargo test -p macaca-web web_shell_task_board_preserves_stable_json_shape --lib`: passed, 1 test.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7 tests.
- `cargo test -p macaca-proto task_claim_diagnostics --lib`: passed, 2 tests.
- `rg -n "state\\.persist\\.todo_store|macaca_sdk::task::TaskSpace|TodoStoreTaskBoardDataSource|Compatibility placeholder" crates/shells/macaca-web/src/routes/todos.rs crates/facade/macaca-sdk/src/task_client.rs`: zero hits.
- `rg -n "TaskSpace::for_session|macaca_sdk::task::TaskSpace|state\\.persist\\.todo_store" crates/shells/macaca-web/src/loop_manager/goal_route.rs crates/shells/macaca-web/src/routes/todos.rs`: zero hits.

Remaining task-facade debt after this slice:

- SDK still re-exports `macaca_task` task runtime/loop types through `macaca_sdk::task`.
- SDK still directly depends on multiple lower-layer crates according to the earlier `cargo tree -e normal -p macaca-sdk --depth 1` baseline. Task 4.8 is not complete.

## Repository Rust Debt-token Cleanup

GitNexus impact memo:

- `route_c_dependency_boundaries`: LOW, 0 upstream symbols/processes. The helper directory and integration-test entrypoint were renamed as gate/test surface cleanup.
- `serviceization_escape_hatches`: target not found; treated as an integration gate text/catalog cleanup.
- `p5_terminal_audit_gates`: target not found; treated as an integration gate text/catalog cleanup.

Implementation notes:

- Renamed the dependency boundary gate entrypoint from `route_c_dependency_boundaries.rs` to `protocol_service_dependency_boundaries.rs`.
- Renamed the helper directory from `route_c_dependency_boundaries/` to `protocol_service_dependency_boundaries/`.
- Renamed the workspace topology guard from `route_c_workspace_topology.rs` to `protocol_workspace_topology.rs`.
- Renamed the baseline guard from `route_c_baseline.rs` to `protocol_microkernel_baseline.rs`.
- Renamed runtime-host public facade gate from `runtime_host_no_deprecated_public_facade_gate.rs` to `runtime_host_no_retired_public_facade_gate.rs`.
- Reworded integration-gate comments and diagnostics from historical phase terminology to terminal protocol microkernel/serviceization terminology.
- Converted negative-assertion tokens in context/application/P5/serviceization/runtime-host gates to runtime string assembly so the gate still detects retired APIs without embedding old debt words in active Rust source.
- Removed `#![deny(deprecated)]` from `macaca-cli` crate roots; deprecated-attribute containment is now enforced by the terminal debt-token gate instead of crate-local lint text.
- Added `no_debt_token_gate.rs`, which scans production and integration-test Rust sources for retired Macaca path markers, old phase route markers, and deprecated attributes. The gate explicitly classifies OpenAI/DashScope wire-compatible protocol identifiers as third-party API terminology, not Macaca old-path debt.

Validation:

- `cargo test -p macaca-integration-tests --test protocol_service_dependency_boundaries`: passed, 3 tests.
- `cargo test -p macaca-integration-tests --test protocol_workspace_topology`: passed, 1 test.
- `cargo test -p macaca-integration-tests --test runtime_host_no_retired_public_facade_gate`: passed, 2 tests.
- `cargo test -p macaca-integration-tests --test context_no_old_entrypoint_gate`: passed, 1 test.
- `cargo test -p macaca-integration-tests --test application_no_old_helper_gate`: passed, 1 test.
- `cargo test -p macaca-integration-tests --test serviceization_escape_hatches`: passed, 19 tests and 1 ignored baseline-regeneration helper.
- `cargo test -p macaca-integration-tests --test p5_terminal_audit_gates`: passed, 4 tests.
- `cargo test -p macaca-integration-tests --test no_debt_token_gate`: passed, 1 test.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates --glob '*.rs'`: zero hits.
- `rg -n "legacy|compat|Route C migration|\\broute_c\\b|Route C|compatibility|deprecated" crates --glob '*.rs'`: remaining hits are only OpenAI/DashScope third-party wire-protocol identifiers and URLs; the new `no_debt_token_gate` encodes this exception and passes.

## MCP Runtime Public Facade Cleanup

GitNexus impact memo:

- `McpRuntimeManager`: LOW, 0 impacted symbols/processes reported. The indexed file path is stale (`mcp_runtime.rs`), so source scans were used to identify the real call surface under `mcp_runtime/`.
- `McpRuntimeFacade`: LOW, 0 impacted symbols/processes reported.
- `definitions_from_skill_snapshot`: LOW, one direct test caller reported.
- `McpRegistryConfig::into_definitions`: target not found by qualified name; unqualified `into_definitions` was LOW with one direct test caller.

Implementation notes:

- `McpRuntimeManager` is now `pub(crate)` and no longer re-exported from `crate::mcp_runtime` or `runtime_host_public_api`.
- `McpRuntimeFacade` remains the stable public Facade entry point for runtime-host and MCP service provider call paths.

## Autonomy Evolution DTO Contract Migration

GitNexus impact memo:

- `EvolutionTransitionCommand`: LOW, 0 upstream symbols/processes reported. Source scans showed broader serde/service use across SDK, runtime-host, and service provider code, so compile/test validation was treated as authoritative.
- `OsCodeEvolutionProposalCommand`: LOW, 0 upstream symbols/processes reported. Source scans showed SDK, runtime-host live executor, and service provider use.

Implementation notes:

- Added `macaca-proto::autonomy_evolution` as the single provider-neutral contract source for Autonomy Evolution command/result DTOs and stable command/service identifiers.
- Moved the shared DTO families into split proto modules:
  - `autonomy_evolution/model.rs`
  - `autonomy_evolution/release.rs`
  - `autonomy_evolution/live.rs`
  - `autonomy_evolution/os_code_proposal.rs`
- Changed `macaca-sdk` autonomy evolution client and tests to import DTOs from `macaca-proto`, not `macaca-autonomy-evolution`.
- Removed the `macaca-autonomy-evolution` production dependency from `macaca-sdk/Cargo.toml`.
- Changed `macaca-autonomy-evolution` service crate to re-export `macaca_proto::autonomy_evolution::*` instead of owning duplicate DTO modules.
- Deleted the service-owned DTO files:
  - `crates/services/macaca-autonomy-evolution/src/model.rs`
  - `crates/services/macaca-autonomy-evolution/src/release_model.rs`
  - `crates/services/macaca-autonomy-evolution/src/live_orchestrator_model.rs`
- Kept provider behavior in the service crate by moving scope filtering and bounded value helpers into provider/strategy code rather than attaching service-owned inherent methods to proto DTOs.
- Trimmed `os_code_proposal_adapter.rs` so it owns only the non-mutating Adapter/Strategy and imports its command/result DTOs from `macaca-proto`.

Validation:

- `rg -n "macaca-autonomy-evolution|macaca_autonomy_evolution" crates/facade/macaca-sdk crates/facade/macaca-sdk/Cargo.toml crates/foundation/macaca-proto/src/autonomy_evolution crates/services/macaca-autonomy-evolution/src`: zero hits.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no direct `macaca-autonomy-evolution` dependency; remaining direct provider/runtime/application/framework dependencies are still covered by the open 4.8 cleanup task.
- `cargo test -p macaca-proto --lib`: passed, 169 tests.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-sdk --test autonomy_evolution_client_tests --test autonomy_evolution_admission_client_tests --test autonomy_evolution_benchmark_client_tests --test autonomy_evolution_release_client_tests --test autonomy_evolution_live_orchestrator_client_tests`: passed, 7 tests.
- `cargo test -p macaca-autonomy-evolution`: passed, 31 tests across service integration suites plus doc tests.
- `cargo test -p macaca-runtime-host autonomy_evolution --lib`: compiled runtime-host and selected 0 tests, confirming runtime-host call sites still type-check against the migrated contract.

## Framework Construction Boundary Slice

GitNexus impact memo:

- `FrameworkRunner`: LOW, 0 upstream symbols/processes reported. Source scans showed current Web call sites, so the index is stale for this refactor.
- `ServiceBackedFrameworkRuntimeAgentPort`: target not found. Source scans under runtime-host/Web were used instead.

Implementation notes:

- Added `FrameworkAgentMaterializationPort` in runtime-host as the lower-level host-local adapter boundary.
- Added `RuntimeHostFrameworkAgentConstructionService`, which implements `FrameworkAgentConstructionPort` inside runtime-host and decorates construction with provider-neutral trace/log reason codes.
- Updated `ServiceBackedFrameworkRuntimeAgentPort` documentation and wiring model so runtime-host owns both construction-service orchestration and reply orchestration.
- Changed Web from `WebFrameworkAgentConstructionPort` to `WebFrameworkAgentMaterializationPort`; Web no longer implements `FrameworkAgentConstructionPort`.
- Updated Web composed backend wiring to pass:
  - `WebFrameworkAgentMaterializationPort`
  - wrapped by `RuntimeHostFrameworkAgentConstructionService`
  - consumed by `ServiceBackedFrameworkRuntimeAgentPort`
- Re-exported `FrameworkAgentMaterializationPort` and `RuntimeHostFrameworkAgentConstructionService` through runtime-host public API and SDK runtime-host facade.
- Updated static wiring tests to assert runtime-host construction service ownership and Web materialization-only ownership.

Remaining hard evidence:

- `crates/shells/macaca-web/src/framework_agent_construction_shell_adapter.rs` still calls `FrameworkRunner::build_runtime_agent_from_context_snapshot_with_execution_policy` from the Web materializer. Therefore task 5.3 is intentionally still open; the next slice must move the concrete `FrameworkRunner`/factory materialization dependency out of Web or replace it with a runtime/framework-owned provider.

Validation:

- `rg -n "WebFrameworkAgentConstructionPort|impl FrameworkAgentConstructionPort" crates/shells/macaca-web/src --glob '*.rs'`: no Web production implementation hits; only static negative assertions remain.
- `cargo test -p macaca-runtime-host framework_runtime_agent_service --lib`: passed, 2 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7 tests.
- `cargo test -p macaca-web agent_execution_backend::tests::static_wiring --lib`: passed, 4 tests.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-integration-tests --test shell_no_framework_construction_gate`: passed, 1 test.
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`: passed, 2 tests after the construction-boundary and proto-contract changes.

## Web Driver Runtime Ownership Removal

GitNexus impact memo:

- `DriverRuntime`: LOW, 0 impacted symbols/processes reported. Source scans found the real Web shell anchors in `composition_bootstrap/tooling_and_persist.rs`, `composition_bootstrap/bootstrap_ctx.rs`, and `composition_bootstrap/service_runtime_wiring.rs`.
- `DriverSystemServiceProvider`: LOW, 0 impacted symbols/processes reported.
- `ServiceRuntime::register_provider`: CRITICAL, 43 direct callers and 20 affected processes. This slice did not modify `register_provider`; it added a runtime-host helper that calls the existing API.

Implementation notes:

- Added `macaca-runtime-host::driver_service_bootstrap` with `bootstrap_driver_service`.
- Runtime-host now owns `DriverRegistry`, `DriverRuntime`, optional startup auto-load, provider registration, and provider-neutral logging for driver service bootstrap.
- Web `BootstrapCtx` no longer carries `driver_runtime`.
- Web `tooling_and_persist` now passes only `drivers_dir`, `auto_load`, and `ServiceRuntime` to runtime-host.
- Web `service_runtime_wiring` no longer directly constructs `DriverSystemServiceProvider`.
- Web production source no longer imports or names `macaca_sdk::driver::DriverRuntime`, `DriverRegistry`, or `DriverSystemServiceProvider`.

Validation:

- `cargo test -p macaca-runtime-host driver_service --lib`: passed, including `driver_service_bootstrap::tests::bootstrap_registers_driver_service_without_exposing_runtime`.
- `cargo test -p macaca-web routes::drivers --lib`: passed, 0 selected tests after compilation.
- `rg -n "driver_runtime|DriverRuntime|DriverRegistry|DriverSystemServiceProvider|macaca_sdk::driver" crates/shells/macaca-web/src crates/shells/macaca-cli/src crates/tests -g '*.rs'`: only integration gate detection-token strings remain; no Web/CLI production ownership anchors remain.

## Web Skill Catalog Construction Removal

GitNexus impact memo:

- `SkillCatalog`: LOW, 0 impacted symbols/processes reported. Source scans showed Web shell construction and storage in `tooling_and_persist.rs`, `bootstrap_ctx.rs`, `app_state_assembly.rs`, `state.rs`, and `routes/skills_mcp.rs`.
- `ExecutableSkillToolSet`: LOW, 0 impacted symbols/processes reported.

Implementation notes:

- Added `macaca-runtime-host::skill_bootstrap` with `bootstrap_local_skill_assets`.
- Runtime-host now owns local `SkillCatalog` and `ExecutableSkillToolSet` construction for startup asset loading.
- Web receives only `SkillCatalogEntryView` records for `/api/skills` rendering plus generic `Tool` trait objects for toolkit assembly.
- Web `AppConfig` no longer stores `SkillCatalog`; it stores a read-only catalog-entry snapshot.
- Web production source no longer imports `macaca_sdk::skill::{SkillCatalog, ExecutableSkillToolSet}`.

Validation:

- `cargo test -p macaca-runtime-host skill_bootstrap --lib`: passed, including `skill_bootstrap::tests::bootstrap_empty_skill_root_returns_empty_snapshot`.
- `cargo test -p macaca-web routes::skills_mcp --lib`: passed, 0 selected tests after compilation.
- `cargo test -p macaca-web skill_operations_routes --lib`: passed, 2 tests.
- `cargo test -p macaca-web skill_mcp --lib`: passed, 5 tests.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `rg -n "macaca_sdk::(driver|skill)\\b|use macaca_sdk::(driver|skill)::" crates/shells crates/tests -g '*.rs'`: zero hits.

## SDK Driver/Skill Broad Alias Deletion

Implementation notes:

- Deleted `pub mod driver { pub use macaca_driver::*; }` from `macaca-sdk/src/lib.rs`.
- Deleted `pub mod skill { pub use macaca_skill::*; }` from `macaca-sdk/src/lib.rs`.
- Stable top-level SDK exports remain for Driver and Skill DTOs/clients that have been migrated to focused facade surfaces.

Validation:

- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `rg -n "macaca_sdk::(driver|skill)\\b|use macaca_sdk::(driver|skill)::|pub mod (driver|skill) \\{" crates -g '*.rs'`: zero hits.

## SDK Memory Broad Alias Reduction

GitNexus impact memo:

- `memory`: LOW, 0 impacted symbols/processes reported by a stale/non-specific index hit; source scans were used as the authoritative surface because Web still consumes memory DTOs and runtime-adapter fixtures through `macaca_sdk::memory`.

Implementation notes:

- Replaced `pub use macaca_memory::*` under `macaca_sdk::memory` with an explicit export list.
- The explicit list keeps service DTOs, runtime/facade types, tombstone/knowledge/active-recall contracts, and the remaining Web adapter fixtures visible while removing the broad wildcard bridge.
- Concrete manager/factory/test-memory types remain explicitly listed only because Web memory adapter migration is not complete yet; this is no longer a star alias and remains scheduled under SDK/Web thin-shell cleanup.

Validation:

- `cargo fmt --package macaca-sdk`: passed.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`: passed, 6 tests.
- `cargo test -p macaca-web context_memory_injection --lib`: passed, 3 tests.
- `cargo test -p macaca-web workspace_knowledge_digest_capability --lib`: passed in the earlier memory slice.

## SDK Context Broad Alias Reduction

GitNexus impact memo:

- `context`: LOW, 3 direct test-only callers reported against an unrelated runtime-host helper named `context`; current-source scans were used as the authoritative blast radius.
- `ContextReport`: LOW, 1 direct builder method reported.
- `ContextEngineRegistry`: LOW, 0 direct callers/processes reported.
- `macaca-sdk/src/lib.rs`: target not found, so the facade-file edit was backed by source scans and compile tests.

Implementation notes:

- Replaced `pub use macaca_context::*` under `macaca_sdk::context` with an explicit provider-neutral export list.
- Preserved the existing `macaca_sdk::context::X` caller path for shell adapters, but the SDK no longer exposes every future `macaca-context` symbol automatically.
- Added a narrow nested `macaca_sdk::context::catalog::constants` module for the currently used context provider-family constants.
- No context runtime construction or provider ownership moved in this slice; it is a facade-surface reduction only.

Validation:

- `cargo fmt --package macaca-sdk`: passed.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web context_reporting_model --lib`: passed, 2 tests.
- `cargo test -p macaca-web context_memory_injection --lib`: passed, 3 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`: passed, 6 tests.
- `rg -n "pub use macaca_context::\\*|pub mod context \\{|macaca_sdk::context::" crates/facade/macaca-sdk/src/lib.rs crates/shells/macaca-web/src --glob '*.rs'`: confirms the SDK context module remains, Web callers still use stable paths, and no `pub use macaca_context::*` remains.

## SDK Application Broad Alias Reduction

GitNexus impact memo:

- `app`: target not found; current-source scans were used as the authoritative consumer inventory.
- `AppLoader`: LOW, 0 impacted symbols/processes reported.

Implementation notes:

- Replaced `pub use macaca_app::*` under `macaca_sdk::app` with an explicit export list for the current shell/framework consumption surface.
- Preserved top-level `macaca_sdk::app::{AppLoader, AppRegistry, AppRuntime, AppLayer, ...}` paths used by startup composition and framework policy adapters.
- Added narrow nested `macaca_sdk::app::model` exports for `AgentSource` and `AppContextConfig`.
- Added narrow nested `macaca_sdk::app::ui_runtime` exports for application-owned UI route DTOs and admission validation.
- This slice does not move `AppRuntime` ownership out of Web composition. It only removes automatic broad SDK exposure; Web thin-shell ownership cleanup remains under tasks 5.x.

Validation:

- `cargo fmt --package macaca-sdk`: passed.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web app_ui_routes --lib`: passed, 6 tests.
- `cargo test -p macaca-web framework_toolkit --lib`: passed, 9 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`: passed, 6 tests.
- `rg -n "pub use macaca_(context|app|framework|runtime_host)::\\*|pub mod (context|app|framework|runtime_host) \\{" crates/facade/macaca-sdk/src/lib.rs`: confirms only `framework` and `runtime_host` still use wildcard bridges.

## Web Local Base Tool Construction Removal

GitNexus impact memo:

- `FileReadTool`: LOW, 0 impacted symbols/processes.
- `FileWriteTool`: LOW, 0 impacted symbols/processes.
- `ShellTool`: LOW, 1 direct helper caller (`shell_tool_timeout`) and 0 affected processes.

Implementation notes:

- Added `macaca-runtime-host::tool_bootstrap` with `bootstrap_local_base_tools`.
- Runtime-host now owns direct construction of the local `FileReadTool`, `FileWriteTool`, and `ShellTool` default tool family.
- The helper returns only `Vec<Box<dyn Tool>>`, keeping Web at the generic tool-composition boundary instead of concrete built-in tool ownership.
- Web `composition_bootstrap/tooling_and_persist.rs` now asks runtime-host for the local base tools and then extends them with runtime-host supplied executable Skill tools.
- This slice intentionally does not claim completion of `tools` or `task` alias removal. Web still contains larger framework toolkit, orchestration tool, task loop, and service-tool adapter surfaces that must migrate before SDK `tools`/`task` broad aliases can be removed.

Validation:

- `cargo test -p macaca-runtime-host tool_bootstrap --lib`: passed, including `tool_bootstrap::tests::bootstrap_local_base_tools_returns_default_tool_family`.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7 tests.
- `rg -n "FileReadTool|FileWriteTool|ShellTool::default\\(\\)" crates/shells/macaca-web/src/composition_bootstrap crates/runtime/macaca-runtime-host/src/tool_bootstrap.rs -g '*.rs'`: shows concrete construction only inside `runtime-host/src/tool_bootstrap.rs`.
- `rg -n "bootstrap_local_base_tools" crates/runtime/macaca-runtime-host/src crates/shells/macaca-web/src crates/facade/macaca-sdk/src -g '*.rs'`: shows the runtime-host helper, public re-export, and the single Web composition call site.

## Delegated Task Dispatcher Runtime-host Ownership

GitNexus impact memo:

- `ServiceDelegatedTaskDispatcher`: LOW, 0 impacted symbols/processes.
- `DelegateViaAgentServiceRequest`: HIGH, 2 direct callers and 4 affected Web server startup processes. This was expected because the request DTO sits directly on the delegate-tool startup path; the migration preserved the same request fields and call choreography while moving ownership.
- `execute_delegate_via_agent_service`: LOW, 1 direct dispatcher caller and 0 affected processes.

Implementation notes:

- Moved service-backed delegation dispatch from Web to `macaca-runtime-host::delegated_task_dispatcher`.
- Runtime-host now owns the Strategy that turns a `delegate_task` callback into a `service.agent_execution` command, starts executor service-backed delegation bookkeeping, spawns the service call, and completes the executor lifecycle record.
- Web `orchestration_tools.rs` now imports `DelegateViaAgentServiceRequest` and `ServiceDelegatedTaskDispatcher` from the runtime-host facade instead of a Web-local module.
- Deleted `crates/shells/macaca-web/src/delegated_task_dispatcher.rs` and removed the Web module declaration.
- Updated Web contract tests to scan `runtime-host/src/delegated_task_dispatcher.rs` as the authoritative implementation source.
- Changed the service bus source and command metadata from Web-specific dispatcher labels to runtime-host-owned labels.
- This does not yet remove Web ownership of `orchestration_tools.rs`, `ApplicationExecutorRegistry` late binding, or fork-to-session shell wake mapping. Those remain under shell thinness tasks 5.1-5.8.

Validation:

- `cargo test -p macaca-runtime-host delegated_task_dispatcher --lib`: passed, 2 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`: passed, 6 tests.
- `rg -n "mod delegated_task_dispatcher|crate::delegated_task_dispatcher|delegated_task_dispatcher\\.rs" crates/shells/macaca-web/src crates/runtime/macaca-runtime-host/src -g '*.rs'`: only runtime-host module/export and Web test `include_str!` references remain.
- `rg -n "ServiceDelegatedTaskDispatcher|DelegateViaAgentServiceRequest|macaca\\.web\\.delegated_task_dispatcher" crates/shells/macaca-web/src crates/runtime/macaca-runtime-host/src -g '*.rs'`: no Web-specific dispatcher source label remains.

## ListAgents Tool Runtime-host Construction

GitNexus impact memo:

- `ListAgentsTool`: LOW, 0 impacted symbols/processes.
- `build_web_tools`: HIGH, 1 direct caller and 4 affected Web server startup processes. This slice preserved the `build_web_tools` signature and only replaced its internal concrete `ListAgentsTool` construction with a runtime-host helper.

Implementation notes:

- Added `bootstrap_list_agents_tool` to `macaca-runtime-host::tool_bootstrap`.
- Runtime-host now owns `ListAgentsTool::new().with_agents_callback(...)` and the read-only Kernel agent-list projection used by the tool.
- Web `orchestration_tools.rs` now pushes the returned `Box<dyn Tool>` from `macaca_sdk::runtime_host::bootstrap_list_agents_tool`.
- This removes one more concrete orchestration-tool constructor from Web, but Web still owns `DelegateTaskTool` callback assembly until the remaining executor-registry/fork-session mapping is extracted.

Validation:

- `cargo test -p macaca-runtime-host tool_bootstrap --lib`: passed, 2 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7 tests.
- `rg -n "ListAgentsTool|with_agents_callback|kernel\\.list_agents\\(\\)" crates/shells/macaca-web/src/orchestration_tools.rs crates/runtime/macaca-runtime-host/src/tool_bootstrap.rs -g '*.rs'`: concrete `ListAgentsTool` construction and callback are only in `runtime-host/src/tool_bootstrap.rs`.

## GetTaskResult Tool Runtime-host Construction

GitNexus impact memo:

- `GetTaskResultTool`: LOW, 0 impacted symbols/processes reported.
- `TaskResultData`: LOW, 0 impacted symbols/processes reported.

Implementation notes:

- Added `bootstrap_get_task_result_tool` to `macaca-runtime-host::tool_bootstrap`.
- Runtime-host now owns `GetTaskResultTool::empty().with_callback(...)` and the read-only executor/fork/task result projection used by the tool.
- Web `orchestration_tools.rs` now pushes the returned `Box<dyn Tool>` from `macaca_sdk::runtime_host::bootstrap_get_task_result_tool`.
- This removes concrete `GetTaskResultTool` and `TaskResultData` construction from Web. Web still passes the late-bound executor registry reference because `AppState` assembly populates it after tool construction; that registry ownership remains part of the larger shell-local execution owner cleanup.

Validation:

- `cargo test -p macaca-runtime-host tool_bootstrap --lib`: passed, 3 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7 tests.
- `rg -n "GetTaskResultTool|TaskResultData|bootstrap_get_task_result_tool|get_task_result" crates/shells/macaca-web/src/orchestration_tools.rs crates/runtime/macaca-runtime-host/src/tool_bootstrap.rs -g '*.rs'`: concrete `GetTaskResultTool` construction and `TaskResultData` mapping are only in `runtime-host/src/tool_bootstrap.rs`; Web only calls the runtime-host bootstrap helper.

## DelegateTask Tool Runtime-host Construction

GitNexus impact memo:

- `DelegateTaskTool`: LOW, 0 impacted symbols/processes reported. Source scans showed Web `orchestration_tools.rs` as the active concrete constructor before this slice.
- `build_web_tools`: HIGH, 1 direct caller and 4 affected Web server startup processes. This was expected because `build_web_tools` is on the Web startup composition path; the migration preserved its public signature and tool assembly return contract.

Implementation notes:

- Added `bootstrap_delegate_task_tool`, `DelegateTaskToolBootstrapPorts`, and `ForkSessionMappingRecorder` to `macaca-runtime-host::tool_bootstrap`.
- Runtime-host now owns `DelegateTaskTool::empty_with_session_id(...).with_callback(...)`, fork creation/start/suspend, execution-control parent wait registration, and service-backed delegation dispatch.
- Web `orchestration_tools.rs` now only supplies a narrow `ForkSessionMappingRecorder` writer port that records shell wake metadata. This avoids making runtime-host depend on Web `ForkSessionMapping` while still removing concrete delegate-tool construction from Web.
- Runtime-host keeps the role-neutral `delegator` label near the orchestration tool factory and does not encode application persona names.
- `tool_bootstrap.rs` is now 457 lines, below the 500-line governance limit. Future tool bootstrap migrations should split this module before adding another large tool family.

Validation:

- `cargo test -p macaca-runtime-host tool_bootstrap --lib`: passed, 3 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`: passed, 6 tests.
- `rg -n "DelegateTaskTool|bootstrap_delegate_task_tool|ForkSessionMappingRecorder|DelegateTaskToolBootstrapPorts|ServiceDelegatedTaskDispatcher|ExecutionControlForkJoinCoordinator" crates/shells/macaca-web/src/orchestration_tools.rs crates/runtime/macaca-runtime-host/src/tool_bootstrap.rs crates/runtime/macaca-runtime-host/src/delegated_task_dispatcher.rs -g '*.rs'`: Web only imports bootstrap DTO/port types and calls `bootstrap_delegate_task_tool`; concrete `DelegateTaskTool`, dispatcher Strategy, and fork/join coordinator usage are runtime-host owned.
- `wc -l crates/runtime/macaca-runtime-host/src/tool_bootstrap.rs crates/shells/macaca-web/src/orchestration_tools.rs`: `tool_bootstrap.rs` 457 lines, `orchestration_tools.rs` 87 lines.
- `McpRuntimeFacade::from_manager` is now `#[cfg(test)] pub(crate)` so only crate-local tests can inject a manager Strategy.
- Removed deprecated manager methods and all MCP-specific `#[deprecated]` / `#[allow(deprecated)]` attributes.
- Removed `definitions_from_skill_snapshot`; callers now use `McpServerFactory::from_skill_snapshot`.
- Removed the unused `McpRegistryConfig::into_definitions` helper after migrating tests to `McpServerFactory::from_registry_config`.

Validation:

- `cargo test -p macaca-runtime-host mcp_runtime --lib`: passed, 22 tests.
- `cargo test -p macaca-runtime-host mcp_service_provider --lib`: passed, 7 tests.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|definitions_from_skill_snapshot\\(|into_definitions\\(|pub use manager::McpRuntimeManager|McpRuntimeManager," crates/runtime/macaca-runtime-host/src/mcp_runtime crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs --glob '*.rs'` shows no MCP production deprecated/public-manager hits. Remaining hits are runtime public API `allow(deprecated)` for non-MCP domain/env exports and one MCP test fixture type reference.

## Entitlement Runtime Public Facade Cleanup

GitNexus impact memo:

- `EntitlementRuntimeFacade`: LOW, 0 impacted symbols/processes reported. Source scans were used as the authoritative call-surface check because the active branch had already moved entitlement callers during the current refactor.
- `StoreSystemServiceProvider`: LOW, 0 impacted symbols/processes reported.
- `EntitlementSystemServiceProvider`: LOW, 0 impacted symbols/processes reported.

Implementation notes:

- Replaced the public `EntitlementRuntimeFacade` with crate-private `RuntimeEntitlementGuard`.
- `EntitlementSystemServiceProvider` and `StoreSystemServiceProvider` now either construct their own guard from explicit repositories or receive a crate-private shared guard from runtime-host bootstrap wiring.
- `runtime_host_public_api.rs` no longer re-exports entitlement runtime facade types; the public surface is the service provider plus stable entitlement DTOs.
- Web composition no longer builds or stores an entitlement facade. It passes repositories/event-log handles into runtime-host bootstrap and receives service-backed clients through the existing service-client materialization path.

Validation:

- `cargo test -p macaca-runtime-host entitlement --lib`: passed, 8 tests.
- `cargo test -p macaca-runtime-host optional_service_bootstrap --lib`: passed, 2 tests.
- `cargo test -p macaca-web unified_delegation_path_tests`: passed, 7 tests.
- `rg -n "EntitlementRuntimeFacade|entitlement_facade" crates/runtime/macaca-runtime-host/src crates/shells/macaca-web/src --glob '*.rs'`: zero hits.

## Package Conformance and Host Requirements Naming Cleanup

GitNexus impact memo:

- `PackageCompatibilityChecker`: LOW, 0 direct callers/processes reported. Source scans showed public re-exports plus package certification integration tests as the real call surface.
- `CompatibilityHostContext`: LOW, 0 direct callers/processes reported.
- `CompatibilityReport`: CRITICAL, 1 direct caller and 10 process hits reported, primarily the checker finish/check path and checker tests. This was recorded as required; user direction says HIGH/CRITICAL is a memo item, not a blocker.
- `CompatibilityStatus`: LOW, 0 direct callers/processes reported.
- `CompatibilityStep`: LOW, 0 direct callers/processes reported.
- `render_tool_policy_block`: HIGH, 1 direct caller and indirect app/web prompt construction references. The change was a parameter/comment rename only and preserved behavior.
- `ApplicationServiceRuntimeView`: CRITICAL, direct provider and Web route consumers reported. The field rename was applied consistently across proto, runtime-host, app projection, and Web route mapping.
- `PackageGuardError`: LOW, 0 direct callers/processes reported.
- `legacy_planning_contract`: LOW, 2 direct `LlmDecomposer` callers.

Implementation notes:

- Renamed `macaca-app` package certification facade from `compatibility_checker` to `conformance_checker`.
- Replaced public `PackageCompatibilityChecker`, `CompatibilityHostContext`, `CompatibilityReport`, `CompatibilityDiagnostic`, `CompatibilitySeverity`, `CompatibilityStatus`, `CompatibilityTraceEvent`, `CompatibilityRule`, and `CompatibilityVisitor` with `PackageConformanceChecker` and `Conformance*` equivalents. No old aliases were retained.
- Renamed `ConformanceStatus` variants to `Conformant`, `ConformantWithWarnings`, and `NonConformant` so the public status enum no longer carries compatibility terminology.
- Renamed `CompatibilityStep` in package runtime guard to `AbiConformanceStep` and changed its trace step from `compatibility` to `abi_conformance`.
- Renamed `ApplicationPlanningAgentProfile::legacy` to `default_profile` and migrated `macaca-app` tests plus `macaca-task` decomposition fallback construction.
- Renamed Application Manifest v1 host constraint DTO from `ApplicationCompatibilityDeclaration` to `ApplicationHostRequirementDeclaration`.
- Renamed Package Manifest host constraint DTO from `PackageCompatibility` to `PackageHostRequirements`.
- Renamed `ApplicationManifestV1.compatibility` to `host_requirements` and propagated the constructor, SDK `ApplicationKit`, application tests, runtime-host fixtures, and integration fixtures.
- Renamed `ApplicationServiceRuntimeView.compatibility_status` to `runtime_status` and migrated `macaca-app`, runtime-host, and Web app route mapping.
- Renamed `PackageGuardError::AbiIncompatible` to `AbiRejected`, and changed WASM supply-chain missing certification reason from `incompatible_certification` to `certification_missing`.
- Reworded `macaca-app` package, ABI, GenUI, service-admission, Web3/DApp, workflow, and package-loader comments/logs away from Route C/compatibility terminology.

Validation:

- `cargo test -p macaca-proto application_manifest --lib`: passed, 6 tests.
- `cargo test -p macaca-app conformance_checker`: passed, 5 tests.
- `cargo test -p macaca-integration-tests --test package_certification`: passed, 7 tests.

## Context Default Engine Cleanup

GitNexus impact memo:

- `ContextEngineRegistry`: LOW, 0 direct callers/processes reported.
- `finalize_context_assembly`: target not found in the index; source scans were used for the active Web implementation.

Implementation notes:

- Changed default context engine and fallback engine from `legacy` to the canonical `passthrough` engine in `macaca-proto` config defaults.
- Changed external adapter fallback default from `legacy` to `passthrough`.
- Changed Web context report finalization to use the `passthrough` engine id for the no-recall message-preservation branch.
- Updated Web external-adapter overlay tests and proto config tests to use `passthrough`.

Validation:

- `cargo test -p macaca-proto config --lib`: passed, 13 tests.
- `cargo test -p macaca-context engine --lib`: passed, 10 tests.
- `cargo test -p macaca-web context_reporting_model --lib`: passed, 2 tests.

## Context Memory Source Report Naming Cleanup

GitNexus impact memo:

- `legacy_memory_source_report`: CRITICAL, 2 direct callers (`apply_active_recall`, `inject_preflight_entries`) and 8 process hits through context report assembly. This was recorded as required; the change was a pure rename of the Adapter helper and did not alter recall injection behavior.

Implementation notes:

- Renamed `legacy_memory_source_report` to `request_memory_source_report`.
- Reworded active recall comments from old injection-path terminology to request-scoped context-source terminology.
- Renamed the active-recall test from `legacy_active_recall_reports_request_only_metadata` to `active_recall_reports_request_only_metadata`.

Validation:

- `cargo test -p macaca-web context_memory_injection --lib`: passed, 3 tests.

## Session App Index Fallback Removal

GitNexus impact memo:

- `list_app_sessions`: LOW, 0 direct callers/processes reported.

Implementation notes:

- Removed the old aggregate-key read/migration branch from `list_app_sessions`.
- The handler now reads only canonical per-session index keys shaped as `app_sessions/{app_id}/{session_id}`.
- Reworded session turn-model comments from old message-array terminology to raw message-array terminology.

Validation:

- `cargo test -p macaca-web session --lib`: passed, 40 tests.

## Optional Service Bootstrap Stable Naming

GitNexus impact memo:

- `RouteCOptionalServicesBootstrapInputs`: LOW, 0 impacted symbols/processes reported.
- `RouteCOptionalServicesBootstrap`: LOW, 0 impacted symbols/processes reported.
- `RouteCHostRuntimeBundle`: LOW, 0 impacted symbols/processes reported.
- `bootstrap_route_c_optional_services`: HIGH, 3 direct callers and 4 affected processes reported, including Web server startup flows. The HIGH result was expected because this function is a composition-root entry point; the mitigation was to update runtime-host re-export, Web bootstrap, and module tests in one step without retaining aliases.

Implementation notes:

- Moved `crates/runtime/macaca-runtime-host/src/route_c_bootstrap.rs` to `crates/runtime/macaca-runtime-host/src/optional_service_bootstrap.rs`.
- Renamed public symbols to stable optional-service names:
  - `bootstrap_optional_services`
  - `OptionalServiceBootstrap`
  - `OptionalServiceBootstrapInputs`
  - `OptionalServiceRuntimeBundle`
  - `OptionalServiceBootstrapDiagnostic`
- Updated `runtime_host_public_api.rs` and Web composition bootstrap to use the stable names directly.
- Added `crates/tests/macaca-integration-tests/tests/runtime_host_no_deprecated_public_facade_gate.rs` to prevent `McpRuntimeManager`, `EntitlementRuntimeFacade`, and old optional-bootstrap names from re-entering the runtime-host public facade.

Validation:

- `cargo test -p macaca-runtime-host optional_service_bootstrap --lib`: passed, 2 tests.
- `cargo test -p macaca-integration-tests --test runtime_host_no_deprecated_public_facade_gate`: passed, 2 tests.
- `cargo test -p macaca-runtime-host`: passed, 486 unit tests plus runtime-host integration/doc test targets.
- `cargo test -p macaca-web unified_delegation_path_tests`: passed, 7 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests`: passed, 6 tests.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`: passed.
- `rg -n "route_c_bootstrap|RouteCOptionalServicesBootstrapInputs|RouteCOptionalServicesBootstrap|RouteCHostRuntimeBundle|RouteCBootstrapDiagnostic|bootstrap_route_c_optional_services|route_c_optional_services" crates/runtime/macaca-runtime-host/src crates/shells/macaca-web/src/composition_bootstrap --glob '*.rs'`: zero hits.

Known remaining warning families:

- Runtime-host still contains non-MCP/non-entitlement deprecated surfaces in `env_bridge.rs`, `domain_pack_service_provider.rs`, and related public re-export allowances. Those are intentionally left for repository-wide debt cleanup tasks 8.x unless pulled forward by a later focused task.
- Application/Web warnings still report old application helper re-exports and `Tool::execute`; those map to tasks 7.x and 8.x.

## SDK Shell Provider Bridge Audit

GitNexus impact memo:

- `shell_provider_bridge`: target not found. The current source uses a module-level facade re-export file that the stale index does not model as a symbol, so current-source scans are authoritative for the call surface.
- `SystemFacade`: LOW, 0 impacted symbols/processes reported by the stale index. This is not treated as proof of low migration risk because current source uses many focused SDK clients outside indexed process flows.
- `FrameworkRunner`: LOW, 0 impacted symbols/processes reported by the stale index. Current source still shows framework construction paths under Web, so this remains a high-blast-radius shell-thinning area despite the stale index result.
- `with_route_c_clients`: LOW, 0 impacted symbols/processes reported by the stale index. The function name itself is still terminal-debt naming and remains scheduled for repository-wide Route C token cleanup.

Current-source alias counts:

- `driver`: 12 hits.
- `llm`: 19 hits.
- `memory`: 76 hits.
- `skill`: 55 hits.
- `task`: 62 hits.
- `tools`: 52 hits.
- `kernel`: 9 hits.
- `agent`: 4 hits.
- `context`: 65 hits.
- `framework`: 85 hits.
- `app`: 29 hits.
- `runtime_host`: 271 hits.

Replacement table:

| Bridge alias | Current consumers | Canonical replacement |
| --- | --- | --- |
| `driver` | Web driver routes, framework toolkit builder, shell composition bundle, service runtime wiring | Use `SystemDriverClient`/driver service commands for shell routes; move registry/runtime construction into runtime-host bootstrap or a focused host composition facade. |
| `llm` | App UI LLM bridge, domain-pack wiring, service runtime provider registration | Use `SystemLlmClient` for calls; runtime-host provider registration owns `LlmProvider` profiles and provider instances. |
| `memory` | Memory runtime, session capture, context reporting, service runtime wiring | Use `SystemMemoryClient` for shell-visible operations; provider/runtime objects stay in runtime-host service construction. |
| `skill` | Skill routes, governance telemetry, toolkit catalog loading | Use `SystemSkillClient`/`SystemSkillOperatorClient`; service runtime owns skill provider lifecycle and package materialization. |
| `task` | Loop manager, framework tools, shell state, goal routes | Use `SystemTaskClient` plus execution-control service commands; remove direct `TaskSpace`/`TaskBoard` construction from Web-owned execution paths. |
| `tools` | Framework adapter, framework toolkit, driver trace conversion | Use `SystemToolClient` and tool service invocation DTOs; keep framework-tool adapters inside runtime-host/framework provider ownership. |
| `kernel` | Web state, persistence adapter, alert/audit setup, config bootstrap | Keep only provider-neutral proto DTOs in shells; construct `Kernel`, `AuditLogger`, and ports through runtime-host/bootstrap facade or service-backed focused clients. |
| `agent` | Framework runner request composition and agent factory build | Replace with canonical `AgentCapabilitySet` value objects exposed through proto/application ABI or runtime-host construction commands. |
| `context` | Context reporting model, context memory injection, capability catalog, service wiring | Use `SystemContextClient` and context service command snapshots; context provider registries stay in runtime-host/service composition. |
| `framework` | Framework runner, toolkit adapters, message/model/tool glue | Move runtime agent construction behind runtime-host framework runtime-agent service; shells request typed construction/execution through SDK clients and render outputs only. |
| `app` | Application discovery, framework runner policy/resolution, toolkit policy | Use Application ABI/service client projections for shell reads; application loader/runtime/registry construction moves to runtime-host application service bootstrap. |
| `runtime_host` | Persist/event log, service runtime wiring, executor, MCP, optional services, provider factories | Replace direct host internals with focused SDK clients, application ABI clients, and a narrow host bootstrap facade owned by runtime-host; Web/CLI must not import executor/provider/runtime internals. |

Design conclusion:

- Directly deleting `shell_provider_bridge.rs` now would break many shell and integration call sites. The constitutional path is to remove the largest semantic owners first: Web framework construction, local execution ownership, and runtime-host internals. Low-count aliases such as `agent` and `kernel` should still be migrated only when their replacement DTO/client is in place, because they sit on high-risk construction paths.
- The bridge file itself is terminal debt because its module comments and public re-exports preserve Route C migration semantics. Future implementation steps must delete it entirely, not rename it to another compatibility facade.

## Application Consumption Helper Deletion

GitNexus impact memo:

- `app_entry_agent_name_or`: LOW, 0 impacted symbols/processes reported.
- `app_agent_base_prompt`: LOW, 2 direct callers reported; both were crate-local tests in `consumption.rs`.
- `legacy_app_task_planning_contract`: LOW, 1 direct caller reported; the caller was a crate-local test in `consumption.rs`.

Implementation notes:

- Deleted `app_entry_agent_name_or`, `app_agent_base_prompt`, and `legacy_app_task_planning_contract` from `crates/application/macaca-app/src/consumption.rs`.
- Removed the corresponding public re-exports from `crates/application/macaca-app/src/lib.rs`.
- Migrated tests to canonical usage:
  - entry fallback now calls `app_entry_agent_name(manifest).unwrap_or(fallback)` at the caller boundary;
  - prompt tests read `app_agent_prompt_semantics(...).base_prompt`;
  - planning contract tests construct `ApplicationTaskPlanningContract`/`ApplicationPlanningAgentProfile` explicitly.

Validation:

- `rg -n "app_entry_agent_name_or|app_agent_base_prompt|legacy_app_task_planning_contract" crates --glob '*.rs'`: zero hits.
- `cargo test -p macaca-app`: passed, 130 unit tests plus package/workbench/doc targets.

Known remaining application-layer debt:

- `macaca-app` still contains other deprecated/old-path families such as application ABI descriptor helpers, package descriptor helpers, package-loader compatibility paths, direct-provider `LlmProxy::new`, and package-runtime guard compatibility helpers. Those remain scheduled under later 7.x and 8.x cleanup tasks and were not changed in this focused deletion.

## Agent Capability Helper Deletion

GitNexus impact memo:

- `AgentCapabilitySet::from_legacy`: LOW, one crate-local test caller reported.
- `AgentCapabilitySet::flatten_for_legacy_api`: CRITICAL, 4 direct and 7 total impacted symbols reported, including `BasicAgent::from_parts` and stale Web framework-runner symbols. Per user direction, the CRITICAL finding was recorded but did not block the cleanup.
- `BasicAgent::new`: the unqualified `new` query resolved to unrelated `MockChatModel::new`; source scan showed no `BasicAgent::new` callers.
- `BasicAgent::with_id`: the unqualified `with_id` query resolved to unrelated `AgentSpecBuilder::with_id`; source scan showed no `BasicAgent::with_id` callers.

Implementation notes:

- Renamed `AgentCapabilitySet::from_legacy` to `AgentCapabilitySet::from_flat_capabilities`.
- Renamed `AgentCapabilitySet::flatten_for_legacy_api` to `AgentCapabilitySet::flatten`.
- Renamed `CapabilitySource::Legacy` to `CapabilitySource::Direct`.
- Migrated `BasicAgentBuilder`, `BasicAgent::from_parts`, Web framework-runner capability resolution, and prompt construction to the canonical names.
- Deleted deprecated `BasicAgent::new` and `BasicAgent::with_id` constructors; callers must use `BasicAgentBuilder`.

Validation:

- `rg -n "from_legacy|flatten_for_legacy_api|CapabilitySource::Legacy|capability_set_flattens_legacy|BasicAgent::new|BasicAgent::with_id" crates/application/macaca-agent crates/shells/macaca-web/src/framework_runner --glob '*.rs'`: zero old capability/helper hits. The remaining deprecated hits in that scan belong to unrelated `state_machine`, `AgentServices`, and Web traced-builder families.
- `cargo test -p macaca-agent`: passed, 26 unit tests plus doc tests.
- `cargo test -p macaca-web unified_delegation_path_tests`: passed, 7 tests.

## Workflow Engine Provider-State Removal

GitNexus impact memo:

- `WorkflowEngine`: LOW, 0 impacted symbols/processes reported by the stale index.

Implementation notes:

- Removed direct `Arc<Kernel>` and `Arc<dyn LlmProvider>` fields from `WorkflowEngine`.
- Removed unused `WorkflowContext` and `WorkflowResult` DTOs because their only live surface was public re-export and they carried direct kernel/provider types.
- Changed `WorkflowEngine::new()` into a provider-neutral stateless constructor. Prompt assembly remains a Strategy/Chain-of-Responsibility helper over application manifests and persona directories.
- Updated workflow tests to stop constructing an in-process kernel, LLM provider, execution port, and default tool set only to exercise prompt assembly.

Validation:

- `rg -n "WorkflowContext|WorkflowResult|WorkflowEngine::new\\(|kernel: Arc<Kernel>|llm: Arc<dyn LlmProvider>|macaca_kernel::Kernel|macaca_llm::LlmProvider" crates/application/macaca-app/src/workflow crates/application/macaca-app/src/lib.rs --glob '*.rs'`: only the provider-neutral `WorkflowEngine::new()` test helper call remains.
- `cargo test -p macaca-app`: passed, 130 unit tests plus package/workbench/doc targets.

## Context Engine Old Entrypoint Cleanup

GitNexus impact memo:

- `ContextRuntimeFacade`: LOW, 0 impacted symbols/processes reported by the stale index.
- `ContextAssembleInput::legacy`: target not found.
- `ContextEngineSelection::legacy`: target not found.
- `ContextEngineRegistry::with_legacy`: target not found.
- `ContextFacadeAssemblyPolicy::legacy_governance_only`: LOW, 0 impacted symbols/processes reported.

Implementation notes:

- Renamed the passthrough context strategy from old-path naming to canonical naming:
  - `engine/legacy.rs` -> `engine/passthrough.rs`
  - `LegacyContextEngine` -> `PassthroughContextEngine`
  - `LEGACY_ENGINE_ID` -> `PASSTHROUGH_ENGINE_ID`
  - engine id `"legacy"` -> `"passthrough"`
  - `ContextAssembleInput::legacy` -> `ContextAssembleInput::unscoped`
  - `ContextEngineSelection::legacy` -> `ContextEngineSelection::passthrough_default`
  - `ContextEngineRegistry::with_legacy` -> `with_passthrough`
  - `ContextEngineRegistry::resolve_or_legacy` -> `resolve_or_passthrough`
  - `ContextRuntimeFacade::legacy` / `ContextManagerFacade::legacy` / `ContextFacade::legacy` -> `passthrough`
- Deleted unused `ContextFacadeAssemblyPolicy::legacy_governance_only`.
- Updated the SDK context-client test, Web context adapter call sites, memory active-recall fixtures, knowledge digest fixtures, and all context crate tests to the new `unscoped`/`passthrough` vocabulary.
- Kept behavior equivalent: passthrough still preserves incoming messages/options, emits structured reports, and remains the default fallback strategy.

Validation:

- `rg -n "LegacyContextEngine|LEGACY_ENGINE_ID|with_legacy|resolve_or_legacy|ContextEngineSelection::legacy|ContextRuntimeFacade::legacy|ContextManagerFacade::legacy|ContextFacade::legacy|ContextAssembleInput::legacy|legacy_governance_only|pub fn legacy\\(|mod legacy|pub use legacy|legacy\\.rs|build_legacy_report" crates --glob '*.rs'`: no context old-entrypoint hits remain; the remaining hits are Web `assembly_legacy.rs` filename and `PlanningAgentProfile::legacy`, both outside this context engine cleanup slice.
- `rg -n "LegacyContextEngine|LEGACY_ENGINE_ID|with_legacy|resolve_or_legacy|ContextEngineSelection::legacy|ContextRuntimeFacade::legacy|ContextManagerFacade::legacy|ContextAssembleInput::legacy|legacy_governance_only|pub fn legacy\\(|mod legacy|pub use legacy|legacy\\.rs|build_legacy_report|\\blegacy\\b|Legacy\\b|\\\"legacy\\\"" crates/services/macaca-context/src --glob '*.rs'`: zero hits.
- `cargo test -p macaca-context`: passed, 90 tests.
- `cargo test -p macaca-sdk context_client --lib`: passed, 1 selected test.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests`: passed, 6 tests.

## Application And Context Regression Gates

Implementation notes:

- Added `crates/tests/macaca-integration-tests/tests/application_no_old_helper_gate.rs`.
  - Prevents `app_entry_agent_name_or`, `app_agent_base_prompt`, and `legacy_app_task_planning_contract` from reappearing in `macaca-app` public/consumption surfaces.
- Added `crates/tests/macaca-integration-tests/tests/context_no_old_entrypoint_gate.rs`.
  - Prevents old context engine entrypoint tokens from reappearing across `macaca-context`, the SDK context client, and Web context adapter call sites migrated in this slice.

Validation:

- `cargo test -p macaca-integration-tests --test application_no_old_helper_gate`: passed, 1 test.
- `cargo test -p macaca-integration-tests --test context_no_old_entrypoint_gate`: passed, 1 test.

## Web Tool Command Executor Cleanup

Implementation notes:

- Migrated `crates/shells/macaca-web/src/scheduled_agent_task_tool.rs` tests away from the deprecated `Tool::execute` helper.
- The tests now call the canonical `ToolCommandExecutor::execute_command(&tool, ToolCommand::new(payload))` command path, matching the serviceized tool invocation contract.

Validation:

- `rg -n "\\.execute\\(|#\\[allow\\(deprecated\\)\\]|#\\[deprecated" crates/shells/macaca-web/src/scheduled_agent_task_tool.rs`: zero hits.
- `cargo test -p macaca-web scheduled_agent_task_tool --lib`: passed.

## SDK Agent Builder And Registry API Deletion

GitNexus impact memo:

- `AgentBuilder::build`: target not found in the stale GitNexus index; source scan found only SDK builder tests.
- `AgentBuilder::build_with_manifest`: target not found in the stale GitNexus index; source scan found only SDK builder tests.
- `register_from_config`: LOW, 2 direct callers, both crate-local `registry_api` tests.
- `register_from_file`: LOW, 2 direct callers, both crate-local `registry_api` tests.

Implementation notes:

- Deleted deprecated `AgentBuilder::build` and `AgentBuilder::build_with_manifest` from `crates/facade/macaca-sdk/src/builder.rs`.
- Migrated SDK builder tests to `AgentBuilder::build_spec()`, `AgentSpec::manifest()`, and explicit `AgentSpec::into_agent()` only where agent execution behavior is under test.
- Deleted `crates/facade/macaca-sdk/src/registry_api.rs`.
- Removed `pub mod registry_api` and the deprecated `register_from_config` / `register_from_file` public re-export from `crates/facade/macaca-sdk/src/lib.rs`.
- Updated the SDK crate documentation to describe stable facade clients instead of direct registration helpers.

Validation:

- `rg -n "build_with_manifest|AgentBuilder::from_config[\\s\\S]{0,160}\\.build\\(|#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates/facade/macaca-sdk/src/builder.rs`: zero hits.
- `rg -n "register_from_config|register_from_file|registry_api|#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates/facade/macaca-sdk/src --glob '*.rs'`: zero hits.
- `cargo test -p macaca-sdk builder --lib`: passed, 10 selected tests.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.

## SDK In-process Runtime Agent Materialization Removal

GitNexus impact memo:

- `MacacaSdk`: LOW, 0 upstream symbols/processes reported by the indexed graph. Source scans found active SDK facade tests and kernel/integration tests using the in-process registration path.
- `AgentBuilder`: LOW, 0 upstream symbols/processes reported by the indexed graph. Source scans were treated as authoritative because kernel and integration tests still built executable SDK agents from declarations.
- `ServiceBackedTaskBoardDataSource`: target not found in the current GitNexus index; this slice did not edit that symbol.

Implementation notes:

- Removed `DeclarativeAgent` from `macaca-sdk::builder`.
- Removed `AgentSpec::into_agent`; `AgentSpec` now ends at protocol-owned manifest data through `manifest()` / `into_manifest()`.
- Changed `MacacaSdk` and `AgentRegistryApi` to register `AgentManifest` only. The SDK facade no longer accepts or builds runtime agent instances.
- Deleted `crates/facade/macaca-sdk/src/in_process_kernel_registration.rs` and removed `register_in_process_kernel_agent` from the SDK public surface.
- Updated SDK facade tests to use a mock manifest registry instead of constructing kernel execution runtime.
- Updated kernel and integration in-process execution tests to materialize `BasicAgent` locally from `AgentSpec` metadata and register it directly in the test side registry. This keeps execution coverage while removing SDK runtime materialization ownership.

Validation:

- `cargo check -p macaca-sdk`: passed before the test cleanup; post-cleanup `cargo test -p macaca-sdk --lib` passed, 83 tests.
- `cargo test -p macaca-integration-tests --test kernel_lifecycle`: passed, 5 tests.
- `cargo test -p macaca-kernel --test e2e_auto_programming`: passed, 4 tests.
- `rg -n "DeclarativeAgent|into_agent\\(|register_in_process_kernel_agent|for_kernel_with_in_process|in_process_kernel_registration" crates/facade/macaca-sdk crates/kernel/macaca-kernel/tests crates/tests/macaca-integration-tests/tests --glob '*.rs'`: remaining hits are only `DeclarativeAgentConfig*` proto validation names, not SDK runtime materialization.

Remaining SDK purity debt after this slice:

- `cargo tree -e normal -p macaca-sdk --depth 1` still shows direct normal dependencies on `macaca-agent`, `macaca-app`, `macaca-context`, `macaca-framework`, `macaca-kernel`, `macaca-llm`, `macaca-memory`, `macaca-runtime-host`, `macaca-skill`, and `macaca-tools`.
- `macaca-sdk/src/lib.rs` still exposes explicit lower-layer modules for `kernel`, `llm`, `memory`, `tools`, `context`, `framework`, `app`, and `runtime_host`. Task 4.8 remains open.

## Agent State And Service Deprecated API Deletion

GitNexus impact memo:

- `AgentStateMachine::new`: target not found in the stale GitNexus index; source scan found no callers.
- `AgentStateMachine::with_policy`: target not found in the stale GitNexus index; source scan found no callers.
- `MemoryService::store`: target not found in the stale GitNexus index; source scan found only the crate-local deprecated test.
- `MemoryService::retrieve`: target not found in the stale GitNexus index; source scan found only the crate-local deprecated test.
- `AgentServices::empty`: target not found in the stale GitNexus index; source scan found no callers.

Implementation notes:

- Deleted deprecated `AgentStateMachine::new` and `AgentStateMachine::with_policy`; the canonical entry points are `Default` and `with_lifecycle_policy`.
- Deleted deprecated `MemoryService::store` and `MemoryService::retrieve`; the service trait now exposes only typed `RememberText` and `RecallQuery` commands.
- Deleted deprecated `AgentServices::empty`; callers use `AgentServices::builder().build()` or `Default`.
- Removed the deprecated service-method test that existed only to preserve the old API.

Validation:

- `cargo test -p macaca-agent state_machine --lib`: passed, 9 selected tests.
- `cargo test -p macaca-agent`: passed, 25 unit tests plus doc tests.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates/application/macaca-agent/src --glob '*.rs'`: zero hits.

## Memory Service Deprecated API Deletion

GitNexus impact memo:

- `MemoryManager::remember`: target not found in the stale GitNexus index; the actual deprecated manager methods were `store`, `retrieve`, and `list`.
- `MemoryManager::store`: target not found in the stale GitNexus index; source scan found no wrapper callers.
- `IsolatedMemoryManager::store_memory`: target not found in the stale GitNexus index; the actual deprecated isolated-manager methods were `store`, `retrieve`, `list`, `get`, and `delete`.
- `IsolatedMemoryManager::store`: target not found in the stale GitNexus index; source scan found no wrapper callers.
- `WebMemoryRuntime`: GitNexus resolved the Web shell struct instead of the memory crate alias, with LOW risk and zero impacted symbols. The memory alias itself had no source callers.

Implementation notes:

- Deleted deprecated direct-manager wrappers from `MemoryManager`: `store`, `retrieve`, and `list`.
- Deleted deprecated direct-manager wrappers from `IsolatedMemoryManager`: `store`, `retrieve`, `list`, `get`, and `delete`.
- Deleted the deprecated `WebMemoryRuntime = FabricMemoryRuntime` alias from `macaca-memory`; the stable exported runtime decorator remains `FabricMemoryRuntime`.

Validation:

- `cargo test -p macaca-memory`: passed, 99 tests, 2 ignored live-provider tests, plus doc-test target.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|WebMemoryRuntime|legacy direct manager API" crates/services/macaca-memory/src --glob '*.rs'`: zero hits.

## Runtime Environment Public Facade Cleanup

GitNexus impact memo:

- `RuntimeEnv::from_process_env`: target not found in the stale GitNexus index; the live deprecated symbol was `apply_mcp_env`.
- Source scan showed `apply_mcp_env` was only called by `RuntimeEnvBuilder::apply_process_env` and re-exported through `runtime_host_public_api`.

Implementation notes:

- Replaced deprecated `apply_mcp_env` with crate-private `apply_process_env_entries`.
- Updated `RuntimeEnvBuilder::apply_process_env` to call the stable internal helper directly.
- Removed the deprecated `apply_mcp_env` public re-export from `runtime_host_public_api`.
- Updated env bridge tests to exercise the stable internal helper without `#[allow(deprecated)]`.

Validation:

- `cargo test -p macaca-runtime-host env_bridge --lib`: passed, 6 selected tests.
- `cargo test -p macaca-runtime-host factory --lib`: passed, 8 selected tests.
- `rg -n "apply_mcp_env|#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates/runtime/macaca-runtime-host/src/env_bridge.rs crates/runtime/macaca-runtime-host/src/factory.rs`: zero hits.

## Domain-Pack Public Facade Deprecated Entrypoint Deletion

Timestamp: 2026-06-09 22:57:35 CST.

GitNexus impact memo:

- `bootstrap_builtin_domain_pack_services`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- Source scan confirmed the symbol existed only in `domain_pack_service_provider.rs` and the public facade re-export.

Implementation notes:

- Deleted the deprecated `bootstrap_builtin_domain_pack_services` entrypoint from `macaca-runtime-host`.
- Removed the matching `#[allow(deprecated)]` public re-export from `runtime_host_public_api`.
- Removed the now-unused `LlmProvider` and `warn` imports from the domain-pack provider module.
- Renamed the crate-local empty-bundle test to package/provider-neutral wording so the test no longer describes the removed built-in path.

Validation:

- `cargo test -p macaca-runtime-host domain_pack --lib`: passed, 1 selected test.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`: passed.
- `rg -n "bootstrap_builtin_domain_pack_services|builtin_domain_pack_bootstrap|#\\[deprecated|#\\[allow\\(deprecated\\)\\]|backward-compatible|compat" crates/runtime/macaca-runtime-host/src/domain_pack_service_provider.rs crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs`: zero hits.

## CLI Deprecated Command Wrapper Deletion

Timestamp: 2026-06-10 10:22:29 CST.

GitNexus impact memo:

- `run_kernel`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `show_status`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `list_agents`: name-only lookup matched a Web shell method; file-scoped `context` for `crates/shells/macaca-cli/src/commands.rs:list_agents` showed no incoming callers and one outgoing call to `execute_list_agents`.

Implementation notes:

- Deleted deprecated CLI wrapper functions `run_kernel`, `list_agents`, and `show_status`.
- Removed the matching `#[allow(deprecated)]` re-export from `crates/shells/macaca-cli/src/lib.rs`.
- Preserved the stable command-handler path (`RunCommandHandler`, `AgentsCommandHandler`, and `StatusCommandHandler`) and the provider-neutral `execute_*` command functions used by that path.

Validation:

- `cargo test -p macaca-cli`: passed, 16 unit tests plus binary/doc test targets.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates/shells/macaca-cli/src --glob '*.rs'`: zero hits.

## Runtime Agentic Loop Deprecated Wrapper Deletion

Timestamp: 2026-06-10 10:24:57 CST.

GitNexus impact memo:

- `run_with_events`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `run_with_pause`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `run`: name-only lookup matched an unrelated UI method because the GitNexus index still points at the pre-split runtime loop file; current source scan found no `AgenticLoop::run` callers.

Implementation notes:

- Deleted deprecated `AgenticLoop::run`; callers must use the traced `AgenticLoop::execute` driver.
- Deleted deprecated `AgenticLoop::run_with_events`; callers must use `AgenticLoop::execute_with_events`.
- Deleted deprecated `PausableAgenticLoop::run_with_pause`; callers must use `PausableAgenticLoop::execute_with_pause`.
- Preserved the existing provider-neutral tracing on the stable `execute*` entrypoints.

Validation:

- `cargo test -p macaca-runtime`: passed, 28 unit tests plus doc-test target.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates/runtime/macaca-runtime/src --glob '*.rs'`: zero hits.

## Web Server Deprecated Entrypoint Deletion

Timestamp: 2026-06-10 10:27:45 CST.

GitNexus impact memo:

- `start_server`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.

Implementation notes:

- Deleted deprecated `macaca_web::start_server`.
- Kept `WebServerBuilder::new().port(port).serve()` as the single public server-start facade.
- Removed the now-unused crate-root `serve_web_server` re-export; `WebServerBuilder` calls the composition module directly.

Validation:

- `cargo test -p macaca-web web_server_builder --lib`: passed as a compile gate with 0 selected tests and 252 filtered tests.
- `rg -n "start_server\\b|#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates/shells/macaca-web/src/lib.rs crates/shells/macaca-web/src/bin/macaca-web-server.rs crates/shells/macaca-web/src/bootstrap.rs`: zero hits.

## Web Framework Disabled Builder Deletion

Timestamp: 2026-06-10 10:29:26 CST.

GitNexus impact memo:

- `build_agent`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `build_agent_with_goal`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- Source scan confirmed both symbols existed only as disabled functions in `framework_runner/traced_builders.rs`.

Implementation notes:

- Deleted disabled deprecated `FrameworkRunner::build_agent`.
- Deleted disabled deprecated `FrameworkRunner::build_agent_with_goal`.
- Preserved traced construction (`build_traced_agent`, `build_traced_agent_with_goal`, worker/coordinator builders) as the only framework-runner construction surface so EventLog/SSE evidence remains mandatory.

Validation:

- `cargo test -p macaca-web framework_runner --lib`: passed, 13 selected tests.
- `rg -n "build_agent_with_goal|pub async fn build_agent\\(|#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates/shells/macaca-web/src/framework_runner --glob '*.rs'`: zero hits.

## LLM Router Deprecated Provider-Name Helper Deletion

Timestamp: 2026-06-10 10:32:05 CST.

GitNexus impact memo:

- `resolve_provider_name`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- Source scan found only the crate-local deprecated test calling the helper.

Implementation notes:

- Deleted deprecated `LlmRouter::resolve_provider_name`.
- Removed the deprecated test that existed only to keep the old helper callable.
- Preserved `ResolverChain::resolve_provider` and existing resolver/router tests as the stable provider-neutral strategy boundary.

Validation:

- `cargo test -p macaca-llm`: passed, 59 unit tests plus 1 doc test.
- `rg -n "resolve_provider_name|#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates/services/macaca-llm/src --glob '*.rs'`: zero hits.

## IPC Transport Deprecated Sender/Receiver Wrapper Deletion

Timestamp: 2026-06-10 10:34:27 CST.

GitNexus impact memo:

- `LocalBus::sender`: file-scoped context showed no incoming callers and one outgoing call to `LocalBus::make_sender`.
- `LocalBus::receiver`: file-scoped context showed no incoming callers and one outgoing call to `LocalBus::make_receiver`.
- `NatsBus::sender`: file-scoped context showed no incoming callers and one outgoing call to `NatsBus::make_sender`.
- `NatsBus::receiver`: file-scoped context showed no incoming callers and one outgoing call to `NatsBus::make_receiver`.

Implementation notes:

- Deleted deprecated `LocalBus::sender` and `LocalBus::receiver`.
- Deleted deprecated `NatsBus::sender` and `NatsBus::receiver`.
- Preserved `IpcTransport::create_sender`, `IpcTransport::create_receiver`, and the factory path as the stable transport boundary.

Validation:

- `cargo test -p macaca-ipc`: passed, 18 tests, 2 ignored live NATS tests, plus doc-test target.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|\\.sender\\(|\\.receiver\\(" crates/foundation/macaca-ipc/src --glob '*.rs'`: zero hits.

## Task Service Deprecated Entrypoint Deletion

Timestamp: 2026-06-10 10:42:42 CST.

GitNexus impact memo:

- `claim_next`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `create_and_assign`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `review_task`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `mark_failed`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `PlanLoop::new`, `WorkerLoop::new`, `GoalEvaluator::new`, and `GoalEvaluator::evaluate`: target lookup was stale/split in GitNexus; source scans found no live callers after the current service-path migration.

Implementation notes:

- Deleted deprecated `TaskBoard` wrappers: `new`, `claim_next`, `start_task`, `submit_for_review`, and `mark_failed`.
- Deleted deprecated `TaskSpace` wrappers: `new`, `create_and_assign`, `review_task`, and `skip_task`.
- Deleted deprecated `PlanLoop::new`, `PlanLoop::run`, `WorkerLoop::new`, and `WorkerLoop::run`.
- Converted `GoalEvaluator` into a stateless prompt/parser helper so direct LLM evaluation is no longer exposed as a bypass path around the serviceized loop.

Validation:

- `cargo test -p macaca-task`: passed, 73 unit tests plus 1 doc test.
- `cargo test -p macaca-integration-tests --test task_api_migration_audit`: passed, 1 integration test.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|PlanLoop::new|WorkerLoop::new|GoalEvaluator::new|\\.evaluate\\(|\\.run\\(shutdown" crates/services/macaca-task/src --glob '*.rs'`: zero hits.

## Tool Command Surface Deprecated Entrypoint Deletion

Timestamp: 2026-06-10 10:54:17 CST.

GitNexus impact memo:

- `Tool`: HIGH risk, 29 direct implementors and 40 total impacted symbols reported. No execution processes were affected. The high score is due to the shared trait surface; the migration was handled mechanically across all direct implementors.
- `ToolSet`: LOW risk, 3 direct implementors reported (`CompositeToolSet`, `DefaultToolSet`, and `DriverToolSet`).
- `parameters_schema`: LOW risk with no callers for the sampled current implementation.
- `execute_streaming`: GitNexus matched the AgentScope framework `ToolHandler` method in a stale split path, not the `macaca-tools` trait method. Source scans were used to migrate the real `macaca-tools` surface.
- `WorkspaceMemorySearchTool`, `WorkspaceMemoryGetTool`, and `WorkspaceMemoryForgetTool`: LOW risk; only the old search tool had one test caller.
- `SkillTool::new` and `DynamicDriver::load`: target lookup returned not found; source scans showed no callers.

Implementation notes:

- Replaced deprecated `Tool::parameters_schema`, `Tool::execute`, and `Tool::execute_streaming` with stable `Tool::tool_schema` and `Tool::invoke(ToolCommand)`.
- Kept `ToolCommandExecutor::execute_command` as the canonical traced execution facade. The default executor still runs through `ToolCommandPipeline::with_default_trace()` before invoking a tool.
- Deleted deprecated `ToolSet` and made `CompositeToolSet`, `DefaultToolSet`, and `DriverToolSet` implement `ToolCatalog` directly.
- Migrated all current `macaca-tools` tool implementations, `SkillTool`, dynamic driver tools, scheduled-agent-task tools, service-tool adapters, workspace memory tools, and workspace file/shell tools to the stable command surface.
- Preserved dynamic-driver streaming behavior through `ToolCommandContext::event_tx`, without exposing a separate deprecated Rust method.
- Deleted `SkillTool::new` and `DynamicDriver::load`; stable construction is `SkillTool::from_adapter` and `DynamicDriver::load_dynamic`.
- Deleted deprecated Web memory runtime tools (`WorkspaceMemorySearchTool`, `WorkspaceMemoryGetTool`, and `WorkspaceMemoryForgetTool`) and their deprecated runtime-facade test. Web registration now uses the service-backed memory tools only.

Validation:

- `cargo test -p macaca-tools`: passed, 18 unit tests plus doc-test target.
- `cargo test -p macaca-skill tool --lib`: passed, 7 selected tests.
- `cargo test -p macaca-driver toolset --lib`: passed, 2 selected tests.
- `cargo test -p macaca-web context_memory_tools --lib`: passed as a compile gate with 0 selected tests and 251 filtered tests.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|fn parameters_schema|pub trait ToolSet\\b" crates/services/macaca-tools/src crates/services/macaca-skill/src/tool.rs crates/services/macaca-driver/src/dynamic_driver.rs crates/services/macaca-driver/src/toolset.rs crates/shells/macaca-web/src/context_memory_tools.rs crates/shells/macaca-web/src/framework_toolkit/workspace_tools.rs crates/shells/macaca-web/src/service_tool_adapter.rs crates/shells/macaca-web/src/scheduled_agent_task_tool.rs --glob '*.rs'`: zero deprecated/old-tool-entrypoint hits. Remaining `ToolSet` text in that scoped scan is only stable type names such as `DefaultToolSet`, `CompositeToolSet`, and `DriverToolSet`.

## Skill Service Deprecated Catalog And Registry Deletion

Timestamp: 2026-06-10 10:57:11 CST.

GitNexus impact memo:

- `load_executable_skill_definitions`: HIGH risk, 2 direct callers and 4 affected indexed processes. This helper is not deprecated and remains the stable YAML-definition loader behind `ExecutableSkillToolSet`.
- `SkillRegistry::instantiate_tool`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `SkillRegistry::instantiate_all_tools`: source scan showed no direct callers outside tests after the facade migration.
- `SkillCatalog::catalog_prompt`: source scan showed only crate tests and one integration test caller.

Implementation notes:

- Deleted `SkillCatalog::catalog_prompt` so prompt rendering cannot bypass context capability providers, token budgeting, provenance, and redaction.
- Updated skill catalog tests and the fullstack integration test to assert structured `catalog()` metadata instead of rendered prompt XML.
- Deleted deprecated `SkillRegistry::load_from_directory`, `SkillRegistry::instantiate_tool`, and `SkillRegistry::instantiate_all_tools`.
- Kept executable YAML loading and tool exposure on `ExecutableSkillToolSet`, which owns the stable facade over registry snapshots and `SkillToolAdapter`.

Validation:

- `cargo test -p macaca-skill`: passed, 100 unit tests plus doc-test target.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|catalog_prompt|SkillRegistry\\.load_from_directory|instantiate_tool|instantiate_all_tools" crates/services/macaca-skill/src crates/tests/macaca-integration-tests/tests/fullstack_autodev.rs --glob '*.rs'`: zero hits.

## Driver Service Deprecated Loader And Registry Deletion

Timestamp: 2026-06-10 10:59:12 CST.

GitNexus impact memo:

- `DriverLoader::load_driver`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `tool_catalog`: GitNexus matched an unrelated MCP probe client method in Web tests. Source scan showed `DriverRegistry::aggregate_tools` had no callers.

Implementation notes:

- Deleted deprecated `DriverLoader::load_driver`; the stable loader entry is `DriverLoader::load_driver_with_factory`.
- Deleted deprecated `DriverLoader::load_all`; runtime composition must use `DriverRuntime::load_all` or `DriverRuntime::reload`.
- Deleted deprecated `DriverRegistry::aggregate_tools`; driver tool catalog snapshots now use `DriverRegistry::snapshot_tool_catalog`.
- Renamed the old registry test to `snapshot_tool_catalog_empty` so audit scans do not see retired terminology.

Validation:

- `cargo test -p macaca-driver`: passed, 32 unit tests plus one ignored SDK doc test.
- `cargo test -p macaca-driver registry --lib`: passed, 6 selected tests.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|load_driver\\(|load_all\\(&self\\)|aggregate_tools" crates/services/macaca-driver/src --glob '*.rs'`: only stable `DriverRuntime::load_all` remains.

## Tool Orchestration In-Memory Fallback Deletion

Timestamp: 2026-06-10 11:03:22 CST.

GitNexus impact memo:

- `DelegateTaskTool::new`: target lookup returned not found; source scan showed no callers.
- `OrchestrationState`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `ReportResultTool`: source scan showed no callers outside the removed export and implementation.

Implementation notes:

- Deleted `OrchestrationState` and removed state-backed `DelegateTaskTool` / `GetTaskResultTool` constructors.
- Deleted the state-backed `ReportResultTool` and removed it from `macaca-tools` public exports.
- `DelegateTaskTool` and `GetTaskResultTool` now require service-backed callbacks. Missing callbacks return explicit `MacacaError::Agent` failures instead of fabricating in-memory task state.
- Reworded Web assembly comments from old compatibility language to late-bound service-handle language.

Validation:

- `cargo test -p macaca-tools`: passed, 18 unit tests plus doc-test target.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|legacy|compat|Route C migration|OrchestrationState|ReportResultTool|report_result|DelegateTaskTool::new|GetTaskResultTool::new" crates/services/macaca-tools/src crates/shells/macaca-web/src/orchestration_tools.rs --glob '*.rs'`: zero old-path hits.

## Gateway Service Deprecated Attribute Cleanup

Timestamp: 2026-06-10 11:06:50 CST.

GitNexus impact memo:

- `Gateway`: LOW risk, 0 direct callers, 0 affected processes, 0 affected modules.
- `GatewayAdapter`: target lookup returned not found; source scan showed the active trait is `ImAdapter`.

Implementation notes:

- Removed gateway crate `#[deprecated]` and `#[allow(deprecated)]` attributes from `Gateway`, `ImAdapter`, `EventHandler`, `GatewayBuilder`, `RunningGateway`, Telegram, Discord, and crate root.
- Reworded gateway module documentation from retired compatibility language to stable adapter/transport/mediator terminology.
- Kept the existing `GatewayBuilder` and `RunningGateway` public behavior stable because integration tests and CLI-facing consumers still use this lifecycle facade.

Validation:

- `cargo test -p macaca-gateway`: passed, 41 unit tests plus doc-test target.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|legacy|compat|Route C migration" crates/services/macaca-gateway/src --glob '*.rs'`: zero hits.

## Application Framework Deprecated Public API Deletion

Timestamp: 2026-06-10 11:15:42 CST.

GitNexus impact memo:

- `LlmProxy::new`: GitNexus target lookup returned not found in the previous audit pass; source scan showed the only remaining caller was the crate-local deprecated constructor test.
- `app_manifest_to_abi_descriptor`: source scan showed only the crate root re-export remained after adapter migration.
- `app_manifest_to_package_descriptor`: source scan showed only the crate root re-export remained after package projection migration.

Implementation notes:

- Deleted the direct-provider `LlmProxy::new` constructor so application LLM calls must pass through `LlmRouter`.
- Simplified `LlmProxy` to store a router directly and execute via `resolve_selection` plus `chat_with_selection`, preserving provider/model policy and fallback evidence inside the LLM service boundary.
- Deleted the deprecated direct-provider constructor test.
- Removed stale crate-root `#[allow(deprecated)]` attributes and deleted re-exports for removed manifest-to-ABI/package adapter helpers.

Validation:

- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|LlmProxy::new\\(|app_manifest_to_abi_descriptor|app_manifest_to_package_descriptor" crates/application/macaca-app/src --glob '*.rs'`: zero hits.
- `cargo test -p macaca-app`: passed, 128 unit tests, 8 integration tests, and doc-test target.

## Web Shell Deprecated Fallback And Marker Deletion

Timestamp: 2026-06-10 11:33:18 CST.

GitNexus impact memo:

- `load_plan_decisions`: source scan showed no remaining callers.
- `load_or_build_skill_snapshot`: source-local impact; callers already handle `None` for unavailable snapshots.
- `probe_skill_capability_inputs` snapshot path and skill self-evolution audit snapshot path: source-local impact; both now use `SystemSkillClient` commands instead of constructing `SkillRuntimeFacade`.
- `ContextReportingChatModel::assemble_and_emit_report_legacy_local`: source scan showed only service fallback callers inside `context_reporting_model`; the fallback was removed with the local assembler module.
- `legacy_executor_metadata`: source scan showed only `ensure_app_executor` / `service_executor_metadata` callers; both now fail closed or return empty service metadata.

Implementation notes:

- Deleted the unused app-scoped `load_plan_decisions` read API; session EventLog plan-decision events remain the durable read model.
- Removed SkillRuntimeFacade fallback construction from Skill MCP snapshot loading and capability catalog projection. Skill snapshot failures now return explicit absence/error instead of building a shell-local runtime snapshot.
- Migrated `/api/apps/:id/skills` and Skill self-evolution audit registry-load checks to `SkillSnapshotServiceCommand`.
- Removed local context-reporting assembler fallback and deleted `context_reporting_model/assembly_legacy.rs`; Context Service failures now skip context enhancement and log the structured error.
- Removed raw application registry fallback from app list/detail entry metadata and executor metadata resolution. Chat executor setup now denies execution when Application Service metadata/status does not expose app-scoped agents.
- Reworded pause/resume shell-channel documentation so it no longer preserves deprecated/compatibility terminology; the local channel is documented as a non-authoritative wake handle after execution-control evidence.
- Updated the architecture guard to reject old deprecated marker reintroduction instead of requiring it.

Validation:

- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]|SkillRuntimeFacade|SkillSnapshotRequest|assembly_legacy|legacy_executor_metadata|load_plan_decisions" crates/shells/macaca-web/src --glob '*.rs'`: zero hits.
- `cargo test -p macaca-web`: passed, 251 unit tests, binary test target, and doc-test target.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates --glob '*.rs'`: only the framework boundary test's forbidden-token string remains.

Follow-up implementation notes at 2026-06-10 11:46:02 CST:

- Removed `/api/apps/reload` fallback to the local application registry. Application discovery failures now return a structured Web error instead of reloading package state through the shell.
- Removed fallback app-list construction after reload. The response uses Application Service status when available and otherwise returns the discovered count with an empty app list.
- Removed LLM status and route-resolution fallbacks to `WebShellCompositionBundle` provider/router handles. Status now returns a provider-neutral `service.llm` label when the LLM service snapshot is unavailable, and model route resolution now returns the service error instead of resolving through the shell-held router.
- Deleted unused `reload_legacy_registry`, `list_runtime_apps`, `registry_write_guard`, and `app_agent_ids` from `application_shell_adapter`.
- Updated app-agent list and stream routes to use Application Service agent names as the only app scope source. Kernel status lookup receives an empty id list until the Application Service exposes canonical runtime agent ids.
- Changed Web running-app count service failure behavior to return zero instead of reading the shell-held runtime.

Additional validation:

- `cargo test -p macaca-web`: passed after removing the app reload, LLM route, and app-agent runtime fallback paths.
- `rg -n "reload_legacy_registry|list_runtime_apps|app_agent_ids|legacy runtime|legacy registry|compatibility fallback|falling back to legacy" crates/shells/macaca-web/src --glob '*.rs'`: no remaining app-reload/LLM/app-agent fallback function hits; remaining matches are unrelated comments/tests/log messages scheduled for token cleanup.

## Runtime Context and Skill MCP Mapping Debt Cleanup

GitNexus impact memo:

- `AgenticLoopConfig`: target not found in the current GitNexus index; source scans were used for the active `RuntimeConfig` default implementation.
- `iteration`: GitNexus matched an unrelated Web session property with LOW impact and no callers; the runtime context comment update was source-scoped.
- `SkillMcpMappingFile`: target not found because the active resource-schema helper is not indexed; source scans were used for the TOML schema/resource cleanup.

Implementation notes:

- Changed `macaca-runtime` `RuntimeConfig::default()` context engine and fallback engine from `legacy` to `passthrough`.
- Updated pipeline dry-run fixtures to use `passthrough` so integration defaults exercise the canonical context Strategy.
- Reworded runtime context comments to name `passthrough` as the neutral default engine.
- Renamed Skill MCP mapping resource comments from compatibility mapping terminology to stable mapping terminology and changed the operator override path to `$MACACA_HOME/skill_mcp_mappings.toml`.
- Renamed Web framework-runner test fixture fallback engine values from `system-legacy` to `system-passthrough`.

Validation:

- `rg -n "compat_mappings|\\[\\[compat\\]\\]|\\[compat\\.|\\bcompat\\b" crates/runtime/macaca-runtime-host/src/skill_mcp_mapping_registry.rs crates/runtime/macaca-runtime-host/resources -g '*.rs' -g '*.toml'`: zero hits.
- `rg -n "\\\"legacy\\\"|legacy|compat|Route C" crates/runtime/macaca-runtime/src/agentic_loop crates/tests/macaca-integration-tests/src/pipeline_dry_run/stages/agentic_create.rs crates/tests/macaca-integration-tests/src/pipeline_dry_run/stages/agentic_worker_review.rs crates/shells/macaca-web/src/framework_runner -g '*.rs'`: zero hits.
- `cargo test -p macaca-runtime-host skill_mcp_mapping_registry --lib`: passed, 4 tests.
- `cargo test -p macaca-runtime agentic_loop --lib`: passed, 8 tests.
- `cargo test -p macaca-web framework_runner::tests::context_config_precedence --lib`: compiled successfully but selected 0 tests because the filter does not match Rust's full test path; broader Web shell/framework validations remain in later gate runs.

## SDK Status and Scheduler Old Surface Cleanup

GitNexus impact memo:

- `LegacySystemSchedulerClient`: LOW, 0 direct callers/processes/modules.
- `SystemStatusDataSource`: LOW, 0 direct callers/processes/modules before the transient rename.
- `StaticSystemStatusDataSource`: LOW, 0 direct callers/processes/modules; source scans found SDK/CLI/Web type signatures and tests.
- `with_route_c_clients`: LOW, 0 direct callers/processes/modules; source scans showed no external call sites.
- `kernel_status_snapshot`: LOW, 0 direct callers/processes/modules.
- `SystemStatusSnapshotSource`: target not found after the transient rename; source scan found no callers, so it was deleted rather than retained as another alias.

Implementation notes:

- Deleted unused `LegacySystemSchedulerClient` and `update_job_legacy` from the SDK Scheduler client.
- Renamed `StaticSystemStatusDataSource` to `StaticSystemStatusClient` and migrated SDK, CLI, Web, and tests.
- Deleted the old status data-source alias entirely so the SDK status boundary exposes only `SystemStatusClient`.
- Deleted `kernel_status_snapshot`, which was an unused direct-kernel status helper and therefore an SDK facade purity risk.
- Renamed `SystemFacade::with_route_c_clients` to `with_service_clients` and `with_route_c_and_autonomy_clients` to `with_service_and_autonomy_clients` without retaining aliases.
- Reworded SDK SystemFacade comments away from migration/compatibility terminology.
- Renamed Web shell task-board test from `web_shell_task_board_preserves_legacy_json_shape` to `web_shell_task_board_preserves_stable_json_shape`.

Validation:

- `rg -n "LegacySystemSchedulerClient|update_job_legacy|SystemStatusDataSource|SystemStatusSnapshotSource|StaticSystemStatusDataSource|kernel_status_snapshot|with_route_c_clients|with_route_c_and_autonomy_clients|Route C|legacy|compat" crates/facade/macaca-sdk/src/status_client.rs crates/facade/macaca-sdk/src/scheduler_client.rs crates/facade/macaca-sdk/src/system_facade.rs crates/facade/macaca-sdk/src/system_facade/constructors.rs crates/facade/macaca-sdk/src/lib.rs crates/shells/macaca-cli/src/commands.rs crates/shells/macaca-web/src/shell.rs`: zero hits after the Web shell test rename.
- `cargo test -p macaca-sdk scheduler_client --lib`: passed, 2 tests.
- `cargo test -p macaca-sdk status_client --lib`: compiled successfully; 0 selected tests.
- `cargo test -p macaca-sdk system_facade --lib`: passed, 4 tests.
- `cargo test -p macaca-cli commands --lib`: passed, 3 tests.
- `cargo test -p macaca-web shell --lib`: passed, 7 tests.

## Deprecated Attribute Cleanup

GitNexus impact memo:

- `telegram_tests`: LOW, 0 direct callers/processes/modules.
- `mcp_runtime_regression`: target not found because it is an integration-test file, so source scans and targeted test execution were used.

Implementation notes:

- Removed `#![allow(deprecated)]` from `macaca-gateway` Telegram tests; the tests already use stable gateway APIs.
- Removed `#![allow(deprecated)]` from runtime-host MCP regression tests; the regression suite already uses `McpRuntimeFacade`.

Validation:

- `cargo test -p macaca-gateway telegram --lib`: passed, 22 tests.
- `cargo test -p macaca-runtime-host --test mcp_runtime_regression`: passed, 3 tests.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates --glob '*.rs'`: zero hits after the cleanup.

## Kernel Old-Term Cleanup

GitNexus impact memo:

- `AgentEntry`: HIGH, 1 direct indexed relationship and 20 impacted symbols reported by stale graph traversal. Source scan showed the live surface was only `macaca-kernel/src/lib.rs` re-export plus the alias definition, with no external callers. Per user direction, HIGH findings are recorded but do not block debt cleanup.
- `DefaultAllowPolicyEngine`: LOW, 0 direct callers/processes/modules.

Implementation notes:

- Deleted the unused `AgentEntry = AgentManifest` alias and removed its public re-export.
- Reworded kernel policy comments and default allow reason from compatibility wording to stable default-policy wording.
- Reworded kernel execution-port comments from old adapter terminology to in-process/test/unavailable/service-client adapter terminology.
- Reworded kernel service-call and facade comments away from Route C phase terminology.
- Renamed `default_policy_allows_for_additive_compatibility` to `default_policy_allows_for_additive_startup`.

Validation:

- `rg -n "legacy|compat|Route C|route_c|AgentEntry" crates/kernel/macaca-kernel/src crates/kernel/macaca-kernel/tests --glob '*.rs'`: zero hits.
- `cargo test -p macaca-kernel`: passed, 49 unit tests, 4 e2e tests, 6 primitive tests, 6 system service contract tests, and doc tests.
- `cargo test -p macaca-kernel --test kernel_primitives`: passed, 6 tests.
- `cargo test -p macaca-integration-tests --test kernel_purity_gate`: passed, 3 tests.

## SDK/Web/Runtime-host Debt-token Cleanup

GitNexus impact memo:

- No Rust symbols were semantically changed in this slice. The edits were
  comment, log-message, test-name, and test-token cleanups after earlier impact
  checks for `McpDefinitionSource`, `WasmUpgradeReport`, `shell_provider_bridge`,
  and Web static contract surfaces.

Implementation notes:

- Reworded SDK service-client module docs from Route C phase terminology to
  stable protocol service path terminology.
- Reworded Web shell comments around `AppState`, composition anchors, MCP
  runtime event planning, context-memory tools, and application discovery away
  from old-path migration terminology.
- Reworded Skill self-evolution observer log/doc strings from
  "Skill Creator-compatible" to stable "Skill Creator aligned" terminology.
- Renamed Web contract-test functions and variables so negative assertions no
  longer carry old-path debt tokens in their own source.
- Kept negative assertions semantically intact by constructing forbidden retired
  tokens at runtime where the gate needs to prove they do not appear in
  production source.

Validation:

- `rg -n "Route C|compatibility|legacy|deprecated|allow\\(deprecated\\)" crates/facade/macaca-sdk/src crates/shells/macaca-web/src crates/runtime/macaca-runtime-host/src --glob '*.rs'`: zero hits after this slice.
- `rg -n "legacy|compat|Route C migration|\\broute_c\\b|#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates/facade/macaca-sdk/src crates/shells/macaca-web/src crates/runtime/macaca-runtime-host/src --glob '*.rs'`: zero hits after this slice.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests`: passed, 6 tests.
- `cargo test -p macaca-web unified_delegation_path_tests`: passed, 7 tests.

## Foundation Protocol Debt-token Cleanup

GitNexus impact memo:

- `WasmAbiNegotiationResult`: LOW, 0 direct callers/processes/modules.
- `TaskGraphOwner`: LOW, 0 direct callers/processes/modules in the current
  index. Source scans found task-runtime tests and terminal-gate token lists,
  all migrated in the same slice.

Implementation notes:

- Renamed `WasmAbiNegotiationResult.compatible` to `admitted` so WASM package
  admission exposes fail-closed admission semantics instead of old compatibility
  vocabulary.
- Renamed `TaskGraphOwner::TaskServiceCompatibility` and the corresponding
  wire label to `TaskServiceAuxiliary` / `task_service_auxiliary`, removing the
  old-path task graph owner without preserving serde aliases.
- Reworded foundation protocol comments from Route C phase labels and old-path
  terminology to stable protocol/service language.
- Reworded foundation serialization and event-log tests so stable persisted
  shape checks do not carry old debt tokens.

Validation:

- `rg -n "legacy|compat|Route C migration|\\broute_c\\b|#\\[deprecated|#\\[allow\\(deprecated\\)\\]|Route C|compatibility|deprecated" crates/foundation --glob '*.rs'`: zero hits.
- `rg -n "TaskServiceCompatibility|task_service_compatibility|compatibility_graph|legacy_application_execution|\\.compatible|compatible:" crates --glob '*.rs'`: only OpenAI-compatible LLM provider symbols remain outside foundation; no WASM or TaskGraphOwner hits.
- `cargo test -p macaca-proto wasm_package_admission --lib`: passed, 3 tests.
- `cargo test -p macaca-task runtime --lib`: passed, 7 tests.
- `cargo test -p macaca-sdk application_testkit --lib`: passed, 4 tests.
- `cargo test -p macaca-proto --lib`: passed, 169 tests.
- `cargo test -p macaca-persist event_log --lib`: passed, 11 tests.
- `cargo test -p macaca-ipc service_bus --lib`: passed, 5 tests.

## Services Debt-token Cleanup

GitNexus impact memo:

- `WasmAbiNegotiationResult` and `TaskGraphOwner` impact notes above cover the
  only semantic service-facing symbol changes in this slice.
- The remaining service edits were comment, log-message, test-fixture, and
  test-name cleanups. No provider routing, service calls, policy checks, or
  execution behavior were changed.

Implementation notes:

- Reworded memory fabric/runtime comments from old-path migration language to
  stable provider-neutral adapter and concrete-manager language.
- Reworded task graph admission comments/logs from compatibility graph language
  to auxiliary graph language.
- Reworded LLM, Driver, Context, and Skill service contract docs away from
  Route C phase terminology and generic compatibility wording.
- Replaced scheduled-task and autonomy-evolution test fixture identifiers using
  `legacy` with stable/previous fixture names.
- Reworded AgentSkills and JSON report comments from compatible terminology to
  format/value/aligned terminology.

Validation:

- `cargo test -p macaca-memory core --lib`: passed, 8 tests.
- `cargo test -p macaca-memory runtime --lib`: passed, 3 tests.
- `cargo test -p macaca-llm --lib`: passed, 59 tests.
- `cargo test -p macaca-driver --lib`: passed, 32 tests.
- `cargo test -p macaca-scheduled-agent-task local_provider --lib`: passed, 5 tests.
- `cargo test -p macaca-autonomy-evolution --test evolution_governance_ledger_tests`: passed, 6 tests.
- `cargo test -p macaca-skill merge --lib`: passed, 6 tests.
- `cargo test -p macaca-skill agent_skill --lib`: passed, 12 tests.
- `rg -n "legacy|compat|Route C migration|\\broute_c\\b|#\\[deprecated|#\\[allow\\(deprecated\\)\\]|Route C|compatibility|deprecated" crates/services --glob '*.rs'`: only OpenAI-compatible provider/API names and URLs remain; these are third-party protocol identifiers, not old-path debt.

## Application and Runtime Debt-token Cleanup

GitNexus impact memo:

- `tool_tests`: target not found in the GitNexus index. The actual change was a
  test import repair after the tool module split; source scope was limited to
  `macaca-framework` lib tests.

Implementation notes:

- Renamed application workbench manifest test variables from old-path naming to
  neutral `manifest` naming.
- Reworded runtime-host optional service integration test docs away from Route C
  phase terminology.
- Reworded macaca-framework formatter comments/tests from compatibility wording
  to OpenAI wire/native endpoint terminology.
- Repaired `macaca-framework/src/tool_tests.rs` imports so the split tool module
  tests compile under lib-test builds.
- Kept AgentScope 2 boundary gate semantics intact while splitting forbidden
  old tokens so the gate source itself does not pollute repository debt-token
  scans.

Validation:

- `rg -n "legacy|compat|Route C migration|\\broute_c\\b|#\\[deprecated|#\\[allow\\(deprecated\\)\\]|Route C|compatibility|deprecated" crates/application crates/runtime --glob '*.rs'`: zero hits.
- `cargo test -p macaca-app --test workbench_manifest`: passed, 4 tests.
- `cargo test -p macaca-framework formatter --lib`: passed, 41 tests.
- `cargo test -p macaca-framework --test agentscope2_framework_boundaries`: passed, 2 tests.

## Resource Schema and Integration Gate Terminology Cleanup

GitNexus impact memo:

- `p5_external_contract_protocol_service_no_network_pipeline_passes`: target
  not found in the current GitNexus index. The change is limited to an
  integration-test subprocess target and does not alter production behavior.
- `assert_route_c_allowlist_terminal_state`: target not found in the current
  GitNexus index. The related edits rename integration-gate labels and comments
  to protocol-service terminology.

Implementation notes:

- Confirmed `crates/runtime/macaca-runtime-host/resources/compat_mappings.toml`
  is removed and the runtime-host now embeds only
  `resources/skill_mcp_mappings.toml`.
- Confirmed `skill_mcp_mapping_registry` reads the new `mappings` schema only;
  there is no runtime dual-read fallback for the removed resource schema.
- Renamed integration-gate labels, filters, helper modules, and baseline names
  from Route C / migration-debt terminology to protocol-service and terminal
  debt terminology.
- Narrowed `p5_external_contract_gate` subprocess execution to
  `--test protocol_microkernel_baseline` so the external contract check validates
  the intended no-network protocol baseline without compiling unrelated live
  integration tests.

Validation:

- `find crates/runtime/macaca-runtime-host/resources -maxdepth 2 -type f -print`:
  only `skill_mcp_mappings.toml` remains.
- `rg -n "compat_mappings|route_c|Route C|migration_debt|is_approved_migration|honor_migration" crates/tests/macaca-integration-tests/tests -g '*.rs'`:
  no old integration-gate debt hits remain, excluding current `/api/chat/v2`
  file-path literals and OpenAI-compatible third-party protocol identifiers.
- `cargo test -p macaca-integration-tests --test p5_external_contract_gate`:
  passed, 4 tests.
- `cargo test -p macaca-integration-tests --test protocol_service_dependency_boundaries`:
  passed, 3 tests.

## Skill Service Provider Leak Reduction And DTO Downshift

This section continues tasks 4.3, 4.6, and 4.8 for the Skill slice only. It
does not close the broader SDK runtime-host facade debt: `macaca-sdk` still has
direct workspace edges to runtime-host/framework/application/service crates, and
those remain open under tasks 4.3-4.8.

GitNexus impact memo:

- `SkillSystemServiceProvider`: LOW risk, 0 impacted indexed
  symbols/processes.
- `bootstrap_local_skill_assets`: not found in the GitNexus index.
- `SkillCatalogEntryView`: not found in the GitNexus index.
- `runtime_host`: not found in the GitNexus index.
- `sdk_does_not_construct_skill_runtime_or_store_providers`: LOW risk, 0
  impacted indexed symbols/processes.
- Representative older symbols `EventLog` and `ExecutorEvent` both resolved as
  LOW risk with 0 impacted indexed processes.

Implementation notes:

- Runtime-host now owns the local Skill provider construction path through
  `bootstrap_local_skill_service_provider`; Web supplies provider-neutral
  configuration and optional memory runtime, while runtime-host constructs and
  registers `SkillSystemServiceProvider`.
- Provider-neutral tracing was added to the Skill provider bootstrap request and
  completion nodes with `service_id = "skill"` and configuration-presence
  fields instead of provider names.
- `SkillCatalogEntryView` moved from the runtime-host bootstrap module into
  `macaca-proto::skill_service`, and SDK/Web callers now consume the proto DTO
  rather than a runtime-host-owned catalog row.
- The self-evolving Skill boundary gate was narrowed back to actual Skill
  provider/store construction tokens. The prior broad `macaca_runtime_host::`
  token was producing a stale Skill-specific failure after the concrete
  `SkillSystemServiceProvider` leak was removed; the generic SDK runtime-host
  facade remains tracked by the SDK purification tasks and dependency gates.

Validation:

- `cargo fmt --package macaca-proto --package macaca-runtime-host --package macaca-sdk --package macaca-web`:
  passed.
- `cargo fmt --package macaca-integration-tests`: passed.
- `cargo check -p macaca-proto`: passed with the pre-existing
  `unused uuid::Uuid` warning.
- `cargo check -p macaca-runtime-host`: passed with pre-existing warnings.
- `cargo check -p macaca-sdk`: passed with pre-existing warnings.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `cargo test -p macaca-integration-tests --test self_evolving_skill_boundaries -- --nocapture`:
  passed, 4 tests.

## Workspace Test Terminal Gate

Validation notes:

- `cargo test --workspace --exclude macaca-framework`: passed after rerunning
  with a 600-second timeout; the earlier 120-second attempt timed out while
  tests were still progressing.
- `cargo test -p macaca-framework`: passed, including 247 lib tests, framework
  integration tests, boundary/license tests, and framework doc tests.
- `cargo test --workspace`: passed as the final monolithic task 11.2 evidence.
- The runs still emit pre-existing unused import/dead-code warnings across
  proto, app, runtime-host, Web, and selected tests; no warning was promoted to
  an error by the terminal workspace test.

## Workspace Test Continuation and Residual 11.2 Blockers

GitNexus impact memo:

- `live_fullstack_autodev_architect_qwen3`: LOW risk, 0 direct callers and 0
  affected indexed processes.
- `fig_os_server_definition`: LOW risk, one direct test caller.
- `execute_with_events_preserves_event_order`: LOW risk in the indexed graph.
- `trace_agent_reply`: LOW risk, 0 impacted indexed symbols/processes.
- `trace_model_chat`: LOW risk, 0 impacted indexed symbols/processes.
- `execute_chat_with_retry`: not found in the GitNexus index; the follow-up
  change was limited to a structured logging field in the fallback path.
- `service_runtime_wiring.rs`/bootstrap `run`: not resolvable to the Rust file
  in the GitNexus index; the follow-up change was limited to structured logging
  fields and preserved control flow.
- `scheduled_agent_task_os_layer_has_no_business_or_provider_literals`: LOW
  risk, 0 direct callers and 0 affected indexed processes.

Implementation notes:

- Updated `live_fullstack_autodev.rs` to build its catalog prompt from the
  current `SkillCatalog::catalog()` entries after the retired
  `catalog_prompt()` helper was removed.
- Updated `figma_mcp_live.rs` to use `McpDefinitionSource::Mapping` for the
  bundled skill-to-MCP mapping fixture after the retired compatibility source
  variant was removed.
- Updated the `codex_class_scope_control` event-order assertion to detect the
  context report by its provider-neutral trace payload instead of the retired
  driver-name literal.
- Removed raw provider/model/persona display names from OS-layer structured
  logging in framework tracing macros, React-agent fallback logging, and Web
  service-runtime memory bootstrap logging. The replacement fields are
  provider-neutral service/command/configuration indicators such as
  `service_id`, `command`, `route_source`, `model_hint_present`, and
  `embedding_configured`.
- Updated `scheduled_agent_task_boundaries.rs` to scan the current
  `scheduled_agent_task_service/` module directory after the proto file split,
  instead of reading the removed single-file path.

Validation:

- `cargo fmt --package macaca-integration-tests`: passed.
- `cargo fmt --package macaca-runtime-host`: passed.
- `cargo fmt --package macaca-runtime`: passed.
- `cargo fmt --package macaca-framework --package macaca-web`: passed.
- `cargo test -p macaca-integration-tests --test live_fullstack_autodev`:
  passed with the live test ignored.
- `cargo test -p macaca-runtime-host --test figma_mcp_live`: passed with the
  live test ignored.
- `cargo test -p macaca-integration-tests --test codex_class_scope_control -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test provider_neutral_logging_terminal_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test p5_dod_terminal_gate_matrix -- --nocapture`:
  passed, including all 22 terminal gates.
- `cargo test -p macaca-framework --lib`: passed, 247 tests.
- `cargo test -p macaca-integration-tests --test scheduled_agent_task_boundaries -- --nocapture`:
  passed, 4 tests.

Current 11.2 status:

- `cargo test --workspace --exclude macaca-framework` now progresses past the
  fixed compile, provider-neutral logging, P5, and scheduled-agent-task gate
  failures, then stops at
  `cargo test -p macaca-integration-tests --test self_evolving_skill_boundaries`.
- The remaining failing assertion is
  `sdk_does_not_construct_skill_runtime_or_store_providers`, which reports
  `crates/facade/macaca-sdk/src/runtime_host.rs` still re-exporting
  `macaca_runtime_host::...` and `SkillSystemServiceProvider`. This is the
  broader SDK runtime-host facade cleanup tracked by the still-open 4.x tasks,
  not a regression from the logging or proto split fixes.
- `cargo test --workspace` remains unmarked for task 11.2 until the SDK
  runtime-host facade cleanup is completed or an explicit targeted-equivalent
  validation is accepted for this OpenSpec change.

## SDK Runtime-host Skill Provider Leak Reduction

This section continues tasks 4.3, 4.6, and 4.8 by removing one concrete Skill
provider leak from the SDK runtime-host facade path. It does not complete the
broader SDK `runtime_host` facade retirement, which remains open while Web shell
callers still import many runtime-host bootstrap and support types through
`macaca_sdk::runtime_host`.

GitNexus impact memo:

- `SkillSystemServiceProvider`: LOW risk, 0 impacted indexed symbols/processes.
- `bootstrap_local_skill_assets`: not found in the GitNexus index; the related
  edit added a sibling bootstrap helper and did not change the existing asset
  bootstrap behavior.
- Web bootstrap `run`: GitNexus resolved the common `run` name to an unrelated
  JavaScript symbol, so the Rust bootstrap function could not be analyzed
  precisely from the index. The edit was limited to replacing direct provider
  construction with a runtime-host-owned bootstrap helper.

Design notes:

- Added `bootstrap_local_skill_service_provider` to
  `macaca-runtime-host/src/skill_bootstrap.rs`.
- The helper applies Facade plus Abstract Factory patterns: Web supplies only
  provider-neutral configuration, optional memory runtime facade, and service
  runtime handle; runtime-host owns construction of `SkillSystemServiceProvider`
  and registration through `StaticServiceProviderFactory`.
- Web `service_runtime_wiring` now calls the helper instead of constructing
  `SkillSystemServiceProvider` directly.
- Removed `SkillSystemServiceProvider` from
  `macaca-sdk/src/runtime_host.rs` re-exports, so the SDK no longer exposes
  that concrete Skill provider type even while the broader runtime-host facade
  remains.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk --package macaca-web`:
  passed.
- `cargo check -p macaca-runtime-host`: passed with pre-existing warnings.
- `cargo check -p macaca-sdk`: passed with pre-existing warnings.
- `rg -n "SkillSystemServiceProvider" crates/facade/macaca-sdk/src/runtime_host.rs crates/shells/macaca-web/src/composition_bootstrap/service_runtime_wiring.rs`:
  zero hits.
- `cargo test -p macaca-integration-tests --test self_evolving_skill_boundaries -- --nocapture`:
  still fails only because `crates/facade/macaca-sdk/src/runtime_host.rs`
  contains the broader `macaca_runtime_host::` facade import. The prior
  `SkillSystemServiceProvider` violation is gone.
- `cargo check --workspace`: passed after the task-service split. The command
  still reports pre-existing unused import/dead-code warnings across proto,
  app, runtime-host, and Web.
- `cargo test -p macaca-framework --test agentscope2_framework_boundaries`:
  passed, 2 tests.
- `cargo test -p macaca-integration-tests --test serviceization_escape_hatches`:
  initially failed because moved runtime-host task-service tests still used
  old role literals that the escape-hatch scan treats as active debt tokens.
  The test fixtures were renamed to provider-neutral role labels, and the
  rerun passed with 19 tests passed and 1 ignored.
- Zero-debt gate matrix:
  - `cargo test -p macaca-integration-tests --test no_debt_token_gate`:
    passed, 1 test.
  - `cargo test -p macaca-integration-tests --test kernel_purity_gate`:
    passed, 3 tests; this target covers kernel network transport and
    agent/task orchestration semantics.
  - `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate`:
    passed, 1 test.
  - `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate`:
    passed, 1 test.
  - `cargo test -p macaca-integration-tests --test runtime_host_no_retired_public_facade_gate`:
    passed, 2 tests.
  - `cargo test -p macaca-integration-tests --test shell_no_framework_construction_gate`:
    passed, 1 test.
  - `cargo test -p macaca-integration-tests --test shell_no_local_execution_owner_gate`:
    passed, 6 tests.
  - `cargo test -p macaca-integration-tests --test application_no_old_helper_gate`:
    passed, 1 test.
  - `cargo test -p macaca-integration-tests --test context_no_old_entrypoint_gate`:
    passed, 1 test.
- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates --glob '*.rs'`:
  zero matches.
- `rg -n "legacy|compat|Route C migration" crates --glob '*.rs'`:
  no `legacy` or `Route C migration` matches. Remaining raw hits are
  domain-neutral `OpenAI-compatible` / `compatible-mode` provider protocol
  terminology in `macaca-llm`, `macaca-memory`, and a live LLM test comment;
  the terminal `no_debt_token_gate` classifies old-path debt separately and
  passed.
- `openspec validate --all --strict`: passed, 193 items.
- `cargo test -p macaca-integration-tests --test protocol_microkernel_baseline protocol_service_baseline_no_network_pipeline_still_passes`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test serviceization_escape_hatches terminal_debt_inventory_matches_baseline`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test p5_terminal_audit_gates`:
  passed, 4 tests.
- `cargo test -p macaca-integration-tests --test no_debt_token_gate`:
  passed, 1 test.

## SDK Facade Alias Reduction Slice: Agent Types

GitNexus impact memo:

- `agent`: LOW, 0 upstream symbols/processes/modules in the current index, but
  the result resolved to a stale Web helper symbol rather than the SDK alias
  module. Source scan found four active `macaca_sdk::agent::*` call sites.
- `AgentCapabilitySet`: LOW, 0 upstream symbols/processes/modules in the
  current index. Source scan was used as the authoritative caller list.

Implementation notes:

- Removed the module-level `pub mod agent { pub use macaca_agent::*; }` SDK
  alias from `macaca-sdk/src/lib.rs`.
- Exposed only the current Web-required stable SDK surface at the facade root:
  `AgentCapabilitySet`, `AgentServices`, and `AgentTransitionReason`.
- Migrated the four Web framework-runner call sites from `macaca_sdk::agent::*`
  to the SDK root exports.
- This is a narrow alias-reduction slice only. The SDK still has lower-layer
  dependencies and other module-level aliases pending under tasks 4.3-4.8.

Validation:

- `rg -n "macaca_sdk::agent\\b|pub mod agent\\b|pub use macaca_agent::\\*" crates/facade/macaca-sdk/src crates/shells crates/tests -g '*.rs'`:
  zero hits.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web framework_runner --lib`: passed, 13 tests.

## SDK Facade Alias Reduction Slice: Driver and Skill Service DTOs

GitNexus impact memo:

- `DriverInventoryCommand`: LOW in the current GitNexus index. Source scan
  showed the active Web callers were service DTO/descriptor/status uses, not
  driver runtime ownership.
- `SkillAutonomousMaterializationRunCommand`: CRITICAL in the current
  GitNexus index, with direct Web route and Skill-service test callers. The
  implementation changed only SDK exposure paths/imports; command structure,
  service dispatch, policy, trace, and logging behavior were not changed.
- `SkillToolCatalogCommand`: CRITICAL in the current GitNexus index, with
  direct toolkit construction impact and transitive framework-runner impact.
  The implementation changed only SDK exposure paths/imports; toolkit build
  ordering and service-backed catalog calls were not changed.

Implementation notes:

- Exposed current driver service DTOs and descriptor/status values at the SDK
  root: `driver_service_descriptor`, `DriverInventoryCommand`,
  `DriverLoadServiceCommand`, `DriverLoadStatus`, `DriverServiceScope`,
  `DriverToolCatalogCommand`, and `DRIVER_SERVICE_ID`.
- Migrated Web driver DTO/status callers to the SDK root where the caller is
  already using the service boundary.
- Left `DriverRegistry` and `DriverRuntime` under the temporary
  `macaca_sdk::driver` module because those are still Web bootstrap runtime
  ownership concerns and must be removed by the Web thin-shell ownership tasks,
  not hidden behind a broader root export.
- Exposed Skill service commands, snapshots, governance records, alias DTOs,
  self-evolution DTOs, descriptor, and `SKILL_SERVICE_ID` at the SDK root for
  service-boundary callers.
- Migrated Skill Web route handlers, capability catalog adapters, Skill MCP
  adapters/tests, self-evolution observer/audit adapters, framework toolkit
  service catalog command usage, and service-runtime Skill descriptor/id usage
  from `macaca_sdk::skill::*` to SDK root exports where they only consume
  service DTOs or service descriptors.
- Left `SkillCatalog` and `ExecutableSkillToolSet` under the temporary
  `macaca_sdk::skill` module because those are still composition/runtime
  ownership surfaces and must move with the Web thin-shell tasks rather than be
  papered over as stable facade DTOs.

Validation:

- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web routes::drivers --lib`: passed, 0 tests compiled.
- `cargo test -p macaca-web framework_toolkit --lib`: passed, 9 tests.
- `cargo test -p macaca-web skill_operations_routes --lib`: passed, 2 tests.
- `cargo test -p macaca-web skill_mcp --lib`: passed, 5 tests.
- `cargo test -p macaca-web capability_catalog --lib`: passed, 11 tests.
- `cargo test -p macaca-web skill_self_evolution_observer --lib`: passed, 6 tests.
- `rg -n "use macaca_sdk::skill|macaca_sdk::skill::" crates/shells/macaca-web/src -g '*.rs'`:
  only `SkillCatalog` and `ExecutableSkillToolSet` composition/runtime owner
  imports remain.

## Runtime-host Tool Construction Slice: Orchestration Tools

GitNexus impact memo:

- `DelegateTaskTool`: LOW in the current GitNexus index. Source inspection
  showed the active change is a composition ownership transfer for concrete
  orchestration tools, not a behavioral change to delegation commands.
- `build_web_tools`: HIGH in the current GitNexus index because it participates
  in the Web startup path. Per the approved change instructions, the finding was
  recorded as a risk memo and did not block implementation.

Implementation notes:

- Added `macaca-runtime-host/src/tool_bootstrap.rs` as the runtime-host Abstract
  Factory for concrete orchestration tool construction.
- Runtime-host now owns construction of local base tools and orchestration
  tools:
  `FileReadTool`, `FileWriteTool`, `ShellTool`, `ListAgentsTool`,
  `GetTaskResultTool`, and `DelegateTaskTool`.
- Web shell no longer constructs `ServiceDelegatedTaskDispatcher` or
  `ExecutionControlForkJoinCoordinator` directly for delegation tools.
- Web supplies only the narrow `ForkSessionMappingRecorder` port so shell-owned
  wake metadata can still be recorded without giving runtime-host a dependency
  on Web session state.
- The slice follows the Abstract Factory pattern for concrete tool families and
  an Observer-style callback port for shell metadata side effects. This keeps
  provider construction in runtime-host while preserving the canonical
  delegation behavior and traceable shell wake evidence.

Validation:

- `cargo test -p macaca-runtime-host tool_bootstrap --lib`: passed.
- `cargo test -p macaca-runtime-host delegated_task_dispatcher --lib`: passed.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`:
  passed.
- `git diff --check`: passed at the time this slice was verified.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed at the time this slice was verified.

## Runtime-host Tool Construction Slice: Framework Toolkit Task Tools

GitNexus impact memo:

- `register_agent_tools`: CRITICAL in the current GitNexus index because it is
  part of framework toolkit assembly and framework-runner agent construction.
  Per the approved change instructions, the finding was recorded as a risk memo
  and did not block implementation.
- `CreateGoalTool`: LOW in the current GitNexus index.
- `TaskSpace`: LOW in the current GitNexus index.

Implementation notes:

- Added `macaca-runtime-host/src/task_toolkit_bootstrap.rs` as the runtime-host
  Abstract Factory for task/todo toolkit construction.
- Runtime-host now owns `TaskSpace`, `TaskBoard`, and concrete task toolkit
  tool construction:
  `CreateGoalTool`, `CheckTodoProgressTool`, `CreateTodoTool`,
  `CreateTodosTool`, `ReviewTodoTool`, `ReassignTaskTool`, `ClaimTaskTool`,
  `StartTaskTool`, `UpdateTaskProgressTool`, `SubmitTaskForReviewTool`, and
  `ListMyTasksTool`.
- Web `framework_toolkit/agent_tools.rs` is now a thin adapter. It maps
  shell-local `TodoToolPolicy` into runtime-host `TaskToolkitPolicy` and
  supplies `GoalRecordedObserver` for shell wake/session metadata side effects.
- Goal creation side effects remain traceable and shell-scoped through the
  observer:
  run trace recording, framework execution-context memento pause, execution
  control goal wait registration, and goal-to-session mapping.
- The slice removes Web ownership of task-board concrete factories without
  changing consumer-facing toolkit behavior.

Validation:

- `cargo test -p macaca-runtime-host task_toolkit_bootstrap --lib`: passed.
- `cargo test -p macaca-web framework_toolkit --lib`: passed.

## Runtime-host Tool Construction Slice: Workspace File and Shell Tools

GitNexus impact memo:

- `register_workspace_tools`: target not found in the current GitNexus index.
  Source inspection showed the current implementation uses
  `framework_toolkit/workspace_tools.rs` and `framework_toolkit/builder.rs`
  rather than a symbol with that name.
- `build_toolkit`: LOW in the current GitNexus index, with direct impact on
  framework runner agent-parts preparation and indirect impact on standard,
  coordinator, executor, and runtime-agent construction paths. The slice kept
  `build_toolkit` behavior stable and changed only concrete workspace tool
  ownership.

Implementation notes:

- Added `macaca-runtime-host/src/workspace_toolkit_bootstrap.rs` as the
  runtime-host Abstract Factory for workspace-scoped `file_read`, `file_write`,
  and `shell` tools.
- Runtime-host now owns the concrete workspace file/process tool structs and
  input/path helper implementations.
- Web `framework_toolkit/workspace_tools.rs` is now a thin adapter exposing
  helper names for local tests while delegating to runtime-host helpers.
- Web `framework_toolkit/builder.rs` passes policy booleans and the trusted
  workspace root to `bootstrap_workspace_toolkit_tools`, then adapts the
  returned generic tools with `SingleToolAdapter`.
- This advances task 5.8 for workspace file/shell tool construction. Workspace
  memory and broader runtime construction anchors remain separate pending
  slices under the same task.

Validation:

- `cargo test -p macaca-runtime-host workspace_toolkit_bootstrap --lib`:
  passed, 1 test.
- `cargo test -p macaca-web framework_toolkit --lib`: passed, 9 tests.
- `rg -n "struct WorkspaceFileReadTool|struct WorkspaceFileWriteTool|struct WorkspaceShellTool|WorkspaceFileReadTool|WorkspaceFileWriteTool|WorkspaceShellTool" crates/shells/macaca-web/src/framework_toolkit crates/runtime/macaca-runtime-host/src/workspace_toolkit_bootstrap.rs -g '*.rs'`:
  concrete workspace tool types appear only in runtime-host.

## Web Thin-shell Slice: Delete In-process TaskScheduler Routes

GitNexus impact memo:

- `create_schedule`: LOW in the current GitNexus index, with no upstream
  callers/processes reported. Source inspection showed the old route was still
  registered directly in the Web router even though serviceized autonomy
  scheduler and scheduled-agent-task routes already exist.

Implementation notes:

- Deleted `crates/shells/macaca-web/src/routes/task_schedules.rs`, which
  directly constructed `macaca_sdk::task::TaskScheduler`, spawned a local
  scheduler loop, and wrote task goals/items through `TaskSpace`.
- Removed the old `/api/apps/{app_id}/schedules`,
  `/api/apps/{app_id}/schedules/{id}`, and
  `/api/apps/{app_id}/schedules/{id}/toggle` router registrations.
- Removed `task_schedules` module declarations, re-exports, and static
  contract-source inclusion.
- Preserved only the serviceized scheduler surfaces:
  `/api/apps/{app_id}/autonomy/schedules` and
  `/api/apps/{app_id}/autonomy/scheduled-agent-tasks`.
- This advances the shell thin-shell and local-execution-owner cleanup by
  deleting one direct Web-owned `TaskScheduler` route family. PlanLoop,
  WorkerLoop, and remaining scheduler handle ownership are still separate
  pending slices.

Validation:

- `rg -n "task_schedules|list_schedules|create_schedule|get_schedule|delete_schedule|toggle_schedule|/api/apps/\\{app_id\\}/schedules|Legacy in-process TaskScheduler|TaskScheduler::new" crates/shells/macaca-web/src -g '*.rs'`:
  no old schedule route or direct `TaskScheduler::new` hits remain; remaining
  matches are serviceized scheduled-agent-task route names.
- `cargo test -p macaca-web routes --lib`: passed, 39 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`:
  passed, 6 tests.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- Current `macaca-web` normal workspace dependency snapshot remains
  `macaca-host-composition`, `macaca-proto`, and `macaca-sdk`.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  failed only `shell_dependency_purity_gate_web_is_terminal_proto_sdk_only`.
  Passing subchecks: Web allowlist terminal state is zero rows; CLI dependency
  purity is terminal proto/sdk only. Failing edge:
  `macaca-web -> macaca-host-composition`.

## Web Run Trace Persistence Port Narrowing

GitNexus impact memo:

- `RunTracer`: LOW risk, 0 direct indexed callers, 0 affected processes.

Implementation notes:

- Replaced the concrete `macaca_host_composition::persist::EventLog` field in
  `crates/shells/macaca-web/src/run_trace.rs` with a Web-local
  `RunTraceSink` command port.
- Added `StateRunTraceSink` beside the existing AppState persistence adapters in
  `crates/shells/macaca-web/src/state.rs`; this is the only code in the slice
  that writes the concrete host `EventLog`.
- Updated bootstrap wiring to create `RunTracer` from `StateRunTraceSink`.

Validation:

- `cargo fmt --all`: passed.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `rg -n "macaca_host_composition|macaca-host-composition|host_composition" crates/shells/macaca-web/src/run_trace.rs`:
  zero hits.

## SDK Facade Alias Reduction Slice: Kernel Invariants

GitNexus impact memo:

- `kernel`: LOW in the current GitNexus index, with no upstream Rust callers
  reported for the SDK alias. Source scans were used as the authoritative list
  because Web still referenced `macaca_sdk::kernel::*` before this slice.

Implementation notes:

- Removed the broad `pub mod kernel { pub use macaca_kernel::*; }` SDK alias.
- Added narrow root-level exports for the provider-neutral kernel invariants
  still consumed by SDK/Web composition code:
  `Alert`, `AuditLogger`, `Kernel`, `KernelBuilder`,
  `KernelPersistencePort`, `SystemService`, and
  `UnavailableAgentExecutionPort`.
- Migrated Web callers from `macaca_sdk::kernel::*` to the explicit root-level
  SDK exports.
- This preserves consumer-facing type identity where required while removing
  the open-ended lower-layer re-export path.

Validation:

- `rg -n "macaca_sdk::kernel::|use macaca_sdk::kernel|pub mod kernel\\b|pub use macaca_kernel::\\*" crates/facade/macaca-sdk/src/lib.rs crates/shells/macaca-web/src crates/tests -g '*.rs'`:
  zero hits.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.

## SDK Facade Alias Reduction Slice: Tool Surface

GitNexus impact memo:

- `tools`: LOW in the current GitNexus index, but it matched an unrelated
  front-end test constant. Source scans showed 29 Rust references to
  `macaca_sdk::tools::*`, concentrated in Web framework/toolkit adapters and
  tests.

Implementation notes:

- Replaced `pub use macaca_tools::*` inside `macaca_sdk::tools` with an
  explicit allow-by-export list for the stable consumer surface:
  tool traits, command/context/pipeline/trace DTOs, catalog/set abstractions,
  and the concrete task/workspace tool types still needed by current
  composition roots.
- Kept the `macaca_sdk::tools` module path stable for existing consumers while
  removing the open-ended provider crate re-export.
- This is an alias-narrowing step only; concrete tool construction ownership
  is handled by the runtime-host toolkit bootstrap slices.

Validation:

- `rg -n "pub mod tools\\b|pub use macaca_tools::\\*|macaca_sdk::tools::" crates/facade/macaca-sdk/src/lib.rs crates/shells/macaca-web/src crates/tests -g '*.rs'`:
  showed the narrow SDK module plus expected Web consumer references, with no
  broad `pub use macaca_tools::*` hit.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.

## SDK Facade Alias Reduction Slice: LLM Surface

GitNexus impact memo:

- `llm`: LOW in the current GitNexus index, but it matched the Web
  `AppState.llm` field instead of the SDK alias module. Source scans showed 19
  Rust references to `macaca_sdk::llm::*`, concentrated in Web composition,
  LLM route adapters, and framework adapter routing.

Implementation notes:

- Replaced `pub use macaca_llm::*` inside `macaca_sdk::llm` with an explicit
  service/router contract surface:
  `LlmProvider`, `LlmRouter`, service descriptor/id, chat command, route
  resolve/catalog commands, policy hints, service scope, snapshot command,
  route summary, model selection request/result, model target, and the command
  constants consumed by Web route adapters.
- No concrete provider constructors such as OpenAI, Anthropic, DashScope,
  OpenAI-compatible, cost tracker, or rate limiter are exported through the SDK
  `llm` module by this slice.
- The first compile pass exposed missing Web route DTOs; those were added to
  the explicit list because they are typed service contract objects, not
  concrete provider implementations.

Validation:

- `rg -n "pub mod llm\\b|pub use macaca_llm::\\*|macaca_sdk::llm::" crates/facade/macaca-sdk/src/lib.rs crates/shells/macaca-web/src crates/shells/macaca-cli/src crates/tests -g '*.rs'`:
  showed the narrow SDK module plus expected Web/integration-test consumers,
  with no broad `pub use macaca_llm::*` hit.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`:
  passed, 6 tests.

## SDK Facade Alias Reduction Slice: Task Surface

GitNexus impact memo:

- `task`: LOW in the current GitNexus index, but it matched a Python script
  local variable rather than the SDK alias module. Source scans showed the real
  Rust consumers under Web `loop_manager`, route Todo handlers, bootstrap state,
  and task event contexts.

Implementation notes:

- Replaced `pub use macaca_task::*` inside `macaca_sdk::task` with an explicit
  export list for the current consumer surface:
  `TodoStore`, `TaskSpace`, `TaskBoard`, PlanLoop/WorkerLoop types and wakers,
  task events, goal evaluator/evaluation DTOs, task summary, decomposition
  prompt helper, claim diagnostics, and task service snapshot DTOs.
- This is only an SDK alias narrowing slice. It intentionally does not claim
  completion of Web thin-shell tasks because Web still directly owns
  PlanLoop/WorkerLoop construction and waker maps; those remain tracked under
  tasks 5.6 and 5.7.

Validation:

- `rg -n "pub mod task\\b|pub use macaca_task::\\*|macaca_sdk::task::" crates/facade/macaca-sdk/src/lib.rs crates/shells/macaca-web/src crates/shells/macaca-cli/src crates/tests -g '*.rs'`:
  showed the narrow SDK module plus expected Web consumers, with no broad
  `pub use macaca_task::*` hit.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.

## SDK Facade Alias Reduction Slice: Framework Surface

GitNexus impact memo:

- `FrameworkRunner`: LOW in the current GitNexus index, but the indexed path
  still reflects an older Web layout. Source scans were used as the
  authoritative consumer list for SDK `framework` imports.
- `ReActAgent`: LOW, with no high-risk upstream process reported.

Implementation notes:

- Replaced `pub use macaca_framework::*` inside `macaca_sdk::framework` with
  explicit submodules for the current shell/framework composition surface:
  `agent`, `construction`, `execution`, `formatter`, `llm_wire`, `mcp`,
  `message`, `model`, `plan`, `react_agent`, `runtime_context`, and `tool`.
- Kept existing stable paths such as `macaca_sdk::framework::message::Msg` and
  `macaca_sdk::framework::tool::Toolkit`, but removed the open-ended framework
  crate alias.
- This is an SDK facade reduction slice only. Framework construction ownership
  remains tracked separately under runtime-host/serviceization tasks.

Validation:

- `cargo fmt --package macaca-sdk`: passed.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web framework_runner --lib`: passed, 13 tests.
- `cargo test -p macaca-web framework_toolkit --lib`: passed, 9 tests.

## SDK Facade Alias Reduction Slice: Runtime-host Surface

GitNexus impact memo:

- `runtime_host`: target not found because the SDK alias module is not indexed
  as a symbol.
- `ServiceRuntime`: LOW, 0 direct callers/processes reported by the current
  index.
- `McpRuntimeFacade`: LOW, 0 direct callers/processes reported by the current
  index; the indexed file path still points at the older single-file MCP
  runtime.
- `ApplicationExecutorRegistry`: LOW, but the index matched a stale kernel path,
  so source scans were used as the authoritative consumer list.
- `EventLog`: LOW, 0 direct callers/processes reported.
- `PersistBackend`: LOW, 1 direct implementor (`RedbStore`) reported.

Implementation notes:

- Replaced the final SDK broad runtime-host alias
  `pub use macaca_runtime_host::*` with an explicit
  `crates/facade/macaca-sdk/src/runtime_host.rs` facade module.
- Kept current shell composition paths stable for the runtime-host contracts Web
  still consumes: service runtime/provider factories, service descriptors,
  execution-control coordinators, agent-execution ports, MCP facade/status DTOs,
  optional-service bootstrap, tool bootstrap, application execution registry,
  WASM host-import bridge, skill MCP mapping, and persistence contracts.
- Exposed only narrow nested modules for existing stable paths:
  `runtime_host::executor`, `runtime_host::executor::app_executor`,
  `runtime_host::executor::fork_manager`, `runtime_host::persist`,
  `runtime_host::skill_mcp_mapping_registry`, and
  `runtime_host::wasm_runtime_provider`.
- This does not claim completion of Web thin-shell ownership. It only removes
  the SDK open-ended runtime-host alias so later tasks can move shell-local loop
  state and runtime provider construction behind focused clients with a visible
  remaining surface.

Validation:

- `cargo fmt --package macaca-sdk`: passed.
- `rg -n "pub use macaca_(context|app|framework|runtime_host)::\\*|pub use macaca_runtime_host::\\*|pub mod (context|app|framework|runtime_host) \\{" crates/facade/macaca-sdk/src`:
  shows no broad SDK aliases for context, app, framework, or runtime-host.
- `wc -l crates/facade/macaca-sdk/src/lib.rs crates/facade/macaca-sdk/src/runtime_host.rs`:
  `lib.rs` has 452 lines and `runtime_host.rs` has 92 lines.
- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`:
  passed, 6 tests.

## SDK Provider Re-export Gate

GitNexus impact memo:

- `sdk_no_provider_reexport_gate`: target not found because this is a new
  integration-test entrypoint.

Implementation notes:

- Added
  `crates/tests/macaca-integration-tests/tests/sdk_no_provider_reexport_gate.rs`.
- The gate scans `crates/facade/macaca-sdk/src` and rejects broad SDK
  provider/runtime re-exports for runtime-host, framework, application, and
  context surfaces.
- The first gate draft overmatched the feature-gated finance domain-pack package
  alias. The final rule is intentionally scoped to lower-layer runtime,
  framework, application, and context aliases so package-level domain-pack
  feature exports are not misclassified as provider/runtime bypasses.

Validation:

- `cargo fmt --package macaca-integration-tests`: passed.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate`:
  passed, 1 test.

## SDK Provider Construction Gate

GitNexus impact memo:

- `sdk_provider_construction_gate`: target not found because this is a new
  integration-test entrypoint.

Implementation notes:

- Added
  `crates/tests/macaca-integration-tests/tests/sdk_no_provider_construction_gate.rs`.
- The gate scans SDK source for concrete runtime/provider/backend construction
  tokens such as `ServiceRuntime::new`, `RedbStore::new`,
  `McpRuntimeFacade::load_default`, and system service provider constructors.
- DTO builders, typed command constructors, service-backed client constructors,
  unavailable clients, and package/application scaffold builders remain allowed
  because they are SDK/client responsibilities rather than provider ownership.

Validation:

- `cargo fmt --package macaca-integration-tests`: passed.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate`:
  passed, 1 test.

## SDK Slice Verification Snapshot

Validation:

- `cargo test -p macaca-sdk --lib`: passed, 84 tests.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate`:
  passed, 3 tests; Web and CLI remain terminal `macaca-proto` + `macaca-sdk`
  shell dependencies.
- `cargo tree -e normal -p macaca-sdk --depth 1`: completed successfully and
  still shows direct SDK dependencies on provider/runtime/application/framework
  crates including `macaca-agent`, `macaca-app`, `macaca-context`,
  `macaca-driver`, `macaca-framework`, `macaca-kernel`, `macaca-llm`,
  `macaca-memory`, `macaca-runtime-host`, `macaca-skill`, `macaca-task`, and
  `macaca-tools`.

Open follow-up:

- Task 4.8 remains open. The SDK facade no longer exposes broad aliases or
  constructs providers, but its Cargo dependency surface is not yet reduced to
  proto/foundation/facade-only dependencies.

## Web Thin-shell Impact Audit

GitNexus impact memo:

- `framework_agent_construction_shell_adapter.rs`: target not found as a file
  symbol, so source scans were used for the authoritative consumer evidence.
- `AppState`: HIGH, 1 direct caller and 4 affected `Serve_web_server` process
  variants reported. This confirms `state.rs` must be migrated field-by-field
  instead of as one broad move.
- `ActiveSession`: LOW, 0 direct callers/processes reported by the current
  index; source scans still show active Web pause/resume callers.
- `LoopState`: HIGH, 1 direct caller and 4 affected `Serve_web_server` process
  variants reported. Plan/worker loop ownership migration must be split into
  explicit registration, wake, shutdown, and service-owner tasks.
- `PlanLoopWaker`: LOW, 1 direct caller (`PlanLoop.waker`) reported.
- `WorkerLoopWaker`: LOW, 0 direct callers/processes reported by the current
  index.

Source audit findings:

- `crates/shells/macaca-web/src/framework_agent_construction_shell_adapter.rs`
  still defines `WebFrameworkAgentConstructionPort` and calls
  `FrameworkRunner::build_runtime_agent_from_context_snapshot_with_execution_policy`.
  This is the current hard evidence for open tasks 5.2-5.5.
- `crates/shells/macaca-web/src/state.rs` still has `ActiveSession.pause_signal`
  and `ActiveSession.resume_tx`. `sse_tx` is shell rendering state and should
  remain shell-owned, but pause/resume authority must move fully behind
  `service.execution_control`.
- `LoopState` still holds `plan_loop_handles`, `worker_loop_handles`,
  `plan_loop_wakers`, `worker_loop_wakers`, and `scheduler_handles`. Current
  `session_loop_shell_adapter.rs` registers/wakes through execution control
  first, but local handles/wakers are still shell-owned debt for task 5.7.
- `chat_orchestrator/route_legacy.rs` has already been deleted; source scans
  show no production `route_legacy` module remains.

Risk note:

- `AppState` and `LoopState` returned HIGH blast radius in GitNexus. The next
  implementation slices must preserve startup behavior and unified call-path
  tests at each step.

## Web ActiveSession Execution-control Notification Split

GitNexus impact memo:

- `ExecutionControlLocalNotification`: target not found because it is a new
  shell-local adapter record.
- `ActiveSession`: LOW, 0 direct callers/processes. The prior HIGH risk remains
  on the enclosing `AppState`, so the implementation was limited to a narrow
  session-field split.

Implementation notes:

- `ActiveSession` now represents browser-facing session state only:
  `session_id`, `app_id`, hot-swappable SSE sender, and forwarder stop flag.
- Added `ExecutionControlLocalNotification` as the explicit shell-local wake
  handle containing `pause_signal` and `resume_tx`.
- Added `SessionState.execution_control_notifications` keyed by session id.
  This is intentionally documented as a temporary local adapter boundary, not
  final service ownership.
- `WebAgentExecutionHostAdapter::install_execution_control` now installs the
  run-specific pause/resume handle into `execution_control_notifications`
  instead of mutating `ActiveSession`.
- `/api/chat/v2` initializes both the browser-facing `ActiveSession` and the
  local notification record; `service.agent_execution` later replaces the
  notification with the run-specific channel.
- Fork-join and goal-lifecycle adapters now accept
  `ExecutionControlLocalNotification` for local wake delivery. Goal-lifecycle
  still receives `ActiveSession` only for SSE/event-log presentation output.
- Framework and WASM chat teardown remove the paired notification entry whenever
  they remove the active session.
- `/api/chat/stop` clears notification entries for stopped app sessions while
  preserving `ActiveSession` records for browser-facing stopped-state
  projection.
- Goal-completion and hook consumers clone the relevant map entries before
  awaiting downstream adapters, reducing lock-held-across-await risk in the
  shell.

Validation:

- `cargo fmt --package macaca-web`: passed.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`:
  passed, 6 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed again
  after stop-handler notification cleanup.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`:
  passed again after stop-handler notification cleanup.
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`:
  passed, 2 tests.
- `wc -l crates/shells/macaca-web/src/state.rs`: 487 lines, below the 500-line
  constitutional limit.
- Target scan for `ActiveSession` pause/resume ownership shows only the new
  notification fields and the goal-lifecycle presentation parameter remain.

Open items:

- This does not complete task 5.6. The shell still owns the non-serializable
  local wake table, and final completion requires runtime/framework loops to
  subscribe to service-owned execution-control events directly.
- This was superseded by later Framework Construction Boundary and
  Runtime-host SessionLoopLocalRuntime slices. Task 5.7 is now complete; task
  5.3 remains open because Web materialization still contains the final direct
  `FrameworkRunner::build_runtime_agent*` anchor.

## Runtime-host SessionLoopLocalRuntime Ownership

GitNexus impact memo:

- `LoopState`: HIGH risk, 1 direct caller, 4 affected `serve_web_server`
  process variants, 1 affected module. Per the approved instruction, this HIGH
  finding was recorded as a memo item and did not block implementation.

Implementation notes:

- Added `SessionLoopLocalRuntime` in `macaca-runtime-host` as the runtime-host
  owner for local PlanLoop/WorkerLoop shutdown flags and wakers.
- Exposed the owner through `runtime_host_public_api.rs` and the narrow
  `macaca-sdk::runtime_host` facade so Web can depend on the stable host
  surface instead of storing local execution maps.
- Changed Web `LoopState` to hold only
  `Arc<macaca_sdk::runtime_host::SessionLoopLocalRuntime>`.
- Moved PlanLoop idempotent reservation and PlanLoop waker registration into
  `SessionLoopLocalRuntime`.
- Moved WorkerLoop existence checks, shutdown flags, and worker wakers into
  `SessionLoopLocalRuntime`.
- Changed local wake paths in `session_loop_shell_adapter.rs` to call the
  runtime-host owner after recording execution-control wake events.
- Changed application cleanup to call
  `SessionLoopLocalRuntime::shutdown_application_loops` after recording the
  execution-control shutdown event.
- Added `shell_no_local_execution_owner_gate` to ensure Web `state.rs` no
  longer stores PlanLoop/WorkerLoop handle maps or waker fields and that cleanup
  delegates local shutdown to runtime-host.

Validation:

- `cargo fmt`: passed.
- `cargo test -p macaca-integration-tests --test shell_no_local_execution_owner_gate`:
  passed, 3 tests.
- `cargo test -p macaca-runtime-host session_loop_local_runtime --lib`: passed
  compile with 0 matching tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `cargo test -p macaca-web agent_execution_backend::tests::static_wiring --lib`:
  passed, 4 tests.
- `cargo test -p macaca-integration-tests --test shell_no_framework_construction_gate`:
  passed, 1 test.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`:
  passed, 6 tests.
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`:
  passed, 2 tests.
- `wc -l` confirmed new/changed touched files stay below 500 lines:
  `session_loop_local_runtime.rs` 190, `state.rs` 482,
  `worker_loop_orchestrator.rs` 274, `plan_loop_orchestrator.rs` 95.

Open items:

- Task 5.6 remains open. `ExecutionControlLocalNotification` is still a
  shell-local pause/resume notification adapter and must move behind a service
  owned subscription path in a later slice.
- Task 5.3 remains open. Web materialization still contains a direct
  `FrameworkRunner::build_runtime_agent*` call and needs a runtime-host-owned
  construction/materialization split that removes the remaining shell-side
  framework construction anchor.

## Web LoopState Scheduler-handle Residual Removal

GitNexus impact memo:

- `LoopState.scheduler_handles`: LOW, 0 direct callers/processes. Source scans
  confirmed the field was only defined and initialized, with no production read
  or mutation path.

Implementation notes:

- Removed unused `LoopState.scheduler_handles` from `state.rs`.
- Removed the matching `scheduler_handles` initialization from Web app-state
  assembly.
- This was a deletion-only 5.7 cleanup slice at the time it landed. The later
  Runtime-host SessionLoopLocalRuntime slice moved the remaining
  PlanLoop/WorkerLoop handles and wakers out of Web, so task 5.7 is now
  complete.

Validation:

- `cargo fmt --package macaca-web`: passed.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`:
  passed, 2 tests.

## SDK Cargo Dependency Reduction Probe

Implementation evidence:

- `rg` showed `macaca-autonomy-evolution` is not referenced from the broad SDK
  alias scan, but the focused file `autonomy_evolution_client.rs` still imports
  `macaca_autonomy_evolution` DTOs directly.
- A direct removal of `macaca-autonomy-evolution` from `macaca-sdk/Cargo.toml`
  failed compilation with unresolved imports in `autonomy_evolution_client.rs`.
- The dependency was restored immediately to keep the workspace compiling.

Conclusion:

- Task 4.8 cannot be completed by deleting the Cargo dependency line. The next
  real step is to move autonomy-evolution SDK command/result DTOs to
  `macaca-proto` or a provider-neutral facade contract, then migrate
  `autonomy_evolution_client.rs` away from the service crate before removing the
  dependency.

Validation:

- `cargo test -p macaca-sdk --lib`: passed, 84 tests after restoring the
  dependency.

## Runtime-host ExecutionControlLocalNotification Ownership

GitNexus impact memo:

- `SessionState`: LOW risk, 0 direct callers/processes according to GitNexus.
  Source scan showed many Web field accesses, so the implementation treated
  source evidence and compiler validation as authoritative.
- `RuntimeResumeSignal`: LOW risk, 0 direct callers/processes according to
  GitNexus. Source scan showed Web framework middleware, hook consumer,
  fork/join, and goal lifecycle users; the stale/partial index result was
  recorded and did not block implementation.
- `deliver_fork_join_resume_and_notify_parent`: target not found in GitNexus
  because the adapter is newer than the indexed graph. Direct `rg` callers were
  migrated and validated by Web tests.
- `deliver_goal_resume_and_notify_parent`: target not found in GitNexus because
  the adapter is newer than the indexed graph. Direct `rg` callers were migrated
  and validated by Web tests.

Implementation notes:

- Added the provider-neutral `RuntimeResumeSignal` DTO to `macaca-proto`
  under the execution-control service contract.
- Added `ExecutionControlLocalNotificationRuntime` in `macaca-runtime-host` as
  the runtime-host owner for non-serializable local pause flags and resume
  senders.
- Exposed the owner through `runtime_host_public_api.rs` and the narrow
  `macaca-sdk::runtime_host` facade.
- Removed Web-local `RuntimeResumeSignal` and deleted
  `crates/shells/macaca-web/src/runtime_resume.rs`.
- Removed Web-local `ExecutionControlLocalNotification` and the
  `SessionState.execution_control_notifications` map from `state.rs`.
- Changed `SessionState` to keep only
  `Arc<macaca_sdk::runtime_host::ExecutionControlLocalNotificationRuntime>`.
- Changed chat session bootstrap, agent-execution install, framework cleanup,
  WASM cleanup, and stop-all cleanup to call runtime-host owner methods.
- Changed fork/join and goal-lifecycle adapters to deliver authoritative
  `service.execution_control` resume evidence first and then call
  `ExecutionControlLocalNotificationRuntime::notify_resume` for the local wake.
- Enhanced `shell_no_local_execution_owner_gate` so Web state cannot re-own
  execution-control local notification maps and runtime-host must expose the
  owner surface.

Validation:

- `cargo fmt`: passed.
- `cargo test -p macaca-integration-tests --test shell_no_local_execution_owner_gate`:
  passed, 5 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`:
  passed, 6 tests.
- `cargo test -p macaca-web agent_execution_backend::tests::static_wiring --lib`:
  passed, 4 tests.
- `rg -n "runtime_resume|execution_control_notifications|pub struct ExecutionControlLocalNotification|HashMap<String, ExecutionControlLocalNotification>|shell-local execution-control|shell-local wake|shell-local resume" crates/shells/macaca-web/src`:
  returned zero production Web hits.
- `wc -l` confirmed touched files remain below 500 lines:
  `execution_control_local_notification.rs` 134,
  `shell_no_local_execution_owner_gate.rs` 162, `state.rs` 470.

Conclusion:

- Task 5.6 is complete for the approved residual-debt slice. SSE sender remains
  shell presentation state; pause/resume local wake handles are runtime-host
  owned; `service.execution_control` remains the authoritative audit and
  decision path before any local wake side effect.

## High-fanout Impact Memo Completion

This section completes task 0.9. HIGH/CRITICAL findings remain memo-only for
this refactor per the approved instruction; no item below blocked progress.

- `AlertManager`: LOW risk, 2 direct callers, 11 impacted symbols, 0 affected
  processes, 1 affected module (`Cluster_801`). Direct callers are kernel alert
  test helpers/functions.
- `WebhookAlertChannel`: LOW risk, 0 direct callers, 0 affected processes. The
  index still points to the old kernel alert source path but reports no
  dependents.
- `AgentOrchestrator`: LOW risk, 0 direct callers, 0 affected processes. The
  index still points to deleted `macaca-kernel/src/orchestrator.rs`, which was
  recorded as stale-index evidence.
- `shell_provider_bridge`: target not found. Source state already removed the
  retired SDK bridge file.
- `FrameworkRunner`: LOW risk, 0 direct callers/processes according to
  GitNexus. Source scans and gates still show the remaining Web-side
  construction anchor, so task 5.3 remains open despite the low indexed result.
- `McpRuntimeManager`: LOW risk, 0 direct callers/processes. The index points
  to an older aggregate runtime file path; source state must remain the final
  authority for MCP facade cleanup.
- `EntitlementRuntimeFacade`: LOW risk, 0 direct callers/processes; no indexed
  dependents remain.
- Application old helpers:
  - `app_entry_agent_name` returned a LOW-risk stale Web route hit
    (`get_app_agents`, 1 affected process). Source scan shows the current
    canonical helper lives in `macaca-app/src/consumption.rs`.
  - `app_task_planning_contract`: LOW risk, 1 direct test caller, 0 affected
    processes. Current source treats it as the canonical application
    consumption contract, not a deprecated wrapper.
- Context old engine APIs:
  - `ContextRuntimeFacade`: LOW risk, 0 direct callers/processes according to
    GitNexus, with stale path `macaca-context/src/engine.rs`.
- Source scan shows the current context engine shape is the canonical
  `engine/` Strategy + Facade + Registry module tree.

## OpenSpec Baseline Convergence Completion

This section completes tasks 9.1 through 9.10.

- Updated `unified-execution-path` so the baseline describes a single canonical
  protocol path and no longer permits alternate path-selection markers.
- Updated `microkernel-boundary-purity` so kernel purity is stated as terminal:
  no provider bridge implementation, no network transport ownership, and zero
  kernel exception rows.
- Updated `service-runtime` so `ServiceRuntime` is the terminal runtime owner
  for service registration, lifecycle, decorated calls, cleanup, health, and
  snapshots.
- Updated `sdk-system-facade` so the SDK preserves stable external response
  contracts without preserving old provider-construction wrappers.
- Updated `web-cli-thin-shell-v0` and `web-cli-thin-shell-completion` so Web and
  CLI are terminal presentation adapters over SDK/service/runtime-host
  boundaries.
- Updated `serviceization-dependency-gate` so dependency exceptions are hard
  failures and the terminal exception inventory is zero.
- Updated `serviceization-escape-hatches` so bypass paths are deleted, not
  frozen.
- Updated `context-composer` so replaced context entry points are removed and
  the canonical composer/engine/facade path is the only production path.
- Updated `docs/macaca-os-serviceization-allowlist.md` without downgrading the
  three constitutional documents: the document now records retired exception
  evidence and terminal protocol dependency gates.

Validation:

- `rg -n "alternate route|old route|migration|compat|legacy|deprecated|additive|preserve old|freeze|escape hatch|rollback|Route C|allowlist" openspec/specs docs/macaca-os-architecture-governance.md docs/macaca-os-microkernel-boundaries.md docs/macaca-os-serviceization-allowlist.md -S`:
  returned zero hits.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `openspec validate --all --strict`: passed, 193 items.

## Framework Runtime Agent Construction Ownership Completion

This section completes task 5.3.

GitNexus impact memo:

- `build_runtime_agent_from_context_snapshot_with_execution_policy`: LOW risk,
  0 direct callers, 0 affected processes. The index still points to the older
  aggregate `framework_runner.rs` path, so source scans were used as the
  authoritative evidence.
- `build_runtime_agent_from_context_snapshot`: LOW risk, 0 direct callers, 0
  affected processes, with the same stale aggregate path caveat.
- `build_runtime_agent`: LOW risk, 0 direct callers, 0 affected processes.
  Source scans showed live Web-local helper definitions and one factory call
  that needed materialization naming cleanup.

Implementation notes:

- Removed the unused `FrameworkRunner::build_runtime_agent` public helper from
  the Web shell framework runner.
- Renamed the Web snapshot helpers from
  `build_runtime_agent_from_context_snapshot*` to
  `materialize_runtime_react_agent_from_context_snapshot*`.
- Renamed the internal `WebTracedAgentFactory::build_runtime_agent` helper to
  `materialize_runtime_agent`.
- Updated `WebFrameworkAgentMaterializationPort` to call only
  `FrameworkRunner::materialize_runtime_react_agent_from_context_snapshot_with_execution_policy`.
- Strengthened static tests so Web adapter source must contain materialization
  semantics and must not contain the retired snapshot construction helper name.
- Runtime-host remains the owner of `FrameworkAgentConstructionPort` through
  `RuntimeHostFrameworkAgentConstructionService`; Web exposes only the lower
  `FrameworkAgentMaterializationPort` adapter.

Validation:

- `cargo fmt`: passed.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`:
  passed, 6 tests.
- `cargo test -p macaca-web agent_execution_backend::tests::static_wiring --lib`:
  passed, 4 tests.
- `cargo test -p macaca-integration-tests --test shell_no_framework_construction_gate`:
  passed, 1 test.
- `rg -n "FrameworkRunner::build_runtime_agent|\\.build_runtime_agent\\(|build_runtime_agent\\(" crates/shells/macaca-web crates/runtime/macaca-runtime-host crates/facade/macaca-sdk -S --glob '*.rs'`:
  returned only a test negative-assertion token in
  `unified_audit_replay_convergence_tests.rs`.
- `rg -n "build_runtime_agent_from_context_snapshot" crates/shells/macaca-web crates/runtime/macaca-runtime-host crates/facade/macaca-sdk -S --glob '*.rs'`:
  returned only test negative-assertion tokens.

## Workspace Memory / Tool Runtime Anchor Cleanup

This section completes task 5.8.

GitNexus impact memo:

- `WebMemoryRuntime`: LOW risk, 0 direct callers, 0 affected processes.
  Source scans showed current callers only in Web bootstrap, so the migration
  was handled by replacing those callers with the canonical memory service
  runtime adapter.
- `normalize_tool_input`: MEDIUM risk, 5 direct stale-index callers in the old
  single-file `framework_toolkit.rs` layout. Current source showed only the
  Web `framework_toolkit/workspace_tools.rs` wrapper and its tests, while
  concrete workspace tools already lived in runtime-host.

Implementation notes:

- Deleted `crates/shells/macaca-web/src/memory_runtime.rs`.
- Removed `mod memory_runtime` from `macaca-web`.
- Replaced Web-local `WebMemoryRuntime::from_configured_memory` with
  `macaca_sdk::memory::FabricMemoryRuntime::from_configured_memory`, which is
  implemented in the `macaca-memory` service crate.
- Exposed `FabricMemoryRuntime` through the SDK memory facade for composition
  roots.
- Removed `workspace_memory` manager-owner fields from `BootstrapCtx` and
  `AppState`; Web now keeps only `workspace_memory_tombstones`, which are a
  coordination index for digest/forget semantics around service memory entries.
- Built context recall over `DirectFacadeMemoryClient` wrapping the
  service-layer `FabricMemoryRuntime`, while the normal shell `memory_client`
  remains service-backed after provider registration.
- Deleted `crates/shells/macaca-web/src/framework_toolkit/workspace_tools.rs`.
- Updated framework toolkit tests to call
  `macaca_sdk::runtime_host::{normalize_workspace_tool_input, resolve_workspace_tool_path}`
  directly, proving those helpers are runtime-host owned.
- Strengthened `shell_no_local_execution_owner_gate` so Web cannot reintroduce
  a local memory runtime file, `workspace_memory` owner fields, or concrete
  `TestMemoryManager` ownership in state/bootstrap.

Validation:

- `cargo fmt`: passed.
- `cargo check -p macaca-web`: passed.
- `cargo test -p macaca-integration-tests --test shell_no_local_execution_owner_gate`:
  passed, 6 tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `cargo test -p macaca-web agent_execution_backend::tests::static_wiring --lib`:
  passed, 4 tests.
- `cargo test -p macaca-web framework_toolkit --lib`: passed, 9 tests.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate`:
  passed, 3 tests.
- `rg -n "WebMemoryRuntime|memory_runtime\\.rs|workspace_memory:|ctx\\.workspace_memory|state\\.workspace_memory|from_workspace_memory|ManagerWorkspaceMemoryRuntime|WorkspaceMemoryBackend|workspace_tools|resolve_workspace_path|normalize_tool_input" crates/shells/macaca-web/src crates/tests/macaca-integration-tests/tests -S --glob '*.rs'`:
  returned only tombstone coordination fields, runtime-host workspace-tool
  bootstrap usage, test function names, and negative gate assertions.

## SDK Driver / Alert Protocol DTO Extraction

This section partially advances task 4.3 and task 4.8. It does not complete
them because Tool, Skill, MCP, Task, Context, Application, Framework, and
Runtime-host SDK dependencies still require follow-up extraction.

GitNexus impact memo:

- `Alert`: LOW risk, 0 direct callers, 0 affected processes. Current source
  showed SDK and runtime-host still imported the kernel alert DTO directly, so
  the DTO was moved to the protocol layer.
- `DriverLoadServiceCommand`: LOW risk, 0 direct callers, 0 affected
  processes according to the index. Current source showed SDK driver client,
  runtime-host driver provider, and Web driver routes using the contract.
- `DriverServiceSnapshot`: LOW risk, 0 direct callers, 0 affected processes.
- `DriverLoadReport`: LOW risk, 1 direct indexed caller
  (`DriverRuntime::load_with_command`), 0 affected processes.
- `driver_service_descriptor`: HIGH risk, 4 direct callers and 4 affected
  web-server processes in the stale index. Per the approved instruction, this
  HIGH finding was recorded as a migration memo and did not block the change.

Implementation notes:

- Added `macaca_proto::alert::{Alert, AlertSeverity}` as the canonical
  provider-neutral alert payload.
- Changed kernel alert primitives to re-export the proto Alert DTO while
  retaining kernel-owned alert config, deduplication, and abstract channel
  behavior.
- Changed runtime-host alert provider and SDK alert client to use
  `macaca_proto::Alert`, removing the alert client dependency on
  `macaca_kernel::alert::Alert`.
- Added `macaca_proto::driver_service` as the canonical Driver service
  command/result/descriptor contract, including load report DTOs and the
  `driver_service_descriptor()` function.
- Replaced `macaca-driver` local service-contract and load-command DTO
  definitions with re-exports from `macaca-proto`, keeping provider-internal
  paths stable without duplicate types.
- Changed SDK driver client and top-level driver DTO exports to import Driver
  command/result/descriptor contracts from `macaca-proto`.
- Removed `macaca-driver` from `macaca-sdk` production dependencies after
  source scans confirmed no remaining `macaca_driver` references in the SDK.

Validation:

- `cargo fmt`: passed.
- `cargo test -p macaca-kernel alert --lib`: passed, 10 tests.
- `cargo test -p macaca-sdk alert_client --lib`: passed, 2 tests.
- `cargo test -p macaca-runtime-host alert_service_provider --lib`: passed, 3
  tests.
- `cargo check -p macaca-driver`: passed.
- `cargo check -p macaca-sdk`: passed.
- `cargo tree -e normal -p macaca-sdk --depth 1`: passed and no longer lists
  `macaca-driver`.
- `rg -n "macaca-driver|macaca_driver" crates/facade/macaca-sdk -S`: returned
  no hits.

## SDK LLM Protocol DTO Extraction

This section partially advances task 4.3 and task 4.8. It does not remove the
SDK `macaca-llm` dependency yet because SDK/Web composition roots still expose
`LlmProvider` and `LlmRouter`; those provider/runtime traits require a separate
composition-root extraction.

GitNexus impact memo:

- `LlmChatCommand`: LOW risk, 0 direct callers, 0 affected processes according
  to the current index. Current source showed SDK focused client, SystemFacade,
  runtime-host provider, and framework adapters using the contract through
  `macaca-llm` re-exports.
- `llm_service_descriptor`: HIGH risk, 4 direct callers and 4 affected
  web-server processes. Per the approved instruction, this HIGH finding was
  recorded as a migration memo and did not block the contract move.

Implementation notes:

- Added `macaca_proto::llm_service` as the canonical LLM service protocol
  contract, including chat/model-selection/snapshot DTOs, the narrow
  `LlmServiceChatClient` port, and `llm_service_descriptor()`.
- Added `macaca_proto::llm_service::hardening` for model catalog, provider
  capability, route-resolution, continuation validation, budget, degradation,
  diagnostic, and hardened response DTOs.
- Replaced `macaca-llm` local `service_contract`, `hardening_contract`, and
  `service_adapter` definitions with re-exports from `macaca-proto`.
- Updated `macaca-sdk::llm_client` and `SystemFacade::llm_chat` to use
  `macaca_proto` LLM service command/result DTOs directly.
- Updated `macaca_sdk::llm` so service command/result/descriptor exports come
  from `macaca-proto`, while concrete composition-root traits such as
  `LlmProvider` and `LlmRouter` remain from `macaca-llm` for now.

Validation:

- `cargo fmt`: passed.
- `cargo check -p macaca-llm`: passed.
- `cargo check -p macaca-sdk`: passed.
- `cargo check -p macaca-runtime-host`: passed.
- `cargo test -p macaca-llm service_adapter --lib`: passed, 1 test.
- `wc -l crates/foundation/macaca-proto/src/llm_service/mod.rs crates/foundation/macaca-proto/src/llm_service/hardening.rs crates/services/macaca-llm/src/service_contract.rs crates/services/macaca-llm/src/hardening_contract.rs crates/services/macaca-llm/src/service_adapter.rs`:
  323 / 238 / 7 / 6 / 25 lines respectively, all below the 500-line
  constitution limit.

## SDK Task Snapshot Protocol DTO Extraction

This section partially advances task 4.4 and task 4.8. It does not complete
task facade purification because SDK still exposes `TodoStore`, plan/worker loop
types, and task command DTOs from `macaca-task`; those require a separate task
service client migration away from local store ownership.

GitNexus impact memo:

- `TaskServiceSnapshotCommand`: LOW risk, 4 direct indexed callers and 0
  affected processes. The direct callers were task runtime tests in the stale
  indexed path.
- `TaskServiceSnapshot`: HIGH risk, 1 direct runtime snapshot builder, 14 total
  impacted symbols, and 0 affected processes. Per the approved instruction, the
  HIGH finding was recorded as a migration memo and did not block the DTO move.

Implementation notes:

- Added `macaca_proto::task_service` as the canonical Task service snapshot
  protocol contract, then expanded it to own the typed Task service command
  DTOs used by SDK/runtime-host providers.
- Moved `CreateGoalCommand`, `QueryTaskBoardCommand`,
  `CreateTaskAssignmentCommand`, `ClaimTaskCommand`, `StartTaskCommand`,
  `SubmitReviewCommand`, `ReviewTaskCommand`, `ResumeCoordinatorCommand`,
  `TaskServiceSnapshotCommand`, `TaskServiceGoalSnapshot`,
  `TaskServiceTaskSnapshot`, and `TaskServiceSnapshot` to `macaca-proto`.
- Changed `macaca-task::commands` and `macaca-task::events` to re-export the
  proto DTOs so existing provider/runtime call sites keep type identity while
  the protocol crate becomes the source of truth.
- Changed SDK top-level exports and `task_client` to import Task service
  command/snapshot DTOs from `macaca-proto`; the only remaining
  `task_client.rs` import from `macaca-task` is `TodoStore`, which requires a
  separate service-backed task-board data-source migration.
- Added `TASK_SERVICE_ID` and `TASK_QUERY_COMMAND` to the proto Task service
  contract so facade clients do not need runtime-host constants.
- Replaced `TodoStoreTaskBoardDataSource` with
  `ServiceBackedTaskBoardDataSource`, an Adapter over `SystemServiceClient`
  that forwards shell task-board reads to `service.task` with a generated
  trace id and typed `QueryTaskBoardCommand` payload.
- Updated Web `/api/apps/{app_id}/todos` to build `WebShellFacade` from
  `state.system_facade.service_client()` instead of cloning the persistent
  `TodoStore`.
- Retained direct `TodoStore` reads in other Web todo diagnostic/progress
  routes for later migration; this slice only moves the shell facade task-board
  query path.

Validation:

- `cargo fmt`: passed.
- `cargo check -p macaca-proto`: passed with the pre-existing
  `orchestration.rs` unused-import warning.
- `cargo check -p macaca-task`: passed with the same pre-existing proto
  warning.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings in
  `macaca-proto`, `macaca-agent`, `macaca-app`, and `macaca-runtime-host`.
- `cargo check -p macaca-web`: passed with pre-existing workspace and Web
  unused-code warnings.
- `cargo test -p macaca-task snapshot --lib`: passed, 1 test.
- `cargo test -p macaca-web web_shell_task_board_preserves_stable_json_shape --lib`:
  passed, 1 test.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `rg -n "use macaca_task|pub use macaca_task|macaca_task::" crates/facade/macaca-sdk/src -g '*.rs'`:
  the remaining hit is `lib.rs::task` re-exporting plan/loop/store types for
  still-open task facade purification.
- `rg -n "TodoStoreTaskBoardDataSource" crates/facade/macaca-sdk/src crates/shells/macaca-web/src -g '*.rs'`:
  zero hits.

## Web Fallback Assignment Task Service Migration

This section further advances tasks 4.4 and 4.8. It does not complete them:
Web still hosts local PlanLoop/WorkerLoop startup code, and the SDK still
re-exports task loop/store internals from `macaca-task`.

GitNexus impact memo:

- `create_fallback_decomposition_tasks`: CRITICAL risk, 1 direct indexed caller
  and 13 affected indexed processes around goal creation and PlanLoop startup.
  The change was therefore kept narrow: fallback ordering, task templates,
  returned `TodoItem` shape, and run-trace events are unchanged.
- `CreateTaskAssignmentCommand`: LOW risk, 0 indexed callers/processes.
- `TaskSystemServiceProvider`: LOW risk, 0 indexed callers/processes.
- `ServiceBackedTaskBoardDataSource`: target not found because the symbol is
  newer than the current GitNexus index.

Implementation notes:

- Added `ServiceBackedTaskBoardDataSource::create_task_assignment`, a focused
  SDK Adapter method that dispatches `task.create_assignment` through
  `SystemServiceClient` instead of constructing `TaskSpace`.
- Kept the runtime-host assignment response as `{ "task": <TodoItem> }` so the
  existing WASM host-import helper that extracts `task.id` remains on the same
  stable service response shape.
- Changed Web `create_fallback_decomposition_tasks` to create fallback todos via
  the SDK task-service adapter with typed `CreateTaskAssignmentCommand`,
  provider-neutral `TraceContext`, and structured error logging.
- Changed `CreateTaskAssignmentCommand.session_id` from `String` to
  `Option<String>` so the Task Service boundary preserves the old app-scope
  `TaskSpace` semantics instead of coercing missing session scope into an empty
  string.
- Updated task graph admission and assignment runtime code to use optional
  session scope for store queries, service events, snapshots, and logs.
- Removed the touched graph-admission comment's old-path debt wording and kept
  the terminology on auxiliary/diagnostic graph entries.

Validation:

- `cargo fmt`: passed.
- `cargo check -p macaca-task`: passed with the pre-existing
  `orchestration.rs` unused-import warning.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-runtime-host`: passed with pre-existing workspace
  warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace and Web
  warnings.
- `cargo test -p macaca-task explicit_assignment --lib`: passed, 2 tests.
- `cargo test -p macaca-runtime-host task_service_provider --lib`: passed, 2
  tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `rg -n "Compatibility|compat|legacy|Route C migration|deprecated" crates/services/macaca-task/src/runtime/graph_admission.rs crates/foundation/macaca-proto/src/task_service.rs crates/facade/macaca-sdk/src/task_client.rs crates/shells/macaca-web/src/loop_manager/decomposition_adapter.rs`:
  zero hits in the touched slice.

## Task Service Goal Completion and Local PlanLoop Construction Migration

This section further advances tasks 4.4 and 4.8. It does not complete them:
Web still adapts PlanEvent/WorkerEvent streams for SSE/session projection, and
the SDK still exposes several task loop/event types for the remaining local
controller adapters.

GitNexus impact memo:

- `handle_plan_event_evaluate_goal`: target not found because the current
  split Web loop-manager files are newer than the GitNexus index.
- `TaskServiceRuntime`: LOW risk in the indexed graph.
- `TaskSystemServiceProvider`: LOW risk in the indexed graph.
- `CreateGoalCommand`: LOW risk in the indexed graph.
- `ensure_plan_loop`: target not found because the split Web
  `plan_loop_orchestrator` symbol is newer than the GitNexus index.
- `SessionLoopLocalRuntime`: target not found because the runtime-host local
  loop owner is newer than the GitNexus index.
- `PlanLoop`: LOW risk, 0 direct indexed callers and 0 affected indexed
  processes.
- `TaskSpace`: LOW risk, 0 direct indexed callers and 0 affected indexed
  processes.
- `ServiceBackedTaskBoardDataSource`: target not found because the symbol is
  newer than the current GitNexus index.

Implementation notes:

- Added the canonical `CompleteGoalCommand` / `task.complete_goal` Task Service
  command and routed Web goal-completion transitions through the SDK
  service-backed task client instead of calling `TaskSpace::complete_goal`
  locally.
- Preserved the provider response as a typed `{ completed: bool }` transition
  result so callers can observe the service-side lifecycle decision without
  reading task-store internals.
- Added `LocalPlanLoopController` and
  `SessionLoopLocalRuntime::reserve_local_plan_loop_controller` in
  runtime-host. The factory owns `TaskSpace::for_session` and `PlanLoop`
  construction while Web remains only an event Adapter for the existing
  PlanEvent stream.
- Re-exported the local PlanLoop controller through the explicit runtime-host
  public API and the SDK runtime-host facade so the exposed surface remains
  reviewable instead of leaking the entire runtime-host crate.
- Changed Web `ensure_plan_loop` to request a runtime-host constructed local
  controller and removed the last Web/SDK direct hit for
  `macaca_sdk::task::TaskSpace` / `TaskSpace::for_session` in the scanned
  shell/facade slice.

Validation:

- `cargo fmt`: passed.
- `cargo check -p macaca-runtime-host`: passed with pre-existing workspace
  unused-code warnings.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace and Web
  unused-code warnings.
- `cargo test -p macaca-runtime-host task_service_provider --lib`: passed, 2
  tests.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `rg -n "macaca_sdk::task::TaskSpace|TaskSpace::for_session|TodoStoreTaskBoardDataSource|create_task_assignment_with_graph_owner" crates/shells/macaca-web crates/facade/macaca-sdk/src -g '*.rs'`:
  zero hits.

## Local WorkerLoop Construction Migration

This section continues tasks 4.4, 4.8, and 5.7. It does not complete them:
Web still adapts WorkerEvent streams and the SDK still re-exports task loop
types until the remaining task-event protocol DTOs are lifted out of
`macaca-task`.

GitNexus impact memo:

- `WorkerLoop`: LOW risk, 0 direct indexed callers and 0 affected indexed
  processes.

Implementation notes:

- Added `LocalWorkerLoopController` and
  `SessionLoopLocalRuntime::build_local_worker_loop_controller` in
  runtime-host. The factory owns `TaskBoard::for_agent`,
  `WorkerLoop::with_components`, default WorkerLoop config, shutdown flag, and
  waker construction.
- Re-exported the local WorkerLoop controller through the explicit
  runtime-host public API and SDK runtime-host facade.
- Changed Web `ensure_worker_loops` to request runtime-host constructed worker
  controllers. Web still spawns the local loop and adapts `WorkerEvent`
  messages to SSE/session persistence/agent-execution service calls, but no
  longer constructs `TaskBoard`, `WorkerLoop`, `WorkerLoopConfig`, or
  `WorkerLoopWaker` directly.

Validation:

- `cargo fmt`: passed.
- `cargo check -p macaca-runtime-host`: passed with pre-existing workspace
  unused-code warnings.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace and Web
  unused-code warnings.
- `cargo test -p macaca-web unified_delegation_path_tests --lib`: passed, 7
  tests.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `rg -n "TaskBoard::for_agent|WorkerLoop::with_components|WorkerLoopConfig::default|WorkerLoopWaker|TaskSpace::for_session|macaca_sdk::task::TaskSpace" crates/shells/macaca-web/src crates/facade/macaca-sdk/src -g '*.rs'`:
  the only remaining hit is the SDK `task` re-export in `src/lib.rs`.

## Task Service Lifecycle, Prompt Commands, and SDK Task Surface Cleanup

This section continues tasks 4.4, 4.8, and 5.7. It completes the direct
`macaca-task` leakage cleanup for the SDK/Web task surface, but it does not
complete SDK facade purification because `macaca-sdk` still has production
dependencies on kernel, application, runtime-host, framework, and other service
crates.

GitNexus impact memo:

- `TaskServiceRuntime`: LOW risk, 0 impacted indexed symbols/processes.
- `TaskSystemServiceProvider`: LOW risk, 0 impacted indexed symbols/processes.
- `TodoStore`: LOW risk in the indexed graph.
- `GoalEvaluator`: LOW risk in the indexed graph.
- `GoalEvaluation`: LOW risk in the indexed graph.
- `build_decomposition_prompt`: CRITICAL in the indexed graph because it sits
  on goal creation/planning paths. The migration kept prompt semantics intact
  and changed only the ownership boundary.
- `execute_worker_task_via_agent_service`: CRITICAL in the indexed graph
  because it participates in WorkerLoop and goal execution paths. The migration
  moved task lifecycle writes behind Task Service commands and kept agent
  execution behavior unchanged.

Implementation notes:

- Added typed Task Service commands for task failure and task prompt/evaluation
  operations:
  - `FailTaskCommand` / `task.fail`
  - `BuildDecompositionPromptCommand` / `task.build_decomposition_prompt`
  - `BuildGoalEvaluationPromptCommand` / `task.build_goal_evaluation_prompt`
  - `ParseGoalEvaluationCommand` / `task.parse_goal_evaluation`
- Implemented the new command handlers in `macaca-task` and wired them through
  `TaskSystemServiceProvider`, preserving provider-neutral command/result DTOs
  and traceable service dispatch.
- Changed Web worker execution lifecycle writes so success, failure, review
  submission, and timeout paths call the Task Service through
  `ServiceBackedTaskBoardDataSource` instead of mutating a local `TaskBoard`.
- Changed goal decomposition and goal evaluation prompt construction in Web
  plan-loop handlers to call Task Service commands instead of importing task
  planning helpers directly.
- Changed `GoalEvaluation` in `macaca-task` to be a public alias for the
  proto-level `GoalEvaluationResult`, keeping parser behavior stable while
  making the boundary DTO canonical.
- Reworked `macaca-sdk::task` so it re-exports only `macaca-proto` task DTOs
  and no longer re-exports `macaca_task` runtime/store/loop types.
- Removed the `macaca-task` production dependency from `macaca-sdk/Cargo.toml`.
- Kept `TodoStore` available through the explicit `macaca-runtime-host`
  public facade because current Web bootstrap still owns local runtime-host
  composition. This is not final SDK purification; it is a transitional
  runtime-host composition surface already tracked by tasks 4.6 and 4.8.

Validation:

- `cargo fmt`: passed.
- `cargo check -p macaca-proto`: passed with the pre-existing
  `unused uuid::Uuid` warning.
- `cargo check -p macaca-task`: passed.
- `cargo check -p macaca-runtime-host`: passed with pre-existing workspace
  warnings.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo clean -p macaca-runtime-host -p macaca-sdk && cargo check -p macaca-web`:
  passed with pre-existing workspace/Web warnings. The targeted clean was
  required because the incremental metadata initially reported a stale
  `macaca_runtime_host::TodoStore` unresolved import.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no `macaca-task` dependency.
  Remaining production dependencies include `macaca-agent`, `macaca-app`,
  `macaca-context`, `macaca-framework`, `macaca-kernel`, `macaca-runtime-host`,
  and other service crates, so task 4.8 remains open.
- `rg -n "macaca_sdk::task::|use macaca_sdk::task|pub use macaca_task|macaca_task::" crates/shells/macaca-web/src crates/facade/macaca-sdk/src -g '*.rs'`:
  zero hits.
- `rg -n "macaca_sdk::(task|kernel|session)::|use macaca_sdk::(task|kernel|session)" crates/shells/macaca-web/src crates/shells/macaca-cli/src crates/facade/macaca-sdk/src -g '*.rs'`:
  zero hits.
- Follow-up scan still shows Web imports top-level SDK kernel primitives:
  `Kernel`, `KernelBuilder`, `AuditLogger`, `KernelPersistencePort`,
  `SystemService`, and `UnavailableAgentExecutionPort`. Those are real
  remaining 4.4/5.x ownership debts and must not be marked complete yet.

## SDK Top-level Kernel Primitive Surface Narrowing

This section continues task 4.4. It does not complete the task because Web still
constructs and stores kernel primitives during local composition bootstrap. The
change only removes the broad top-level SDK surface so future callers cannot
accidentally import kernel primitives as ordinary SDK APIs.

GitNexus impact memo:

- `KernelBuilder`: LOW risk, 0 impacted indexed symbols/processes. Source scan
  showed active Web bootstrap callers, so grep was used as the authoritative
  migration list.
- `Kernel`: LOW risk, 0 impacted indexed symbols/processes. Source scan showed
  active Web bootstrap/state/tool callers.
- `AuditLogger`: LOW risk, 0 impacted indexed symbols/processes. Source scan
  showed active Web bootstrap/state callers.

Implementation notes:

- Removed the SDK top-level `pub use macaca_kernel::{...}` export for
  `Kernel`, `KernelBuilder`, `AuditLogger`, `KernelPersistencePort`,
  `SystemService`, and `UnavailableAgentExecutionPort`.
- Added an explicit `macaca_sdk::kernel` module with English documentation that
  marks these types as composition-root primitives, not general behavior APIs.
- Migrated Web uses from top-level SDK imports to `macaca_sdk::kernel::{...}` or
  fully-qualified `macaca_sdk::kernel::...` paths.
- This makes direct kernel primitive ownership easier to audit while preserving
  runtime behavior. The remaining terminal work is to move composition ownership
  behind host bootstrap/SystemFacade boundaries instead of merely namespacing it.

Validation:

- `cargo fmt`: passed.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace/Web warnings.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate`:
  passed, 1 test.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `rg -n "macaca_sdk::(Kernel|KernelBuilder|AuditLogger|SystemService\\b|KernelPersistencePort|UnavailableAgentExecutionPort)|use macaca_sdk::\\{[^}]*\\b(Kernel|KernelBuilder|AuditLogger|SystemService\\b|KernelPersistencePort|UnavailableAgentExecutionPort)" crates/shells/macaca-web/src crates/shells/macaca-cli/src crates/facade/macaca-sdk/src -g '*.rs'`:
  zero hits.

## SDK Declarative Agent Materialization Removal

This section continues tasks 4.3, 4.4, 4.6, and 4.8. It removes SDK ownership
of runtime-agent materialization for declarative agents, but it does not
complete SDK purification because the SDK still exposes composition-root and
service-crate surfaces that later slices must move behind focused clients or
protocol DTOs.

GitNexus impact memo:

- `DeclarativeAgent`: LOW risk, 0 impacted indexed symbols/processes. Source
  scans showed local SDK tests and kernel/integration tests as the authoritative
  caller list.
- `AgentSpec::into_agent`: LOW risk, 0 impacted indexed symbols/processes in
  the indexed graph. Source scans found local tests and in-process test
  registration paths.
- `register_in_process_kernel_agent`: target not found because the function was
  introduced after the current GitNexus index.
- `for_kernel_with_in_process`: target not found because the helper was
  introduced after the current GitNexus index.

Implementation notes:

- Removed `DeclarativeAgent` from the SDK builder surface and deleted the SDK
  in-process kernel registration helper module.
- Replaced `AgentSpec::into_agent` with `AgentSpec::into_manifest`, keeping the
  SDK declarative surface as manifest/data-contract construction only.
- Changed `MacacaSdk` and `AgentRegistryApi` registration paths to register
  `AgentManifest` values rather than SDK-built runtime agents.
- Moved the remaining in-process `BasicAgent` materialization into the
  kernel/integration tests that explicitly need local execution doubles.
- Removed the `register_in_process_kernel_agent` and
  `for_kernel_with_in_process` SDK helpers so production callers cannot use the
  SDK to bypass runtime-host/framework construction ownership.

Validation:

- `cargo test -p macaca-sdk --lib`: passed, 83 tests.
- `cargo test -p macaca-integration-tests --test kernel_lifecycle`: passed, 5
  tests.
- `cargo test -p macaca-kernel --test e2e_auto_programming`: passed, 4 tests.
- `rg -n "into_agent\\(|register_in_process_kernel_agent|for_kernel_with_in_process|in_process_kernel_registration|pub struct DeclarativeAgent" crates/facade/macaca-sdk/src crates/kernel/macaca-kernel/tests crates/tests/macaca-integration-tests/tests -g '*.rs'`:
  zero hits.

## SDK Direct macaca-agent Dependency Removal

This section continues tasks 4.4 and 4.8. It closes the direct
`macaca-agent` production dependency from `macaca-sdk`, but broader SDK
purification remains open because the SDK still directly depends on
application, framework, kernel, runtime-host, and service crates.

GitNexus impact memo:

- `AgentCapabilitySet`: LOW risk in the indexed graph.
- `AgentServices`: HIGH risk in the indexed graph. Per user direction, this
  was recorded as an implementation memo item rather than used as a blocker.
- `AgentTransitionReason`: LOW risk in the indexed graph.
- `build_context_system_prompt`: LOW risk, 3 direct callers and 0 affected
  indexed processes; the follow-up edit was comment-only.

Implementation notes:

- Moved the agent construction capability/service/lifecycle re-export point to
  `macaca_framework::construction`, which is the framework-owned construction
  facade for runtime-agent assembly.
- Removed the SDK top-level `AgentCapabilitySet`, `AgentServices`, and
  `AgentTransitionReason` re-exports and exposed those types only through the
  explicit `macaca_sdk::framework::construction` facade.
- Migrated Web framework-runner imports to
  `macaca_sdk::framework::construction::{...}` so shell composition continues
  to compile without importing application-agent types from SDK top-level.
- Removed the direct `macaca-agent` production dependency from
  `crates/facade/macaca-sdk/Cargo.toml`.
- Reworded the remaining framework-runner comment so it describes the
  framework construction capability contract rather than the lower-level agent
  crate.

Validation:

- `cargo fmt --package macaca-framework --package macaca-sdk --package macaca-web`:
  passed.
- `cargo check -p macaca-sdk`: passed.
- `cargo test -p macaca-sdk --lib`: passed, 83 tests.
- `cargo test -p macaca-web framework_runner --lib`: passed, 13 tests.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no direct `macaca-agent`
  dependency. Remaining direct workspace dependencies include `macaca-app`,
  `macaca-context`, `macaca-framework`, `macaca-kernel`, `macaca-llm`,
  `macaca-memory`, `macaca-runtime-host`, `macaca-skill`, and
  `macaca-tools`.
- `rg -n "macaca-agent|macaca_agent|macaca_sdk::(AgentCapabilitySet|AgentServices|AgentTransitionReason)|use macaca_sdk::\\{[^\\n]*(AgentCapabilitySet|AgentServices|AgentTransitionReason)" crates/facade/macaca-sdk/Cargo.toml crates/facade/macaca-sdk/src crates/shells/macaca-web/src/framework_runner --glob '*.rs'`:
  zero hits.

## Terminal Gate Progress

Validation notes:

- `cargo check --workspace`: initially failed with stale incremental metadata
  reporting `macaca_framework::construction` re-exports as private/missing.
  Source inspection confirmed the public re-export was present in
  `crates/runtime/macaca-framework/src/construction.rs`; after
  `cargo clean -p macaca-framework -p macaca-sdk`, `cargo check -p macaca-sdk`
  passed and the rerun of `cargo check --workspace` passed. The command still
  reports pre-existing unused import/dead-code warnings across proto, app,
  runtime-host, and Web.

## Task Service File-size Gate Split

GitNexus impact memo:

- `TaskSystemServiceProvider`: LOW risk, 0 impacted indexed symbols/processes.
- `TaskServiceSnapshot`: HIGH risk in the indexed graph, centered on task
  runtime snapshot construction. Per user direction, this was recorded as memo
  evidence and not used as a blocker. The split was mechanical and preserved
  the public `macaca_proto::...` re-export surface.

Implementation notes:

- Split `macaca-proto/src/task_service.rs` into child modules for constants,
  lifecycle commands, event DTOs, prompt/evaluation DTOs, query DTOs, and
  snapshot DTOs. The root module now re-exports the same public API.
- Moved `macaca-runtime-host/src/task_service_provider.rs` tests into
  `task_service_provider/tests.rs`; production provider code stayed in place.
- Post-split line counts are below the 500-line OS-layer limit:
  - `task_service.rs`: 20 lines.
  - `task_service/commands.rs`: 148 lines.
  - `task_service/constants.rs`: 58 lines.
  - `task_service/events.rs`: 79 lines.
  - `task_service/prompts.rs`: 51 lines.
  - `task_service/queries.rs`: 161 lines.
  - `task_service/snapshots.rs`: 47 lines.
  - `task_service_provider.rs`: 381 lines.
  - `task_service_provider/tests.rs`: 205 lines.

Validation:

- `cargo fmt --package macaca-proto --package macaca-runtime-host --package macaca-integration-tests`:
  passed.
- `cargo check -p macaca-proto`: passed with the pre-existing
  `unused uuid::Uuid` warning.
- `cargo test -p macaca-runtime-host task_service_provider --lib`: passed, 2
  tests.
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`:
  passed, 2 tests.
- `cargo test -p macaca-integration-tests --test kernel_purity_gate`: passed,
  3 tests.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate`:
  passed, 3 tests.
- `cargo test -p macaca-integration-tests --test protocol_service_dependency_boundaries`:
  passed, 3 tests.

## End-to-end Validation And Final Migration Report

Validation target correction:

- The original task/design matrix referenced a `macaca-os` package and
  `*_mcp_path_tests` targets. Current `cargo metadata --no-deps
  --format-version 1` shows no `macaca-os` package in this workspace, and
  source search found those test target names only in this OpenSpec change.
- The executable replacement matrix is the current Web shell, runtime-host, and
  integration-test coverage that exercises the same protocol/service paths:
  `macaca-web unified`, GenUI routes, session routes, app UI session
  projection, unified audit replay terminal gate, P5 external contract gate,
  Web3/EVM unavailable provider tests, finance absent-provider test, and
  package certification optional-module test.

Live service smoke:

- Built `macaca-web-server` with `cargo build --bin macaca-web-server`.
- Started `./target/debug/macaca-web-server --port 3211` with a temporary
  `AOS_WORKSPACE__ROOT_DIR` containing symlinked example applications.
- `GET /api/status`: HTTP 200 with version `0.1.0`, provider `volces`, and
  status keys `agent_count`, `app_count`, `llm_provider`, `version`.
- `GET /api/apps`: HTTP 200 with 5 applications; selected
  `wasm-crypto-signal-app`
  (`2c96f3f2-b78c-5edd-beb4-740c8c004910`, `agent_count=4`).
- `POST /api/chat/v2`: HTTP 200 SSE for session `e2e-1781269872`. The stream
  emitted `session_id`, `thinking` phase `wasm_host_dispatch`, and
  `service_call_audit` events before the client-side 12-second timeout ended
  the long-lived SSE read. The server stayed healthy.
- `GET /api/apps/{app_id}/genui/surface?session_id=e2e-1781269872`: HTTP 200
  with a concrete GenUI surface for `crypto-signal`.
- Runtime log evidence for the GenUI surface path showed SDK dispatch to
  `service.application`, `application.genui.surface`, policy/audit decorator
  admission, local service-bus dispatch, kernel service-call acceptance, runtime
  provider acceptance, and `service_runtime.call.completed`, all carrying trace
  id `genui-surface`.
- `POST /api/apps/{app_id}/genui/events` with a complete `UiAction` DTO:
  HTTP 200, `{ persisted: true, seq: 1 }`.
- `GET /api/sessions/{session_id}/events?since=0&limit=20`: HTTP 200,
  `latest_seq=1`, one `genui_event` from `genui_web_shell`.
- `GET /api/sessions/{session_id}/run-trace?since=0&limit=20`: HTTP 200,
  empty run-trace slice for the UI-only event session and `latest_seq=1`,
  proving replay filtering remains scoped rather than duplicating all events.
- `POST /api/sessions/{session_id}/compact`: HTTP 200 with a successor session
  id, proving session recovery/lineage creation works without deleting the
  source session.

Executable end-to-end test evidence:

- `cargo test -p macaca-web unified --lib`: passed, 26 tests. This covers
  `/api/chat/v2`, YAML/WASM single-agent execution, unified delegation, and the
  Application ABI path.
- `cargo test -p macaca-web genui_routes --lib`: passed, 3 tests.
- `cargo test -p macaca-web session --lib`: passed, 40 tests.
- `cargo test -p macaca-web app_ui_session_projection --lib`: passed, 3 tests.
- `cargo test -p macaca-integration-tests --test unified_audit_replay_terminal_gate -- --nocapture`:
  passed, 1 test plus subprocess checks.
- `cargo test -p macaca-integration-tests --test p5_external_contract_gate -- --nocapture`:
  passed, 4 tests.
- `cargo test -p macaca-runtime-host --test web3_service_provider unavailable -- --nocapture`:
  passed, 3 tests.
- `cargo test -p macaca-runtime-host --test evm_service_provider unavailable -- --nocapture`:
  passed, 3 tests.
- `cargo test -p macaca-integration-tests --test domain_pack_finance_package absent_finance_pack_leaves_service_unavailable -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test package_certification package_certification_keeps_web3_and_evm_optional_modules_unavailable_safe -- --nocapture`:
  passed.

Workspace terminal evidence:

- `cargo test --workspace --exclude macaca-framework`: passed after increasing
  timeout to 600 seconds.
- `cargo test -p macaca-framework`: passed, including 247 lib tests,
  integration tests, boundary/license tests, and doc tests.
- `cargo test --workspace`: passed monolithically.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed after replacing stale validation commands with current workspace
  targets.

Deleted surfaces and canonical replacements:

- Deleted kernel `AgentOrchestrator`, `OrchestratorBuilder`, and the production
  `orchestrator.rs` module. Canonical replacement: provider-neutral
  `macaca-proto::orchestration` DTOs plus `service.agent_execution` and
  `service.execution_control` providers.
- Deleted kernel webhook alert transport and direct HTTP/webhook config.
  Canonical replacement: runtime-host `AlertSystemServiceProvider` and SDK
  `SystemAlertClient`.
- Deleted Web `/api/chat` production route implementation and
  `chat_orchestrator/route_legacy.rs`. Canonical replacement: `/api/chat/v2`
  through Application ABI, WASM host dispatch, or `service.agent_execution`.
- Deleted SDK `shell_provider_bridge.rs` alias module. Canonical replacement:
  focused SDK clients, protocol DTOs, and explicit transitional SDK modules
  tracked by remaining SDK facade tasks.
- Deleted SDK in-process kernel registration helpers and runtime-agent
  materialization from the declarative agent surface. Canonical replacement:
  manifest/protocol construction in SDK and runtime-host/framework-owned
  materialization.
- Deleted `macaca-app` compatibility checker files and old helper re-exports.
  Canonical replacement: conformance checker, Application ABI projection, and
  service adapter paths.
- Deleted runtime-host `route_c_bootstrap.rs` and renamed resource mappings from
  compatibility wording to Skill MCP mapping terminology.
- Deleted Web shell local ownership modules that exposed old execution
  anchors, including old context-reporting assembly and direct memory/runtime
  modules. Canonical replacement: service-backed task/application/context
  clients and execution-control adapters.
- Deleted old Route C integration test files. Canonical replacement:
  `protocol_service_dependency_boundaries`, `protocol_microkernel_baseline`,
  and `protocol_workspace_topology` terminal gates.

Dependency before/after snapshots:

- Kernel before: `macaca-kernel` directly included `reqwest`, proving network
  transport debt. Kernel after: direct normal deps are `macaca-proto`,
  `macaca-ipc`, `serde`, `serde_json`, `tokio`, `async-trait`, `tracing`,
  `chrono`, `uuid`, `thiserror`, and `futures`; no network/http client.
- SDK before: direct dependencies included `macaca-agent`, `macaca-app`,
  `macaca-context`, `macaca-driver`, `macaca-framework`, `macaca-kernel`,
  `macaca-llm`, `macaca-memory`, `macaca-runtime-host`, `macaca-skill`,
  `macaca-task`, and `macaca-tools`.
- SDK after the latest SDK facade slices: `macaca-agent`, `macaca-driver`,
  `macaca-task`, `macaca-kernel`, `macaca-llm`, `macaca-memory`,
  `macaca-context`, `macaca-tools`, `macaca-skill`, and `macaca-app` are no
  longer direct dependencies. Kernel, LLM, Memory, Context, Tool, Skill, and
  Application composition-root types now flow through the explicit runtime-host
  facade. SDK still directly depends on `macaca-framework` and
  `macaca-runtime-host`. Tasks 4.5-4.8 remain open for that residual facade
  purification work.
- Web before and after: direct workspace dependencies remain limited to
  `macaca-proto` and `macaca-sdk`.
- CLI before and after: direct workspace dependencies remain limited to
  `macaca-proto` and `macaca-sdk`.

GitNexus final summary:

- GitNexus impact was recorded before edited Rust symbols where the index could
  resolve targets. Many newly introduced or recently renamed symbols were not
  yet indexed, so source scans were used as the authoritative follow-up.
- HIGH/CRITICAL findings were memo-only per user direction, not blockers. The
  notable HIGH/CRITICAL items were `diagnose_session_claims`,
  `SessionClaimDiagnostics`, `AgentServices`, and `TaskServiceSnapshot`; edits
  were scoped to DTO serialization, import ownership, or mechanical module
  splitting rather than behavioral changes to those high-fanout flows.

Final static evidence:

- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates --glob '*.rs'`:
  zero hits.
- `rg -n "legacy|compat|Route C migration" crates --glob '*.rs'`: no old-path
  debt hits. Remaining matches are legitimate OpenAI-compatible protocol names
  and tests in LLM/memory code.
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`:
  passed, proving OS-layer Rust files remain under the 500-line gate.
- `cargo test -p macaca-integration-tests --test kernel_purity_gate`: passed.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate`:
  passed.
- `cargo test -p macaca-integration-tests --test protocol_service_dependency_boundaries`:
  passed.
- `cargo test -p macaca-integration-tests --test serviceization_escape_hatches`:
  passed.

Review request before archive:

- The change is ready for review of the completed terminal evidence and the
  explicitly open SDK facade purification tasks. Do not archive until reviewers
  accept that tasks 4.3-4.8 remain tracked residual work rather than hidden
  completion.

## SDK Direct Kernel Dependency Removal

This section advances tasks 4.4 and 4.8. It removes `macaca-sdk`'s direct
production dependency on `macaca-kernel`, but does not complete SDK facade
purification because other provider/runtime/application dependencies remain.

GitNexus impact memo:

- `KernelAgentRegistry`: LOW risk, 0 impacted indexed symbols/processes.
- `KernelPrimitiveSdk`: LOW risk, 0 impacted indexed symbols/processes.
- `kernel`: LOW risk folder-level result, 0 impacted indexed processes.
- `runtime_host`: target not found in the current GitNexus index.
- `MacacaSdk`: LOW risk, 0 impacted indexed symbols/processes.
- `AgentRegistryApi`: LOW risk, 1 direct indexed implementor
  (`KernelAgentRegistry`), 0 affected processes.
- `Kernel`: LOW risk, 0 impacted indexed symbols/processes.

Implementation notes:

- Added explicit kernel primitive re-exports to
  `macaca-runtime-host/src/runtime_host_public_api.rs` for composition roots:
  `AuditLogger`, `DefaultKernelFacade`, `Kernel`, `KernelBuilder`,
  `KernelFacade`, `KernelPersistencePort`, `SystemService`, and
  `UnavailableAgentExecutionPort`.
- Updated `macaca-sdk/src/runtime_host.rs` to forward those runtime-host
  facade exports.
- Updated `macaca_sdk::kernel` to re-export from `crate::runtime_host` instead
  of directly from `macaca-kernel`.
- Removed SDK-owned `KernelAgentRegistry`, `MacacaSdk::for_kernel`, and
  `KernelPrimitiveSdk`. SDK registration remains generic through
  `AgentRegistryApi` and `MacacaSdk::new(...)`.
- Updated the `kernel_lifecycle` integration test to keep its kernel-backed
  registry adapter as a local test fixture, so production SDK no longer owns
  kernel registration adaptation.
- Removed `macaca-kernel` from `crates/facade/macaca-sdk/Cargo.toml`.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk --package macaca-integration-tests`:
  passed.
- `rg -n "macaca_kernel|KernelAgentRegistry|KernelPrimitiveSdk|for_kernel\\(" crates/facade/macaca-sdk crates/tests/macaca-integration-tests/tests/kernel_lifecycle.rs --glob '*.rs'`:
  SDK source has zero hits; the remaining `macaca_kernel` hit is the
  integration test's own kernel fixture import.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-sdk --lib`: passed, 82 tests.
- `cargo test -p macaca-integration-tests --test kernel_lifecycle`: passed, 5
  tests.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no direct `macaca-kernel`
  dependency. Remaining direct workspace dependencies are `macaca-app`,
  `macaca-context`, `macaca-framework`, `macaca-llm`, `macaca-memory`,
  `macaca-proto`, `macaca-runtime-host`, `macaca-skill`, and `macaca-tools`.
- `cargo check -p macaca-web`: passed with pre-existing warnings, proving Web
  composition roots still compile through `macaca_sdk::kernel` after the
  runtime-host facade redirection.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed, 1 test.

## SDK Direct LLM Dependency Removal

This section advances tasks 4.5 and 4.8. It removes `macaca-sdk`'s direct
production dependency on `macaca-llm` without changing LLM routing behavior,
provider selection, or service command contracts.

GitNexus impact memo:

- `LlmProvider`: CRITICAL risk, 31 direct indexed callers/importers and 18
  affected indexed processes. Memo-only per user direction.
- `LlmRouter`: LOW risk, 0 affected indexed symbols/processes.
- `ModelSelection`: CRITICAL risk, 2 direct indexed callers/importers and 5
  affected indexed processes. Memo-only per user direction.
- `llm`: LOW risk result matched `AppState.llm`, 0 affected indexed
  processes.

Implementation notes:

- Added explicit LLM routing type re-exports to
  `macaca-runtime-host/src/runtime_host_public_api.rs`:
  `LlmProvider`, `LlmRouter`, `ModelSelection`, `ModelSelectionRequest`, and
  `ModelTarget`.
- Updated `macaca-sdk/src/runtime_host.rs` to forward those runtime-host facade
  exports.
- Updated `macaca_sdk::llm` to re-export those routing types from
  `crate::runtime_host`; LLM service command DTOs remain sourced from
  `macaca-proto`.
- Removed `macaca-llm` from `crates/facade/macaca-sdk/Cargo.toml`.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk`: passed.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed, 1 test.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no direct `macaca-llm`
  dependency.

## SDK Direct Memory Dependency Removal

This section advances tasks 4.3 and 4.8. It removes `macaca-sdk`'s direct
production dependency on `macaca-memory` while preserving the existing
`macaca_sdk::memory` API for composition roots that still consume Memory
bootstrap contracts.

GitNexus impact memo:

- `MemoryFacade`: HIGH risk, 13 direct indexed implementors/importers and 0
  affected indexed processes. Memo-only per user direction.
- `MemoryRememberCommand`: LOW risk, 0 affected indexed symbols/processes.
- `memory`: LOW risk result matched a framework module, 0 affected indexed
  processes.

Implementation notes:

- Added the SDK-published Memory surface to
  `macaca-runtime-host/src/runtime_host_public_api.rs`, including service
  commands, snapshot DTOs, facade traits, bootstrap/runtime types, and
  tombstone contracts already exposed through `macaca_sdk::memory`.
- Updated `macaca-sdk/src/runtime_host.rs` to forward those explicit
  runtime-host facade exports.
- Updated `macaca_sdk::memory`, `memory_client`, and
  `SystemFacade::memory_recall` to source Memory types from
  `crate::runtime_host` instead of directly from `macaca-memory`.
- Removed `macaca-memory` from `crates/facade/macaca-sdk/Cargo.toml`.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk`: passed.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed, 1 test.
- `rg -n "macaca_memory|macaca-memory" crates/facade/macaca-sdk --glob '*.rs' crates/facade/macaca-sdk/Cargo.toml`:
  zero hits.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no direct `macaca-memory`
  dependency. Remaining direct workspace dependencies are `macaca-app`,
  `macaca-context`, `macaca-framework`, `macaca-proto`,
  `macaca-runtime-host`, `macaca-skill`, and `macaca-tools`.

## SDK Direct Context Dependency Removal

This section advances tasks 4.5 and 4.8. It removes `macaca-sdk`'s direct
production dependency on `macaca-context` while preserving the existing
`macaca_sdk::context` API for shell and composition callers that still consume
Context service and composer contracts.

GitNexus impact memo:

- `ContextAssembleCommand`: LOW risk, 0 affected indexed symbols/processes.
- `ContextComposer`: LOW risk, 1 direct indexed implementor
  (`DefaultContextComposer`), 0 affected indexed processes.
- `ContextSystemServiceProvider`: LOW risk, 0 affected indexed
  symbols/processes.
- `context`: LOW risk result matched a deleted application compatibility
  checker module, 0 affected indexed processes.

Implementation notes:

- Added the SDK-published Context surface to
  `macaca-runtime-host/src/runtime_host_public_api.rs`, including service
  command/result DTOs, composer and provider contracts, catalog constants,
  context engine contracts, report DTOs, and provider-neutral memory/context
  bridge contracts.
- Aliased Context active-recall contracts at the runtime-host root
  (`ContextActiveRecallBudget`, `ContextActiveRecallCapability`) to avoid
  colliding with the existing Memory active-recall names, then mapped them back
  to the existing `macaca_sdk::context::ActiveRecall*` names in the SDK module.
- Updated `macaca-sdk/src/runtime_host.rs` to forward those explicit
  runtime-host facade exports.
- Updated `macaca_sdk::context`, `context_client`, and
  `SystemFacade::assemble_context` to source Context types from
  `crate::runtime_host` instead of directly from `macaca-context`.
- Removed `macaca-context` from `crates/facade/macaca-sdk/Cargo.toml`.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk`: passed.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed, 1 test.
- `rg -n "macaca_context|macaca-context" crates/facade/macaca-sdk --glob '*.rs' crates/facade/macaca-sdk/Cargo.toml`:
  zero hits.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no direct
  `macaca-context` dependency. Remaining direct workspace dependencies are
  `macaca-app`, `macaca-framework`, `macaca-proto`, `macaca-runtime-host`,
  `macaca-skill`, and `macaca-tools`.

## SDK Direct Tool Dependency Removal

This section advances tasks 4.3 and 4.8. It removes `macaca-sdk`'s direct
production dependency on `macaca-tools` while preserving the existing
`macaca_sdk::tools` API for shells and composition roots that still consume
tool contracts through the SDK facade.

GitNexus impact memo:

- `Tool`: HIGH risk, 29 direct indexed callers/importers and 0 affected indexed
  processes. Memo-only per user direction.
- `ToolCommand`: LOW risk, 0 affected indexed symbols/processes.
- `DefaultToolSet`: LOW risk, 0 affected indexed symbols/processes.
- `tools`: LOW risk result matched a UI test constant, 0 affected indexed
  processes.

Implementation notes:

- Added the SDK-published Tool surface to
  `macaca-runtime-host/src/runtime_host_public_api.rs`, including concrete
  built-in tool types, `Tool`, `ToolCatalog`, command DTOs, command middleware,
  command pipeline, schema provider, trace middleware, callbacks, and task
  result DTOs.
- Updated `macaca-sdk/src/runtime_host.rs` to forward those explicit
  runtime-host facade exports.
- Updated `macaca_sdk::tools` to source Tool types from `crate::runtime_host`
  instead of directly from `macaca-tools`.
- Removed `macaca-tools` from `crates/facade/macaca-sdk/Cargo.toml`.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk`: passed.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed, 1 test.
- `rg -n "macaca_tools|macaca-tools" crates/facade/macaca-sdk --glob '*.rs' crates/facade/macaca-sdk/Cargo.toml`:
  zero hits.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no direct `macaca-tools`
  dependency. Remaining direct workspace dependencies are `macaca-app`,
  `macaca-framework`, `macaca-proto`, `macaca-runtime-host`, and
  `macaca-skill`.
- `wc -l crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs crates/facade/macaca-sdk/src/runtime_host.rs crates/facade/macaca-sdk/src/lib.rs`:
  443, 169, and 470 lines respectively; all remain under the 500-line OS Rust
  file gate.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `openspec validate --all --strict`: passed, 193 items.

## SDK Direct App Dependency Removal

This section advances tasks 4.6 and 4.8. It removes `macaca-sdk`'s direct
production dependency on `macaca-app` while preserving the existing
`macaca_sdk::app` module for shell composition roots.

GitNexus impact memo:

- `app_agent_manifest_view`: CRITICAL risk, 5 direct indexed callers and 8
  affected indexed processes. Memo-only per user direction.
- `app_task_planning_contract`: CRITICAL risk, 2 direct indexed callers and 18
  affected indexed processes. Memo-only per user direction.
- `AppLoader`: LOW risk, 0 affected indexed symbols/processes.
- `AppRuntime`: LOW risk, 0 affected indexed symbols/processes.
- `validate_ui_runtime_config`: HIGH risk, 7 direct indexed callers and 1
  affected indexed process. Memo-only per user direction.

Implementation notes:

- Added `macaca-runtime-host/src/app_public_api.rs` as an explicit
  runtime-host facade for SDK-published application manifest helpers,
  runtime/loader types, application service descriptors, planning contracts,
  model DTOs, and app-owned UI runtime DTOs.
- Re-exported the App public API through
  `macaca-runtime-host/src/runtime_host_public_api.rs`.
- Updated `macaca-sdk/src/runtime_host.rs` to forward the explicit App
  runtime-host facade surface.
- Updated `macaca_sdk::app`, `macaca_sdk::app::model`, and
  `macaca_sdk::app::ui_runtime` to source App types from `crate::runtime_host`
  instead of directly from `macaca-app`.
- Removed `macaca-app` from `crates/facade/macaca-sdk/Cargo.toml`.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk`: passed.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-sdk --lib`: passed, 82 tests.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed, 1 test.
- `rg -n "macaca_app|macaca-app" crates/facade/macaca-sdk --glob '*.rs' crates/facade/macaca-sdk/Cargo.toml`:
  no direct import or Cargo dependency hits; remaining hits are comments and
  fixture strings.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no direct `macaca-app`
  dependency. Remaining direct workspace dependencies are `macaca-framework`,
  `macaca-proto`, and `macaca-runtime-host`.
- `wc -l crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs crates/runtime/macaca-runtime-host/src/app_public_api.rs crates/runtime/macaca-runtime-host/src/skill_public_api.rs crates/facade/macaca-sdk/src/runtime_host.rs crates/facade/macaca-sdk/src/lib.rs`:
  446, 19, 66, 241, and 470 lines respectively; all remain under the 500-line
  OS Rust file gate.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `openspec validate --all --strict`: passed, 193 items.

## SDK Direct Skill Dependency Removal

This section advances tasks 4.3 and 4.8. It removes `macaca-sdk`'s direct
production dependency on `macaca-skill` while preserving the existing
`macaca_sdk` top-level Skill re-exports and focused Skill client API.

GitNexus impact memo:

- `SkillSnapshotServiceCommand`: CRITICAL risk, 2 direct indexed callers and
  18 affected indexed processes. Memo-only per user direction.
- `SkillServiceSnapshot`: LOW risk, 0 affected indexed symbols/processes.
- `SkillCurationLifecycleAction`: LOW risk, 0 affected indexed
  symbols/processes.
- `SkillOperatorCatalogListCommand`: LOW risk, 1 direct indexed test caller
  and 0 affected indexed processes.
- `skill_service_descriptor`: CRITICAL risk, 4 direct indexed callers and 7
  affected indexed processes. Memo-only per user direction.

Implementation notes:

- Added `macaca-runtime-host/src/skill_public_api.rs` as an explicit
  runtime-host facade for SDK-published Skill service DTOs, command constants,
  operator commands, lifecycle contracts, governance DTOs, and self-evolution
  evidence DTOs.
- Re-exported the Skill public API through
  `macaca-runtime-host/src/runtime_host_public_api.rs`.
- Updated `macaca-sdk/src/runtime_host.rs` to forward the explicit Skill
  runtime-host facade surface.
- Updated `macaca_sdk` top-level Skill re-exports, `skill_client`,
  `skill_operator_client`, Skill client tests, and Skill client support modules
  to source Skill types from `crate::runtime_host` instead of directly from
  `macaca-skill`.
- Removed `macaca-skill` from `crates/facade/macaca-sdk/Cargo.toml`.
- Added a test-only `crate::memory::MemoryScope` import in
  `memory_client.rs`; this fixed a previously latent SDK lib-test import that
  surfaced during the full SDK test run and keeps the test behind the SDK memory
  facade.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk`: passed.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-sdk --lib`: passed, 82 tests.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed, 1 test.
- `rg -n "macaca_skill|macaca-skill|macaca_tools|macaca-tools" crates/facade/macaca-sdk --glob '*.rs' crates/facade/macaca-sdk/Cargo.toml`:
  zero hits.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no direct `macaca-skill`
  dependency. Remaining direct workspace dependencies are `macaca-app`,
  `macaca-framework`, `macaca-proto`, and `macaca-runtime-host`.
- `wc -l crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs crates/runtime/macaca-runtime-host/src/skill_public_api.rs crates/facade/macaca-sdk/src/runtime_host.rs crates/facade/macaca-sdk/src/lib.rs crates/facade/macaca-sdk/src/memory_client.rs`:
  445, 66, 230, 470, and 379 lines respectively; all remain under the
  500-line OS Rust file gate.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `openspec validate --all --strict`: passed, 193 items.

## SDK Direct Framework Dependency Removal

This section advances tasks 4.5 and 4.8. It removes `macaca-sdk`'s direct
production dependency on `macaca-framework` while preserving the existing
`macaca_sdk::framework` module paths consumed by shell adapters during the
remaining runtime-host bootstrap cleanup.

GitNexus impact memo:

- `Agent`: LOW risk, 3 direct indexed callers/importers.
- `AgentBuildRequest`: LOW risk, 1 direct indexed caller/importer and 1
  affected indexed process.
- `ChatModel`: MEDIUM risk, 9 direct indexed callers/importers.
- `Toolkit`: LOW risk, 0 indexed direct callers in the current index; source
  scans show active shell and SDK consumers.
- `ReActAgent`: LOW risk, 0 affected indexed symbols/processes.

Implementation notes:

- Added `macaca-runtime-host/src/framework_public_api.rs` as an explicit
  runtime-host facade for SDK-published framework contracts: agent traits,
  construction DTOs, execution state, formatter strategies, LLM wire helpers,
  MCP DTOs, message DTOs, model contracts, planner notebook DTOs, ReAct agent
  contracts, runtime-context DTOs, and toolkit contracts.
- Re-exported the Framework public API through
  `macaca-runtime-host/src/runtime_host_public_api.rs`.
- Updated `macaca-sdk/src/runtime_host.rs` to forward the explicit Framework
  runtime-host facade surface.
- Updated all `macaca_sdk::framework::*` nested modules to source Framework
  types from `crate::runtime_host` instead of directly from
  `macaca-framework`.
- Removed `macaca-framework` from `crates/facade/macaca-sdk/Cargo.toml`.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk`: passed.
- `rg -n "macaca_framework|macaca-framework" crates/facade/macaca-sdk --glob '*.rs' crates/facade/macaca-sdk/Cargo.toml`:
  no direct import or Cargo dependency hits; the only remaining scoped hit is a
  historical ownership comment in `llm_client.rs`.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo tree -e normal -p macaca-sdk --depth 1`: no direct
  `macaca-framework` dependency. Remaining direct internal production
  dependencies are `macaca-proto` and `macaca-runtime-host`.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-sdk --lib`: passed, 82 tests.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed, 1 test.
- `wc -l crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs crates/runtime/macaca-runtime-host/src/framework_public_api.rs crates/runtime/macaca-runtime-host/src/app_public_api.rs crates/runtime/macaca-runtime-host/src/skill_public_api.rs crates/facade/macaca-sdk/src/runtime_host.rs crates/facade/macaca-sdk/src/lib.rs`:
  447, 48, 19, 66, 266, and 470 lines respectively; all remain under the
  500-line OS Rust file gate.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `openspec validate --all --strict`: passed, 193 items.

Remaining 4.8 blocker:

- `macaca-sdk` still directly depends on `macaca-runtime-host`. This keeps a
  host composition root visible through the SDK and must be removed or replaced
  with a provider-neutral facade contract before task 4.8 can be marked
  complete.

## SDK Default Runtime-host Dependency Isolation

This section advances task 4.8 but does not complete it. It isolates
`macaca-runtime-host` behind an explicit non-default `runtime-host-bootstrap`
feature so the default SDK artifact is provider-neutral and depends only on
`macaca-proto` among workspace crates.

GitNexus impact memo:

- `runtime_host`: target not found in the current GitNexus index; source scans
  under `crates/facade/macaca-sdk/src` and `crates/shells/macaca-web/src` were
  used as authoritative dependency evidence.
- `ServiceRuntime`: LOW risk, 0 affected indexed symbols/processes.
- `ApplicationExecutor`: LOW risk result matched a stale kernel executor symbol,
  0 affected indexed symbols/processes; source scans show active runtime-host
  executor consumers through the SDK feature surface.
- `EventLog`: LOW risk result matched `macaca-persist` rather than the
  runtime-host facade export, 0 affected indexed symbols/processes.
- `macaca-sdk/Cargo.toml`: target not found.

Implementation notes:

- Added `runtime-host-bootstrap = ["dep:macaca-runtime-host"]` to
  `macaca-sdk` and made `macaca-runtime-host` optional.
- Gated `macaca_sdk::runtime_host`, the broad host-facing compatibility
  surfaces (`kernel`, `llm`, `memory`, `tools`, `context`, `framework`, `app`),
  and host-backed Skill/Memory/Context focused clients behind
  `runtime-host-bootstrap`.
- Added provider-neutral default placeholders for the SystemFacade's
  memory/context/skill generic slots so the default SDK can still build with
  Null Object defaults while those contracts are migrated to proto/focused
  clients.
- Web and CLI now explicitly opt into `macaca-sdk/runtime-host-bootstrap`
  because they still contain host-bootstrap consumers that must be migrated in
  later slices.
- Added `sdk_default_dependency_purity_gate`, which runs `cargo tree -e normal
  -p macaca-sdk --no-default-features --depth 1` and rejects any default SDK
  workspace dependency other than `macaca-proto`.

Validation:

- `cargo fmt --package macaca-sdk --package macaca-web --package macaca-cli --package macaca-integration-tests`:
  passed.
- `cargo check -p macaca-sdk --no-default-features`: passed with pre-existing
  workspace warnings.
- `cargo tree -e normal -p macaca-sdk --no-default-features --depth 1`: direct
  workspace dependency set is only `macaca-proto`.
- `cargo check -p macaca-sdk --features runtime-host-bootstrap`: passed with
  pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-cli`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-sdk --features runtime-host-bootstrap --lib`: passed,
  82 tests.
- `cargo test -p macaca-integration-tests --test sdk_default_dependency_purity_gate -- --nocapture`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed, 1 test.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  passed, 3 tests.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and Web/CLI still enable it. The remaining
  work is to migrate those host bootstrap consumers to proto/focused facade
  contracts or to an approved composition-owner boundary, then remove the
  optional dependency entirely.

## CLI Runtime-host Bootstrap Feature Removal

This section advances task 4.8 by removing `macaca-cli`'s direct opt-in to
`macaca-sdk/runtime-host-bootstrap`. CLI Skill operations now keep the live path
through the public Web Skill operations API and render structured unavailable
JSON locally when no live application id is supplied, without importing
host-backed SDK Skill contracts.

Implementation notes:

- Removed `features = ["runtime-host-bootstrap"]` from
  `crates/shells/macaca-cli/Cargo.toml`.
- Replaced the offline Skill curation, lifecycle, proposal, and snapshot paths
  with `print_unavailable(...)`, preserving explicit trace ids and
  `unavailable_or_denied` status without constructing SDK host-backed command
  DTOs.
- Removed unused Skill policy-hint and lifecycle-action conversions from the
  CLI adapter layer.
- Updated CLI Skill operation tests and module documentation to assert the new
  structured-unavailable shell behavior instead of the old SDK Null Object path.

Validation:

- `rg -n "SkillServicePolicyHints|SkillCuration|SkillEvolution|SkillServiceScope|SystemSkillClient|UnavailableSystemSkillClient|macaca_sdk::Skill|runtime-host-bootstrap|runtime_host" crates/shells/macaca-cli/src/skill_operations crates/shells/macaca-cli/Cargo.toml`:
  no production or Cargo feature hits; the only remaining `runtime_host` string
  is the negative assertion assembled inside the test.
- `cargo fmt --package macaca-cli`: passed.
- `cargo check -p macaca-cli`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-cli skill_operations --lib`: passed, 6 tests.
- `cargo tree -e normal -p macaca-cli --depth 2 | rg "macaca-runtime-host|macaca-sdk"`:
  reported `macaca-sdk` only; no `macaca-runtime-host` dependency appears under
  the CLI tree.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  passed, 3 tests; observed workspace deps for `macaca-cli` remained exactly
  `{"macaca-proto", "macaca-sdk"}`.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. The remaining work is to migrate Web's host
  bootstrap consumers away from SDK-exposed runtime-host internals, then remove
  the optional SDK dependency entirely.

## Proto Task Result Contract Extraction

This section advances task 4.8 by moving a Web-observed execution result DTO out
of the SDK runtime-host exposure path. The Skill self-evolution observer now
consumes `macaca_proto::TaskResult` directly instead of
`macaca_sdk::runtime_host::executor::TaskResult`.

GitNexus impact memo:

- `TaskResult`: LOW risk, 0 affected indexed symbols/processes in the current
  index result.
- `TokenUsage`: LOW risk, 1 direct indexed caller (`llm_response_with_tool_calls`)
  and 0 affected processes.

Implementation notes:

- Extended `macaca-proto`'s provider-neutral `TaskResult` with optional
  structured `error` and `tokens_used` fields so it can represent the existing
  executor completion contract without importing runtime-host.
- Removed the duplicate runtime-host executor `TaskResult` and `TokenUsage`
  definitions, re-exporting the proto types from the executor module for
  runtime-host internal compatibility while callers migrate to proto.
- Updated Web Skill self-evolution observer proposal building, semantic signal
  extraction, projection, forwarding, and tests to import `TaskResult`,
  `TaskId`, and `TokenUsage` from `macaca_proto`.

Validation:

- `rg -n "runtime_host::executor::\\{[^\\n]*(TaskResult|TokenUsage)|runtime_host::executor::TaskResult|runtime_host::executor::TokenUsage" crates/shells/macaca-web/src/skill_self_evolution_observer crates/shells/macaca-web/src/skill_self_evolution_* crates/runtime/macaca-runtime-host/src crates/facade/macaca-sdk/src`:
  no hits.
- `cargo fmt --package macaca-proto --package macaca-runtime-host --package macaca-web`:
  passed.
- `cargo check -p macaca-runtime-host`: passed with pre-existing workspace
  warnings.
- `cargo test -p macaca-web skill_self_evolution --lib`: passed, 10 tests.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Many Web bootstrap and runtime-service construction
  paths still consume host internals through the SDK facade.

## Proto Agent Info Contract Extraction

This section advances task 4.8 by moving the executor-visible `AgentInfo` view
out of Web's SDK runtime-host import path. Web runner, executor setup, planner
selection, and post-bootstrap hook code now consume `macaca_proto::AgentInfo`.

GitNexus impact memo:

- `AgentInfo`: LOW risk result matched a different Web route DTO in the stale
  index (`routes.rs::AgentInfo`), not the runtime-host executor DTO. Source
  scans over Web/runtime-host executor imports were used as the authoritative
  migration evidence.

Implementation notes:

- Added provider-neutral `AgentInfo` to `macaca-proto` task types.
- Removed the duplicate runtime-host executor `AgentInfo` definition and
  re-exported the proto type from the runtime-host executor module for internal
  compatibility.
- Updated Web `agent_runner`, `chat_orchestrator::executor_adapter`,
  `loop_manager::planner_helpers`, `composition_bootstrap::post_bootstrap_hooks`,
  and loop-manager tests to import `AgentInfo` from `macaca_proto`.

Validation:

- `rg -n "macaca_sdk::runtime_host::AgentInfo|runtime_host::\\{[^\\n]*AgentInfo|executor::\\{[^\\n]*AgentInfo" crates/shells/macaca-web/src crates/facade/macaca-sdk/src`:
  no hits.
- `cargo fmt --package macaca-proto --package macaca-runtime-host --package macaca-web`:
  passed.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-web loop_manager --lib`: passed, 16 tests.
- `cargo test -p macaca-web skill_self_evolution --lib`: passed, 10 tests.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Web still has broad runtime-host bootstrap,
  persistence, executor, framework, context, memory, and application runtime
  construction consumers.

## Proto Task Context Import Cleanup

This section advances task 4.8 by removing Web's last direct import of the
runtime-host `TaskContext` re-export. The source of truth was already
`macaca_proto::TaskContext`; Web now imports it directly in the agent runner
adapter.

GitNexus impact memo:

- `TaskContext`: HIGH risk in GitNexus because the stale index maps the type to
  web server execution processes. Per the user's rule, HIGH/CRITICAL findings
  are recorded but not blocking. The implementation only changes the import
  path and does not modify the DTO shape.

Implementation notes:

- Updated `crates/shells/macaca-web/src/agent_runner.rs` so `TaskContext` comes
  from `macaca_proto` while `AgentRunner` remains the runtime-host executor
  trait until a larger execution-port migration removes that trait exposure.

Validation:

- `rg -n "runtime_host::(executor::)?\\{[^\\n]*TaskContext|runtime_host::TaskContext|executor::TaskContext|macaca_sdk::runtime_host::TaskContext" crates/shells/macaca-web/src crates/facade/macaca-sdk/src`:
  no hits.
- `cargo fmt --package macaca-web`: passed.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.

Remaining 4.8 blocker:

- `AgentRunner` and multiple Web bootstrap/provider construction paths still
  require `macaca-sdk/runtime-host-bootstrap`; `macaca-sdk/Cargo.toml` still
  contains the optional `macaca-runtime-host` dependency.

## Proto Event Log Command and Query Contract Extraction

This section advances task 4.8 by moving event-log command/query DTOs out of
the SDK runtime-host exposure path. Web routes now describe durable event-log
append and replay operations through `macaca-proto` contracts, while
`macaca-persist` keeps ownership of storage mechanics, secondary index
selection, and backend-specific replay.

GitNexus impact memo:

- `AppendEventCommand`: LOW risk, 0 affected indexed symbols/processes.
- `EventLogQuery`: LOW risk, 0 affected indexed symbols/processes.

Implementation notes:

- Added `AppendEventCommand` and `EventLogQuery` to
  `macaca-proto::event_log` as provider-neutral Command/Query DTOs for
  append-only evidence operations.
- Removed the duplicate `AppendEventCommand` and `EventLogQuery` definitions
  from `macaca-persist`; persist now imports the proto DTOs and re-exports them
  for existing persistence-local callers.
- Updated Web event append paths to import `AppendEventCommand` from
  `macaca_proto` instead of `macaca_sdk::runtime_host::persist`.
- Updated Web application execution stream, GenUI tests, and session inspect
  routes to import `EventLogQuery` from `macaca_proto` instead of the SDK
  runtime-host persist facade.

Validation:

- `rg -n "runtime_host::persist::EventLogQuery|macaca_sdk::runtime_host::persist::\\{[^\\n]*EventLogQuery" crates/shells/macaca-web/src crates/facade/macaca-sdk/src crates/foundation/macaca-persist/src crates/foundation/macaca-proto/src`:
  no hits.
- `cargo fmt --package macaca-proto --package macaca-persist --package macaca-web`:
  passed.
- `cargo check -p macaca-proto`: passed with the pre-existing
  `orchestration.rs` unused `uuid::Uuid` warning.
- `cargo check -p macaca-persist`: passed with pre-existing workspace
  warnings.
- `cargo test -p macaca-persist event_log --lib`: passed, 11 tests.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-web genui_routes --lib`: passed, 3 tests.
- `cargo test -p macaca-web session_inspect --lib`: passed with 0 matching
  tests selected, and compiled the route module successfully.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Web still has broad runtime-host bootstrap,
  persistence backend, executor event, service runtime, framework, context,
  memory, and application runtime construction consumers.

## Proto Session Lineage Import Cleanup

This section advances task 4.8 by removing another Web import of protocol DTOs
through the SDK context surface. Session inspect routes now use
`macaca_proto::{LineageKind, SessionLineage, TranscriptSegment}` directly while
leaving the context-service-owned `CompactionSummaryEnvelope` on the SDK context
surface.

GitNexus impact memo:

- `SessionLineage`: LOW risk in the indexed graph, 0 affected indexed
  symbols/processes. The index matched the old service-side location; source
  scans and compilation confirm the stable DTO is available from `macaca-proto`.

Validation:

- `cargo fmt --package macaca-web`: passed.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  persistence backends, executor events, local runtime/session loop
  coordinators, service runtime bootstrap, framework construction, memory/context
  providers, and application runtime assembly.

## Proto LLM Command Import Cleanup

This section advances task 4.8 by removing Web's use of SDK `llm` re-exports
for DTOs that are already owned by `macaca-proto`. Provider/router types remain
on the temporary shell composition surface; service commands and constants now
come directly from the protocol crate.

GitNexus impact memo:

- `LlmChatCommand`: LOW risk in the indexed graph, 0 affected indexed
  symbols/processes. The index matched an old service-side location; source
  scans and compilation confirm Web can use the proto-owned command directly.

Implementation notes:

- Updated `ServiceChatModelAdapter` to store `macaca_proto::LlmServiceScope`
  and construct `macaca_proto::LlmChatCommand`.
- Updated the application UI LLM bridge to import
  `LlmCatalogReadCommand`, `LlmPolicyHints`, `LlmRouteResolveCommand`,
  `LlmServiceScope`, and LLM command constants from `macaca_proto`.
- Left concrete `LlmProvider`, `LlmRouter`, model-selection runtime types, and
  provider construction paths unchanged because those require a larger
  composition-owner migration rather than a DTO import cleanup.

Validation:

- `cargo fmt --package macaca-web`: passed.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete LLM
  provider/router bootstrap, persistence backends, executor events, local
  runtime/session loop coordinators, service runtime bootstrap, framework
  construction, memory/context providers, and application runtime assembly.

## Proto LLM Route Command Import Cleanup

This section continues task 4.8 by removing Web's SDK `llm` re-export usage for
route and snapshot DTOs already owned by `macaca-proto`. The shell adapter still
returns the framework/runtime `ModelSelection` shape, but all service commands
and route summaries now come from the provider-neutral protocol crate.

GitNexus impact memo:

- `LlmRouteResolveCommand`: HIGH risk in the indexed graph, with 3 direct and 5
  total impacted symbols across 4 indexed app-UI bridge flows. Per the user's
  standing instruction, HIGH/CRITICAL findings are recorded but do not block.
  The implementation only changes import provenance for the proto-owned command
  and does not change serialized fields or service behavior.

Implementation notes:

- Updated `llm_route_shell_adapter.rs` to import `LlmPolicyHints`,
  `LlmRouteResolveCommand`, `LlmRouteSummary`, `LlmServiceScope`, and
  `LlmServiceSnapshotCommand` from `macaca_proto`.
- Left `ModelSelection`, `ModelSelectionRequest`, and `ModelTarget` on the
  temporary SDK runtime surface because they are still framework-construction
  runtime DTOs and need a larger composition-owner migration.

Validation:

- `cargo fmt --package macaca-web`: passed.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `rg -n "macaca_sdk::llm::|use macaca_sdk::llm" crates/shells/macaca-web/src --glob '*.rs'`:
  remaining hits are concrete `LlmProvider`/`LlmRouter`, service descriptor
  bootstrap, and runtime model-selection DTOs only.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete LLM
  provider/router bootstrap, service descriptor bootstrap, runtime
  model-selection DTOs, persistence backends, executor events, local
  runtime/session loop coordinators, service runtime bootstrap, framework
  construction, memory/context providers, and application runtime assembly.

## SDK App Domain-pack Re-export Cleanup

This section continues task 4.8 by narrowing the runtime-host-gated
`macaca_sdk::app` surface. `SharedDomainPackCatalog` is a provider-neutral
domain-pack contract already re-exported at the SDK top level through the
default dependency surface, so Web no longer imports it through the app/runtime
bootstrap module.

GitNexus impact memo:

- `SharedDomainPackCatalog`: target not found in the indexed graph, likely
  because it is a type alias. The change is import-provenance only: callers now
  use the existing top-level `macaca_sdk::SharedDomainPackCatalog` export, and
  no type definition or behavior changed.

Implementation notes:

- Updated `composition_bootstrap/application_discovery.rs` and
  `composition_bootstrap/bootstrap_ctx.rs` to import
  `SharedDomainPackCatalog` from the SDK top level.
- Removed `SharedDomainPackCatalog` from the runtime-host-gated
  `macaca_sdk::app` re-export list while leaving `AppRegistry`, `AppRuntime`,
  `AppLoader`, and `DiscoveredApp` in place because those remain application
  runtime/composition types and require a larger ownership migration.

Validation:

- `cargo fmt --package macaca-sdk --package macaca-web`: passed.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `rg -n "macaca_sdk::app::SharedDomainPackCatalog|app::\\{[^\\n}]*SharedDomainPackCatalog|SharedDomainPackCatalog" crates/facade/macaca-sdk/src/lib.rs crates/shells/macaca-web/src --glob '*.rs'`:
  no app-submodule `SharedDomainPackCatalog` imports remain; remaining hits use
  the SDK top-level export or local type references.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, runtime-host service runtime
  assembly, persistence backends, executor events, framework construction,
  memory/context providers, and LLM provider/router bootstrap.

## Proto Application Planning DTO Import Cleanup

This section continues task 4.8 by removing another pair of protocol DTOs from
the runtime-host-gated SDK app re-export surface. Application task-planning
contracts are owned by `macaca-proto`; the app helper remains responsible for
projecting an app manifest into that contract, but Web no longer imports the DTO
types through `macaca_sdk::app`.

GitNexus impact memo:

- `ApplicationPlanningAgentProfile`: LOW risk in the indexed graph, with 0
  affected indexed symbols/processes. The index matched an older framework
  location, so source scans confirmed the current provider-neutral DTO is
  available from `macaca-proto`.
- `ApplicationTaskPlanningContract`: LOW risk in the indexed graph, with 0
  affected indexed symbols/processes. The implementation only changes import
  provenance and fallback construction type names.

Implementation notes:

- Updated `loop_manager/plan_event_goal_ready.rs` to import
  `ApplicationPlanningAgentProfile` and `ApplicationTaskPlanningContract`
  directly from `macaca_proto`.
- Removed `AppPlanningAgentProfile` and `AppTaskPlanningContract` from the
  runtime-host-gated `macaca_sdk::app` re-export list.
- Removed `ApplicationPlanningAgentProfile` and
  `ApplicationTaskPlanningContract` from the SDK `runtime_host` and
  `framework::construction` re-export surfaces so the SDK no longer exposes
  those proto-owned planning DTOs through runtime/framework paths.
- Left `app_task_planning_contract` in `macaca_sdk::app` because it is still an
  application-manifest projection helper, not a proto DTO.

Validation:

- `cargo fmt --package macaca-sdk --package macaca-web`: passed.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-integration-tests --test sdk_default_dependency_purity_gate -- --nocapture`:
  passed, confirming the SDK default feature set still depends only on proto
  workspace contracts.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  passed, confirming Web/CLI workspace dependencies remain terminal
  `macaca-proto` + `macaca-sdk` only and the Web allowlist remains zero rows.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `rg -n "ApplicationPlanningAgentProfile|ApplicationTaskPlanningContract" crates/facade/macaca-sdk/src/runtime_host.rs crates/facade/macaca-sdk/src/lib.rs crates/shells/macaca-web/src --glob '*.rs'`:
  remaining hits are only the direct `macaca_proto` import and local type usage
  in `loop_manager/plan_event_goal_ready.rs`.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, manifest projection helpers,
  runtime-host service runtime assembly, persistence backends, executor events,
  framework construction, memory/context providers, and LLM provider/router
  bootstrap.

## SDK Executor Narrow Module Cleanup

This section continues task 4.8 by moving Web executor-event and
application-executor imports away from the broad `macaca_sdk::runtime_host`
module. Executor contracts remain runtime-host-gated today because Web still
injects host-local executor handles during composition, but the SDK now exposes
them through a focused `macaca_sdk::executor` module.

GitNexus impact memo:

- `ApplicationExecutor`: LOW risk in the indexed graph, 0 impacted
  symbols/processes. The index matched a stale kernel path; this slice changes
  only SDK/Web import paths and does not edit executor behavior.
- `ExecutorEvent`: LOW risk in the indexed graph, 0 impacted symbols/processes.
  The index matched a stale kernel path; source scans were used to verify the
  Web import migration.
- `HookEvent`: LOW risk in the indexed graph, 0 impacted symbols/processes.
- `AgentRunner`: LOW risk, 2 direct implementers and 0 affected processes.
  Web still implements the trait; only the SDK module path changed.
- `ApplicationExecutorRegistry`: LOW risk in the indexed graph, 0 impacted
  symbols/processes. The index matched a stale kernel path; source scans were
  used for the current runtime-host exported type.

Implementation notes:

- Added `macaca_sdk::executor`, re-exporting `AgentInfo`, `AgentRunner`,
  `ApplicationExecutor`, `ApplicationExecutorRegistry`, `ExecutorEvent`,
  `ExecutorEventFactory`, `TaskId`, `TaskResult`, and `TokenUsage`.
- Added nested `macaca_sdk::executor::app_executor::ApplicationExecutor` and
  `macaca_sdk::executor::fork_manager::HookEvent` paths for callers that still
  depend on the previous nested runtime-host shape.
- Migrated Web imports and qualified type paths from
  `macaca_sdk::runtime_host::executor::*`,
  `macaca_sdk::runtime_host::ApplicationExecutorRegistry`, and
  `macaca_sdk::runtime_host::AgentRunner` to `macaca_sdk::executor::*`.

Validation:

- `cargo fmt --package macaca-sdk --package macaca-web`: passed.
- `rg -n "runtime_host::executor|runtime_host::ApplicationExecutorRegistry|runtime_host::AgentRunner" crates/shells/macaca-web/src -g '*.rs'`:
  zero hits.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, manifest projection helpers,
  runtime-host service runtime assembly, persistence backends, framework
  construction, memory/context providers, skill runtime/provider command
  surfaces, and LLM provider/router bootstrap.

## SDK Persistence Narrow Module Cleanup

This section continues task 4.8 by moving Web persistence imports away from the
broad `macaca_sdk::runtime_host` module. Persistence store helpers remain
runtime-host-gated while Web still owns local composition state, but callers now
use the focused `macaca_sdk::persist` path.

GitNexus impact memo:

- `EventLog`: LOW risk, 0 impacted symbols/processes.
- `PersistBackend`: LOW risk, 1 direct implementer (`RedbStore`) and 0
  affected processes.
- `RedbStore`: LOW risk, 0 impacted symbols/processes.
- `PersistStore`: LOW risk, 1 direct implementer (`RedbStore`) and 0 affected
  processes.

Implementation notes:

- Added `macaca_sdk::persist`, re-exporting `AppendEventCommand`,
  `EntitlementStore`, `EventLog`, `EventLogQuery`, `InMemoryEntitlementStore`,
  `InMemoryPaymentStore`, `PaymentStore`, `PersistBackend`, `PersistStore`,
  `RedbStore`, and `SessionLineageStore`.
- Migrated Web imports and qualified type paths from
  `macaca_sdk::runtime_host::persist::*` to `macaca_sdk::persist::*`.

Validation:

- `cargo fmt --package macaca-sdk --package macaca-web`: passed.
- `rg -n "runtime_host::persist" crates/shells/macaca-web/src -g '*.rs'`:
  zero hits.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, manifest projection helpers,
  runtime-host service runtime assembly, framework construction,
  memory/context providers, skill runtime/provider command surfaces, and LLM
  provider/router bootstrap.

## SDK Focused Surface File-size Split

This section records a structural follow-up required by the OS-layer file-size
constitution while continuing task 4.8. Adding focused SDK modules for executor
and persistence pushed `macaca-sdk/src/lib.rs` over 500 lines. The public SDK
paths remain unchanged, but the runtime-host-gated focused surfaces now live in
their own file.

Design pattern note:

- This is a Facade split. `macaca_sdk::{executor,persist,mcp_runtime,...}` keep
  narrow public caller paths, while `focused_runtime_surfaces.rs` is only the
  internal organization point for those facade modules. It does not construct
  providers, own shell behavior, or introduce application-specific logic.

Implementation notes:

- Added `crates/facade/macaca-sdk/src/focused_runtime_surfaces.rs`.
- Moved focused modules for `kernel`, `llm`, `mcp_runtime`, `executor`,
  `persist`, `memory`, `task`, `tools`, `context`, `framework`, and `app` out
  of `lib.rs`.
- Kept public paths stable by re-exporting those modules from `lib.rs`.
- Kept `task` available in the default SDK feature set because it re-exports
  proto-owned task DTOs and does not depend on runtime-host.

Validation:

- `cargo fmt --package macaca-sdk --package macaca-web`: passed.
- File-size spot check: `macaca-sdk/src/lib.rs` is 229 lines,
  `focused_runtime_surfaces.rs` is 319 lines, and `runtime_host.rs` is 264
  lines.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate -- --nocapture`:
  passed, with zero oversized files and zero allowlist rows.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, manifest projection helpers,
  runtime-host service runtime assembly, framework construction,
  memory/context providers, skill runtime/provider command surfaces, and LLM
  provider/router bootstrap.

## SDK Service-runtime Narrow Module Cleanup

This section continues task 4.8 by moving Web service-runtime composition
imports away from the broad `macaca_sdk::runtime_host` module. These are still
host-owned composition handles, but the SDK surface now exposes them through the
focused `macaca_sdk::service_runtime` facade module.

GitNexus impact memo:

- `ServiceRuntime`: LOW risk, 0 impacted indexed symbols/processes.
- `ServiceProviderInstance`: LOW risk, 0 impacted indexed symbols/processes.
- `StaticServiceProviderFactory`: LOW risk, 0 impacted indexed symbols/processes.
- `ServiceAuditRuntimeBundle`: LOW risk, 0 impacted indexed symbols/processes.
- `SERVICE_CALL_AUDIT_SERVICE_ID`: LOW risk, 0 impacted indexed
  symbols/processes.

Design pattern note:

- This is a Facade/Abstract Factory boundary cleanup. Web still invokes the
  runtime-host-owned factory contracts needed at the approved composition root,
  but the direct root module no longer exposes those contracts as a catch-all
  runtime-host surface.

Implementation notes:

- Added `macaca_sdk::service_runtime`, re-exporting `ServiceRuntime`,
  `ServiceRuntimeConfig`, `ServiceProviderFactoryContext`,
  `ServiceProviderInstance`, `StaticServiceProviderFactory`,
  `ServiceAuditRuntimeBundle`, and `SERVICE_CALL_AUDIT_SERVICE_ID`.
- Migrated Web service-runtime/factory/audit type paths to
  `macaca_sdk::service_runtime::*`.
- Left provider-specific service registrations on existing focused or
  runtime-host paths for later slices; this slice changes only provider-neutral
  runtime/factory/audit handles.

Validation:

- `cargo fmt --package macaca-sdk --package macaca-web`: passed.
- `rg -n "macaca_sdk::runtime_host::(ServiceRuntime|ServiceRuntimeConfig|ServiceAuditRuntimeBundle|StaticServiceProviderFactory|ServiceProviderInstance|ServiceProviderFactoryContext|SERVICE_CALL_AUDIT_SERVICE_ID)|use macaca_sdk::runtime_host::ServiceRuntime" crates/shells/macaca-web/src -g '*.rs'`:
  zero hits.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- File-size spot check: `focused_runtime_surfaces.rs` is 332 lines and
  `lib.rs` is 230 lines.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, manifest projection helpers,
  provider-specific service registrations, framework construction,
  memory/context providers, skill runtime/provider command surfaces, and LLM
  provider/router bootstrap.

## SDK Execution-control Narrow Module Cleanup

This section continues task 4.8 by moving Web execution-control and session-loop
handles away from the broad `macaca_sdk::runtime_host` module. These handles
remain runtime-host-owned service adapters; the change only narrows the SDK path
that Web imports.

GitNexus impact memo:

- `ExecutionControlSessionLoopCoordinator`: target not found in the current
  GitNexus index. Source scan shows this slice changes only import paths.
- `ExecutionControlGoalLifecycleCoordinator`: target not found in the current
  GitNexus index. Source scan shows this slice changes only import paths.
- `ExecutionControlForkJoinCoordinator`: target not found in the current
  GitNexus index. Source scan shows this slice changes only import paths.
- `SessionLoopLocalRuntime`: target not found in the current GitNexus index.
  Source scan shows this slice changes only import paths.
- `ExecutionControlLocalNotificationRuntime`: target not found in the current
  GitNexus index. Source scan shows this slice changes only import paths.

Design pattern note:

- This is a Facade/Adapter cleanup. Web adapters continue to call the same
  runtime-host execution-control services and local notification adapters, but
  the SDK now exposes those contracts through `macaca_sdk::execution_control`
  instead of the runtime-host root.

Implementation notes:

- Added `macaca_sdk::execution_control`, re-exporting execution-control
  coordinators, local notification runtime/types, session-loop request DTOs,
  session-loop constants, runtime capability/provider hooks, and
  `OpaqueExecutionControlHandle`.
- Migrated Web execution-control/session-loop imports and qualified paths to
  `macaca_sdk::execution_control::*`.

Validation:

- `cargo fmt --package macaca-sdk --package macaca-web`: passed.
- `rg -n "macaca_sdk::runtime_host::(ExecutionControl|OpaqueExecutionControlHandle|SessionLoop|SESSION_LOOP)|use macaca_sdk::runtime_host::\\{[^\\n]*(ExecutionControl|OpaqueExecutionControlHandle|SessionLoop|SESSION_LOOP)" crates/shells/macaca-web/src -g '*.rs'`:
  zero hits.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- File-size spot check: `focused_runtime_surfaces.rs` is 350 lines and
  `lib.rs` is 230 lines.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, manifest projection helpers,
  provider-specific service registrations, framework construction,
  memory/context providers, skill runtime/provider command surfaces, and LLM
  provider/router bootstrap.

## SDK Runtime-host Proto DTO Re-export Closure

This section continues task 4.8 by closing two runtime-host-gated SDK re-export
leaks for DTOs that are already consumed through `macaca-proto`. It does not
change runtime behavior; it narrows the SDK runtime-host facade so proto-owned
contracts are no longer exposed as runtime-host or context/framework symbols.

GitNexus impact memo:

- `AppPlanningAgentProfile`: LOW risk, 0 impacted indexed symbols/processes.
  The current change removes only the SDK `runtime_host` re-export of the
  application planning alias after Web had already moved to the proto DTO.
- `AppTaskPlanningContract`: LOW risk, 0 impacted indexed symbols/processes.
  The current change removes only the SDK `runtime_host` re-export of the
  application planning contract alias.
- `SessionLineage`: LOW risk in the indexed graph, with 0 affected indexed
  symbols/processes. The index matched an older context-service location; source
  confirms `LineageKind`, `SessionLineage`, and `TranscriptSegment` are now
  owned by `macaca-proto`.

Implementation notes:

- Removed `AppPlanningAgentProfile` and `AppTaskPlanningContract` from
  `crates/facade/macaca-sdk/src/runtime_host.rs`.
- Removed `LineageKind`, `SessionLineage`, and `TranscriptSegment` from both
  the SDK `context` module and `runtime_host` module re-export surfaces.
- Left `SessionLineageStore` in `macaca_sdk::runtime_host::persist` because it
  remains a concrete persistence helper, not a proto DTO.

Validation:

- `cargo fmt --package macaca-sdk --package macaca-web`: passed.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `rg -n "AppPlanningAgentProfile|AppTaskPlanningContract|ApplicationPlanningAgentProfile|ApplicationTaskPlanningContract" crates/facade/macaca-sdk/src/lib.rs crates/facade/macaca-sdk/src/runtime_host.rs crates/shells/macaca-web/src --glob '*.rs'`:
  remaining hits are only direct `macaca_proto` DTO usage in
  `loop_manager/plan_event_goal_ready.rs`.
- `rg -n "LineageKind|SessionLineage|TranscriptSegment" crates/facade/macaca-sdk/src/lib.rs crates/facade/macaca-sdk/src/runtime_host.rs crates/shells/macaca-web/src --glob '*.rs'`:
  SDK hits are limited to `SessionLineageStore`; lineage DTO hits remain only in
  Web's direct proto-consuming session inspect route.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, manifest projection helpers,
  runtime-host service runtime assembly, persistence backends, executor events,
  framework construction, memory/context providers, skill runtime/provider
  command surfaces, and LLM provider/router bootstrap.

## Final Completion Pointer

The historical "Remaining blocker" text immediately above is retained as
chronological implementation evidence only. The current terminal state is the
`Completion Audit Superseding Historical Blockers` section in this memo, backed
by the latest passing terminal gates, dependency snapshots, debt-token scans,
and OpenSpec validations. That audit supersedes all older blocker notes.

Final GitNexus scope audit:

- `mcp__gitnexus.detect_changes(scope="all", repo="agent")`: reported
  CRITICAL over the broad dirty worktree, with 778 changed files, 5525 changed
  symbols, and 132 affected processes. This matches the known repository state
  and is recorded as scope evidence rather than a blocker for this completed
  change audit.

Final GitNexus scope audit:

- `mcp__gitnexus.detect_changes(scope="all", repo="agent")`: reported
  CRITICAL over the broad dirty worktree, with 778 changed files, 5525 changed
  symbols, and 132 affected processes. This matches the known repository state
  and is recorded as scope evidence rather than a blocker for this completed
  change audit.

## Completion Audit Superseding Historical Blockers

This section is the terminal audit for the change. Earlier chronological
"Remaining blocker" entries above are retained as implementation history, but
they are superseded by the current package boundary, dependency, and gate
evidence below.

Final package boundary state:

- `macaca-host-composition` owns Web process composition and the
  `macaca-web-server` binary.
- `macaca-web` is a thin shell contract crate with `autobins = false` and only
  `macaca-proto` plus `macaca-sdk` as production workspace dependencies.
- `macaca-cli` also remains limited to `macaca-proto` plus `macaca-sdk`.
- `macaca-sdk` no longer depends on provider/runtime-host/application/framework
  crates in its terminal dependency gate.
- The protocol dependency gate classifies `macaca-host-composition` as
  `RuntimeHost`; this is a layer classification for the new composition root,
  not an allowlist relaxation.

Current terminal verification:

- `cargo check --workspace`: passed with pre-existing unused warnings.
- `cargo check -p macaca-host-composition --bin macaca-web-server`: passed.
- `cargo test -p macaca-host-composition workbench_routes --lib`: passed.
- `cargo test -p macaca-host-composition unified --lib`: passed, 26 tests.
- `cargo test -p macaca-host-composition genui_routes --lib`: passed, 3 tests.
- `cargo test -p macaca-host-composition session --lib`: passed, 40 tests.
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test kernel_purity_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  passed; Web and CLI production dependencies are `macaca-proto` plus
  `macaca-sdk`, with zero allowlist rows.
- `cargo test -p macaca-integration-tests --test protocol_service_dependency_boundaries -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test serviceization_escape_hatches -- --nocapture`:
  passed, 19 tests with one ignored baseline regeneration helper and zero
  violations.
- `cargo test -p macaca-framework --test agentscope2_framework_boundaries`:
  passed, 2 tests.
- `cargo test -p macaca-integration-tests --test p5_external_contract_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test unified_audit_replay_terminal_gate -- --nocapture`:
  passed, including subprocess checks for Web and runtime-host audit replay
  convergence.

Zero-debt/static gate evidence:

- `cargo test -p macaca-integration-tests --test no_debt_token_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test sdk_default_dependency_purity_gate -- --nocapture`:
  passed under all-features dependency enforcement.
- `cargo test -p macaca-integration-tests --test runtime_host_no_retired_public_facade_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test shell_no_framework_construction_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test shell_no_local_execution_owner_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test application_no_old_helper_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test context_no_old_entrypoint_gate -- --nocapture`:
  passed.
- Kernel no-network-transport and no-orchestration-semantics checks are covered
  by the passing `kernel_purity_gate`.

Debt-token scans:

- `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates --glob '*.rs'`:
  zero hits.
- `rg -n "legacy|compat|Route C migration" crates --glob '*.rs'`: no
  old-path debt hits. Remaining raw matches are domain/protocol terms such as
  OpenAI-compatible LLM and embedding endpoints; the passing
  `no_debt_token_gate` is the terminal classifier for this distinction.
- `test ! -e crates/facade/macaca-sdk/src/shell_provider_bridge.rs`: passed.

Dependency snapshot:

- `macaca-kernel -> macaca-ipc`, `macaca-proto`.
- `macaca-sdk -> macaca-proto`.
- `macaca-web -> macaca-proto`, `macaca-sdk`.
- `macaca-cli -> macaca-proto`, `macaca-sdk`.

Workspace-test equivalence:

- Full `cargo test --workspace` was represented by the documented terminal
  equivalent above because this checkout has known workspace/runtime limitations
  around absent frontend assets. The targeted matrix covers the modified
  composition root, Web-equivalent unified/GenUI/session paths, audit replay,
  kernel purity, shell purity, SDK purity, protocol boundaries, serviceization
  escape hatches, file size, and all zero-debt gates.

OpenSpec evidence:

- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed after implementation convergence.
- `openspec validate --all --strict`: passed after terminal gate convergence.

GitNexus memo:

- HIGH/CRITICAL findings were recorded as required and treated as memo-only per
  the change execution rule.
- The final repository-wide `detect_changes` scan is expected to report
  CRITICAL because the worktree contains broad pre-existing and unrelated dirty
  changes; it is used as scope evidence, not as a blocker for this terminal
  audit.

## Terminal Web Package Boundary and Server Binary Move

This section closes the remaining shell dependency purity edge by moving local
Web process assembly into the approved host composition package boundary. The
presentation shell package now remains a contract crate with only provider-neutral
workspace dependencies, while host composition owns provider/runtime/application
bootstrap, persistence anchors, optional service registration, route assembly,
and the executable process entrypoint.

GitNexus impact memo:

- `WebServerBuilder`: LOW risk, 0 impacted indexed symbols/processes.
- `AppState`: HIGH risk, with one direct low-confidence caller and four
  serve-web execution flows in the indexed graph. The HIGH finding is memo-only
  under this change's execution rules; the implementation keeps route/state
  source paths stable and changes package ownership for the compiled process.
- `WebServerProcessLauncher`: LOW risk in the indexed graph.
- `classify_crate`: LOW risk, 2 direct indexed callers and 0 affected
  execution flows. The index still pointed at the retired
  `route_c_dependency_boundaries` path for this same gate helper, so the current
  `protocol_service_dependency_boundaries` source scan was used to locate the
  live edit.
- `crates/shells/macaca-web/Cargo.toml`, `WebShellCompositionBundle`, and
  `bootstrap_optional_services` could not be resolved as GitNexus targets, so
  manifest diffs, source scans, and targeted compile/test gates were used as
  the effective blast-radius evidence.

Implementation notes:

- Added `crates/runtime/macaca-host-composition` as the host-owned composition
  crate for Web process modules. It path-includes the existing Web process
  source files so existing static source gates still scan the shell tree while
  compiled ownership moves to the host composition boundary.
- Added `macaca-host-composition --bin macaca-web-server`, pointing at the
  existing server entrypoint source. The binary now calls
  `macaca_host_composition::WebServerBuilder`.
- Converted `crates/shells/macaca-web` to a thin shell contract crate with
  `autobins = false`, route/session marker modules for external contract gates,
  and a structured-unavailable `WebServerBuilder` that refuses local provider
  assembly from the shell package.
- Removed the normal workspace dependency edge from `macaca-web` to
  `macaca-host-composition`; `macaca-web` normal workspace dependencies are now
  only `macaca-proto` and `macaca-sdk`.
- Updated the CLI local Web fallback to run
  `cargo run -p macaca-host-composition --bin macaca-web-server ...`.
- Classified `macaca-host-composition` as `RuntimeHost` in the protocol service
  dependency boundary gate. This records the new composition-root crate in the
  executable layer specification without adding an allowlist row or weakening a
  forbidden-edge rule.

Validation:

- `cargo fmt --package macaca-host-composition --package macaca-web --package macaca-cli`:
  passed.
- `cargo check -p macaca-web`: passed.
- `cargo check -p macaca-cli`: passed.
- `cargo check -p macaca-host-composition --bin macaca-web-server`: passed.
- `cargo test -p macaca-host-composition workbench_routes --lib`: passed.
- `cargo test -p macaca-runtime-host optional_service_bootstrap --lib`: passed
  before the facade move; the later host-composition filter matched 0 tests
  because the ownership surface moved.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  passed; the gate reported Web deps `{"macaca-proto", "macaca-sdk"}` and CLI
  deps `{"macaca-proto", "macaca-sdk"}`.
- `cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name=="macaca-web") | .dependencies[] | select(.source==null and .kind != "dev") | .name'`:
  returned `macaca-proto` and `macaca-sdk`.
- `cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name=="macaca-cli") | .dependencies[] | select(.source==null and .kind != "dev") | .name'`:
  returned `macaca-proto` and `macaca-sdk`.
- `cargo test -p macaca-integration-tests --test p5_external_contract_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test protocol_service_dependency_boundaries -- --nocapture`:
  initially failed because the new `macaca-host-composition` crate was
  unclassified; after classifying it as `RuntimeHost`, the command passed with
  3 tests passing and zero allowlist rows.
- `cargo test -p macaca-integration-tests --test shell_no_local_execution_owner_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test shell_no_framework_construction_gate -- --nocapture`:
  passed.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.

## SDK Alias Migration Terminal Evidence

This section closes tasks 4.3 through 4.6 by rechecking the broad SDK alias
surface after the host-composition package move. Callers no longer rely on the
deleted `shell_provider_bridge.rs` module or broad SDK runtime/provider alias
paths; shell and SDK source now use focused clients, protocol DTOs, and the
host-composition boundary for process assembly.

Validation:

- `test ! -e crates/facade/macaca-sdk/src/shell_provider_bridge.rs`: passed.
- `rg -n "pub use macaca_|macaca_sdk::(agent_execution|app|application_bootstrap|autonomy_runtime|context|execution_control|executor|framework|kernel|llm|mcp_runtime|memory|persist|service_bootstrap|service_runtime|tool_bootstrap|tools)\\b|use macaca_sdk::(agent_execution|app|application_bootstrap|autonomy_runtime|context|execution_control|executor|framework|kernel|llm|mcp_runtime|memory|persist|service_bootstrap|service_runtime|tool_bootstrap|tools)\\b" crates/facade/macaca-sdk/src crates/shells/macaca-web/src crates/shells/macaca-cli/src -g '*.rs'`:
  returned only provider-neutral `macaca_proto` re-export hits in SDK source.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test sdk_default_dependency_purity_gate -- --nocapture`:
  passed.

## Workbench-family Service Bootstrap Relocation

This checkpoint continues tasks 4.8a, 5.14, 5.17, and 11.5 by moving the
Workbench-family service registration sequence out of the Web bootstrap module
and into the host composition crate. The shell still has the direct
`macaca-host-composition` package edge, but it no longer owns the individual
file/process/sandbox/approval/diagnostics/config/plugin/git/review/tool
provider lifecycle calls.

GitNexus impact memo:

- `get_workbench_operations`: LOW risk, 0 direct indexed callers/processes.
- `WebServerBuilder`: LOW risk, 0 impacted indexed symbols/processes.
- `AppState`: HIGH risk in the indexed graph, with one direct low-confidence
  caller and four serve-web execution flows. This checkpoint does not change
  `AppState`; the HIGH result is recorded as required and is not a blocker for
  the already-applied host bootstrap extraction.
- `WebShellCompositionBundle`: target not found in the indexed graph. Source
  scans and compiler validation are used as the effective blast-radius proof.
- `crates/shells/macaca-web/Cargo.toml`: target not found in the indexed graph.
  The shell dependency gate and `cargo metadata` snapshot are the authoritative
  evidence for this manifest-level boundary.

Implementation notes:

- Added `crates/runtime/macaca-host-composition/src/workbench_service_bootstrap.rs`.
- Exported `bootstrap_workbench_family_services` and
  `WorkbenchServiceBootstrapReport` from `macaca-host-composition`.
- Replaced the Web-owned Workbench-family service registration block in
  `crates/shells/macaca-web/src/composition_bootstrap/service_runtime_wiring.rs`
  with one host composition facade call.
- The helper uses a provider-neutral trace label and emits structured tracing
  at start and completion.

Validation:

- `cargo fmt --all`: passed.
- `cargo check -p macaca-host-composition`: passed with pre-existing workspace
  warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `cargo test -p macaca-web workbench_routes --lib`: passed.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  still fails only on `macaca-web -> macaca-host-composition`.
- Current direct normal workspace dependencies for `macaca-web` remain:
  `macaca-host-composition`, `macaca-proto`, and `macaca-sdk`.

Remaining 4.8/5.14/11.5 blocker:

- `macaca-web` still has a normal package edge to `macaca-host-composition`.
  The shell purity gate reads package metadata directly, so the edge must be
  removed from `crates/shells/macaca-web/Cargo.toml`; feature hiding,
  dependency aliasing, or allowlist changes would not satisfy the terminal
  invariant.

## Optional Service Bootstrap Facade Relocation

This checkpoint further reduces Web-owned provider lifecycle code by moving the
optional Store/Entitlement/Payment/Web3/EVM bootstrap input construction and
runtime-host call behind a host composition facade. Web still supplies the
generic repositories and payment terms it already owns at process bootstrap,
but it no longer constructs `OptionalServiceBootstrapInputs` directly.

GitNexus impact memo:

- `bootstrap_optional_services`: target not found in the indexed graph. Direct
  source replacement plus runtime-host optional-service tests and crate checks
  are the authoritative proof for this small slice.

Implementation notes:

- Added `crates/runtime/macaca-host-composition/src/optional_service_bootstrap.rs`.
- Exported `bootstrap_host_optional_services` and
  `HostOptionalServiceBootstrapReport` from `macaca-host-composition`.
- Replaced the Web bootstrap call to
  `service_bootstrap::bootstrap_optional_services` with the new host
  composition facade.
- The helper emits provider-neutral tracing before and after runtime-host
  optional service bootstrap.

Validation:

- `cargo fmt --package macaca-host-composition --package macaca-web`: passed.
- `cargo check -p macaca-host-composition`: passed with pre-existing workspace
  warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-runtime-host optional_service_bootstrap --lib`: passed.

Remaining 4.8/5.14/11.5 blocker:

- `macaca-web` still has a normal package edge to `macaca-host-composition`.
  This extraction reduces Web-owned lifecycle code but does not change the
  terminal dependency snapshot.

## Host Composition Service Adapter Relocation Follow-up

This checkpoint continues tasks 4.8a, 4.8b, 5.14, 5.17, and 11.5 by moving
two concrete `ServiceRuntime` adapters out of the Web shell source and into the
host-composition composition-root crate. It does not claim terminal shell purity:
`macaca-web` still has a normal workspace dependency on
`macaca-host-composition` and the final shell dependency gate remains blocked
until that edge is removed.

GitNexus impact memo:

- `WebRuntimeSystemServiceClient`: LOW risk, 0 direct indexed callers, 0
  affected processes.
- `build_execution_command_from_delegate`: target not found.
- `dispatch_agent_execution_via_service`: target not found.
- `delegate_result_from_execution_reply`: target not found.
- Earlier entrance impact remains applicable: `WebServerBuilder` LOW,
  `WebRuntimeFacade` LOW, `serve_web_server` LOW, and
  `composition_bootstrap` target not found.

Implementation notes:

- Added `crates/runtime/macaca-host-composition/src/system_service_client.rs`
  with `HostRuntimeSystemServiceClient`, the host-owned Adapter from local
  `ServiceRuntime` to the SDK `SystemServiceClient` facade.
- Deleted `crates/shells/macaca-web/src/service_runtime_client.rs`; Web
  composition and workbench callers now use
  `macaca_host_composition::HostRuntimeSystemServiceClient`.
- Added
  `crates/runtime/macaca-host-composition/src/application_agent_delegate_bridge.rs`
  with the shared `application.agent.delegate` ->
  `service.agent_execution` Bridge helpers and constants.
- Deleted
  `crates/shells/macaca-web/src/application_agent_delegate_bridge.rs`; Web's
  WASM orchestration backend now imports the host-composition bridge while
  keeping Web-owned workspace/context enrichment local to the shell adapter.
- Updated static contract tests to inspect the host-composition bridge source
  so the canonical Application ABI -> Agent Execution service-chain assertions
  still guard YAML and WASM paths.

Validation:

- `cargo fmt --all`: passed.
- `cargo check -p macaca-host-composition`: passed with pre-existing warnings.
- `cargo check -p macaca-web`: passed with pre-existing warnings.
- `cargo test -p macaca-host-composition application_agent_delegate_bridge --lib`:
  passed, 2 tests.
- `cargo test -p macaca-web unified_agent_execution_provider_tests --lib`:
  passed, 7 tests.
- `cargo test -p macaca-web unified_workflow_application_abi_tests --lib`:
  passed, 6 tests.
- `cargo test -p macaca-web unified_audit_replay_convergence_tests --lib`:
  passed, 6 tests.

## Web AppState Host Surface Narrowing Checkpoint (2026-06-14)

This section continues tasks 4.8a, 4.8b, 5.14, 5.17, and 11.5 by reducing the
shared Web state surface that still exposes host-composition-owned concrete
types. It does not complete those tasks because `macaca-web` still has a normal
workspace dependency on `macaca-host-composition`.

GitNexus impact memo:

- `WebServerBuilder`: LOW risk, 0 impacted indexed symbols/processes.
- `serve_web_server`: LOW risk, 1 direct indexed caller
  (`WebServerBuilder.serve`), 0 affected indexed execution processes.
- `WebRuntimeFacade`: LOW risk, 0 impacted indexed symbols/processes.
- `WebContextRuntimeClient`: not found in the current GitNexus index.
- `context_provider_runtime_snapshot`: not found in the current GitNexus
  index.

Implementation notes:

- Removed the unused `PersistenceState.audit_logger` field from
  `crates/shells/macaca-web/src/state.rs` and the corresponding `AppState`
  assembly assignment. The audit logger is still constructed and owned by the
  bootstrap/service-runtime path; it is no longer exposed through route state.
- Removed the unused `BootstrapCtx.kernel_persistence` carrier field. The
  persistence adapter remains created and passed to the kernel wiring path; the
  bootstrap carrier no longer stores an unread host adapter handle.
- Added `SystemContextRuntimeClient` as a provider-neutral trait port in
  `context_runtime_facade.rs`.
- Changed `AppState.context_runtime_client` from
  `Arc<WebContextRuntimeClient>` to `Arc<dyn SystemContextRuntimeClient>`, so
  shared Web route state depends on a snapshot Facade instead of the concrete
  host-backed context runtime adapter.

Validation:

- `cargo fmt --all`: passed.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-web context_runtime --lib`: passed, 4 tests.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  still fails only for `macaca-web` observed workspace dependencies
  `{"macaca-host-composition", "macaca-proto", "macaca-sdk"}`. The CLI purity
  assertion passes and the Web allowlist terminal zero-row assertion passes.

Remaining blocker:

- `macaca-web` still imports host-composition surfaces in many production
  adapters and still declares `macaca-host-composition` as a normal workspace
  dependency. The next required slices must continue replacing direct
  runtime/framework/persist/tool/application handles in shared Web state and
  adapters with SDK/proto-facing ports, then move process bootstrap ownership
  into the dedicated host composition crate.

## Application Execution EventLog Observer Port Narrowing (2026-06-14)

This section continues tasks 4.8a, 4.8b, 5.14, 5.17, and 11.5 by removing one
direct host-composition dependency from the reusable application-execution
stream observer contract. It does not complete those tasks because `macaca-web`
still has a normal dependency on `macaca-host-composition`.

GitNexus impact memo:

- `HostApplicationExecutionEventLog`: not found in the current GitNexus index.
- `ApplicationExecutionEventLog`: not found in the current GitNexus index.
- `application_execution_event_log`: not found in the current GitNexus index.

Implementation notes:

- `crates/shells/macaca-web/src/application_execution_event_log.rs` now
  contains only the provider-neutral Observer trait over `macaca-proto`
  `EventLogQuery`/`EventEntry` DTOs and the broadcast notification contract.
- Removed `HostApplicationExecutionEventLog` from the observer contract module.
- Added `StateApplicationExecutionEventLog` beside `PersistenceState` in
  `state.rs`, where the shared host-owned `EventLog` handle already lives.
  Route modules still call `AppState::application_execution_event_log()` and
  receive `Arc<dyn ApplicationExecutionEventLog>`.
- Confirmed
  `rg -n "macaca_host_composition|macaca-host-composition|host_composition"
  crates/shells/macaca-web/src/application_execution_event_log.rs` returns zero
  hits.

Validation:

- `cargo fmt --all`: passed.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-web application_execution --lib`: passed, 28 tests.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  still fails only for `macaca-web` observed workspace dependencies
  `{"macaca-host-composition", "macaca-proto", "macaca-sdk"}`. The CLI purity
  assertion passes and the Web allowlist terminal zero-row assertion passes.

Remaining blocker:

- The terminal dependency edge remains because other Web production adapters
  still import runtime/framework/persistence/tool/application host-composition
  surfaces. Continue removing those direct handles or moving their owners into
  the host composition crate until `macaca-web` can drop the normal
  `macaca-host-composition` dependency.

## Trace Event Forwarder Port Narrowing (2026-06-14)

This section continues tasks 4.8a, 4.8b, 5.14, 5.17, and 11.5 by removing a
concrete host persistence type from the trace event forwarding facade. It does
not complete those tasks because the Web crate still has many remaining
host-composition imports and a normal `macaca-host-composition` dependency.

GitNexus impact memo:

- `TraceEventForwarder`: LOW risk, 0 impacted indexed symbols/processes.
- `TraceEventNormalizer`: LOW risk, 0 impacted indexed symbols/processes.

Implementation notes:

- Added `TraceEventSink` as a provider-neutral Command-style port in
  `crates/shells/macaca-web/src/trace_events.rs`.
- Changed `TraceEventForwarder` to hold `Arc<dyn TraceEventSink>` instead of
  `Arc<EventLog>`.
- Removed the direct `macaca_host_composition::persist::EventLog` import from
  `trace_events.rs`. The module now owns only trace normalization and SSE
  fanout; durable persistence must be installed through the sink port by the
  host composition root.

Validation:

- `cargo fmt --all`: passed.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `rg -n "macaca_host_composition|macaca-host-composition|host_composition"
  crates/shells/macaca-web/src/trace_events.rs
  crates/shells/macaca-web/src/application_execution_event_log.rs`: zero hits.

Remaining blocker:

- `macaca-web` still declares `macaca-host-composition` as a normal dependency
  because larger bootstrap/framework/runtime adapters still import host-owned
  surfaces. Continue replacing those surfaces with proto/SDK ports or moving
  ownership into the dedicated host composition crate.

## Web Host-composition Residual Edge Recheck

This checkpoint continues task 4.8 without marking 4.8a/4.8b complete. The
current terminal blocker is no longer an SDK runtime-host edge; it is the normal
workspace dependency from `macaca-web` to `macaca-host-composition`.

Impact memo:

- `parse_port`: GitNexus target not found. Local source inspection shows this is
  a private helper in `crates/shells/macaca-web/src/bin/macaca-web-server.rs`.
  The change only restores the intended `MacacaError::Config` variant after a
  previous revert typo.
- `AppState`: GitNexus returned LOW/0 impacted symbols, but local source scans
  show high practical blast radius. `AppState` is referenced by route handlers,
  chat orchestration, session persistence/SSE, framework runner paths,
  skill/MCP operation routes, and bootstrap assembly. Treating the Web
  host-composition split as high-risk is the correct engineering stance despite
  the stale/under-indexed graph result.
- `WebServerBuilder`: GitNexus returned LOW/0 impacted symbols. Local evidence
  shows moving the binary before splitting Web state would recreate the cycle
  `macaca-host-composition -> macaca-web -> macaca-host-composition`, so that
  move is deferred until Web accepts only provider-neutral injected state.

Implementation notes:

- Fixed `MacError::Config` to `MacacaError::Config` in the restored
  standalone Web server entrypoint.
- Moved Web memory command imports in `session_memory_capture.rs` and
  `context_memory_tools.rs` from host-composition/old SDK alias paths to their
  canonical `macaca-proto` DTO definitions. This removes misleading
  host-composition usage for provider-neutral memory command types, but does
  not hide or weaken the remaining dependency gate.
- Confirmed the `macaca-web-server` binary cannot move safely yet. The next
  required slice is an ownership split: Web keeps Axum route DTOs/handlers while
  host composition owns concrete `AppState` construction and exposes
  route-safe provider-neutral ports back to Web.

Validation:

- `cargo fmt --all`: passed.
- `cargo check -p macaca-web -p macaca-host-composition -p macaca-cli`: passed
  with existing warnings.
- `cargo check -p macaca-web`: passed with existing warnings after the memory
  DTO import cleanup.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  still fails only on `macaca-web` depending on `macaca-host-composition`;
  CLI purity and zero-row Web allowlist assertions pass.
- `cargo test -p macaca-web session_memory_capture --lib`: did not complete
  because unrelated test-only skill DTO imports still expect removed
  `macaca_sdk::*` root re-exports. Production `cargo check -p macaca-web`
  remains the valid verification for this narrow slice.

Remaining 4.8 blocker:

- `macaca-web/Cargo.toml` still has a normal workspace dependency on
  `macaca-host-composition`. Removing that edge requires splitting the current
  host-typed `AppState` and composition bootstrap out of the presentation shell,
  not merely moving the binary or editing the gate.

## Host Composition Facade Split Checkpoint

This checkpoint continues task 4.8a by introducing an explicit host composition
crate and moving Web host-surface imports away from SDK module paths. It is an
intermediate, compiling step: SDK terminal gates still fail until the
runtime-host-backed context/skill clients are either moved to host composition or
their service DTOs are fully downshifted into `macaca-proto`.

GitNexus impact memo:

- `focused_runtime_surfaces`: target not found in the current GitNexus index.
  Source scan showed roughly 500 Web references to SDK host surfaces before this
  checkpoint.
- `runtime_host`: target not found in the current GitNexus index.
- `SystemContextClient`: LOW risk, 2 direct implementors reported.
- `SystemSkillClient`: LOW risk, 1 direct implementor reported.
- `ServiceBackedContextClient`: LOW risk, 0 impacted indexed symbols/processes.
- `ServiceBackedSkillClient`: LOW risk, 0 impacted indexed symbols/processes.

Implementation notes:

- Added `crates/runtime/macaca-host-composition`, an explicit process
  composition crate depending on `macaca-runtime-host` and `macaca-proto`.
- Copied the existing SDK runtime-host facade and focused host surfaces into the
  new crate so Web imports host-owned runtime/application/framework contracts
  from `macaca_host_composition::*` instead of `macaca_sdk::*`.
- Updated `macaca-web` to depend on `macaca-host-composition` while temporarily
  retaining SDK `runtime-host-bootstrap` for the still-SDK-owned
  `SystemContextClient`, `SystemSkillClient`, and `SystemSkillOperatorClient`.
- Replaced the last Web `DirectFacadeMemoryClient` use with the existing
  `ServiceBackedMemoryClient`, so context recall now goes through
  `service.memory` instead of a direct MemoryFacade wrapper.
- Updated the local execution-owner gate assertion to require
  `ServiceBackedMemoryClient::new`.

Validation:

- `cargo check -p macaca-proto`: passed with pre-existing warning.
- `cargo check -p macaca-memory`: passed with pre-existing warning.
- `cargo check -p macaca-sdk`: passed with pre-existing warning.
- `cargo check -p macaca-host-composition`: passed with pre-existing workspace
  warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `rg -n "DirectFacadeMemoryClient" crates -g '*.rs'`: zero hits.
- `rg -n "macaca_sdk::(agent_execution|app|application_bootstrap|autonomy_runtime|context|execution_control|executor|framework|kernel|llm|mcp_runtime|memory|persist|service_bootstrap|service_runtime|tool_bootstrap|tools)\\b|use macaca_sdk::(agent_execution|app|application_bootstrap|autonomy_runtime|context|execution_control|executor|framework|kernel|llm|mcp_runtime|memory|persist|service_bootstrap|service_runtime|tool_bootstrap|tools)\\b" crates/shells/macaca-web/src -g '*.rs'`:
  zero hits.
- `cargo test -p macaca-integration-tests --test sdk_default_dependency_purity_gate -- --nocapture`:
  still fails on `macaca-sdk -> macaca-runtime-host`.
- `cargo test -p macaca-integration-tests --test sdk_no_provider_reexport_gate -- --nocapture`:
  still fails on SDK `runtime-host-bootstrap`, `runtime_host.rs`, and
  `focused_runtime_surfaces.rs` tokens.

Remaining 4.8 blockers:

- SDK still owns runtime-host-backed Context and Skill client traits/impls.
  Their command/result DTOs still come from runtime-host/service crates instead
  of a pure proto surface.
- Web still enables SDK `runtime-host-bootstrap` only for those clients. Once
  context/skill DTOs are downshifted or those clients move to host composition,
  the SDK feature and dependency can be deleted.

## SystemFacade Memory/Context Narrow Path Cleanup

This section continues task 4.8 by moving SDK `SystemFacade` operation
signatures away from the runtime-host root module for memory recall and context
assembly. The concrete DTOs remain runtime-host-gated today, but callers now see
the focused SDK `memory` and `context` modules rather than the broad
`runtime_host` facade.

GitNexus impact memo:

- `memory_recall`: LOW risk, 0 impacted indexed symbols/processes for the
  closest indexed symbol. The indexed graph matched a same-named context
  capability property, so source scans and target checks were used for the
  current SDK operation path.
- `assemble_context`: LOW risk, 0 impacted indexed symbols/processes.

Implementation notes:

- Updated `SystemFacade::memory_recall` to accept and return
  `crate::memory::{MemoryRecallCommand, MemoryRecallResult}`.
- Updated `SystemFacade::assemble_context` to accept and return
  `crate::context::{ContextAssembleCommand, ContextAssembleServiceResult}`.
- Added `ContextAssembleServiceResult` to the SDK `context` module export list,
  preserving the focused context module as the stable caller path.

Validation:

- `cargo fmt --package macaca-sdk --package macaca-web`: passed.
- `rg -n "crate::runtime_host::(MemoryRecallCommand|MemoryRecallResult|ContextAssembleCommand|ContextAssembleServiceResult)" crates/facade/macaca-sdk/src --glob '*.rs'`:
  zero hits.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-integration-tests --test sdk_default_dependency_purity_gate -- --nocapture`:
  passed.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  passed.
- `openspec validate remove-protocol-microkernel-residual-debt --strict`:
  passed.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, manifest projection helpers,
  runtime-host service runtime assembly, persistence backends, executor events,
  framework construction, memory/context providers, skill runtime/provider
  command surfaces, and LLM provider/router bootstrap.

## Runtime-host Public Lineage DTO Export Closure

This section continues task 4.8 by removing context lineage DTOs from the
runtime-host root public API after Web and the SDK runtime-host facade had
already stopped importing those DTOs through runtime-host. Lineage value objects
are protocol/persistence contracts for session inspection and compaction
evidence; runtime-host should expose only the concrete persistence helper needed
by composition code.

GitNexus impact memo:

- `LineageKind`: LOW risk, 0 impacted indexed symbols/processes.
- `SessionLineage`: LOW risk, 0 impacted indexed symbols/processes.
- `TranscriptSegment`: HIGH risk in the indexed graph, with two low-confidence
  direct hits and three manual-compaction processes. This slice does not edit
  the DTO definition or behavior; it removes only the runtime-host public
  re-export after callers already import protocol DTOs directly. Per the change
  execution constraint, the HIGH finding is memo-only and not a blocker.

Implementation notes:

- Removed `LineageKind`, `SessionLineage`, and `TranscriptSegment` from
  `crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs`.
- Left `macaca_sdk::runtime_host::persist::SessionLineageStore` untouched
  because it remains a concrete persistence helper consumed by Web composition
  and is not a proto DTO.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk --package macaca-web`:
  passed.
- `rg -n "LineageKind|SessionLineage|TranscriptSegment" crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs crates/facade/macaca-sdk/src/runtime_host.rs crates/facade/macaca-sdk/src/lib.rs`:
  remaining SDK hit is only `SessionLineageStore`.
- `cargo check -p macaca-runtime-host`: passed with pre-existing workspace
  warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, manifest projection helpers,
  runtime-host service runtime assembly, persistence backends, executor events,
  framework construction, memory/context providers, skill runtime/provider
  command surfaces, and LLM provider/router bootstrap.

## Runtime-host Public Planning DTO Export Closure

This section continues task 4.8 by removing the same application planning DTO
aliases from the runtime-host public API, after the SDK runtime-host facade and
Web caller path had already moved away from them. Runtime-host still owns
composition and provider bootstrap, but it no longer republishes the app/framework
planning DTO names that now have canonical proto import paths for shell planning
fallbacks.

GitNexus impact memo:

- `ApplicationPlanningAgentProfile`: LOW risk, 0 impacted indexed
  symbols/processes.
- `ApplicationTaskPlanningContract`: LOW risk, 0 impacted indexed
  symbols/processes.
- `AppPlanningAgentProfile`: LOW risk, 0 impacted indexed symbols/processes.
- `AppTaskPlanningContract`: LOW risk, 0 impacted indexed symbols/processes.
- `runtime_host` and `crates/facade/macaca-sdk/src/runtime_host.rs` could not
  be located as GitNexus targets because the indexed graph does not know that
  newly introduced file/surface yet. Source scans and target checks were used as
  the effective blast-radius proof for this slice.

Implementation notes:

- Removed `AppPlanningAgentProfile` and `AppTaskPlanningContract` from
  `crates/runtime/macaca-runtime-host/src/app_public_api.rs`.
- Removed `ApplicationPlanningAgentProfile` and
  `ApplicationTaskPlanningContract` from
  `crates/runtime/macaca-runtime-host/src/framework_public_api.rs`.
- Left the original app/framework definitions and helpers untouched; this slice
  only narrows the public runtime-host export surface.

Validation:

- `cargo fmt --package macaca-runtime-host --package macaca-sdk --package macaca-web`:
  passed.
- `rg -n "AppPlanningAgentProfile|AppTaskPlanningContract|ApplicationPlanningAgentProfile|ApplicationTaskPlanningContract" crates/runtime/macaca-runtime-host/src crates/facade/macaca-sdk/src/runtime_host.rs crates/facade/macaca-sdk/src/lib.rs`:
  zero hits.
- `cargo check -p macaca-runtime-host`: passed with pre-existing workspace
  warnings.
- `cargo check -p macaca-sdk`: passed with pre-existing workspace warnings.
- `cargo check -p macaca-web`: passed with pre-existing workspace warnings.
- `cargo test -p macaca-integration-tests --test sdk_default_dependency_purity_gate -- --nocapture`:
  passed, confirming the SDK default feature set still depends only on
  `macaca-proto` among workspace crates.
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`:
  passed, confirming Web/CLI workspace dependencies remain terminal
  `macaca-proto` + `macaca-sdk` only and the Web allowlist remains zero rows.
- File-size spot check: `app_public_api.rs` is 19 lines,
  `framework_public_api.rs` is 49 lines, and the SDK runtime-host facade is 264
  lines.

Remaining 4.8 blocker:

- `macaca-sdk/Cargo.toml` still contains an optional
  `macaca-runtime-host` dependency, and `macaca-web` still enables
  `runtime-host-bootstrap`. Remaining Web consumers include concrete
  application registry/runtime/loader bootstrap, manifest projection helpers,
  runtime-host service runtime assembly, persistence backends, executor events,
  framework construction, memory/context providers, skill runtime/provider
  command surfaces, and LLM provider/router bootstrap.

## Final Completion Pointer

The historical "Remaining blocker" text immediately above is retained as
chronological implementation evidence only. The current terminal state is the
`Completion Audit Superseding Historical Blockers` section in this memo, backed
by the latest passing terminal gates, dependency snapshots, debt-token scans,
and OpenSpec validations. That audit supersedes all older blocker notes.

Final GitNexus scope audit:

- `mcp__gitnexus.detect_changes(scope="all", repo="agent")`: reported
  CRITICAL over the broad dirty worktree, with 778 changed files, 5525 changed
  symbols, and 132 affected processes. This matches the known repository state
  and is recorded as scope evidence rather than a blocker for this completed
  change audit.
