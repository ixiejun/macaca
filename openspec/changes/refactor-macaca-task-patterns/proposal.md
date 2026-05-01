# Change: Refactor macaca-task around lifecycle, dependency, and loop templates

## Why

`macaca-task` 是 Agent OS 的正式任务账本与调度核心，但当前关键规则分散在 `TaskBoard`、`TaskSpace`、`PlanLoop`、`WorkerLoop` 多处内联 `if/match` 中：

- Todo 生命周期规则分散，缺少统一状态策略边界
- 依赖 gating 与父 goal claim 判定分散，难以扩展也难以验证
- PlanLoop / WorkerLoop 已有固定流程雏形，但步骤仍混杂在单个 `run()` 中

根据 `docs/design-pattern-refactor-plans/macaca-task.md`，这里适合引入 `State`、`Strategy`、`Template Method`、`Mediator` 的最小切片，在保持行为 1:1 的前提下逐步收敛。

## What Changes

- 为 Todo 生命周期补齐表驱动测试，锁定当前状态迁移契约
- 引入 `TodoLifecyclePolicy`，将 `TaskBoard` / `TaskSpace` 的状态迁移判定收口到统一策略
- 引入 `TaskDependencyResolver`，统一新任务 blocked 判定、完成后 dependents 解锁、claim 前 parent goal gating
- 将 `PlanLoop` 的固定阶段拆成显式 template step，但保持事件语义不变
- 将 `WorkerLoop` 的固定执行流程拆成显式 template step，但保持 `WorkerEvent` 语义不变
- 为被新原语替代的旧入口保留兼容 wrapper，并统一标记 `deprecated`，便于后续迁移检索

## Impact

- Affected specs:
  - `macaca-task-core` (new)
- Affected code:
  - `macaca/crates/macaca-task/src/todo_board.rs`
  - `macaca/crates/macaca-task/src/plan_loop.rs`
  - `macaca/crates/macaca-task/src/worker_loop.rs`
  - `macaca/crates/macaca-task/src/lib.rs`
  - new helper modules under `macaca/crates/macaca-task/src/`
- Affected consumers:
  - `macaca-tools`
  - `macaca-web`
  - `macaca-integration-tests`

## Non-Goals

- 不改变 task/session scope 语义
- 不改变 `PlanEvent` / `WorkerEvent` payload
- 不引入 app/agent/driver 名称硬编码
- 不在本轮改变 planner review、goal evaluation、resume 的业务规则
