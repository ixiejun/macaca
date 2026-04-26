# Change: 将 GoalEvaluator 迁移到 framework agent/model 执行

## Why

当前 goal completion evaluation 仍在 `loop_manager.rs` 里通过 `macaca_task::GoalEvaluator::new(state.llm, default_model).evaluate(...)` 直接调用底层 `LlmProvider`。这条路径绕过了已经迁移完成的 traced framework agent/model 执行入口，也绕过了统一的 agent activity、ExecutorEvent、trace/EventLog 和后续 provider/model routing 扩展点。

根据 `macaca-framework-incremental-refactor-candidates.md`，这是一个跨 `macaca-task`、`macaca-web`、`macaca-framework` 的独立重构点，应单独提案，保持行为 1:1，再逐步实现。

## What Changes

- 将 `GoalEvaluator` 的运行时模型调用从 direct `LlmProvider::chat` 改为 framework agent/model 执行。
- 保留现有 goal evaluation 的外部语义：
  - 输入仍为 goal description、task summaries、completed count、failed count。
  - prompt 的 JSON 输出契约保持兼容：`satisfied`、`summary`、`suggestions`。
  - 解析成功时仍返回 `GoalEvaluation::Satisfied` 或 `GoalEvaluation::NeedsMoreWork`。
  - LLM 调用失败或 JSON 解析失败仍按当前保守策略 fallback 为 satisfied，不阻塞 goal completion。
- 保持 `macaca-task` 不反向依赖 `macaca-web` 或 app runtime：
  - `macaca-task` 可保留 prompt builder / response parser / `GoalEvaluation` 类型。
  - 实际 framework agent/model 执行放在 `macaca-web` 的 PlanEvent consumer / planner helper 层。
- goal evaluation 执行应复用现有 traced framework 入口和 planner agent 选择结果，不新增 application 专属逻辑、不硬编码 fullstack-autodev。
- goal evaluation 的 trace 与 activity 应对用户可见，并进入 EventLog / refresh history 恢复链路。

## Non-Goals

- 不改变 PlanLoop / WorkerLoop 调度语义。
- 不改变 TodoBoard / TodoStore 的 task 状态流转。
- 不改变 `GoalEvaluation` 结果语义、fallback 语义、follow-up planning 语义或 coordinator resume 语义。
- 不引入 fullstack-autodev 专属 evaluator。
- 不在本提案中完成 LLM provider/model routing 的大重构；如果 `refactor-llm-provider-model-routing` 仍未实现，本提案只要求复用当前 framework runner 的模型入口。
- 不改变前端 SSE/EventLog 协议字段，除非现有 traced framework 入口已经发出同名事件。

## Impact

- Affected specs: `goal-evaluation-framework-execution`
- Affected code:
  - `macaca/crates/macaca-task/src/plan_loop.rs`
  - `macaca/crates/macaca-web/src/loop_manager.rs`
  - `macaca/crates/macaca-web/src/framework_runner.rs` 或已有 planner framework helper
  - 相关 tests
- GitNexus findings:
  - `GoalEvaluator` located in `macaca/crates/macaca-task/src/plan_loop.rs`.
  - `gitnexus impact GoalEvaluator --direction upstream --repo agent` returned `LOW` / `impactedCount=0`, but this is likely conservative because the runtime call site is in `loop_manager.rs` and this migration affects core goal completion behavior.
- Risk mitigation:
  - Keep `macaca-task` prompt/parse behavior covered by existing tests.
  - Add/adjust tests around framework goal evaluation helper using a mock/stub model where possible.
  - Validate with `cargo check -p macaca-task` and `cargo check -p macaca-web`.
  - Run at least one E2E smoke path: create goal → all tasks reviewed → goal evaluation → goal complete / needs more work → coordinator resume.
