# Change: 抽取 worker 执行结果模板

## Why

`macaca-web/src/loop_manager.rs` 中 `WorkerEvent::TaskClaimed` 与 `WorkerEvent::RetryTask` 都执行同一类流程：构建 worker agent、调用 `reply`、处理 success/error/panic/timeout、更新 TodoBoard、发 executor lifecycle event、写 run_trace、唤醒 planner。

这些分支目前重复维护，后续很容易出现某一路径遗漏 `submit_for_review`、`mark_failed`、`ExecutorEvent`、`WORKER_SUBMIT_REVIEW` 或错误文案不一致。本次只抽取 worker 执行结果处理模板，保持所有外部行为 1:1 不变。

## What Changes

- 在 `loop_manager.rs` 中新增局部 helper，用于统一处理 worker `reply` 的成功、错误、panic、timeout 结果。
- 让 `TaskClaimed` 和 `RetryTask` 分支复用该 helper。
- 保持现有行为不变：
  - 成功时 `submit_for_review`、`ExecutorEvent::TaskCompleted`、run_trace、planner waker 行为不变
  - 失败时 `mark_failed`、`ExecutorEvent::TaskFailed`、run_trace、错误文案不变
  - timeout 秒数和 panic 文案不变
  - retry 成功时 `WORKER_SUBMIT_REVIEW` detail 仍为 `retry_success`
  - normal 成功时仍额外记录 `WORKER_TASK_SUCCESS` 和 summary preview

## Non-Goals

- 不改变 `PlanLoop` / `WorkerLoop` 调度语义
- 不改变 dependency gating、review、retry 或 coordinator resume 行为
- 不改变 worker agent 构建入口、prompt、tool policy、timeout 时长或 TaskBoard 状态语义
- 不抽取 planner decomposition/review/follow-up 模板；该项属于后续独立候选点
- 不修改 SSE/EventLog/前端历史恢复逻辑

## Impact

- Affected specs: `framework-trace-middleware`
- Affected code:
  - `macaca/crates/macaca-web/src/loop_manager.rs`
- GitNexus impact:
  - `ensure_plan_and_worker_loops` upstream risk is `CRITICAL` because it is on `create_goal`、`start_server`、`post_chat_v2` core paths.
- Risk mitigation:
  - 本次只抽取重复 result 分支，保留原有分支时机、字段、错误文案和状态写入。
  - 使用 helper 单元测试覆盖 normal/retry 成功 summary fallback 与错误文案差异。
