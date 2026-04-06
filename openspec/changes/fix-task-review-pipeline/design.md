## Context

Macaca 的 Plan-Verify 循环中，Worker agent 完成任务后提交 review，PlanLoop 检测到 PendingReview 状态后委派 planner agent 审核。当前实现存在三个缺陷导致 planner 无限循环、review 结果不可见、后续任务启动延迟。

## Goals / Non-Goals

**Goals:**
- PlanLoop ReviewNeeded 事件不重复派发同一任务
- Review 结果通过 SSE + EventLog 可见
- Worker ↔ PlanLoop ↔ WorkerLoop 之间即时唤醒

**Non-Goals:**
- 不改变 review_task() 的核心逻辑
- 不改变 GoalEvaluator 流程

## Decisions

### Decision 1: PlanLoop 内维护 reviewed_tasks HashSet

**选择**: 在 PlanLoop::run() 中维护 `HashSet<TaskId>` 记录已 emit ReviewNeeded 的任务。每个心跳周期先检查 PendingReview 列表，只 emit 不在 HashSet 中的新任务。同时清理已不再是 PendingReview 状态的旧记录。

**理由**: 最小改动，不需要跨 crate 通信，PlanLoop 自身就能解决去重问题。

### Decision 2: Review 事件在 PlanEvent consumer 层广播

**选择**: 不在 ReviewTodoTool 中广播（tool 层没有 SSE/EventLog 访问），而在 PlanLoop consumer 中，当 planner delegate 完成后检测 review 结果并广播。

**理由**: 保持 tool 层的纯粹性（只操作数据），事件广播在 web 层处理。

### Decision 3: 即时唤醒使用现有 Waker 机制

**选择**: Worker 提交 review 后通过 PlanLoopWaker 唤醒 PlanLoop；Review 完成后通过 WorkerLoopWaker 唤醒 WorkerLoop。

**理由**: 已有 Waker 基础设施（PlanLoopWaker/WorkerLoopWaker），只需在正确位置调用。
