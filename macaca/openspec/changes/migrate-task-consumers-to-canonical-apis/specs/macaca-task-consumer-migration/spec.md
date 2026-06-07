## ADDED Requirements

### Requirement: Upper-Layer Task Consumers Must Use Canonical APIs

Upper-layer crates that consume `macaca-task` SHALL use the canonical post-refactor task APIs instead of continuing to call deprecated compatibility wrappers.

#### Scenario: Worker-facing task tools use canonical board APIs

- **GIVEN** worker-oriented task tools in upper-layer crates
- **WHEN** they claim, start, or submit tasks
- **THEN** they SHALL use canonical `TaskBoard` APIs such as `for_agent`, `claim_next_task`, `mark_task_in_progress`, and `submit_task_for_review`
- **AND** they SHALL NOT call deprecated wrappers such as `TaskBoard::new`, `claim_next`, `start_task`, or `submit_for_review`

#### Scenario: Planner-facing task consumers use canonical task space APIs

- **GIVEN** planner or scheduler oriented consumers in upper-layer crates
- **WHEN** they create tasks, review tasks, or construct task spaces
- **THEN** they SHALL use canonical `TaskSpace` APIs such as `for_session`, `create_task_assignment`, and `apply_review_result`
- **AND** they SHALL NOT call deprecated wrappers such as `TaskSpace::new`, `create_and_assign`, or `review_task`

### Requirement: Deprecated Task API Regression Guard

The system SHALL include an automated regression guard that detects reintroduction of deprecated `macaca-task` API usage in upper-layer consumer files.

#### Scenario: Deprecated call is reintroduced in an upper-layer consumer

- **GIVEN** an upper-layer consumer file under `macaca-tools`, `macaca-web`, or `macaca-integration-tests`
- **WHEN** a deprecated `macaca-task` API call is reintroduced
- **THEN** the regression guard SHALL fail
- **AND** it SHALL identify the offending pattern and the canonical replacement expectation

#### Scenario: Deprecated wrappers remain only inside macaca-task

- **GIVEN** deprecated compatibility wrappers are still intentionally retained in `macaca-task`
- **WHEN** the upper-layer consumer audit runs
- **THEN** the guard SHALL allow deprecated wrappers to remain inside `macaca-task`
- **AND** it SHALL only enforce canonical API usage for upper-layer consumers
