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

**补充约束**: PlanEvent consumer 不能把 agent delegate 的自然语言完成结果等同于 Task Board review 完成。它必须在 delegate 返回后重新读取 Task Service 持久状态，只有目标 task 已经离开 `PendingReview` 时才广播 `task_reviewed` 并唤醒 worker loop；如果 task 仍处于 `PendingReview`，consumer 必须记录 run_trace/anomaly 并保持任务可见，避免制造“假完成”事件。

### Decision 3: 即时唤醒使用现有 Waker 机制

**选择**: Worker 提交 review 后通过 PlanLoopWaker 唤醒 PlanLoop；Review 完成后通过 WorkerLoopWaker 唤醒 WorkerLoop。

**理由**: 已有 Waker 基础设施（PlanLoopWaker/WorkerLoopWaker），只需在正确位置调用。

### Decision 4: Fallback decomposition 使用通用交付阶段顺序

**选择**: 当 planner 没有产出 todos 而系统创建 fallback task chain 时，按 capability 推导的通用阶段顺序为 `Research → Analyze → Produce → Validate → Finalize → Execute`。验证、review、QA、polish 类能力必须排在 produce/build/code/artifact 类能力之后。

**理由**: Fallback chain 是保守的线性任务图。它必须先产生主要交付物，再让验证或 review 类型 agent 评审，否则会生成 “review 先运行、生产任务依赖 review” 的反向依赖链，导致 Task Board 长期停留在 PendingReview/Blocked。

### Decision 5: Claim diagnostics DTO 必须支持省略空依赖列表

**选择**: `ClaimGate.incomplete_dependencies` 在空列表时仍可从缺省 JSON 反序列化为 `[]`。

**理由**: 诊断服务返回的 claimable/sequential gate 不一定有依赖列表。缺省字段不能导致 SDK 反序列化失败，否则 shell 无法显示为什么任务没有被 claim。

### Decision 6: PendingReview 派发使用有界退避重试状态机

**选择**: PlanLoop 不再把 `PendingReview` 任务记为一次性已处理，而是维护 per-task review dispatch state：`attempts`、`last_emitted_at`。当任务仍停留在 `PendingReview` 且 review delegate 没有通过 `review_todo` 落库时，PlanLoop 在退避窗口后重新 emit `ReviewNeeded`，直到达到上限；任务离开 `PendingReview` 后立即清理状态。默认退避窗口必须长于正常 planner review 执行窗口，避免健康但较慢的 review delegate 被误判成未落库并产生并发重复审核。

**理由**: 单纯去重可以阻止重复 review 风暴，但会把一次未落库的 planner delegate 变成永久卡死。过短退避会在第一次 delegate 仍运行时排入额外 review 事件。通用 OS 调度层需要同时满足可审计、可恢复和无人值守运行：既不能每个 heartbeat 重复派发，也不能因为一次工具调用失败就让 Task Board 永久停在 review/blocked。
