## ADDED Requirements

### Requirement: PlanLoop ReviewNeeded Deduplication And Retry

The PlanLoop SHALL suppress duplicate `ReviewNeeded` emissions for the same task during the configured retry backoff window. If the task remains `PendingReview` because no persisted `review_todo` decision was accepted, the PlanLoop SHALL re-emit `ReviewNeeded` after the backoff window until the configured retry limit is reached. When the task changes away from `PendingReview` (for example to `Completed`, `NeedsOptimization`, or `Failed`), the PlanLoop SHALL clear its dispatch state for that task.

#### Scenario: Same task not re-emitted
- **GIVEN** task T1 is in PendingReview status
- **WHEN** PlanLoop emits ReviewNeeded for T1 on heartbeat cycle N
- **AND** heartbeat cycle N+1 happens before the retry backoff expires
- **THEN** PlanLoop does NOT emit ReviewNeeded for T1 again
- **AND** planner agent is NOT immediately re-delegated for the same review

#### Scenario: Persisted review missing is retried
- **GIVEN** task T1 is still in PendingReview after a delegated review
- **AND** T1 has remaining review dispatch attempts
- **WHEN** the retry backoff expires
- **THEN** PlanLoop emits ReviewNeeded for T1 again
- **AND** the retry is traceable through structured logs

#### Scenario: Cleanup after status change
- **GIVEN** task T1 was previously emitted as ReviewNeeded and is now Completed
- **WHEN** PlanLoop runs its next heartbeat
- **THEN** T1 is removed from the review dispatch state
- **AND** if T1 somehow returns to PendingReview later, it can be re-emitted

#### Scenario: Retry limit prevents review storms
- **GIVEN** task T1 is still in PendingReview
- **AND** T1 has reached the configured review dispatch retry limit
- **WHEN** PlanLoop runs another heartbeat
- **THEN** PlanLoop does NOT emit ReviewNeeded for T1 again
- **AND** the skipped retry is visible in logs for audit and diagnosis

#### Scenario: Multiple tasks handled independently
- **GIVEN** tasks T1 and T2 are both PendingReview
- **WHEN** T1 is already emitted but T2 is new
- **THEN** only T2 is emitted as ReviewNeeded
