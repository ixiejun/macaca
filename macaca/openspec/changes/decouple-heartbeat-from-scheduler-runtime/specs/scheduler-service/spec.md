## ADDED Requirements

### Requirement: Scheduler Does Not Own Heartbeat Cadence

The Scheduler Service SHALL own scheduled job definitions, schedule
calculation, due-run materialization, leases, and scheduled run history. It
SHALL NOT be required for Heartbeat native cadence, heartbeat profile
evaluation, heartbeat coalescing, heartbeat gates, or heartbeat run lifecycle.

#### Scenario: Scheduler materializes a scheduled service command

- **GIVEN** an active Scheduler job targets a generic service command
- **WHEN** the job becomes due
- **THEN** Scheduler materializes and leases a scheduled run
- **AND** Scheduler does not evaluate or advance Heartbeat native cadence

#### Scenario: Heartbeat cadence continues without Scheduler jobs

- **GIVEN** no Scheduler jobs are registered for an application or agent
- **WHEN** a Heartbeat profile becomes due
- **THEN** Heartbeat native cadence can still be evaluated through Heartbeat Service
- **AND** Scheduler does not need to materialize a run for the heartbeat tick
