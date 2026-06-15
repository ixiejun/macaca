## ADDED Requirements

### Requirement: Execution Control SHALL Own Loop And Resume Semantics

Execution-control and task services SHALL own pause, resume, checkpoint identity, loop wakeup, worker-loop lifecycle, scheduler handles, and replayable resume diagnostics. Presentation shells SHALL only request commands, subscribe to events, and render state.

#### Scenario: Shell requests resume through service
- **WHEN** a user or application requests resume for a paused execution
- **THEN** the shell SHALL send a typed execution-control command through the facade/service path
- **AND** duplicate, stale, denied, or unavailable resume attempts SHALL be reported as structured service outcomes

#### Scenario: Local waker is not semantic owner
- **WHEN** a session loop needs wakeup or shutdown
- **THEN** execution-control/task service state SHALL determine semantic outcome
- **AND** shell-local channels or wakers SHALL only be transport/subscription implementation details, not authoritative state

### Requirement: Execution Control Events SHALL Be Replayable After Shell Restart

Execution-control state transitions SHALL be recorded as sanitized trace/audit mementos so recovery does not depend on in-memory shell state.

#### Scenario: Shell restarts during paused execution
- **WHEN** the shell process restarts during paused or waiting execution
- **THEN** session recovery SHALL reconstruct execution-control state from service snapshots/events
- **AND** it SHALL NOT require old in-memory shell loop handles
