# Change: 抽取 ExecutorEvent 生命周期 helper

## Why

`macaca-web/src/loop_manager.rs` 中 planner decomposition、planner review、planner follow-up、worker task claim 和 worker retry 都重复构造 `ExecutorEvent::TaskStarted`、`ExecutorEvent::TaskCompleted`、`ExecutorEvent::TaskFailed` 以及成功路径的 `TaskResult`。

这些重复代码增加了后续维护 trace 生命周期事件的风险：某一路径可能遗漏 `agent`、`task_id`、`completed_at` 或保持不一致的成功 payload。本次只做局部 helper 抽取，保持现有事件发射时机和 payload 1:1 不变。

## What Changes

- 在 `loop_manager.rs` 中新增私有 helper：
  - 构造 task started 生命周期事件
  - 构造 task completed 生命周期事件及成功 `TaskResult`
  - 构造 task failed 生命周期事件
  - 可选地封装 `ApplicationExecutor::broadcast_event` 调用，前提是不改变广播时机
- 将 planner 和 worker 路径中重复的生命周期事件构造切换为 helper
- 保持现有 `task_id`、`agent`、`success`、`output`、`error`、`artifacts`、`completed_at`、`tokens_used` 字段语义不变
- 保持 live SSE、EventLog、浏览器刷新历史恢复所依赖的事件名称和 payload schema 不变

## Non-Goals

- 不改变 PlanLoop/WorkerLoop 调度语义
- 不改变 task claim、review、retry、dependency gating 或 coordinator resume 行为
- 不调整 planner/worker prompt、agent selection 或 tool policy
- 不移动 helper 到 `macaca-framework` crate；本轮只做 web 层低风险收敛
- 不修改 SSE/EventLog 转换逻辑或前端消费逻辑

## Impact

- Affected specs: `framework-trace-middleware`
- Affected code:
  - `macaca/crates/macaca-web/src/loop_manager.rs`
- Expected risk: Medium due to `ensure_plan_and_worker_loops` 位于核心任务编排链路；实现策略是只替换重复构造，不改变控制流。
- Behavioral compatibility:
  - Executor lifecycle events 的发射顺序保持不变
  - Live SSE trace event 输出保持不变
  - EventLog 持久化内容保持不变
  - 刷新浏览器后的历史 trace 恢复保持不变
