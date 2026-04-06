## ADDED Requirements

### Requirement: Immediate PlanLoop Wakeup After Review Submission

When a worker agent submits a task for review (status changes to PendingReview), the system SHALL immediately wake the PlanLoop via PlanLoopWaker instead of waiting for the next heartbeat cycle.

#### Scenario: PlanLoop woken after submit_for_review
- **GIVEN** worker agent completes task T1 and submits for review
- **WHEN** submit_for_review changes status to PendingReview
- **THEN** PlanLoopWaker::wake() is called
- **AND** PlanLoop detects PendingReview within milliseconds, not 5 seconds

### Requirement: Immediate WorkerLoop Wakeup After Review Completion

When a review completes and unblocks dependent tasks (status changes from Blocked to Pending), the system SHALL immediately wake the relevant WorkerLoops via WorkerLoopWaker.

#### Scenario: WorkerLoop woken after review passes
- **GIVEN** task T1 is reviewed and passes (Completed)
- **AND** task T2 depends on T1 and transitions from Blocked to Pending
- **WHEN** unblock_dependents runs
- **THEN** WorkerLoopWaker::wake() is called for the agent owning T2
- **AND** WorkerLoop claims T2 within milliseconds

#### Scenario: WorkerLoop woken for next sequential task
- **GIVEN** agent "backend" has tasks seq 1 (Completed) and seq 2 (Pending)
- **WHEN** task seq 1's review passes
- **THEN** WorkerLoop for "backend" is woken
- **AND** seq 2 is claimed immediately
