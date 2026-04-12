# Change: 抽取 PlanNotebook 持久化 helper

## Why

`macaca-web/src/loop_manager.rs` 中 `planner_notebook_mark_decomposition` 和 `planner_notebook_mark_review` 都重复执行相同的 PlanNotebook 持久化模板：计算 planner session scope、创建 notebook、`load_module_state`、修改 notebook、`save_module_state`。

这段逻辑属于 web 层对 framework `PlanNotebook` 的 glue code。继续复制会让后续新增 planner notebook 写入时容易遗漏 load/save 或错误使用 session scope。本次只抽取 web-side helper，保持现有 notebook 内容和所有外部行为 1:1 不变。

## What Changes

- 在 `loop_manager.rs` 中新增局部 helper，用于统一执行 PlanNotebook 的 load -> mutate -> save 模板。
- 让 decomposition/review 的 notebook 标记函数复用该 helper。
- 保持现有行为不变：
  - `planner_scope_session_id` 计算逻辑不变
  - `PlanNotebook::new()`、`load_module_state`、`save_module_state` 的调用顺序不变
  - decomposition 的 plan id、summary、subtask 名称、subtask detail、finish message 不变
  - review 的 plan id、summary、subtask 名称、subtask detail、finish message 不变
  - `PlanNotebook` 仍然是 agent-local “脑内计划本”，`TodoBoard` 仍然是 durable task source of truth

## Non-Goals

- 不把 helper 下沉到 `macaca-framework/src/plan.rs`
- 不改变 PlanLoop / WorkerLoop 调度语义
- 不改变 planner prompt、framework agent 构建、ExecutorEvent、SSE/EventLog、run_trace 或 browser refresh 恢复行为
- 不改变 `PlanNotebook` 与 `TodoBoard` 的职责边界
- 不引入 application 专属逻辑

## Impact

- Affected specs: `framework-plan-notebook`
- Affected code:
  - `macaca/crates/macaca-web/src/loop_manager.rs`
- GitNexus impact:
  - `planner_notebook_mark_decomposition` upstream risk is `CRITICAL`
  - `planner_notebook_mark_review` upstream risk is `CRITICAL`
  - 两者都经由 `ensure_plan_and_worker_loops` 影响 `create_goal`、`start_server`、`post_chat_v2`
- Risk mitigation:
  - 本次只抽取 load/update/save 模板，保留原有 session scope、notebook 字段和写入顺序。
  - 使用 unit test 锁定 decomposition/review 写入的 plan id、title、subtask 名称、subtask detail 和 finish message。
