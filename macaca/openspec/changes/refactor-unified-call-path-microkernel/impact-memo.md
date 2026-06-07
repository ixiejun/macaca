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
