## ADDED Requirements

### Requirement: Review Result Event Broadcasting

When a task review completes (via ReviewTodoTool), the system SHALL broadcast the review result as both an SSE event and an EventLog entry. The event SHALL include task_id, agent, pass/fail status, feedback, and new task status. A review delegate response alone MUST NOT be treated as completion; the broadcaster SHALL first verify that Task Service persistence moved the task out of `PendingReview`.

#### Scenario: Passed review emits event
- **GIVEN** planner reviews task T1 for agent "backend" with passed=true
- **WHEN** review_task() succeeds
- **THEN** an SSE event with type "plan_decision" and decision_type "task_reviewed" is broadcast
- **AND** an EventLog entry with event_type "task_reviewed" is persisted
- **AND** the payload includes task_id, agent, passed=true, feedback, new_status="Completed"

#### Scenario: Failed review emits event
- **GIVEN** planner reviews task T1 with passed=false
- **WHEN** review_task() succeeds
- **THEN** an SSE event with decision_type "task_reviewed" is broadcast
- **AND** the payload includes passed=false, new_status="NeedsOptimization" or "Failed"

#### Scenario: Delegate response without persisted review does not emit completed event
- **GIVEN** task T1 is still in `PendingReview`
- **WHEN** a delegated review agent returns natural-language completion without a successful `review_todo` persistence update
- **THEN** no `task_reviewed` event is broadcast
- **AND** the system records an auditable anomaly explaining that the review delegate did not persist a Task Board decision

#### Scenario: Review event visible in agent tab after refresh
- **GIVEN** a review was completed and persisted to EventLog
- **WHEN** the frontend reloads session data
- **THEN** the review event appears in the planner agent's trace tab
