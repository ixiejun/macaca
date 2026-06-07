## Context

`GoalEvaluator` 当前定义在 `macaca-task/src/plan_loop.rs`，同时承担三件事：

- 构造 goal completion evaluation prompt。
- 直接通过 `Arc<dyn macaca_llm::LlmProvider>` 调用模型。
- 解析模型返回的 JSON 为 `GoalEvaluation`。

运行时调用点在 `macaca-web/src/loop_manager.rs` 的 `PlanEvent::EvaluateGoalCompletion` 分支。这个分支已经位于 web/app runtime 层，拥有 `AppState`、planner agent name、session_id、executor、run_trace 和 EventLog 上下文，因此它是接入 traced framework agent/model 的合适位置。

依赖方向约束：

- `macaca-task` 是 task/plan 基础 crate，不应依赖 `macaca-web`。
- `macaca-task` 不应为了这次重构直接依赖 app runtime 或 web-only framework runner。
- framework agent 构建、trace hook、tool middleware、agent activity 和 EventLog 桥接目前集中在 `macaca-web`。

## Goals / Non-Goals

**Goals**

- goal evaluation 的模型执行进入 framework agent/model 路径。
- goal evaluation 对用户可见：planner/evaluator 的 thinking/tool/model response trace 可实时推送并可历史恢复。
- 保持 evaluation prompt、JSON parse、fallback、follow-up planning、goal complete、coordinator resume 的外部行为不变。
- 保持 `macaca-task` 与 web/app runtime 的依赖边界清晰。
- 为后续 `refactor-llm-provider-model-routing` 统一模型路由预留接入点。

**Non-Goals**

- 不改变 PlanLoop 什么时候发出 `EvaluateGoalCompletion`。
- 不改变 goal evaluation 失败时“默认 satisfied”的保守策略。
- 不新增新的 planner/evaluator task 到 TodoBoard。
- 不把 goal evaluation 变成 fullstack-autodev 专属流程。
- 不一次性把 PlanLoop/WorkerLoop 调度本身下沉到 framework pipeline。

## Decisions

### Decision 1: `macaca-task` 保留纯 prompt/parse 能力

`macaca-task` 应继续拥有：

- `GoalEvaluation` enum。
- goal evaluation prompt 构造逻辑。
- `parse_eval_response` 及现有 JSON/code-fence 容错测试。

但运行时的模型调用不应继续由 `GoalEvaluator` 直接持有 `Arc<dyn LlmProvider>` 完成。实现时可选择：

- 将 `GoalEvaluator` 改成纯 helper，例如 `build_prompt(...)` + `parse_eval_response(...)`。
- 或新增纯结构 `GoalEvaluationPrompt` / `GoalEvaluationParser`，并把 direct LLM `GoalEvaluator::evaluate` 标注 deprecated 后移除调用点。

关键约束是：runtime 不再调用 direct `LlmProvider::chat`。

### Decision 2: framework 执行放在 `macaca-web` planner helper 层

`loop_manager.rs` 的 `EvaluateGoalCompletion` 分支应调用一个 web-side helper，例如：

- `run_goal_evaluation_framework_call(...)`
- 或扩展已有 `run_planner_framework_call(...)` 支持 `PlannerFrameworkCallKind::GoalEvaluation` 并返回 reply text。

该 helper 负责：

- 设置 planner/evaluator agent activity 为 `Working`。
- 复用现有 traced framework agent 构建入口。
- 调用 framework agent/model 执行 prompt。
- 发出与 planner framework call 一致的 ExecutorEvent lifecycle / trace / EventLog。
- 返回文本 reply 给 `GoalEvaluation` parser。
- 在 finally 路径恢复 agent activity 为 `Idle`。

### Decision 3: 复用现有 planner agent 选择，不硬编码 application agent

Goal evaluation 应使用当前 app 的 planner/evaluator 能力选择结果。实现上优先复用现有 `plan_agent_name_for_loop` 或同一套 capability-driven planner 选择逻辑。

不得新增类似 `if app == fullstack-autodev { agent = "planner" }` 的专门化分支。

如果未来 app 显式配置了 goal-evaluator capability，可以在后续提案中升级选择策略；本提案只要求不破坏当前 planner 路径。

### Decision 4: 保持 fallback 语义在 evaluation 层

当前行为包含两类 fallback：

- 模型调用失败：`loop_manager.rs` 记录 `PLAN_GOAL_EVAL_FALLBACK` 并默认完成 goal。
- JSON 解析失败：`GoalEvaluator::parse_eval_response` 返回 `Satisfied { summary: "Evaluation completed (parsing fallback)" }`。

迁移后仍必须保持：

- framework agent/model 调用失败时，外层仍走现有 fallback branch。
- reply 解析失败时，仍走 parser fallback，不阻塞目标完成。

### Decision 5: Trace 不改变前端协议，但必须走现有持久化链路

Goal evaluation 使用 traced framework agent 后，应通过现有事件桥接进入：

- live SSE。
- EventLog。
- session detail/history restore。
- planner/evaluator tab trace。

本提案不要求新增前端 event type；如果已有 traced framework path 会发 `delegated_*` 或 planner trace 事件，应复用现有协议。

## Risks / Trade-offs

- **风险：crate 依赖方向被破坏。**
  - Mitigation: 不让 `macaca-task` 依赖 `macaca-web`；只把模型执行移到 web helper。

- **风险：framework agent reply 不再严格输出 JSON。**
  - Mitigation: prompt 保持 JSON-only 契约；parser 保持 code fence 容错；解析失败仍 fallback satisfied。

- **风险：goal evaluation trace 重复或缺失。**
  - Mitigation: 复用已有 traced planner helper，不新增平行 EventLog 写入；E2E 验证 live SSE 与刷新恢复。

- **风险：引入行为变化导致 coordinator 无法 resume。**
  - Mitigation: 不改变 `GoalCompleted` event、`complete_goal`、follow-up planning 和 resume mapping。

## Migration Plan

1. 给现有 `GoalEvaluator` 增加/保留 prompt builder 与 parser 测试，建立 baseline。
2. 在 web planner helper 层新增 goal evaluation framework call，并返回 reply text。
3. 将 `EvaluateGoalCompletion` 分支从 direct `GoalEvaluator::new(...).evaluate(...)` 改为：
   - build prompt
   - run framework agent/model
   - parse reply text
   - 复用现有 satisfied / needs-more-work / fallback branches
4. 标注 direct LLM evaluation 入口为 deprecated 或删除运行时调用。
5. 运行 `cargo check -p macaca-task`、`cargo check -p macaca-web` 和相关 unit tests。
6. E2E 验证 goal 完成后 coordinator resume，并确认 planner/evaluator trace live + history restore 都可见。

## Open Questions

- 是否需要在 agent manifest 中引入独立 `goal_evaluation` capability，还是继续复用 planner capability？本提案默认复用当前 planner 选择，避免扩大范围。
- 是否需要把 evaluation prompt builder 下沉到 `macaca-framework` 的 PlanNotebook/plan primitive？本提案暂不做，保持最小迁移。
