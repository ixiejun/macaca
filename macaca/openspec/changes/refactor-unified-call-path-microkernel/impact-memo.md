# Impact memo — GitNexus (non-blocking)

Per user directive and `design.md` D8: HIGH/CRITICAL warnings are recorded only; they do not block merges in this change.

| Symbol / area | GitNexus risk | Blast radius (summary) | Phase |
|---------------|---------------|------------------------|-------|
| `Kernel::execute_agent` | HIGH | kernel tests, fullstack-autodev, web bootstrap | P1 |
| `executor::*` | CRITICAL | web loop_manager, orchestration_tools, kernel e2e | P1.3 |
| `AppState` deprecated fields | HIGH | macaca-web routes, framework_toolkit, loop_manager | P3 |
| `application_execution_hosted::*` | HIGH | WASM/YAML audit authority paths | P1.2.6 |
| `local_simulated_terms` (a2a) | CRITICAL | web, runtime-host, sdk payment paths | P2.1 |
| `payment_policy` module | LOW (internalized) | root re-exports until P2.1 completes | P2 |

Recorded during iteration 1; re-run `gitnexus_impact` before each `[impact-memo]` task edit.

## Iteration 6 additions

| Symbol / area | GitNexus risk | Blast radius (summary) | Phase |
|---------------|---------------|------------------------|-------|
| `ApplicationExecutor::begin_service_backed_delegation` | MEDIUM | orchestration_tools delegate_task, get_task_result, fork suspend/resume | P1.3 |
| `ServiceDelegatedTaskDispatcher` | MEDIUM | macaca-web delegate_task tool only; worker loop unchanged (already service-backed) | P1.3 |
| `ExecutionQueue::admit_running_task` | LOW | service-backed delegations bypass worker channel | P1.3 |

## Iteration 7 additions

| Symbol / area | GitNexus risk | Blast radius (summary) | Phase |
|---------------|---------------|------------------------|-------|
| `ExecutionControlForkJoinCoordinator` | HIGH | orchestration_tools delegate_task, hook_consumer resume, execution_control service provider | P1.3 |
| `ForkManager` (suspend/resume hooks) | CRITICAL | hook_consumer, delegate_task fork lifecycle until P3 executor eviction | P1.3 / P3 |
| `hook_consumer::start_hook_event_consumer` | HIGH | fork-join parent resume path, active_sessions channel adapter | P1.3 |
| `fork_join_shell_adapter` | MEDIUM | new shell adapter; no application-specific branches | P1.3 |
| `ComposedAgentExecutionBackend` | HIGH | delegate_task service path, chat/workflow agent runs | P1.2 |

## Iteration 8 additions

| Symbol / area | GitNexus risk | Blast radius (summary) | Phase |
|---------------|---------------|------------------------|-------|
| `ExecutionControlGoalLifecycleCoordinator` | HIGH | loop_manager GoalCompleted, framework_toolkit create_goal, execution_control service | P1.3 |
| `goal_lifecycle_shell_adapter` | MEDIUM | PlanLoop resume + create_goal wait registration; legacy channel compat | P1.3 |
| `loop_manager::GoalCompleted` consumer | HIGH | goal_to_session resume path; executor lifecycle events unchanged | P1.3 |

## Iteration 9 additions

| Symbol / area | GitNexus risk | Blast radius (summary) | Phase |
|---------------|---------------|------------------------|-------|
| `ServiceBackedFrameworkRuntimeAgentPort` | HIGH | `ComposedAgentExecutionBackend`, all `service.agent_execution` chat/worker/delegate paths | P1.2 |
| `FrameworkAgentConstructionPort` / `WebFrameworkAgentConstructionPort` | MEDIUM | web construction adapter still calls `FrameworkRunner` (compat seam until 4.3.2) | P1.2 |
| `FrameworkRunner::build_runtime_agent_from_context_snapshot_*` | CRITICAL | worker loop, chat, delegate_task agent runs; construction isolated behind port | P1.2 / P3 |

## Iteration 10 additions

| Symbol / area | GitNexus risk | Blast radius (summary) | Phase |
|---------------|---------------|------------------------|-------|
| `unified_agent_execution_provider_tests` (web) | LOW | contract-only; YAML chat/workflow + WASM delegate entry surfaces | P1.2 |
| `unified_agent_execution_provider_tests` (runtime-host) | LOW | `AgentExecutionSystemServiceProvider` intent matrix; no production code change | P1.2 |

## Iteration 11 additions (task 2.4 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `WebAgentRunner::execute_via_application_delegate` | HIGH | ApplicationExecutor YAML workflow steps; kernel AgentRunner trait consumers | P1.4 |
| `application_agent_delegate_bridge` | MEDIUM | YAML + WASM orchestration second hop to `service.agent_execution` | P1.4 |
| `WebApplicationOrchestrationBackend::delegate_agent` | MEDIUM | Application Service provider; WASM host imports; YAML workflow | P1.4 |
| `AgentExecutionIntent::from_delegate_metadata` | LOW | proto wire labels; no application-specific branches | P1.4 |

## Iteration 12 additions (task 2.6 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `hosted_signals_from_host_result` | HIGH | WASM hosted terminal state; all host commands now equally authoritative | P1.6 |
| `WebAgentExecutionHostAdapter::resolve_execution_control_policy` | MEDIUM | manifest projection via Application Service metadata; replaces legacy ChatMainThread branch | P1.6 |
| `ComposedAgentExecutionBackend` (lifecycle emit) | MEDIUM | always emits coarse lifecycle; duplicate suppression markers removed | P1.6 |
| `ApplicationMetadataView.execution_control` | LOW | new sanitized metadata field; service projection only | P1.6 |
| `ExecutionControlPolicyResolver` | MEDIUM | override denied without manifest default; all execution intents | P1.6 |

## Iteration 13 additions (task 2.7 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `unified_audit_replay_convergence_tests` (web) | LOW | static contract + entry-surface inventory; no production behavior change | P1.7 |
| `unified_audit_replay_convergence_tests` (runtime-host) | LOW | `InMemoryServiceCallAuditSink` replay; validates single terminal provider | P1.7 |
| `ExecutionControlPolicy` serde defaults | LOW | YAML manifest parsing; `fullstack-autodev` fixture only | P1.7 |
| `AppManifest.execution_control` test fixtures | LOW | macaca-app unit/integration constructors; compile-time only | P1.7 |

## Iteration 14 additions (task 3.1 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `A2ACoordinator` / `KernelPaymentStorePort` (deleted) | CRITICAL | legacy kernel payment path removed; canonical path is `PaymentSystemServiceProvider` + `SystemPaymentClient` | P2.1 |
| `local_simulated_terms` (proto) | LOW | bootstrap + test fixtures in web/runtime-host/sdk; provider-neutral factory | P2.1 |
| `macaca-kernel::persistence` (payment store removed) | MEDIUM | kernel persistence port now generic KV only; payment mementos owned by persist + payment service | P2.1 |
| `payment_service_provider` integration tests | LOW | service.call policy denial + invalid transition contracts | P2.1 |

## Iteration 15 additions (task 3.2 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `Web3Facade` / `Web3Adapter` / `Web3TraceEventSink` (deleted) | CRITICAL | legacy kernel Web3 path removed; canonical path is `Web3SystemServiceProvider` + `SystemWeb3Client` | P2.2 |
| `DefaultWeb3PolicyEngine` (deleted) | MEDIUM | kernel policy engine removed; service path uses provider admission + entitlement layers | P2.2 |
| `web3_service_provider` integration tests | LOW | unavailable/mock service.call contracts; no production behavior change | P2.2 |
| `route_c_bootstrap` Web3 unavailable default | LOW | base OS boots with structured unavailable Web3; unchanged bootstrap seam | P2.2 |

## Iteration 16 additions (task 3.3 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `EvmFacade` / `EvmAdapter` / `EvmTraceEventSink` (deleted) | CRITICAL | legacy kernel EVM path removed; canonical path is `EvmSystemServiceProvider` + `SystemEvmClient` | P2.3 |
| `MacacaEvmSdk` (deleted) | MEDIUM | deprecated SDK facade removed; service client path is `ServiceBackedEvmClient` | P2.3 |
| `DefaultEvmPolicyEngine` (deleted) | MEDIUM | kernel policy engine removed; service path uses provider admission + entitlement layers | P2.3 |
| `evm_service_provider` integration tests | LOW | unavailable/mock service.call contracts; no production behavior change | P2.3 |
| `route_c_bootstrap` EVM unavailable default | LOW | base OS boots with structured unavailable EVM; unchanged bootstrap seam | P2.3 |

## Iteration 17 additions (task 3.4 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `executor::*` (kernel → runtime-host) | CRITICAL | macaca-web loop_manager, orchestration_tools, hook_consumer, event_persistence, sse, state; execution_control_fork_join | P2.4 |
| `ForkManager` / `ApplicationExecutorRegistry` | CRITICAL | delegate_task fork lifecycle, executor registry bootstrap, SSE lifecycle events | P2.4 |
| `macaca-kernel` executor module (deleted) | HIGH | microkernel purified; scheduling invariants only; persistence/logging ports remain cross-crate seams | P2.4 |
| `executor_runtime_eviction` integration tests | LOW | contract-only; validates runtime-host public surface + static kernel lib.rs contract | P2.4 |
| `serviceization_escape_hatches` executor paths | LOW | allowlist updated to runtime-host paths; violations=0 | P2.4 |

## Iteration 18 additions (task 3.5 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `KernelProviderCompat` / `provider_compat.rs` (deleted) | CRITICAL | kernel construction, web bootstrap, CLI/SDK test fixtures | P2.5 |
| `KernelServiceClientCompat` / `from_service_clients` (deleted) | HIGH | runtime-host dispatch, kernel_builder compat path removed | P2.5 |
| `Kernel::new(config, llm, tools)` (deleted) | HIGH | integration tests migrated to `KernelBuilder::from_execution_port` | P2.5 |
| `LegacyAgentExecutionAdapter` (test/migration only) | MEDIUM | macaca-agent definition + approved fixtures; production uses service-client port | P2.5 |
| `compat.rs` → `skill_mcp_mapping_registry.rs` | LOW | mcp_runtime, McpServerFactory, skill_mcp web path; behavior unchanged | P2.5 |
| `serviceization_escape_hatches` provider-compat allowlist | LOW | provider_compat path removed; sdk test surfaces added | P2.5 |

## Iteration 19 additions (task 3.6 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `AgentExecutionPort` (proto migration) | HIGH | kernel, runtime-host, macaca-agent adapters, web bootstrap, integration tests | P2.6 |
| `Kernel::register_agent(manifest)` **BREAKING** | HIGH | SDK facade, kernel_lifecycle, e2e_auto_programming, live_fullstack_autodev | P2.6 |
| `SwappableAgentExecutionPort` / `UnavailableAgentExecutionPort` (kernel) | MEDIUM | kernel_builder, web bootstrap, runtime-host dispatch tests | P2.6 |
| `LegacyAgentSideRegistry` + `register_legacy_kernel_agent` | MEDIUM | approved migration path for in-process Agent::run fixtures | P2.6 |
| `kernel/services.rs` (deleted) | LOW | no external consumers; execution port moved to kernel + proto | P2.6 |
| allowlist kernel→driver/gateway/skill rows (deleted) | LOW | Route C gate now 7 web rows; kernel provider edges cleared | P2.6 |

## Iteration 20 additions (task 3.7 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `LineageKind` / `SessionLineage` / `TranscriptSegment` (proto migration) | MEDIUM | macaca-persist lineage store, macaca-context compaction, macaca-web routes (re-export unchanged) | P2.7 |
| `macaca-persist/Cargo.toml` (context dep removed) | LOW | foundation layer dependency direction; dev-dep only for pruning contract test | P2.7 |
| `SessionLineageStore` tracing additions | LOW | persist audit trail for lineage save/load; no behavior change | P2.7 |
| `p2_microkernel_exit_validation` integration tests | LOW | static P2 exit contracts; no production runtime change | P2.8 |

## Iteration 21 additions (task 4.1 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `AppState` deprecated fields removed | HIGH | routes, chat_orchestrator, framework_runner, loop_manager, lib bootstrap | P3.1 |
| `WebShellCompositionBundle` | MEDIUM | bootstrap composition root; adapters only approved read path | P3.1 |
| `application_shell_adapter` / `llm_route_shell_adapter` / `mcp_shell_adapter` | MEDIUM | centralized Adapter pattern for registry/runtime/LLM/MCP legacy seams | P3.1 |
| `serviceization_escape_hatches` allowlist (tightened) | LOW | removed route-level migration surfaces; violations=0 | P3.1 |
| `WebMemoryRuntime` (deferred 4.1.4) | MEDIUM | bootstrap + composition bundle; not deleted until memory service fully owns facade | P3.1 |

## Iteration 22 additions (task 4.2 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `ExecutionControlSessionLoopCoordinator` (new) | HIGH | loop_manager register/wake/shutdown; chat_orchestrator cleanup; runtime-host execution_control service | P3.2 |
| `session_loop_shell_adapter` (new) | MEDIUM | web shell Adapter bridging execution_control audit + legacy plan/worker waker maps | P3.2 |
| `ensure_plan_and_worker_loops` wake paths | HIGH | goal decomposition, worker submit review, review delegate complete, worker loop wake helper | P3.2 |
| `cleanup_app_state` session-loop shutdown | MEDIUM | post_chat_stop + post_chat_v2 new-session cleanup; audit before local handle removal | P3.2 |
| `unified_delegation_path_tests` session-loop contract | LOW | static source assertions; no runtime behavior change | P3.2 |

## Iteration 23 additions (task 4.3.1 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `loop_manager.rs` → `loop_manager/` module split | CRITICAL | routes, orchestration_tools, hook_consumer, chat_orchestrator, unified_delegation_path contract tests | P3.3 |
| `ensure_plan_and_worker_loops` / `create_goal` (facade) | HIGH | all goal/worker loop entrypoints; public API unchanged via `mod.rs` re-exports | P3.3 |
| `plan_event_consumer` + Strategy handlers | HIGH | GoalReady/ReviewNeeded/AllTasksDone/AnomalyDetected/GoalCompleted lifecycle | P3.3 |
| `decomposition_adapter` graph_owner fix | MEDIUM | fallback goal decomposition tasks now `ApplicationExecution` authority | P3.3 |
| `contract_source::loop_manager_module_sources` | LOW | static contract tests only; concatenates module sources for escape-hatch scans | P3.3 |

## Iteration 24 additions (task 4.3.2 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `framework_runner.rs` → `framework_runner/` module split | CRITICAL | agent_execution_backend, chat_orchestrator, loop_manager, routes, framework_toolkit, unified_delegation_path contract tests | P3.3 |
| `FrameworkRunner` facade + `build_runtime_agent` | HIGH | all agent construction entrypoints; public API unchanged via `mod.rs` re-exports | P3.3 |
| `WebTracedAgentFactory` / emitter adapters | HIGH | SSE/channel/executor trace streaming + execution-control middleware | P3.3 |
| `build_mode::DriverTraceRoute::label` | LOW | diagnostic route labels; migration debt on `hardcoded-agent-role` allowlist | P3.3 |
| `contract_source::framework_runner_module_sources` | LOW | static contract tests only; concatenates module sources for escape-hatch scans | P3.3 |

## Iteration 25 additions (task 4.3.3 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `chat_orchestrator.rs` → `chat_orchestrator/` module split | CRITICAL | bootstrap routes, unified_delegation_path, unified_agent_execution_provider, unified_audit_replay_convergence, agent_execution_backend contract tests | P3.3 |
| `post_chat_v2` / `run_wasm_chat_fast_path` / `run_framework_chat_path` | HIGH | `/api/chat/v2` SSE entry; WASM vs framework dispatch; session lifecycle + executor event bridges | P3.3 |
| `executor_event_adapter` shared forwarder | MEDIUM | WASM and framework paths; delegated agent SSE + kernel activity sync | P3.3 |
| `route_chat_v2.rs` entry-agent fallback | LOW | `"coordinator"` migration debt on `hardcoded-agent-role` allowlist | P3.3 |
| `contract_source::chat_orchestrator_module_sources` | LOW | static contract tests only; concatenates module sources for escape-hatch scans | P3.3 |

## Iteration 26 additions (task 4.3.4 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `lib.rs` → `composition_bootstrap/` module split | CRITICAL | `serve_web_server`, `WebServerBuilder`, bootstrap routes, unified_agent_execution_provider, unified_audit_replay_convergence, agent_execution_backend contract tests | P3.3 |
| `serve_web_server` phased orchestrator | HIGH | web startup composition root; all service provider registration + `AppState` assembly | P3.3 |
| `BootstrapCtx` carrier | MEDIUM | cross-phase bootstrap state threading; ordering enforced via `Option` + `expect` | P3.3 |
| `contract_source::composition_bootstrap_module_sources` | LOW | static contract tests only; concatenates module sources for escape-hatch scans | P3.3 |

## Iteration 27 additions (task 4.4.1–4.4.3 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `shell_provider_bridge` + bulk `macaca-web` import rewrite | CRITICAL | all web modules importing driver/llm/memory/skill/task/tools/kernel types; composition_bootstrap, framework_runner, loop_manager, routes | P3.4 |
| `macaca-sdk` → `macaca-persist` attempt | HIGH | blocked by `application-execution-sdk-no-runtime-provider-construction`; reverted | P3.4 |
| `macaca-runtime-host::persist` alias | MEDIUM | shell bootstrap persist types; web already depends on runtime-host | P3.4 |
| `app→sdk→runtime-host→app` cycle | HIGH | prevents SDK re-export of runtime-host; 4.4.4终态 deferred | P3.4 / P4 |
| Route C allowlist cleared (7 rows) | LOW | gate now enforces zero allowlist globally; web provider direct edges gone | P3.4 |

## Iteration 28 additions (tasks 4.3.6 + 6.1.6 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `routes.rs` → `routes/` module split | CRITICAL | bootstrap route table, session/loop_manager/app_ui/workbench imports of `crate::routes::*`; public API preserved via Facade re-exports | P3.3 |
| `os_layer_file_size_gate` (new) | LOW | integration-tests only; 87-row baseline allowlist; blocks new >500-line production sources | P5 |
| `session.rs` (remaining web giant) | HIGH | still on filesize allowlist; next P3 split target | P3.5 |

## Iteration 29 additions (task 4.3.7 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `framework_toolkit.rs` → `framework_toolkit/` module split | CRITICAL | `FrameworkRunner::build_toolkit` consumer; unified_delegation_path contract tests; serviceization escape hatch paths | P3.3 |

## Iteration 30 additions (task 4.3.8 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `session.rs` → `session/` module split | CRITICAL | bootstrap session routes; chat_orchestrator/event_persistence/app_ui_session_projection consumers of `crate::session::*`; public API preserved via Facade re-exports | P3.3 |
| `task_api_migration_audit` path fix | LOW | loop_manager/routes module paths after prior splits | P3.3 |

## Iteration 31 additions (task 4.3.9 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `skill_operations_routes.rs` → `skill_operations_routes/` module split | CRITICAL | bootstrap skill-operations route table (9 handlers); `self_evolving_skill_boundaries` directory scan; thin-adapter contract tests | P3.3 |

## Iteration 32 additions (task 4.3.10 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `app_ui_routes.rs` → `app_ui_routes/` module split | CRITICAL | bootstrap app-ui asset + bridge routes; `app_ui_csp` / `app_ui_session_projection` consumers unchanged | P3.3 |

## Iteration 33 additions (task 4.3.11/4.3.12 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `context_memory_injection.rs` → `context_memory_injection/` module split | CRITICAL | `context_reporting_model::assembly_finalize` recall injectors; public `apply_active_recall` / `apply_preflight_memory` API preserved | P3.3 |
| `context_reporting_model.rs` → `context_reporting_model/` module split | CRITICAL | `framework_runner/agent_factory_build` `ContextReportingChatModel` consumer; Context Service + legacy assembly paths; ChatModel hot path unchanged | P3.3 |

## Iteration 34 additions (task 4.3.13 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `skill_mcp.rs` → `skill_mcp/` module split | CRITICAL | `framework_toolkit::load_or_build_skill_snapshot`; `routes/skills_mcp::probe_skill_mcp_servers` + `SkillMcpStatus` API; public `crate::skill_mcp::*` preserved via Facade re-exports | P3.3 |
| `probe_skill_mcp_servers` | HIGH | `/api/apps/:id/skills` MCP status probe path; integration probe tests | P3.3 |
| `load_or_build_skill_snapshot` | HIGH | toolkit skill snapshot cache + governed activation telemetry side effects | P3.3 |

## Iteration 35 additions (task 4.3.14 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `skill_self_evolution_observer.rs` → `skill_self_evolution_observer/` module split | CRITICAL | `skill_self_evolution_execution_observer` decorator; `observe_agent_execution_result_for_skill_self_evolution` public export preserved via Facade | P3.3 |
| `observe_agent_execution_result_for_skill_self_evolution` | HIGH | Agent Execution completion → Skill proposal command path; live self-evolution SSE/EventLog checkpoints | P3.3 |
| `build_skill_experience_proposal_command` | HIGH | bounded evidence refs, semantic Skill Creator identity, proposal validation gate | P3.3 |

## Iteration 36 additions (task 4.3.15 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `agent_execution_backend/tests.rs` → `agent_execution_backend/tests/` module split | CRITICAL | unified_agent_execution_provider, unified_audit_replay_convergence, serviceization_escape_hatches contract scans; 29 contract tests preserved | P3.3 |
| `agent_execution_backend.rs` → `agent_execution_backend/mod.rs` | MEDIUM | `lib.rs` module path unchanged; Facade re-exports not required (tests-only module) | P3.3 |
| `contract_source::agent_execution_backend_test_module_sources` | LOW | static contract tests only; enumerates test submodule paths for escape-hatch scans | P3.3 |
| `direct_session_pause_resume_channels_stay_inside_approved_adapters` | LOW | guard test skip logic extended for `/tests/` subtrees after Facade split | P3.3 |

## Iteration 37 additions (tasks 4.3.16 + 5.1.1–5.1.5 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `skill_operations.rs` → `skill_operations/` module split | CRITICAL | `command_handlers.rs` + `main.rs` consumers of `execute_*` / `SkillCli*` types; public API preserved via Facade re-exports | P4 |
| `execute_skill_operations_snapshot` / `execute_skill_curation_run` / lifecycle handlers | HIGH | CLI Skill governance operator path; live HTTP adapter vs SDK Null Object dual path | P4 |
| `LiveSkillOperationsClient` | MEDIUM | optional live runtime proof path through public Web REST facade; no Web crate linkage | P4 |
| `contract_source::skill_operations_module_sources` | LOW | static boundary tests only; replaces monolithic `include_str!("skill_operations.rs")` scan | P4 |
| P4 CLI decoupling inventory (pre-existing) | LOW | no code change required for 5.1.1–5.1.4; inventory documents already-compliant state | P4 |

## Iteration 38 additions (tasks 5.2.1–5.2.4 + 5.3.2 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `finance_live_data.rs` / `finance_llm_analysis_provider.rs` removal from runtime-host | CRITICAL | WASM `service.call` finance paths; `bootstrap_builtin_domain_pack_services` consumers; web composition bootstrap | P4 |
| `macaca-domain-pack-finance` new package crate | HIGH | optional composition-root registration; 4 finance service ids + catalog metadata | P4 |
| `finance_domain_pack_registrations` | HIGH | `ServiceRuntime::register_provider` + `start` for market/financials/news/llm.analysis | P4 |
| `InMemoryDomainPackCatalog::with_builtin_defaults` empty catalog | MEDIUM | manifest capability expansion now reports `unresolved_domain_packs` until composition root registers installed packs | P4 |
| `service_runtime_wiring` empty domain-pack bootstrap | MEDIUM | base web OS no longer auto-registers finance providers; explicit package wiring required | P4 |
| `runtime_host_domain_pack_gate` VC-hardcoded scan | LOW | static production-source token gate for runtime-host finance/crypto strings | P4 |

## Iteration 39 additions (composition-root catalog wiring — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `ApplicationSystemServiceProvider::new` + `domain_pack_catalog` param | HIGH | sole production caller `application_discovery`; custom hosts must pass shared catalog | P4 |
| `AppRuntime::with_domain_pack_catalog` | MEDIUM | `start_app` pack expansion; Application Service sync_wasm_service_policy | P4 |
| `domain_pack_wiring` + `domain-pack-finance` feature | MEDIUM | web default feature registers finance catalog + 4 providers; `--no-default-features` restores absent-pack semantics | P4 |
| `AppState.domain_pack_catalog` | MEDIUM | `app_ui_routes/context` allowlist expansion; replaces `with_builtin_defaults()` local catalog | P4 |
| `service_projection` / `yaml_adapter` catalog-aware APIs | LOW | backward-compatible wrappers still default empty catalog for unit tests | P4 |

## Iteration 40 additions (P5 terminal gates 6.1.1–6.1.5 — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `assert_route_c_allowlist_terminal_state` | MEDIUM | fails CI if Route C allowlist regrows; currently 0 rows pass | P5 |
| `workspace_dependency_edges` optional skip | MEDIUM | feature-gated domain packs no longer false-positive `optional-not-base-required` | P5 |
| `macaca-domain-pack-finance` Route C classify | LOW | OptionalModule layer registration for new package crate | P5 |
| `kernel_purity_gate` | LOW | integration-test only; audits macaca-kernel workspace deps | P5 |
| `p5_terminal_audit_gates/*` | LOW | named VC gates; migration surfaces mirror escape-hatch freeze | P5 |

## Iteration 41 additions (P5 §6.2 debt inventory + §6.1.7 shell deps + §6.3.1 baseline — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `collect_production_violations` / `ScanOptions` | LOW | integration-test only; freeze vs inventory dual-mode scan | P5 |
| `migration_debt_baseline` (raw=275) | LOW | CI fails if escape-hatch debt grows/shifts without baseline update | P5 |
| `shell_dependency_purity_gate` | LOW | macaca-cli terminal + macaca-web frozen 7-crate debt | P5 |
| `openspec/specs/unified-execution-path` + `microkernel-boundary-purity` | LOW | new baseline specs promoted from change deltas | P5 |

## Iteration 42 additions (§6.2.1 multi-path retirement + scheduler_client split + archive batch — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `multi-path-coordination-patch` migration surfaces removal | MEDIUM | freeze gate now fails on any new `graph_owner`/`authoritative`/`suppress_executor_lifecycle` production writes outside tests | P5 |
| `abi_hosted_terminal_state_fails_when_any_host_command_fails` | MEDIUM | runtime-host contract test only; validates unified authoritative hosted terminal model | P1 |
| `scheduler_client` Facade split (`scheduler_client/tests.rs`) | LOW | macaca-sdk public API unchanged; filesize gate compliance | P3 |
| `ServiceBackedSchedulerClient` / `UnavailableSystemSchedulerClient` | LOW | scheduler SDK client; no application-specific branches | P3 |
| OpenSpec archive batch (4 changes) | LOW | docs-only move to `changes/archive/`; specs already promoted in iteration 41 | P5 |

## Iteration 43 additions (§6.2.1 application-runtime-direct-start retirement + runtime.rs split — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `bootstrap_manifest` / `bootstrap_manifest_from_path` | MEDIUM | replaces deprecated `start_app*`; Application Service provider + integration tests + unit tests | P1/P5 |
| `application-runtime-direct-start` migration surfaces removal | MEDIUM | freeze gate now fails on any new `AppRuntime::start_app`/`start_app_from_file` production writes | P5 |
| `runtime.rs` + `runtime/tests.rs` split | LOW | macaca-app public API rename; filesize gate compliance (~292 lines main file) | P3 |
| `APPLICATION_START_COMMAND` handler | LOW | runtime-host provider internal bootstrap only; traced service path unchanged | P1 |

## Iteration 44 additions (§6.2.1 autonomy-loop-boundary retirement — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `dispatch_scheduler_lane_tick` / `dispatch_heartbeat_lane_tick` / `dispatch_recovery_wake` | MEDIUM | replaces legacy `run_*_tick_once` supervisor APIs; autonomy service provider + integration tests | P1/P5 |
| `autonomy-loop-boundary` migration surfaces removal | MEDIUM | freeze gate fails on any new ad-hoc scheduler/heartbeat/recovery loop bypass in production | P5 |
| `assert_retired_escape_hatch_family_absent_in_production` | LOW | contract-test helper only; hard assertion for retired families | P5 |

## Iteration 45 additions (§6.2.1 web-direct-runtime-field retirement — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `AppState::service_llm_client` | MEDIUM | replaces direct `llm_client` field reads in shell adapters + framework runner agent factory paths | P3/P5 |
| `web-direct-runtime-field` migration surfaces removal | MEDIUM | freeze gate fails on any new deprecated AppState runtime-field access in production shell code | P5 |
| `llm_route_shell_adapter` / `agent_factory_build` / `agent_factory_coordinator` | LOW | LLM routing + ReAct agent construction; behavior unchanged, access path only | P3 |

## Iteration 46 additions (§6.2.1 direct-runtime-catalog-read retirement — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `snapshot_tool_catalog` / `DriverRegistry` / `DriverRuntime` | MEDIUM | replaces `collect_tools()` in driver service provider catalog assembly; shells already use `SystemDriverClient` | P2/P5 |
| `snapshot_server_definitions` / `McpRuntimeFacade` / `McpRuntimeManager` | MEDIUM | replaces `.definitions().await` in MCP service provider snapshot path; shells use `SystemMcpClient` | P2/P5 |
| `direct-runtime-catalog-read` migration surfaces removal | LOW | freeze gate fails on any new direct runtime catalog reads in production | P5 |

## Iteration 47 additions (§6.2.1 provider-compat-construction retirement — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `LegacyAgentExecutionAdapter` → `InProcessAgentExecutionPort` | MEDIUM | macaca-agent, SDK facade/registry, kernel/integration test fixtures; production uses `ServiceClientAgentExecutionAdapter` | P2.5/P5 |
| `LegacyAgentSideRegistry` → `InProcessAgentSideRegistry` | MEDIUM | dual manifest + runtime registration in SDK/kernel tests | P2.5/P5 |
| `register_legacy_kernel_agent` → `register_in_process_kernel_agent` | LOW | kernel_lifecycle, e2e_auto_programming, live_fullstack_autodev tests | P5 |
| `provider-compat-construction` migration surfaces removal | LOW | freeze gate fails on any reintroduction of forbidden provider-compat tokens | P5 |

## Iteration 48 additions (§6.2.1 autonomy-service-boundary sub-phase — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `LocalSchedulerProvider` → `InProcessSchedulerProvider` | MEDIUM | macaca-scheduler local_provider, runtime-host autonomy_supervisor/scheduler_lane, autonomy_service_provider, integration tests | P5 |
| `LocalHeartbeatProvider` → `InProcessHeartbeatProvider` | MEDIUM | macaca-heartbeat local_provider, runtime-host autonomy_supervisor/heartbeat_lane, autonomy_service_provider | P5 |
| `LocalSchedulerLeasedRun` → `InProcessSchedulerLeasedRun` | LOW | scheduler run_control lease memento only | P5 |
| `local_autonomy_providers_absent_in_production` hard assert | LOW | freeze gate fails on reintroduction of retired Local* provider symbols | P5 |

## Iteration 49 additions (§6.2.1 autonomy-service-boundary sub-phase — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `SchedulerSystemServiceProvider` → `HostSchedulerServiceAdapter` | MEDIUM | runtime-host autonomy_service_provider, lib.rs exports, heartbeat_lane tests | P5 |
| `HeartbeatSystemServiceProvider` → `HostHeartbeatServiceAdapter` | MEDIUM | runtime-host autonomy_service_provider, lib.rs exports, heartbeat_lane tests | P5 |
| `AutonomySupervisor` → `AutonomyLifecycleCoordinator` | MEDIUM | runtime-host autonomy_supervisor, autonomy_service_provider bundle wiring | P5 |
| `"service.scheduler"` / `"service.heartbeat"` → proto constants | LOW | autonomy_dispatch, heartbeat_agent_dispatch metadata provenance | P5 |
| `autonomy_host_adapters_absent_in_production` / `autonomy_supervisor_absent_in_production` / `autonomy_service_id_literals_absent_outside_proto` | LOW | hard asserts for retired autonomy-service-boundary tokens | P5 |

## Iteration 50 additions (§6.2.1 autonomy-service-boundary family retirement — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `autonomy-service-boundary` migration surfaces removal | LOW | freeze gate fails on any reintroduction of legacy autonomy composition symbols | P5 |
| `"service.scheduler"` / `"service.heartbeat"` removed from `forbidden_tokens` | LOW | proto owns canonical constants; literal guard via `assert_production_literal_tokens_absent_outside_allowed_paths` | P5 |
| `autonomy_service_boundary_absent_in_production` family hard assert | LOW | terminal Strangler Fig state for 7th retired escape-hatch family | P5 |
| `migration_debt_baseline` raw 177→175 | LOW | debt inventory baseline only; no production runtime behavior change | P5 |

## Iteration 51 additions (§6.2.1 hardcoded-agent-role sub-phase — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `resolve_required_entry_agent_name` | LOW | chat v2 entry agent resolution; fail-closed when manifest lacks entry_agent | P5 |
| `sse_emitter_adapter` agent_name attribution | LOW | EventLog/SSE events use runtime-resolved agent identity | P5 |
| `DriverTraceRoute::label` framework_sse | LOW | diagnostic trace dedup label only; no routing behavior change | P5 |
| `web_framework_runner_coordinator_literal_absent` hard assert | LOW | prevents reintroduction of shell-level coordinator fallback | P5 |
| `migration_debt_baseline` raw 175→169 | LOW | debt inventory baseline only | P5 |

## Iteration 52 additions (§6.2.1 hardcoded-agent-role test-fixture sub-phase — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `CreateTodoTool::tokenize` capability hints | LOW | dependency inference only; no runtime agent routing change | P5 |
| OS `*_tests.rs` fixture agent ids | LOW | test-only; no production execution path | P5 |
| `os_test_fixture_role_literals_absent` hard assert | LOW | prevents reintroduction of role literals in cleaned modules | P5 |
| `migration_debt_baseline` raw 169→124 | LOW | debt inventory baseline only | P5 |

## Iteration 53 additions (§6.2.1 hardcoded-agent-role terminal sub-phase — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `WorkflowEngine::resolve_coordinator_agent` | LOW | workflow prompt/persona resolution; fail-closed without manifest | P5 |
| `GOAL_PLANNER_EXECUTION_INTENT_LABEL` wire rename | LOW | metadata label only; enum `Planner` unchanged; callers use `metadata_value()` | P5 |
| `hardcoded_agent_role_terminal_literals_absent` hard assert | LOW | prevents reintroduction in cleaned workflow/proto modules | P5 |
| `migration_debt_baseline` raw 124→121 | LOW | `hardcoded-agent-role` family retired to 0 | P5 |

## Iteration 54 additions (§6.2.1 provider-model-routing-name terminal sub-phase — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `REMOTE_TEXT_EMBEDDING_PROVIDER_ID` / `MemoryBackendFactory::build_embedding_provider` | LOW | memory embedding factory; config-driven provider id from `default.toml` | P5 |
| `LlmOptions::default` / `MacacaConfig::default` empty routing fields | LOW | serde defaults only; runtime loads `config/default.toml` | P5 |
| `is_provider_model_routing_canonical_owner` scanner exemption | LOW | `macaca-llm/src/` only; prevents false positives in LLM service | P5 |
| `provider_model_routing_absent_in_production` hard assert | LOW | terminal guard for last escape-hatch family | P5 |
| `migration_debt_baseline` raw 121→0 | LOW | all 10 escape-hatch families retired | P5 |

## Iteration 55 additions (§4.5.1 filesize borderline batch — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `isolated/tests.rs` / `provisioner/tests.rs` extraction | LOW | test-only relocation; production API unchanged | P5 |
| `skill_service_provider_curation/{mod,store}.rs` split | LOW | skill curation command path; `skill_service_provider` dispatch unchanged | P5 |
| `plugin_marketplace_snapshot_decode` Adapter | LOW | snapshot command decode only; marketplace dispatch unchanged | P5 |
| `skill_sanitization_boundary_tests` include_str concat | LOW | source-guard contract only | P5 |
| filesize allowlist 75→71 | LOW | debt inventory shrink; 71 oversized files remain | P5 |

## Iteration 56 additions (§4.5.1 borderline batch + execution_control split — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `governance/pipeline/tests.rs` extraction | LOW | context governance pipeline tests only | P5 |
| `macaca-task/{scheduler,todo_store}/tests.rs` | LOW | task service unit tests; production APIs unchanged | P5 |
| `executor/queue/tests.rs` / `entitlement/tests.rs` | LOW | runtime-host kernel queue + commerce facade tests | P5 |
| `execution_control_service/{mod,command_adapters}.rs` | LOW | proto contract module tree; `pub use execution_control_service::*` unchanged | P5 |
| filesize allowlist 71→65 | LOW | debt inventory shrink; 65 oversized files remain | P5 |

## Iteration 57 additions (§4.5.1 proto/context borderline batch — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `application_manifest/tests.rs` extraction | LOW | proto manifest serde contract tests only | P5 |
| `macaca-context/memory/tests.rs` extraction | LOW | memory recall governance tests; production DTOs unchanged | P5 |
| `agent_execution_service/{mod,autonomous_envelope,command_adapters}.rs` | LOW | proto contract module tree; `pub use agent_execution_service::*` unchanged | P5 |
| filesize allowlist 65→62 | LOW | debt inventory shrink; 62 oversized files remain | P5 |

## Iteration 58 additions (§4.5.1 skill/llm/proto borderline batch — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `governance_store/tests.rs` extraction | LOW | skill governance store contract tests only | P5 |
| `resilient/tests.rs` extraction | LOW | LLM Decorator retry/fallback contract tests; production wrapper unchanged | P5 |
| `scheduled_agent_task_service/{mod,command_adapters}.rs` | LOW | proto contract module tree; `pub use scheduled_agent_task_service::*` unchanged | P5 |
| filesize allowlist 62→59 | LOW | debt inventory shrink; 59 oversized files remain | P5 |

## Iteration 59 additions (§4.5.1 proto heartbeat/tool facade batch — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `heartbeat_service/{mod,domain_vocabulary,profile_command_builders,command_adapters}.rs` | LOW | proto contract module tree; `pub use heartbeat_service::*` unchanged | P5 |
| `heartbeat_service/tests.rs` | LOW | wake/complete-run trace + adapter contract tests only | P5 |
| `tool_service/{mod,descriptor_vocabulary}.rs` | LOW | industrial tool descriptor/plan vocabulary split; public exports unchanged | P5 |
| filesize allowlist 59→57 | LOW | debt inventory shrink; 57 oversized files remain | P5 |

## Iteration 60 additions (§4.5.1 macaca-heartbeat local_provider batch — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `local_provider/{run_lifecycle,native_cadence,completion_sanitizer}.rs` | LOW | heartbeat service provider module tree; public `InProcessHeartbeatProvider` API unchanged | P5 |
| `local_provider.rs` facade shrink (562→172 lines) | LOW | composition root only; command_handler/gates/memento unchanged | P5 |
| filesize allowlist 57→56 | LOW | debt inventory shrink; 56 oversized files remain | P5 |

## Iteration 61 additions (§4.5.1 runtime-host borderline batch — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `skill_service_provider_proposal_materialization/{mod,draft_materialization_builder,identity_vocabulary,content_digest_vocabulary}.rs` | LOW | Skill provider module tree; `apply_command` export unchanged | P5 |
| `application_execution_remote_agent/{registration,transport,registry,provider}.rs` | LOW | remote agent provider module tree; `lib.rs` re-exports unchanged | P5 |
| filesize allowlist 56→54 | LOW | debt inventory shrink; 54 oversized files remain | P5 |

## Iteration 62 additions (§4.5.1 heartbeat/wasm borderline batch — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `heartbeat_lane/tests/{test_doubles,fixtures,nonblocking_tick,cadence_tick,run_history}.rs` | LOW | HeartbeatLane contract tests only; production lane unchanged | P5 |
| `default_provider/{mod,session,artifact_loader}.rs` | LOW | WASM default provider module tree; `DefaultInProcessWasmRuntimeProvider` export unchanged | P5 |
| `hardened_provider/{mod,session,response_mapper}.rs` | LOW | WASM hardened provider module tree; public exports unchanged | P5 |
| filesize allowlist 54→51 | LOW | debt inventory shrink; 51 oversized files remain | P5 |

## Iteration 63 additions (§4.5.1 skill-state + lib facade batch — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `skill_service_provider_state/{mod,governance_read_model,lifecycle_mutations,alias_governance,proposal_store,event_vocabulary}.rs` | LOW | Skill provider governance state module tree; `SkillProviderGovernanceState` + `event_id` exports unchanged | P5 |
| `runtime_host_public_api.rs` + thin `lib.rs` | LOW | runtime-host crate public re-export Facade; downstream `macaca_runtime_host::*` imports unchanged | P5 |
| filesize allowlist 51→49 | LOW | debt inventory shrink; 49 oversized files remain | P5 |

## Iteration 64 additions (§4.5.1 skill_service_provider Command Router split — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `skill_service_provider/{mod,system_service,command_dispatch,runtime_facade_commands,operator_commands,governance_alias_commands,evolution_commands,evaluation_content_commands,delegated_commands}.rs` | LOW | Skill provider adapter module tree; `SkillSystemServiceProvider` public API + `SystemService` contract unchanged | P5 |
| `skill_sanitization_boundary_tests.rs` include_str paths | LOW | source-guard contract scans split provider modules instead of monolith | P5 |
| filesize allowlist 49→48 | LOW | debt inventory shrink; 48 oversized files remain | P5 |

## Iteration 65 additions (§4.5.1 skill_service_provider_tests Contract Test split — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `skill_service_provider_tests/{mod,fixtures,test_doubles,alias_helpers,governance_evaluation_tests,curation_dry_run_tests,curation_run_snapshot_tests,curation_rollback_tests,alias_tests,experience_proposal_tests,experience_routing_tests,experience_validation_tests,governance_replay_tests}.rs` | LOW | Skill provider contract test module tree; 26 lib tests unchanged; `lib.rs` `mod skill_service_provider_tests` path unchanged | P5 |
| filesize allowlist 48→47 | LOW | debt inventory shrink; 47 oversized files remain | P5 |

## Iteration 66 additions (§4.5.1 tool_family_providers Abstract Factory split — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `tool_family_providers/{mod,constants,family_catalog,descriptor_builder,inventory,planning_factory}.rs` | LOW | Industrial tool-family Abstract Factory module tree; public API via `runtime_host_public_api` unchanged | P5 |
| `tool_service_boundaries.rs` multi-file source scan | LOW | architectural boundary contract scans split modules instead of monolith | P5 |
| filesize allowlist 47→46 | LOW | debt inventory shrink; 46 oversized files remain | P5 |

## Iteration 67 additions (§4.5.1 tool_service_invocation Decorator/Router split — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `tool_service_invocation/{mod,admission,dispatch,helpers}.rs` | LOW | Tool invocation router module tree; `ToolInvocationService` public API unchanged; `tool_service_provider` wiring unchanged | P5 |
| filesize allowlist 46→45 | LOW | debt inventory shrink; 45 oversized files remain | P5 |

## Iteration 68 additions (§4.5.1 application_execution_hosted Facade split — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `application_execution_hosted/{mod,types,abi_adapter,host_signal_translator,provider,provider_impl}.rs` | HIGH | WASM/YAML audit authority paths; public API via `runtime_host_public_api` unchanged | P5 |
| `unified_audit_replay_convergence_tests.rs` multi-file source scan | LOW | architectural contract scans split modules instead of monolith | P5 |
| filesize allowlist 45→44 | LOW | debt inventory shrink; 44 oversized files remain | P5 |

## Iteration 69 additions (§4.5.1 host_import_tests Contract Test split — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `host_import_tests/{mod,support,service_call_tests,domain_import_tests,audit_genui_tests,admission_guard_tests,policy_sanitization_tests}.rs` | LOW | WASM host-import contract test module tree; 15 lib tests unchanged; `wasm_runtime_provider/mod.rs` `mod host_import_tests` path unchanged | P5 |
| filesize allowlist 44→43 | LOW | debt inventory shrink; 43 oversized files remain | P5 |

## Iteration 70 additions (§4.5.1 component_model_tests Contract Test split — memo only, non-blocking)

| Symbol / area | GitNexus risk (memo) | Blast radius (summary) | Phase |
|---------------|----------------------|------------------------|-------|
| `component_model_tests/{mod,support,descriptor_admission_tests,host_import_routing_tests,host_command_plan_tests,host_command_orchestration_tests,lifecycle_timeout_tests}.rs` | LOW | Component-model WASM contract test module tree; 12 lib tests unchanged; `wasm_runtime_provider/mod.rs` `mod component_model_tests` path unchanged | P5 |
| filesize allowlist 43→42 | LOW | debt inventory shrink; 42 oversized files remain | P5 |
