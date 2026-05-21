## ADDED Requirements

### Requirement: Runtime Host Heartbeat Agent Dispatch Strategy
Runtime Host SHALL own the generic strategy that maps accepted heartbeat wakes
to manifest-declared agent execution commands.

#### Scenario: Accepted native heartbeat dispatches declared agents
- **WHEN** HeartbeatLane processes an accepted native heartbeat wake
- **AND** Application Service returns enabled heartbeat-agent declarations
- **THEN** Runtime Host SHALL dispatch one provider-neutral Agent Execution
  command per enabled declaration
- **AND** the command SHALL carry heartbeat execution intent, application scope,
  target agent, trace correlation, and bounded metadata.

#### Scenario: No declarations
- **WHEN** HeartbeatLane processes an accepted native heartbeat wake
- **AND** Application Service returns no enabled declarations
- **THEN** Runtime Host SHALL record a structured skip
- **AND** the Heartbeat lane SHALL remain healthy.

#### Scenario: Scheduler is not required
- **WHEN** HeartbeatLane dispatches manifest-declared heartbeat agents
- **THEN** it SHALL NOT require Scheduler due-run materialization or Scheduler
  leases.
