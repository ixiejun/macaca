# Executor Consumer Inventory

Inventory of **production** references to `macaca-kernel` executor symbols, produced for task **2.3.1** (`refactor-unified-call-path-microkernel`).

## Scope

| Included | Excluded |
|----------|----------|
| `macaca/crates/**` production source (`.rs`) | `macaca-kernel/src/executor/**` (definitions / eviction source) |
| Runtime wiring, shell adapters, tool surfaces | `*_tests.rs`, `tests/**`, `mod tests` blocks |
| Proto/tool contracts that name executor semantics | Kernel-internal re-exports (`macaca-kernel/src/lib.rs`, `executor/mod.rs`) |
| | `AgentOrchestrator::delegate_task` in `macaca-kernel/src/orchestrator.rs` (separate type; not `ApplicationExecutor`) |

**Search date:** 2026-06-07  
**Symbols:** `ApplicationExecutor`, `ForkManager`, `delegate_task`, `ExecutorEvent`, `ApplicationExecutorRegistry`

> **P2.4 update (iteration 17):** Physical eviction complete. Canonical import path is `macaca_runtime_host::{ApplicationExecutor, ForkManager, ...}` (or `macaca_runtime_host::executor::*`). Production `macaca_kernel::executor` references = **0**. Definitions live under `macaca-runtime-host/src/executor/`.

## Replacement model (from `design.md` D1, D3, Q3)

| Evicted kernel surface | Target service path |
|------------------------|---------------------|
| Per-app executor registry + worker dispatch | `service.task` (runtime-host task provider / `macaca-task`) |
| Agent delegation + worker execution | `service.agent_execution` (`macaca-runtime-host/src/agent_execution_service_provider.rs`) |
| Fork–join pause/resume, hook lifecycle | `service.execution_control` (`macaca-runtime-host/src/execution_control_service_provider.rs`) |
| Executor lifecycle / hook events → UI | Thin shell SSE adapter consuming `service.realtime` + execution-control/task audit events |
| `delegate_task` tool execution | `service.agent_execution` + `service.task` (tool surface stays in `macaca-tools`; callback moves off kernel) |

## Summary

| Crate | Production files | Primary role |
|-------|------------------|--------------|
| `macaca-web` | 14 | Composition root: registry bootstrap, loop orchestration, SSE, persistence, fork hooks |
| `macaca-tools` | 1 | `delegate_task` / `get_task_result` tool definitions (execution wired in web) |
| `macaca-proto` | 1 | Static `delegate_task` tool schema helper (unused caller today) |
| `macaca-runtime-host` | 1 | WASM delegate **task lifecycle** naming on service path (no direct executor import) |
| `macaca-app` | 1 | Doc comment only |

**Total production consumer rows:** 47 (excluding kernel definitions and tests).

## Consumer table

| file | symbol | usage kind | replacement service path |
|------|--------|------------|--------------------------|
| `shells/macaca-web/src/state.rs` | `ApplicationExecutorRegistry` | `AppState` field — per-app executor lookup handle | `macaca-sdk` task/execution client; shell holds client handle only |
| `shells/macaca-web/src/lib.rs` | `ApplicationExecutorRegistry` | Construction with `AgentRunner`; stored on `AppState` | `service.task` provider registration at composition root |
| `shells/macaca-web/src/lib.rs` | `ApplicationExecutorRegistry` | Bootstrap: wire lazy ref into orchestration tools + WASM backend | `service.task` + `service.application` delegate adapter |
| `shells/macaca-web/src/lib.rs` | `ApplicationExecutorRegistry` | Bootstrap: `register_application` for all started apps | `service.task` register-scope command |
| `shells/macaca-web/src/lib.rs` | `ApplicationExecutorRegistry` | Spawn `hook_consumer` background task | `service.execution_control` event subscription |
| `shells/macaca-web/src/orchestration_tools.rs` | `ApplicationExecutorRegistry` | Lazy `RwLock<Option<Arc<…>>>` shared with delegate tools | `service.task` client via SDK |
| `shells/macaca-web/src/orchestration_tools.rs` | `ApplicationExecutorRegistry` | `get` / `list_applications` in `DelegateTaskTool` callback | `service.task` + `service.agent_execution` |
| `shells/macaca-web/src/orchestration_tools.rs` | `ForkManager` | `executor.fork_manager()` — `create_fork`, `start_fork`, `suspend_fork` in delegate callback | `service.execution_control` fork commands |
| `shells/macaca-web/src/orchestration_tools.rs` | `ForkManager` | `get_fork` in `GetTaskResultTool` callback | `service.execution_control` fork query |
| `shells/macaca-web/src/orchestration_tools.rs` | `delegate_task` | `ApplicationExecutor::delegate_task(...)` in delegate tool callback | `service.agent_execution` delegate + `service.task` task id |
| `shells/macaca-web/src/wasm_orchestration_backend.rs` | `ApplicationExecutorRegistry` | `delegate_agent`: registry presence / per-app executor configured check | `service.application` + `service.agent_execution` (drop registry gate) |
| `shells/macaca-web/src/chat_orchestrator.rs` | `ApplicationExecutorRegistry` | `unregister` on session cleanup | `service.task` unregister-scope |
| `shells/macaca-web/src/chat_orchestrator.rs` | `ApplicationExecutorRegistry` | `get` / `register_application` in `ensure_app_executor` | `service.task` ensure-scope |
| `shells/macaca-web/src/chat_orchestrator.rs` | `ApplicationExecutorRegistry` | `get` + `subscribe_to_events` for WASM fast-path SSE bridge | `service.realtime` + `service.agent_execution` evidence stream |
| `shells/macaca-web/src/chat_orchestrator.rs` | `ApplicationExecutorRegistry` | `get` + `spawn_session_event_collector` on YAML chat path | `service.task` audit + persist via SDK |
| `shells/macaca-web/src/chat_orchestrator.rs` | `ApplicationExecutor` | Passed to `spawn_session_event_collector` | `service.agent_execution` / task audit collector adapter |
| `shells/macaca-web/src/chat_orchestrator.rs` | `ExecutorEvent` | `sync_delegated_agent_activity_from_executor_event` match — kernel activity projection | `service.execution_control` lifecycle events → kernel status via SDK |
| `shells/macaca-web/src/chat_orchestrator.rs` | `ExecutorEvent` | Forward to SSE via `convert_executor_event_to_sse` | `service.realtime` thin-shell adapter |
| `shells/macaca-web/src/loop_manager.rs` | `ApplicationExecutorRegistry` | `get` for planner calls, worker loop setup, plan-loop agent listing | `service.task` scope queries |
| `shells/macaca-web/src/loop_manager.rs` | `ApplicationExecutor` | `broadcast_event` task started/completed/failed during plan-loop service calls | `service.agent_execution` lifecycle emission (provider-owned) |
| `shells/macaca-web/src/loop_manager.rs` | `ApplicationExecutor` | Function params (`run_planner_*` helpers) | Remove; loop consumes `service.execution_control` + `service.task` |
| `shells/macaca-web/src/loop_manager.rs` | `ExecutorEvent` | Factory helpers + broadcast payloads (`executor_task_*`) | `service.execution_control` event DTOs |
| `shells/macaca-web/src/loop_manager.rs` | `delegate_task` | Comment: plan-loop waits on delegated task completion | `service.task` + `service.execution_control` resume signals |
| `shells/macaca-web/src/framework_runner.rs` | `ApplicationExecutor` | `FrameworkRunnerBuildMode::Executor` / `StandardAgentMode::Executor` / `DriverTraceRoute::Executor` type fields | `service.agent_execution` (framework build behind service provider) |
| `shells/macaca-web/src/framework_runner.rs` | `ApplicationExecutor` | `broadcast_event(ExecutorEvent::AgentEvent {…})` driver/tool trace streaming | `service.agent_execution` agent-event channel |
| `shells/macaca-web/src/framework_runner.rs` | `ExecutorEvent` | `AgentEvent` variant construction for traces | `service.realtime` or agent-execution evidence stream |
| `shells/macaca-web/src/agent_execution_backend.rs` | `ApplicationExecutorRegistry` | `get` for optional lifecycle broadcast around service execution | `service.agent_execution` (single lifecycle emitter inside provider) |
| `shells/macaca-web/src/agent_execution_backend.rs` | `ExecutorEvent` | Indirect via `ExecutorEventFactory` → `broadcast_event` started/completed/failed | `service.agent_execution` provider-internal events |
| `shells/macaca-web/src/agent_execution_evidence.rs` | `ApplicationExecutor` | Optional executor in evidence observer; `broadcast_event` | `service.agent_execution` evidence mirror |
| `shells/macaca-web/src/agent_execution_evidence.rs` | `ExecutorEvent` | `AgentEvent` broadcast for observed tool/model steps | `service.realtime` / application-execution mirror |
| `shells/macaca-web/src/event_persistence.rs` | `ApplicationExecutor` | `spawn_session_event_collector` subscribes via executor | Task/audit collector on `service.task` + persist client |
| `shells/macaca-web/src/event_persistence.rs` | `ExecutorEvent` | Match all variants → EventLog + RunTracer + AgentTraceCollector | `service.task` audit events + `macaca-persist` |
| `shells/macaca-web/src/sse.rs` | `ExecutorEvent` | `convert_executor_event_to_sse` — HTTP/SSE DTO adapter | Thin shell: map `service.realtime` / execution-control events |
| `shells/macaca-web/src/sse.rs` | `ForkManager` | `HookEvent` variants inside `ExecutorEvent::HookEvent` SSE mapping | `service.execution_control` hook events |
| `shells/macaca-web/src/sse.rs` | `delegate_task` | JSON field `delegate_task_id` in `ForkWaiting` SSE payload | `service.task` task id in execution-control event DTO |
| `shells/macaca-web/src/session.rs` | `ApplicationExecutorRegistry` | `get` for session SSE reconnect stream | `service.realtime` subscription scoped by session |
| `shells/macaca-web/src/session.rs` | `ApplicationExecutor` | `subscribe_to_events` when no active coordinator | `service.realtime` / execution-control event bus |
| `shells/macaca-web/src/session.rs` | `ExecutorEvent` | `session_status_from_executor_event`, `update_session_status_from_executor_event` | `service.execution_control` session status projection |
| `shells/macaca-web/src/session.rs` | `ForkManager` | `HookEvent` match inside executor-event status helper | `service.execution_control` hook status |
| `shells/macaca-web/src/hook_consumer.rs` | `ApplicationExecutorRegistry` | `list_applications`, `get` — discover executors for hook subscriptions | `service.execution_control` scope listing |
| `shells/macaca-web/src/hook_consumer.rs` | `ForkManager` | `executor.fork_manager().subscribe_to_hooks()` / `get_fork` — coordinator auto-resume | `service.execution_control` hook bus + resume commands |
| `shells/macaca-web/src/hook_consumer.rs` | `ApplicationExecutor` | Doc + indirect access via registry `get` | `service.execution_control` consumer (not executor handle) |
| `services/macaca-tools/src/orchestration.rs` | `delegate_task` | `DelegateTaskTool::name()` + `execute()` tool surface | Execution callback → `service.agent_execution` / `service.task` (wired in web) |
| `services/macaca-tools/src/orchestration.rs` | `ApplicationExecutor` | Doc comment: real execution mode checks executor via callback | Remove comment; document service.client callback contract |
| `foundation/macaca-proto/src/orchestration.rs` | `delegate_task` | `delegate_task_tool_definition()` static JSON schema | Keep proto tool schema; execution path via services only |
| `runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs` | `delegate_task` | `open_agent_delegate_task_lifecycle` / `close_agent_delegate_task_lifecycle` method names | Already on `service.*` route path; align lifecycle with `service.task` |
| `application/macaca-app/src/model.rs` | `delegate_task` | Doc comment on agent capability selection | Manifest/capability metadata only; no runtime executor coupling |

## Notes

1. **Single composition root today:** All live executor/registry/fork usage outside kernel definitions flows through `macaca-web`. No production consumers exist in `macaca-cli`, `macaca-sdk`, `macaca-task`, or `macaca-agent` crates for these five symbols.
2. **`ExecutorEventFactory`:** Used in `loop_manager.rs` and `agent_execution_backend.rs` but not listed separately; it is a helper for `ExecutorEvent` emission and migrates with agent-execution / execution-control providers.
3. **`HookEvent`:** Not a requested symbol; listed under `ForkManager` where web matches fork hook payloads.
4. **Tests referencing executor:** `macaca-integration-tests` (`pipeline_dry_run.rs`, `serviceization_escape_hatches.rs`) and inline `#[cfg(test)]` modules document or gate executor paths but are out of scope for this inventory.

## Related OpenSpec tasks

- **2.3.2** — Fork–join contract on `service.execution_control`
- **2.3.3** — `delegate_task` tool execution off kernel executor
- **2.3.4** — `loop_manager.rs` consumes execution-control + task events
- **2.3.5** — Integration test: delegate + goal paths on unified service chain
