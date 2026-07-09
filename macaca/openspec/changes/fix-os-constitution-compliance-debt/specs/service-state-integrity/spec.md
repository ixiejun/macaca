## ADDED Requirements

### Requirement: Lifecycle Transitions Follow A Declarative Legal-Transition Matrix

Task, Scheduler, Heartbeat, and Autonomy lifecycle transitions SHALL be validated
against a declarative legal-transition specification. A transition whose current
state does not legally reach the target state SHALL return a structured conflict
and perform no mutation. Terminal states SHALL NOT be re-entered or re-completed.

#### Scenario: Illegal transition is refused
- **WHEN** a transition is requested from a terminal state (e.g. succeeded,
  cancelled, skipped) to a non-terminal or different terminal state
- **THEN** the service SHALL return a structured conflict and leave state unchanged

#### Scenario: In-flight run is not overwritten
- **WHEN** a wake/coalesce request targets a run that is currently running
- **THEN** the service SHALL NOT overwrite it to a coalesced state and SHALL
  return a busy/conflict result instead of reporting acceptance

### Requirement: Retry Is Idempotency-Aware

The unified service-call retry path SHALL consult an idempotency indicator from
the service descriptor before retrying, and SHALL NOT automatically retry
non-idempotent operations on transient failure or timeout. Scheduled retries
SHALL respect their backoff time and SHALL NOT be dispatched before their
scheduled instant.

#### Scenario: Non-idempotent operation is not blindly retried
- **WHEN** a non-idempotent operation (e.g. payment settlement or contract
  deployment) fails transiently or times out
- **THEN** the router SHALL NOT re-execute it without an idempotency key and SHALL
  surface a structured failure

#### Scenario: Backoff is honored
- **WHEN** a retry run is scheduled for a future instant
- **THEN** the scheduler SHALL NOT lease or dispatch it until that instant has
  passed

### Requirement: Concurrent Claim And Ordering Are Race-Free

Task/run claiming SHALL use a compare-and-set or lease so two concurrent claimants
cannot both succeed on the same item. Ordered identifiers used for FIFO,
recent-N, and lease selection SHALL sort in creation order regardless of count.

#### Scenario: Duplicate claim is prevented
- **WHEN** two workers concurrently claim the same pending item
- **THEN** at most one SHALL succeed and the other SHALL observe a structured
  already-claimed result

#### Scenario: Identifier ordering is stable past ten items
- **WHEN** more than ten runs/jobs exist
- **THEN** ordering operations SHALL still reflect creation order (e.g. via
  zero-padded or numeric keys)

### Requirement: Crash Recovery Restores All Non-Terminal Work

Crash recovery SHALL roll back not only in-progress leaf items but also
non-terminal aggregate states (e.g. goals in a decomposing/evaluating state with
no active children) so no unit of work is permanently stranded.

#### Scenario: Stranded aggregate is recovered
- **WHEN** the process restarts while a goal is in a non-terminal decomposing or
  evaluating state with no active child tasks
- **THEN** recovery SHALL return it to a resumable pending state
