## ADDED Requirements

### Requirement: Heartbeat Operations UI SHALL not reintroduce presentation-owned autonomy semantics

Macaca SHALL reject Heartbeat Operations UI implementations that make Web or frontend own heartbeat cadence, agent execution, manifest mutation, or Scheduler-backed heartbeat compatibility semantics.

#### Scenario: Frontend attempts to create heartbeat scheduler jobs

- **WHEN** new frontend or Web code represents heartbeat cadence by creating Scheduler jobs
- **THEN** the implementation SHALL be rejected
- **AND** native heartbeat cadence SHALL remain owned by `service.heartbeat`

#### Scenario: Web attempts to edit heartbeat declarations by mutating raw manifests

- **WHEN** new Web code edits raw application manifest files for heartbeat operations
- **THEN** the implementation SHALL be rejected
- **AND** application-owned heartbeat declarations SHALL remain behind Application Service projections or future typed Application Service mutation commands
