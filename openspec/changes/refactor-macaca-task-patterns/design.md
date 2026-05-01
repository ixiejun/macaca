## Context

`macaca-task` 目前已经承载：

- Todo 状态流转
- task dependency / parent goal gating
- goal decomposition 后任务入账
- planner review 与 goal completion 检测
- worker claim / retry / idle 调度

这些职责本身合理，但具体规则主要散落在：

- `TaskBoard::claim_next/start_task/submit_for_review/mark_failed`
- `TaskSpace::create_and_assign/review_task/skip_task/unblock_dependents`
- `PlanLoop::run`
- `WorkerLoop::run`

这会带来两个问题：

1. 状态规则和依赖规则难以独立验证
2. loop 主流程难以局部扩展，而不把更多逻辑继续堆进 `run()`

## Goals / Non-Goals

### Goals

- 用 `TodoLifecyclePolicy` 固化状态机边界
- 用 `TaskDependencyResolver` 固化依赖与 claim gating 边界
- 用显式 template step 收敛 `PlanLoop` / `WorkerLoop`
- 给旧入口保留 deprecated wrapper，便于全仓后续迁移检索

### Non-Goals

- 不改变对外 task schema、status enum、event schema
- 不把 session scope 改成 app scope
- 不重写 `TodoStore`
- 不在本轮把 planner/worker 执行逻辑迁移到别的 crate

## Decisions

### 1. 用策略对象承载状态转移，而不是只抽工具函数

选择：

- 引入 `TodoLifecyclePolicy` trait
- 默认实现 `DefaultTodoLifecyclePolicy`
- 由 `TaskBoard` / `TaskSpace` 持有该策略

原因：

- 这是标准 `State` 模式的低成本落地
- 后续如果要引入更严格审计、不同 review/retry 规则，不需要继续改散落方法

替代方案：

- 只抽纯函数

不选原因：

- 纯函数可以减重复，但不能形成明确扩展边界，也无法自然承载 deprecated migration 目标

### 2. 用单一 `TaskDependencyResolver` 统一三处依赖判定

选择：

- `initial_status_for_new_task`
- `can_claim_task`
- `reevaluate_after_completion`

原因：

- 现在 dependency 逻辑分散在 create、claim、complete 三个阶段
- 用一个 strategy 收口，能保证“只有 Completed 才解锁 dependents”的 contract 不被不同方法各自篡改

### 3. 用内部 template step，而不是引入新的重型 loop 抽象层

选择：

- `PlanLoop` / `WorkerLoop` 保留原 struct 和 `run()` 对外入口
- 新增 step 方法和内部 template helper
- 新 canonical constructor / wrapper API 并保留 deprecated `new()`

原因：

- 这是最小行为变更
- 外部消费者仍可继续工作
- 后续如果要抽通用 loop template，也不会反向破坏本轮切片

### 4. 旧入口一律保留 wrapper，但标记 deprecated

选择：

- 旧 public constructor / 旧 public task action 方法保留
- wrapper 直接委托到新 canonical API
- 仓库内已知调用点同步迁移，避免继续扩散旧入口

原因：

- 符合“可回滚、可查找、便于后续迁移”的要求

## API Shape

### Lifecycle

```rust
pub trait TodoLifecyclePolicy: Send + Sync {
    fn on_claim(&self, task: &TodoItem) -> Option<TodoStatus>;
    fn on_start(&self, task: &TodoItem) -> Option<TodoStatus>;
    fn on_submit_for_review(&self, task: &TodoItem) -> Option<TodoStatus>;
    fn on_review(&self, task: &TodoItem, result: &TodoReviewResult) -> TodoStatus;
    fn on_skip(&self, task: &TodoItem) -> Option<TodoStatus>;
}
```

### Dependency

```rust
pub trait TaskDependencyResolver: Send + Sync {
    fn initial_status_for_new_task(&self, depends_on: &[TaskId], todos: &[TodoItem]) -> TodoStatus;
    fn can_claim_task(&self, task: &TodoItem, goals: &[TodoGoal]) -> bool;
    fn reevaluate_blocked_tasks(&self, todos: &[TodoItem], completed_id: &TaskId) -> Vec<TaskId>;
}
```

### Loop template

`PlanLoop` 和 `WorkerLoop` 不额外引入新的顶层 trait，只把 `run()` 收敛成固定步骤：

- PlanLoop:
  - `process_new_goals`
  - `emit_pending_reviews`
  - `emit_progress_anomalies`
  - `emit_goal_completion_checks`
  - `emit_all_tasks_done_fallback`
- WorkerLoop:
  - `try_claim_task`
  - `handle_claimed_task`
  - `try_retry_task`
  - `handle_retry_task`
  - `emit_idle_if_needed`
  - `wait_for_next_tick`

## Risks / Trade-offs

### Risk 1: 状态策略与现有行为不一致

缓解：

- 第一切片先补状态回归测试
- 所有 wrapper 都先委托新实现，再更新调用方

### Risk 2: dependency resolver 误改 session 语义

缓解：

- resolver 只消费调用方传入的当前作用域 todos / goals
- 不直接访问存储，也不重新定义 scope

### Risk 3: deprecated 覆盖面过大导致 workspace warning 噪音

缓解：

- 只标记“被新 canonical API 替代的 public 入口”
- 同步迁移仓库内已知调用面

## Migration Plan

1. 创建新 policy / resolver / template step
2. 用旧入口 wrapper 委托新入口，并标记 deprecated
3. 迁移 `macaca-tools`、`macaca-web`、`macaca-integration-tests` 已知调用面
4. 运行 crate 测试和 workspace check

## Open Questions

- 本轮不再继续抽通用 `LoopTemplate` trait；如果后续 `macaca-runtime` 也出现相同骨架，再单独提案
