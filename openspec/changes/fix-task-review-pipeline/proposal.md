# Change: Fix Task Review Pipeline — Deduplication, Event Broadcasting, Wakeup

## Why

当前任务 review 链路存在三个系统性问题：
1. **PlanLoop 重复派发 ReviewNeeded**：每 5 秒心跳都重新 emit 同一 PendingReview 任务，导致 planner agent 被重复 delegate 做同一个 review，产生无限循环
2. **Review 结果无事件广播**：ReviewTodoTool 只更新 DB，不写 EventLog、不发 SSE，前端无法看到 review 结果
3. **缺少即时唤醒**：Worker 提交 review 后不唤醒 PlanLoop（延迟 5 秒），review 完成后不唤醒 WorkerLoop（被 unblock 的后续任务延迟 claim）

## What Changes

- PlanLoop 添加 `reviewed_tasks: HashSet<TaskId>` 去重，ReviewNeeded 每个 task 只 emit 一次
- ReviewTodoTool 执行后广播 SSE 事件 + 写 EventLog
- Worker 提交 review 后调用 PlanLoopWaker 立即唤醒
- Review 完成后调用 WorkerLoopWaker 唤醒，让被 unblock 的任务立即被 claim
- PlanLoop 在 task 不再是 PendingReview 后清理 reviewed_tasks 记录

## Impact

- Affected specs: review-deduplication (NEW), review-event-broadcasting (NEW), planloop-wakeup (NEW)
- Affected code:
  - `macaca-task/src/plan_loop.rs` — ReviewNeeded 去重 + reviewed_tasks 状态管理
  - `macaca-web/src/routes.rs` — Worker 提交后唤醒 PlanLoop；Review 后唤醒 WorkerLoop；Review 事件广播
  - `macaca-tools/src/todo.rs` — ReviewTodoTool 无需改（事件广播在 routes 层处理）
