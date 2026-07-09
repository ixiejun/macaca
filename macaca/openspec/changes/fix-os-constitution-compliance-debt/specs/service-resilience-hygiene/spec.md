## ADDED Requirements

### Requirement: Lock Poisoning Degrades Structurally, Not Catastrophically

OS services SHALL NOT let a poisoned synchronous lock cascade into service-wide
panics. A poisoned lock SHALL be recovered (e.g. via inner-value recovery) or
mapped to a structured failure, so one panicking critical section does not
permanently break every subsequent operation, including crash-recovery paths.

#### Scenario: Poisoned lock does not brick the service
- **WHEN** a critical section panics and poisons a shared lock
- **THEN** subsequent operations SHALL either recover the guarded value or return a
  structured failure, not panic in cascade

#### Scenario: Execution-control locks survive poisoning
- **WHEN** an execution-control/session-recovery lock is poisoned
- **THEN** pause/resume/checkpoint operations SHALL still return structured
  results rather than propagating a panic

### Requirement: In-Memory State Growth Is Bounded

The system SHALL bound long-lived in-memory collections (snapshot maps,
terminal-run history, payload stores, audit/diagnostic buffers) in 7x24 services
with an explicit retention bound (size cap, TTL, or eviction) so they do not grow
without limit. Sensitive payloads (e.g. prompts) SHALL NOT reside indefinitely
after completion.

#### Scenario: Terminal records are reclaimed
- **WHEN** runs/tasks reach a terminal state over long uptime
- **THEN** their retained in-memory records SHALL be subject to a bound or
  eviction policy rather than accumulating indefinitely

#### Scenario: Completed payloads are removed
- **WHEN** a scheduled task completes or is cancelled
- **THEN** its stored payload SHALL be removable and subject to cleanup, not
  retained forever

### Requirement: Result Channels Do Not Silently Drop Outcomes

The system SHALL log and reflect in state any send failure where a
completion/failure result is sent over a channel to update authoritative state,
rather than silently discarding it.

#### Scenario: Dropped completion is observable
- **WHEN** a worker sends a task completion result but the consumer has been
  dropped
- **THEN** the failure SHALL be logged at a key node and the task state SHALL not
  be left silently inconsistent
