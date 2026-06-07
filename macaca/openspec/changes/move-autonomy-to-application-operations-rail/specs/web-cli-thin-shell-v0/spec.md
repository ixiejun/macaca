## ADDED Requirements

### Requirement: Autonomy management SHALL be an application Operations dialog

Macaca frontend SHALL expose Autonomy schedule management as an
application-scoped Operations dialog opened from an application-level action
button rather than as a session, coordinator, delegated-agent trace tab, or
persistent right-side agent/task/status panel.

#### Scenario: Operator opens application-scoped Autonomy

- **GIVEN** an operator is viewing an application workspace
- **WHEN** the operator needs to inspect or manage Autonomy schedules
- **THEN** the frontend SHALL render the schedule-management panel in an application-level Operations dialog
- **AND** the panel SHALL call `/api/apps/{app_id}/autonomy/schedules`
- **AND** the panel SHALL NOT appear as a session, coordinator, or delegated-agent trace tab
- **AND** the panel SHALL NOT replace the persistent right-side agents/task/status panel

#### Scenario: Session and agent tabs remain trace scoped

- **GIVEN** the workspace renders coordinator and delegated-agent execution streams
- **WHEN** the session/agent tablist is displayed
- **THEN** every tab in that tablist SHALL represent the coordinator stream or a delegated-agent trace stream
- **AND** the tablist SHALL NOT include an `AUTONOMY` pseudo-agent or other application-level OS capability
