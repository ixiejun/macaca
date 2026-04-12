# Change: 抽取 planner framework 调用 helper

## Why

`macaca-web/src/loop_manager.rs` 中 planner 相关的 goal decomposition、task review、follow-up planning 都在各自分支里重复执行同一类模板：设置 planner 为 Working、发送 executor start、构建 traced framework agent、调用 `reply`、发送 completed/failed、最后恢复 Idle。

这些重复分支都位于 `ensure_plan_and_worker_loops` 核心路径里。后续继续迁移到 `macaca-framework` 时，如果继续复制这段模板，容易出现某一路径漏掉 activity、ExecutorEvent、trace 或错误日志。本次只抽取 planner framework 调用 helper，保持现有端到端行为 1:1 不变。

## What Changes

- 在 `loop_manager.rs` 中新增 planner framework 调用模式/配置，用于描述 decomposition、review、follow-up 三类调用的差异。
- 新增局部 helper 统一执行：
  - `update_agent_activity_by_name(... Working)`
  - `executor_task_started`
  - traced framework agent 构建
  - `agent.reply(...)`
  - `executor_task_completed` / `executor_task_failed`
  - `update_agent_activity_by_name(... Idle)`
- 保持现有行为不变：
  - decomposition/follow-up 仍使用 `build_traced_agent_with_goal`
  - review 仍使用当前实际入口 `build_worker_agent`
  - prompt、planner agent 名称、task/goal id、goal context、session_id、executor event 时机、日志文案、失败文案都保持不变
  - `PLAN_GOAL_DELEGATE`、`PLAN_REVIEW_DELEGATE`、review 后 worker waker、SSE/EventLog plan decision 逻辑都保持在原位置

## Non-Goals

- 不改变 PlanLoop / WorkerLoop 调度语义
- 不改变 planner prompt、任务分解规则、review 规则或 follow-up 规则
- 不改变 planner 选择策略、dependency gating、TodoBoard 状态流转或 coordinator resume 行为
- 不把 review 的 builder 从 `build_worker_agent` 改成其他入口
- 不改 SSE/EventLog payload、前端历史恢复或 trace event 名称

## Impact

- Affected specs: `framework-trace-middleware`
- Affected code:
  - `macaca/crates/macaca-web/src/loop_manager.rs`
- GitNexus impact:
  - `ensure_plan_and_worker_loops` upstream risk is `CRITICAL` because it is on `create_goal`、`start_server`、`post_chat_v2` core paths.
- Risk mitigation:
  - 本次只抽取重复 framework 调用模板，不移动外层 PlanEvent 处理、SSE decision、run_trace phase 或 waker 逻辑。
  - 使用 unit test 锁定 planner 调用模式对应的 builder 选择、成功/失败日志文案和 activity context。
