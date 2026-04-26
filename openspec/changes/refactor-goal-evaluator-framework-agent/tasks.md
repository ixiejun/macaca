## 1. Baseline and Boundaries

- [x] 1.1 Confirm current `GoalEvaluator` prompt / parser tests pass.
- [x] 1.2 Confirm `EvaluateGoalCompletion` direct `LlmProvider::chat` runtime call site in `loop_manager.rs`.
- [x] 1.3 Confirm no `macaca-task -> macaca-web` or `macaca-task -> macaca-framework` dependency is introduced.

## 2. Pure Goal Evaluation Prompt / Parser

- [x] 2.1 Extract or expose a pure goal evaluation prompt builder from `GoalEvaluator`.
- [x] 2.2 Keep `GoalEvaluation` and JSON response parser in `macaca-task`.
- [x] 2.3 Preserve parser behavior for plain JSON, fenced JSON, fenced plain JSON, invalid JSON, and empty response.
- [x] 2.4 Mark the direct LLM evaluation entry as deprecated or remove its runtime usage without changing public result semantics.

## 3. Framework Agent/Model Execution

- [x] 3.1 Add a web-side goal evaluation framework helper, preferably reusing existing planner framework call infrastructure.
- [x] 3.2 Ensure the helper uses the app's existing planner/evaluator selection and does not hardcode application names or agent names beyond current capability-driven defaults.
- [x] 3.3 Ensure the helper emits the same class of agent activity, ExecutorEvent lifecycle, trace, and EventLog events as existing traced planner calls.
- [x] 3.4 Ensure the helper returns reply text for parsing and surfaces framework call failures to the existing fallback branch.

## 4. Runtime Integration

- [x] 4.1 Replace `GoalEvaluator::new(state.llm, default_model).evaluate(...)` in `PlanEvent::EvaluateGoalCompletion` with framework helper execution plus parser.
- [x] 4.2 Preserve `GoalEvaluation::Satisfied` branch behavior: `complete_goal`, `GoalCompleted`, `goal_satisfied` plan decision, run_trace, EventLog, coordinator resume.
- [x] 4.3 Preserve `GoalEvaluation::NeedsMoreWork` branch behavior: follow-up planning prompt, `goal_needs_work` plan decision, run_trace, EventLog.
- [x] 4.4 Preserve call failure fallback behavior: `PLAN_GOAL_EVAL_FALLBACK` and default goal completion.
- [x] 4.5 Preserve parse failure fallback behavior: satisfied result with existing fallback summary.

## 5. Verification

- [x] 5.1 Run `cargo test -p macaca-task goal_evaluation`.
- [x] 5.2 Run focused `macaca-web` tests for planner/framework helper behavior if available or add minimal unit coverage for the new helper's pure pieces.
- [x] 5.3 Run `cargo check -p macaca-task`.
- [x] 5.4 Run `cargo check -p macaca-web`.
- [x] 5.5 E2E smoke test: create a goal, let all tasks complete and review, verify framework goal evaluation runs and coordinator resumes.
- [x] 5.6 Verify live trace: planner/evaluator tab receives goal evaluation trace without browser refresh.
- [x] 5.7 Verify history restore: refresh/re-enter session and confirm goal evaluation trace and final coordinator response remain visible.
