## ADDED Requirements

### Requirement: Goal evaluation SHALL execute through a framework agent/model path

The system SHALL execute runtime goal completion evaluation through the traced framework agent/model path instead of directly calling a raw `LlmProvider`.

#### Scenario: Goal completion evaluation uses framework execution

- **GIVEN** PlanLoop emits `PlanEvent::EvaluateGoalCompletion`
- **WHEN** the web PlanEvent consumer evaluates the goal
- **THEN** it SHALL build and execute the evaluation prompt through a framework agent/model helper
- **AND** it SHALL NOT call `LlmProvider::chat` directly from the goal evaluation runtime path
- **AND** it SHALL preserve the same input data: goal description, task summaries, completed count, and failed count

#### Scenario: Goal evaluation stays visible in trace

- **GIVEN** goal completion evaluation is running
- **WHEN** the framework agent/model path executes the evaluation prompt
- **THEN** the selected planner/evaluator agent SHALL be visible as `Working`
- **AND** trace events SHALL flow through the existing live SSE path
- **AND** the same events SHALL be persisted so browser refresh/session reload can reconstruct the evaluation trace

### Requirement: Goal evaluation result semantics SHALL remain compatible

The system SHALL preserve the existing `GoalEvaluation` semantics while changing only the model execution path.

#### Scenario: Satisfied response remains satisfied

- **GIVEN** the framework model response contains valid JSON with `satisfied: true`
- **WHEN** the response is parsed
- **THEN** the result SHALL be `GoalEvaluation::Satisfied`
- **AND** the satisfied branch SHALL still complete the goal
- **AND** the satisfied branch SHALL still emit `GoalCompleted`, `goal_satisfied` plan decision, run_trace, EventLog, and coordinator resume behavior as before

#### Scenario: Needs-more-work response remains needs-more-work

- **GIVEN** the framework model response contains valid JSON with `satisfied: false`
- **WHEN** the response is parsed
- **THEN** the result SHALL be `GoalEvaluation::NeedsMoreWork`
- **AND** the needs-more-work branch SHALL still trigger follow-up planning with the same prompt semantics
- **AND** the needs-more-work branch SHALL still emit `goal_needs_work` plan decision, run_trace, and EventLog behavior as before

#### Scenario: Parse failure remains conservative

- **GIVEN** the framework model response cannot be parsed as the expected evaluation JSON
- **WHEN** the response parser handles the content
- **THEN** it SHALL return the same conservative satisfied fallback used by the current parser
- **AND** goal completion SHALL NOT be blocked by parser failure

#### Scenario: Framework call failure remains conservative

- **GIVEN** framework agent/model execution fails before a parseable reply is available
- **WHEN** the PlanEvent consumer handles the error
- **THEN** it SHALL preserve the existing `PLAN_GOAL_EVAL_FALLBACK` behavior
- **AND** it SHALL default to completing the goal as before

### Requirement: Goal evaluation migration SHALL preserve crate boundaries

The system SHALL keep task-domain goal evaluation types independent from web/app runtime execution details.

#### Scenario: macaca-task remains runtime-independent

- **GIVEN** goal evaluation prompt construction and response parsing are used by the runtime
- **WHEN** goal evaluation is migrated to framework execution
- **THEN** `macaca-task` SHALL NOT depend on `macaca-web`
- **AND** `macaca-task` SHALL NOT depend on application-specific web runtime state
- **AND** framework agent construction SHALL remain in `macaca-web` or another appropriate runtime integration layer

#### Scenario: No application-specific evaluator branch is introduced

- **GIVEN** an application uses a planner/evaluator agent selected by existing capability-driven rules
- **WHEN** goal evaluation is executed
- **THEN** the system SHALL reuse that selection path
- **AND** it SHALL NOT add fullstack-autodev-specific goal evaluation logic
- **AND** it SHALL NOT hardcode a new application-specific agent mapping
