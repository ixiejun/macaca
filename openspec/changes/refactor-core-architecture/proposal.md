# Change: Refactor Core Architecture — Resolve Audit Items 1-5

## Why

系统审计（`macaca/docs/SYSTEM_AUDIT.md`）中的 **Refactoring Recommendations 1-5** 揭示了 5 个架构级技术债务，严重影响系统的可维护性、可测试性和可扩展性：

1. **`routes.rs` 4,993 行巨型文件**：集中了 HTTP 路由、SSE 流、聊天编排、PlanLoop/WorkerLoop 管理、工作流执行——全部耦合在 HTTP 层
2. **30+ 处 "coordinator" 硬编码**：违反 OS 底座通用性原则，换个应用就要改源码
3. **重复类型定义**：`TaskId`/`DelegatedTask` 在 `macaca-proto` 和 `macaca-kernel` 两处定义，同名不同类型
4. **AppState 27 字段 God Object**：所有状态混在一个 struct，职责不清
5. **AgenticLoop 3 个 run 变体 60% 重复**：`run`/`run_with_events`/`run_with_pause` 共享大量逻辑但各自独立实现

这 5 个问题互相耦合：巨型 `routes.rs` 放大了 `AppState` God Object 的访问面；硬编码入口 agent 让 OS 层失去通用性；重复类型和重复 loop 逻辑又持续增加理解与修改成本。

## What Changes

### Phase 1: 拆分 routes.rs（降风险：最大文件拆小）
- 从 `routes.rs` 提取 `chat_orchestrator.rs`（post_chat SSE 流 + AgenticLoop 驱动）
- 提取 `loop_manager.rs`（PlanLoop/WorkerLoop 生命周期 + ensure_plan_and_worker_loops）
- 提取 `sse.rs`（SSE 事件转换 + 广播 + convert_executor_event_to_sse）
- 提取 `session.rs`（Session CRUD + EventLog 重建 + get_session_by_id）
- routes.rs 保留薄路由注册 + 简单 handler（~500 行）

### Phase 2: 消除 "coordinator" 硬编码
- 引入 `entry_agent` 概念：从 app manifest 读取入口 agent 名
- 替换 routes.rs 中 30+ 处 `"coordinator"` 为动态查找
- 替换 `orchestrator.rs`、`decompose.rs` 中的 hardcoded fallback

### Phase 3: 合并重复类型
- 删除 `macaca-kernel/src/executor/mod.rs` 中的重复 `TaskId` 和 `DelegatedTask`
- kernel 改为 re-export `macaca_proto::TaskId`
- 统一 `DelegatedTask` 字段集到 `macaca-proto`

### Phase 4: 精简 AppState
- 将 27 字段分组为子结构：`PersistenceState`、`LoopState`、`SessionState`、`AppConfig`
- AppState 字段数从 27 降至 ~8

### Phase 5: 提取 AgenticLoop 共享逻辑
- 提取 `run_iteration()` 私有方法，封装单次 LLM 调用 + 工具执行
- 三个 run 变体只处理各自的前后处理（pause 检查、event emit 等）
- `PausableAgenticLoop` 从 100ms 轮询改为 `tokio::sync::Notify`

## Explicit Non-Goal

本提案 **不处理** `SYSTEM_AUDIT.md` 第 6 点“接入或标记未使用模块”。`macaca-memory`、`macaca-ipc`、`macaca-mcp`、`macaca-gateway` 的接入/标记将作为后续单独 change 处理，避免把“核心架构去耦”与“能力面接入”混成一轮高风险重构。

## Impact

- Affected specs: `decompose-routes`, `eliminate-hardcoded-coordinator`, `consolidate-types`, `streamline-appstate`, `extract-agentic-loop`
- Affected code: 几乎所有 `macaca-web` 文件 + `macaca-kernel` + `macaca-runtime` + `macaca-proto`
- **BREAKING**: `macaca-web` 内部模块结构变更（不影响 HTTP API）
- **BREAKING**: `macaca-kernel::executor::TaskId` 类型移除（统一使用 `macaca-proto::TaskId`）
