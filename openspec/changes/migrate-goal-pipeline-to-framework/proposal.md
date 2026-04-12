# Change: Migrate Goal-Task Pipeline to macaca-framework

## Why

Goal-Task 全链路（create_goal → 分解 → 执行 → 审查 → 完成）经过 2+ 周调试仍不稳定，根因是链路中每个环节使用不同的 ad-hoc 模式：

| 环节 | 当前实现 | 问题 |
|------|---------|------|
| Coordinator 对话 | `run_agentic_stream()` — 730 行手写循环 | 无记忆管理，暴力截断 |
| 工具注入 | `AgentToolSet` — 手动拼 base+extra | 无分组、无中间件 |
| PlanLoop 分解 | `delegate_task` + `let _ =` 忽略错误 | 无错误处理，静默失败 |
| Worker 执行 | `delegate_task` + 无限轮询等待结果 | 无超时，session 丢失 |
| PlanLoop 审查 | `delegate_task` + 无去重 | 重复审查 |

`macaca-framework` 已实现完整的 Agent 框架（ReActAgent/Toolkit/WorkingMemory/Formatter），可以用统一的抽象替代这些 ad-hoc 实现，同时获得：
- **标签化记忆** — 替代简单截断
- **工具分组** — 按角色注入不同工具集
- **结构化错误处理** — AgentError 替代 `let _ =`
- **中断支持** — CancellationToken 替代 AtomicBool 轮询
- **Formatter 层** — 解耦消息格式化

## What Changes

### Phase 1: Adapter 桥接层
在 macaca-framework 中添加 macaca-llm 和 macaca-tools 的适配器，让 framework 的 ReActAgent 可以使用现有 LLM 提供商和工具。

### Phase 2: Framework Agent 工厂
新建 `macaca-web/src/framework_runner.rs`，提供 `build_react_agent()` 工厂方法，根据 persona 配置构建 ReActAgent（含 Formatter + WorkingMemory + Toolkit）。

### Phase 3: Coordinator 迁移
在 `chat_orchestrator.rs` 中新增 `post_chat_v2()` 使用 ReActAgent 替代 `run_agentic_stream()`。通过 `?engine=framework` 查询参数切换。

### Phase 4: Worker 执行迁移
重写 `loop_manager.rs` 的 WorkerLoop TaskClaimed 消费者：用 ReActAgent 直接执行任务，替代 `delegate_task` + 结果轮询模式。

### Phase 5: Planner 分解/审查迁移
重写 `loop_manager.rs` 的 PlanLoop GoalReady/ReviewNeeded 消费者：用 ReActAgent 执行分解和审查，有错误处理和重试。

### Phase 6: 验证 + 切换默认
E2E 验证全链路后，将 framework 引擎设为默认，废弃旧的 `run_agentic_stream`。

## Explicit Non-Goal

- **不修改 PlanLoop/WorkerLoop 调度逻辑** — 调度层保留，只替换执行层
- **不修改 HTTP API** — 前端接口不变
- **不修改 TodoStore/TaskBoard** — 持久化层保留
- **不在本轮实现 A2A** — A2A 集成后续单独处理

## Impact

- **修改 crate**: `macaca-framework`（添加 adapter）、`macaca-web`（新增 framework_runner、修改 chat_orchestrator 和 loop_manager）
- **新增文件**: `macaca-web/src/framework_runner.rs`
- **不影响**: HTTP API、前端、PlanLoop/WorkerLoop 调度逻辑、TodoStore
- **渐进式**: 新旧引擎并存，通过参数切换
