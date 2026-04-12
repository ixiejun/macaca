# Macaca Framework 渐进式最小重构候选清单

## 目的

这份文档记录当前项目中可以继续使用 `macaca-framework` 做渐进式重构的候选点。

核心约束是：每次只做一个小点，功能 1:1 还原，不改变现有端到端行为，不因为一次重构引入一串新 bug。

这是一份探索快照，不是已经批准的 OpenSpec 提案。下一步应先从本文档中挑选一个候选点，然后只针对这个单点编写精细 OpenSpec 提案，再进入实现。

## 重构原则

- 每一步都要足够小，能独立 review、独立验证、独立回滚。
- 默认保持行为 1:1；除非后续 OpenSpec 明确批准行为变化，否则不改语义。
- 优先抽取重复 glue code，再考虑改变编排语义。
- OS/framework 层不引入 fullstack-autodev 专属逻辑。
- 低风险清理阶段不迁移 PlanLoop/WorkerLoop 的调度语义。
- 除非选中的小点专门处理 trace/session，否则不改变 EventLog、SSE、浏览器刷新恢复行为。

## 当前上下文

当前 `/api/chat/v2` 主链路已经通过 traced framework 入口构建 agent：

- Coordinator：`FrameworkRunner::build_coordinator`
- Planner/worker traced task：`FrameworkRunner::build_traced_agent_with_goal` 和 `FrameworkRunner::build_worker_agent`
- Kernel executor 兼容桥：`WebAgentRunner::build_runtime_agent`

当前剩余的重构机会主要集中在 web 层重复胶水代码：

- trace hook 和 tool middleware 的事件转换逻辑。
- executor 生命周期事件构造逻辑。
- worker 正常执行和 retry 的重复执行模板。
- planner decomposition/review/follow-up 的重复调用模板。
- PlanNotebook 持久化 helper。
- tool policy / build toolkit 的组织拆分。

## 推荐的小步候选点

| 优先级 | 候选点 | 范围 | 为什么足够小 | 行为约束 |
|---|---|---|---|---|
| 1 | 统一 trace middleware helper | `macaca-web/src/framework_runner.rs` | `SseToolMiddleware`、`ChannelToolMiddleware`、`ExecutorToolMiddleware` 都重复了 tool output 提取、截断和事件 payload 构造。第一步只抽私有 helper。 | SSE event 名称、EventLog event 名称、payload 字段、截断长度、UTF-8 安全截断输出都保持不变。 |
| 2 | 抽取 ExecutorEvent 生命周期 helper | `macaca-web/src/loop_manager.rs` | `TaskStarted`、`TaskCompleted`、`TaskFailed`、`TaskResult` 构造在 planner decomposition/review/follow-up、worker task/retry 中重复出现。 | 只替换重复构造，不改变事件发射时机。 |
| 3 | 抽取 worker 执行结果模板 | `macaca-web/src/loop_manager.rs` | `TaskClaimed` 和 `RetryTask` 都是 `build_worker_agent -> reply -> submit_for_review/mark_failed`。 | 超时、panic 处理、TaskBoard 状态流转、run_trace phase、waker 行为、错误文案都必须完全保持。 |
| 4 | 抽取 planner framework 调用 helper | `macaca-web/src/loop_manager.rs` | goal decomposition、review、follow-up planning 都是 `Working -> build traced agent -> reply -> TaskCompleted/TaskFailed -> Idle`。 | prompt、planner 选择、task/goal id、事件时机、trace phase 都保持不变。 |
| 5 | 抽取 PlanNotebook 持久化 helper | `macaca-web/src/loop_manager.rs`、`macaca-framework/src/plan.rs` | decomposition/review 的 notebook 写入逻辑小而孤立。第一步可以只抽 web-side helper，不立刻下沉到 framework。 | `PlanNotebook` 仍然是 agent-local “脑内计划本”；`TodoBoard` 仍然是 durable task source of truth。 |
| 6 | 拆分 tool policy / build toolkit 代码 | `macaca-web/src/framework_runner.rs` | `AgentToolPolicy`、`TodoToolPolicy`、`register_agent_tools` 已经 capability-driven，但让 `framework_runner.rs` 过大。第一步只移动到 web 内部模块。 | 工具可见性、capability fallback、disallowed assignee、workspace tool 行为全部不变。 |

## 推荐第一步

建议先做候选点 1：统一 trace middleware helper。

原因：

- 范围局限在 `framework_runner.rs`。
- 直接作用于 framework 的 Hook/ToolMiddleware 集成点。
- 只减少重复 trace 逻辑，不改变 orchestration 或 task state。
- 能把最近修复的 UTF-8 安全截断逻辑收敛到单一 helper，避免同类问题在多套 middleware 中再次出现。

对应 OpenSpec 的建议微范围：

- 增加私有 helper，用于从 `ToolResponse` 中提取文本。
- 复用现有 UTF-8 安全的 `truncate_tool_output`。
- 如果不会扩大 diff，再增加 helper 构造 `AgentExecutionEvent::ToolCall` 和 `AgentExecutionEvent::ToolResult`。
- 保持 `SseToolMiddleware`、`ChannelToolMiddleware`、`ExecutorToolMiddleware` 的外部行为完全不变。
- 保留 UTF-8 截断测试；如可控，补一个多 content block 的 tool response 文本提取测试。

## 不建议作为第一步的大项

| 候选点 | 不建议先做的原因 |
|---|---|
| 把 `GoalEvaluator` 从 direct LLM call 改成 framework agent/model 执行 | 会跨 `macaca-task`、`macaca-web`、`macaca-framework`，还可能涉及 crate 依赖方向，应该单独提案。 |
| 用 `SequentialPipeline` 或 `FanoutPipeline` 替换 PlanLoop/WorkerLoop | 会改变编排语义，容易影响 dependency gating、review、resume。 |
| 把 session/trace/resume 全部迁成 framework-level primitive | 方向正确，但范围过大；应先缩小 web glue 和事件 helper 重复。 |
| 把 event source 从硬编码 `"coordinator"` 改成动态 entry agent | 后续有价值，但会影响历史 session restore 假设，建议等 trace helper 统一后再做。 |

## 下一步 OpenSpec 流程

用户选定一个候选点后：

1. 只为该候选点创建一个聚焦的 OpenSpec change。
2. 明确写成非行为变更、行为保持型重构。
3. 如果候选点涉及 trace/session，必须在 scenario 里写清楚 live SSE、EventLog 持久化、浏览器刷新历史恢复都保持不变。
4. implementation tasks 要足够细，每个 task 对应一个小代码改动和一个验证命令。
