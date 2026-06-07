# Change: Refactor Task System to Sequential Execution with Cross-Agent Dependencies

## Why

当前 task 系统基于优先级调度（BinaryHeap），同一 agent 的多个任务可能乱序执行，无法保证"先完成 task1 再执行 task2"的语义。此外，跨 agent 的依赖关系虽然有 `depends_on` 字段，但缺乏显式的阶段编排能力——例如 fullstack-autodev 中 architect 必须先完成架构规划，frontend/backend agent 才能开始编码，当前系统无法可靠保证这一点。

核心问题：
1. **优先级调度 vs 顺序执行**：同一 agent 的任务应该按序号严格顺序执行，而非按优先级抢占
2. **跨 agent 依赖编排**：缺少"阶段"或"依赖组"概念，无法表达"所有 architect 任务完成后才启动 frontend 任务"

## What Changes

- **BREAKING**: `TodoItem.priority` 字段语义变更为辅助排序，主排序改为 `sequence_number`
- **BREAKING**: `TaskBoard.claim_next()` 改为按 `sequence_number` 升序取任务，不再按优先级
- `TodoItem` 新增 `sequence_number: u32` 字段，表示在该 agent+session 内的执行顺序
- `TaskQueue` 从 `BinaryHeap`（优先级堆）改为按序号排序的有序队列
- `LlmDecomposer` 输出增加 `sequence` 字段和跨 agent 依赖声明
- `TaskSpace.create_and_assign()` 自动分配序号（同 agent 内递增）
- `TaskBoard.claim_next()` 只返回序号最小的 Pending 任务
- 增强 `depends_on` 机制，支持跨 agent 依赖的自动解析
- `WorkerLoop` 保证严格顺序：当前任务完成后才 claim 下一个

- 前端 `TaskBoardModal` 按 sequence_number 排序展示，每个任务卡片前显示执行序号
- 前端 `TodoItem` 类型新增 `sequence_number` 字段
- 后端 `list_todos` / `list_agent_todos` API 返回按 sequence_number 排序的结果
- `PriorityBadge` 替换为 `SequenceBadge`，显示 `#1`, `#2` 等序号

## Impact

- Affected specs: task-execution-ordering (NEW), task-dependency-management (NEW), task-ui-display (NEW)
- Affected code:
  - `macaca-proto/src/types.rs` — TodoItem 结构体变更
  - `macaca-task/src/todo_board.rs` — TaskBoard.claim_next() 和 TaskSpace 逻辑
  - `macaca-task/src/todo_store.rs` — 存储查询排序
  - `macaca-task/src/queue.rs` — TaskQueue 排序策略
  - `macaca-task/src/decompose.rs` — LlmDecomposer 输出格式
  - `macaca-task/src/worker_loop.rs` — 顺序执行保证
  - `macaca-task/src/plan_loop.rs` — 依赖解析增强
  - `macaca-kernel/src/executor/queue.rs` — ExecutionQueue 排序
  - `macaca-tools/src/orchestration.rs` — DelegateTaskTool schema
  - `macaca-web/src/routes.rs` — Todo API 返回排序
  - `frontend/lib/types.ts` — TodoItem 类型新增 sequence_number
  - `frontend/components/TaskBoardModal.tsx` — 序号展示 + 排序 + 依赖状态可视化
