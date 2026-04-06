## ADDED Requirements

### Requirement: PlanLoop ReviewNeeded Deduplication

The PlanLoop SHALL emit `ReviewNeeded` for a given task at most once. Subsequent heartbeat cycles SHALL NOT re-emit `ReviewNeeded` for the same task until its status changes away from `PendingReview` (e.g., to `Completed`, `NeedsOptimization`, or `Failed`).

#### Scenario: Same task not re-emitted
- **GIVEN** task T1 is in PendingReview status
- **WHEN** PlanLoop emits ReviewNeeded for T1 on heartbeat cycle N
- **THEN** heartbeat cycle N+1 does NOT emit ReviewNeeded for T1 again
- **AND** planner agent is NOT re-delegated for the same review

#### Scenario: Cleanup after status change
- **GIVEN** task T1 was previously emitted as ReviewNeeded and is now Completed
- **WHEN** PlanLoop runs its next heartbeat
- **THEN** T1 is removed from the deduplication set
- **AND** if T1 somehow returns to PendingReview later, it can be re-emitted

#### Scenario: Multiple tasks handled independently
- **GIVEN** tasks T1 and T2 are both PendingReview
- **WHEN** T1 is already emitted but T2 is new
- **THEN** only T2 is emitted as ReviewNeeded
