# Change: Add Task/Planner/Review Service v1

## Why

`macaca-web::loop_manager` 仍然事实性拥有 goal decomposition、task claim、review、coordinator resume、SSE/EventLog 以及 worker wakeup 的核心 task 语义。这个结构和 Route C 的微内核边界不一致，也让 task lifecycle、review 去重、resume 触发和 trace 审计继续分散在 Web 层。

S4 的目标是把这些 task 语义收敛成 Task Service 边界，让 Web 退回为 adapter，让 `macaca-task` 成为 task/ planner/ review 领域的正式服务归属。这样后续可以把 planner/reviewer/worker execution 逐步切换到 ServiceRuntime 或其他可替换策略，而不再依赖 Web 作为长期协调中枢。

## What Changes

- 新增 Task Service 的命令、事件、快照与 provider/runtime 骨架
- 将 goal decomposition、task claim、review、coordinator resume 的语义收敛到 Task Service boundary
- 将 `macaca-web::loop_manager` 逐步缩减为 Web adapter，而不是长期系统协调器
- 为 task 生命周期、review、resume、goal completion 增加可审计的 structured trace/event 语义
- 保留旧兼容入口并标记 deprecated，而不是删除，便于迁移检索
- 保持现有 task board session-scoped 行为、`/api/chat/v2`、trace、resume、driver、skill/MCP 回归路径不退化

## Impact

- Affected specs: `task-service` (new), `macaca-task-core` (compatibility alignment)
- Affected code:
  - `macaca/crates/macaca-task/src/lib.rs`
  - `macaca/crates/macaca-task/src/plan_loop.rs`
  - `macaca/crates/macaca-task/src/worker_loop.rs`
  - `macaca/crates/macaca-task/src/todo_board.rs`
  - `macaca/crates/macaca-task/src/service.rs` (new)
  - `macaca/crates/macaca-task/src/commands.rs` (new)
  - `macaca/crates/macaca-task/src/events.rs` (new)
  - `macaca/crates/macaca-task/src/runtime.rs` (new)
  - `macaca/crates/macaca-task/src/provider.rs` (new)
  - `macaca/crates/macaca-web/src/loop_manager.rs`
  - `macaca/crates/macaca-web/src/sse.rs`
  - `macaca/crates/macaca-web/src/event_persistence.rs`
  - `macaca/crates/macaca-sdk/src/task_client.rs`
- Affected consumers:
  - `macaca-web`
  - `macaca-sdk`
  - `macaca-integration-tests`

## Non-Goals

- 不在本轮迁移 LLM、Memory、Context provider 逻辑
- 不把 `macaca-web` 变成新的 task service owner
- 不改变 task board session scope 语义
- 不删除旧 public task APIs
- 不引入 app-specific / workflow-specific hardcode
