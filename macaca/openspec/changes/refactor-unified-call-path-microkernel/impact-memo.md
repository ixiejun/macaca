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
