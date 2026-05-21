## ADDED Requirements

### Requirement: Dual Autonomy Supervisor Lanes

The local autonomy runtime SHALL use runtime-host as the approved composition
root for sibling Scheduler and Heartbeat supervisor lanes. SchedulerLane SHALL
own Scheduler due-run materialization and scheduled target dispatch.
HeartbeatLane SHALL own native Heartbeat cadence, profile evaluation,
coalescing, gates, and heartbeat action dispatch.

#### Scenario: Scheduler lane ticks independently

- **GIVEN** local autonomy is enabled and SchedulerLane is healthy
- **WHEN** SchedulerLane ticks
- **THEN** it calls Scheduler Service to materialize or lease due scheduled runs
- **AND** it does not evaluate Heartbeat native cadence

#### Scenario: Heartbeat lane ticks independently

- **GIVEN** local autonomy is enabled and HeartbeatLane is healthy
- **WHEN** HeartbeatLane ticks
- **THEN** it calls Heartbeat Service to evaluate native heartbeat profiles
- **AND** it does not require Scheduler due-run materialization

### Requirement: Independent Lane Degradation

The local autonomy runtime SHALL report structured diagnostics when Scheduler
or Heartbeat lanes are unavailable, degraded, gated, skipped, or failed, and it
SHALL allow the other lane to continue when policy and health permit.

#### Scenario: Scheduler provider is unavailable

- **GIVEN** Scheduler Service returns structured unavailable
- **WHEN** HeartbeatLane has due heartbeat profiles
- **THEN** HeartbeatLane continues through Heartbeat Service when policy permits
- **AND** the supervisor records sanitized lane-degradation trace and audit evidence

#### Scenario: Heartbeat provider is unavailable

- **GIVEN** Heartbeat Service returns structured unavailable
- **WHEN** SchedulerLane has due Scheduler runs
- **THEN** SchedulerLane continues through Scheduler Service when policy permits
- **AND** the supervisor records sanitized lane-degradation trace and audit evidence

## MODIFIED Requirements

### Requirement: Heartbeat Recovery and Scheduled Wake Lane

The local autonomy runtime SHALL integrate Heartbeat as a system wake,
recovery, and native cadence mechanism through HeartbeatLane and
`service.heartbeat`. Recovery wakes and manual/event wakes remain typed wake
intents. Recurring heartbeat cadence SHALL be evaluated by Heartbeat native
profiles rather than by application-facing Scheduler jobs.

#### Scenario: Recovery wake is emitted

- **GIVEN** local autonomy is enabled and recovery wakes are configured
- **WHEN** runtime-host starts after downtime
- **THEN** HeartbeatLane emits a Heartbeat `Recovery` wake intent
- **AND** Heartbeat returns accepted, coalesced, gated, delayed, skipped, or failed result evidence without shell-owned wake logic

#### Scenario: Native heartbeat tick is emitted

- **GIVEN** local autonomy is enabled with Heartbeat native cadence configuration
- **WHEN** the heartbeat profile becomes due
- **THEN** HeartbeatLane sends a native cadence tick to Heartbeat Service
- **AND** Heartbeat evaluates coalescing and gates before any generic target dispatch occurs
