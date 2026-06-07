## Phase 1: 拆分 routes.rs (5,000 → ~500 行)

- [x] 1.1 提取 `sse.rs`：`convert_executor_event_to_sse`、`broadcast_to_app_sessions`、`save_plan_decision`、SSE 事件转换函数
- [x] 1.2 提取 `session.rs`：`get_session_by_id`、`list_sessions`、`delete_session`、`get_session_events`、`persist_session_snapshot`、`build_turns_from_messages`、`ensure_running_assistant_turn`、`stored_turns_or_messages`、所有 Stored* 类型
- [x] 1.3 提取 `chat_orchestrator.rs`：`post_chat` 完整函数 + 内部闭包（pause/resume、tool execution、SSE streaming、AgenticLoop 驱动）
- [x] 1.4 提取 `loop_manager.rs`：`ensure_plan_and_worker_loops` + PlanEvent consumer + WorkerEvent consumer
- [x] 1.5 routes.rs 保留路由注册 + 简单 CRUD handler（apps、agents、todos、goals、schedules、skills、status）
- [x] 1.6 更新 `lib.rs` 的 mod 声明和 Router 注册
- [x] 1.7 cargo check + cargo test 全 workspace 通过
- [x] 1.8 手动 E2E 验证：post_chat SSE 流正常

## Phase 2: 消除 "coordinator" 硬编码

- [x] 2.1 在 `macaca-app/src/model.rs` 的 `AppManifest` 中添加 `entry_agent: Option<String>` 字段
- [x] 2.2 在 `macaca-web/src/state.rs` 的 AppState（或 AppConfig）中存储 per-app entry_agent 映射
- [x] 2.3 替换 `chat_orchestrator.rs`（原 routes.rs post_chat）中所有 `"coordinator"` 为动态 entry_agent 查找
- [x] 2.4 替换 `loop_manager.rs`（原 ensure_plan_and_worker_loops）中的 `"coordinator"` fallback
- [x] 2.5 替换 `macaca-kernel/src/orchestrator.rs:181` 的 hardcoded fallback
- [x] 2.6 替换 `macaca-task/src/decompose.rs:200` 的 hardcoded fallback
- [x] 2.7 替换 `macaca-proto/src/orchestration.rs:243` 的 hardcoded agent 枚举
- [x] 2.8 cargo check + 使用非 fullstack-autodev 应用验证

## Phase 3: 合并重复类型

- [x] 3.1 删除 `macaca-kernel/src/executor/mod.rs` 中的 `TaskId` 定义，改为 `pub use macaca_proto::TaskId`
- [x] 3.2 合并 `macaca-kernel::executor::DelegatedTask` 字段到 `macaca-proto::orchestration::DelegatedTask`
- [x] 3.3 删除 kernel 中的重复 `DelegatedTask`，改为 re-export
- [x] 3.4 更新所有引用：app_executor.rs、fork_manager.rs、queue.rs、worker.rs
- [x] 3.5 cargo check 全 workspace 通过

## Phase 4: 精简 AppState

- [x] 4.1 定义 `PersistenceState` 子结构：session_store, todo_store, event_log, audit_logger, run_tracer
- [x] 4.2 定义 `LoopState` 子结构：plan_loop_handles, worker_loop_handles, plan_loop_wakers, worker_loop_wakers, scheduler_handles
- [x] 4.3 定义 `SessionState` 子结构：active_sessions, cancel_flags, fork_to_session, goal_to_session, delegate_session_id, sessions
- [x] 4.4 定义 `AppConfig` 子结构：app_dirs, app_workspaces, default_model, skills_catalog, alert_manager
- [x] 4.5 更新 AppState 为 ~10 字段
- [x] 4.6 更新所有 handler 中的字段访问路径（`state.todo_store` → `state.persist.todo_store`）
- [x] 4.7 更新 `lib.rs` 中的 AppState 构造
- [x] 4.8 cargo check + cargo test 全 workspace 通过

## Phase 5: 提取 AgenticLoop 共享逻辑

- [x] 5.1 定义 `IterationResult` 枚举：`ToolsExecuted`、`FinalResponse`
- [x] 5.2 提取 `run_iteration()` 私有方法：单次 LLM 调用 + 工具执行 + 返回 IterationResult
- [x] 5.3 重写 `run()` 使用 `run_iteration()` 循环
- [x] 5.4 重写 `run_with_events()` 使用 `run_iteration()` + event emit
- [x] 5.5 重写 `run_with_pause()` 使用 `run_iteration()` + pause/resume + event emit
- [x] 5.6 `PausableAgenticLoop` 从 100ms 轮询改为 `tokio::sync::Notify`
- [x] 5.7 cargo test -p macaca-runtime 全部通过（27 passed）
- [x] 5.8 手动 E2E 验证：delegate_task pause/resume 正常

## Phase 6: 范围守卫

- [x] 6.1 验证本 change 仅覆盖 `SYSTEM_AUDIT.md` Refactoring Recommendations 1-5，不包含第 6 点"接入或标记未使用模块"
- [x] 6.2 确认 `macaca-memory`、`macaca-ipc`、`macaca-mcp`、`macaca-gateway` 没有被混入本轮 implementation checklist

## Phase 7: 验证

- [x] 7.1 cargo check + cargo test 全 workspace 通过
- [ ] 7.2 手动 E2E：create_goal → decompose → execute → review → goal complete → coordinator resume
- [ ] 7.3 TERMINATE ALL PROCESSES 功能正常
- [ ] 7.4 前端刷新后历史事件正确加载
