## Context

Macaca Agent OS 经过快速迭代，核心代码集中在 `routes.rs`（4,993 行），形成了"万能文件"反模式。系统审计识别了 5 个互相关联的架构债务。本重构按依赖顺序分 5 个 Phase 实施，每个 Phase 独立可验证。

## Goals / Non-Goals

**Goals:**
- routes.rs 从 ~5,000 行降至 ~500 行
- 消除所有 OS 层 "coordinator" 硬编码
- 类型定义单一来源（macaca-proto）
- AppState 字段数从 27 降至 ~8
- AgenticLoop 消除 60% 代码重复

**Non-Goals:**
- 不改变 HTTP API 或前端接口
- 不改变持久化 schema
- 不改变 PlanLoop/WorkerLoop 行为逻辑
- 不引入新功能
- 不在本轮处理 `SYSTEM_AUDIT.md` 第 6 点“接入或标记未使用模块”

## Decisions

### Decision 1: routes.rs 拆分策略

按职责提取 4 个模块：

```
macaca-web/src/
├── routes.rs              # 路由注册 + 薄 handler（~500 行）
├── chat_orchestrator.rs   # post_chat SSE + AgenticLoop + pause/resume（~1,500 行）
├── loop_manager.rs        # ensure_plan_and_worker_loops + PlanEvent/WorkerEvent 消费者（~800 行）
├── sse.rs                 # convert_executor_event_to_sse + broadcast_to_app_sessions（~200 行）
├── session.rs             # get_session_by_id + EventLog 重建 + persist_session_snapshot（~400 行）
├── hook_consumer.rs       # 已存在
├── agent_runner.rs        # 已存在
├── state.rs               # AppState（Phase 4 会重构）
└── lib.rs                 # server bootstrap
```

**拆分原则**：提取的模块通过 `pub(crate)` 函数暴露，routes.rs 调用这些函数。不改变任何行为。

### Decision 2: entry_agent 动态查找

**选择**: 在 `DiscoveredApp`/`AppManifest` 中增加 `entry_agent: Option<String>` 字段。如果未配置，fallback 到第一个有 `delegate_task` 工具的 agent。

**替代**: 用 capability-based routing 自动发现入口 agent → 过于复杂，不可预测

### Decision 3: 类型合并策略

- `macaca-kernel::executor::TaskId` → 直接使用 `macaca_proto::TaskId`
- `macaca-kernel::executor::DelegatedTask` → 合并字段到 `macaca_proto::orchestration::DelegatedTask`
- kernel 中添加 `pub use macaca_proto::TaskId;` re-export

### Decision 4: AppState 分组

```rust
pub struct AppState {
    pub kernel: Arc<Kernel>,
    pub llm: Arc<dyn LlmProvider>,
    pub tools: Box<dyn ToolSet>,
    pub persist: PersistenceState,     // session_store, todo_store, event_log, audit_logger, run_tracer
    pub loops: LoopState,              // plan_loop_handles, worker_loop_handles, plan_loop_wakers, worker_loop_wakers, scheduler_handles
    pub sessions: SessionState,        // active_sessions, cancel_flags, fork_to_session, goal_to_session, delegate_session_id, sessions
    pub executor_registry: Arc<ApplicationExecutorRegistry>,
    pub app_config: AppConfig,         // app_dirs, app_workspaces, default_model, skills_catalog, alert_manager
}
```

### Decision 5: AgenticLoop 重构

提取 `run_iteration()` 方法：
```rust
async fn run_iteration(&self, messages, options, tools, config) -> IterationResult {
    // 1. Call LLM
    // 2. If tool_calls: execute tools, return ToolsExecuted
    // 3. If no tool_calls: return FinalResponse
}

enum IterationResult {
    ToolsExecuted { messages_delta, tool_results },
    FinalResponse { content },
    Error { error },
}
```

三个 run 变体只处理：
- `run`: 纯循环
- `run_with_events`: 循环 + emit AgentExecutionEvent
- `run_with_pause`: 循环 + emit + pause/resume 检查

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|------|--------|------------|
| 拆分 routes.rs 可能引入回归 | 行为变化 | 每次提取后 cargo test + 手动 E2E 验证 |
| 类型合并破坏 kernel 内部 | 编译错误 | 逐步替换，每次 cargo check |
| AppState 分组改变所有 handler 签名 | 大量文件改动 | 先拆分 routes.rs，再分组 AppState（减少改动面积） |

## Migration Plan

按顺序执行，每个 Phase 独立可回滚：
1. Phase 1（routes.rs 拆分）→ cargo test 验证
2. Phase 2（消除硬编码）→ cargo check + 新 app 测试
3. Phase 3（合并类型）→ cargo check 全 workspace
4. Phase 4（精简 AppState）→ cargo check + 手动测试
5. Phase 5（AgenticLoop）→ cargo test -p macaca-runtime
