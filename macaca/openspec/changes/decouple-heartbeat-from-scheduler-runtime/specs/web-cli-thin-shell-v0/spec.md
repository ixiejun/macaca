## ADDED Requirements

### Requirement: Schedule Management UI Shall Not Expose Heartbeat Native Cadence

Frontend and Web schedule-management surfaces SHALL manage Scheduler jobs only.
They SHALL NOT expose Heartbeat native cadence as a normal application-facing
Scheduler target kind.

#### Scenario: Operator creates an application schedule

- **GIVEN** the operator opens the application schedule-management UI
- **WHEN** the operator creates or edits a Scheduler job
- **THEN** the UI presents Scheduler-owned target kinds only
- **AND** Heartbeat native cadence is not offered as a normal Scheduler target
- **AND** the UI sends provider-neutral Scheduler commands through `/api/apps/{app_id}/autonomy/*`

### Requirement: Heartbeat Diagnostics Shall Use Heartbeat Routes

The system SHALL require any future shell or frontend surface for heartbeat
profiles, heartbeat runs, or heartbeat diagnostics to call Heartbeat-focused
service adapters and remain separate from Scheduler job CRUD semantics.

#### Scenario: Operator inspects heartbeat diagnostics

- **GIVEN** a future heartbeat diagnostics surface exists
- **WHEN** the operator inspects agent heartbeat state
- **THEN** the shell calls Heartbeat-focused service routes or facade clients
- **AND** the shell does not construct timers, providers, heartbeat profiles, or heartbeat loops
