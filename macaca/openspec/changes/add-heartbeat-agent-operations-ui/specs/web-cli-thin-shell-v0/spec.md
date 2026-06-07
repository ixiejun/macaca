## ADDED Requirements

### Requirement: Frontend SHALL expose application-scoped Heartbeat Operations beside Scheduler Operations

Macaca frontend SHALL expose a Heartbeat Operations surface adjacent to the existing Scheduler Operations surface for the selected application, while keeping both surfaces inside the application-level operations dialog.

#### Scenario: User opens heartbeat operations

- **WHEN** a user opens application operations for an application
- **THEN** frontend SHALL offer adjacent Scheduler and Heartbeat controls
- **AND** the Heartbeat surface SHALL render manifest-declared heartbeat agents, native heartbeat profile summaries, and recent heartbeat run mementos
- **AND** frontend SHALL NOT create Scheduler jobs to represent heartbeat cadence

#### Scenario: User edits heartbeat profile policy

- **WHEN** a user edits native heartbeat profile enabled state, interval, or metadata
- **THEN** frontend SHALL call an application-scoped Web command adapter
- **AND** Web SHALL delegate the operation to the focused Heartbeat SDK client
- **AND** the response SHALL include trace and audit correlation when available

### Requirement: Web SHALL keep heartbeat operations sanitized and provider-neutral

Macaca Web SHALL aggregate heartbeat operations state only from sanitized Application Service and Heartbeat Service DTOs.

#### Scenario: Heartbeat operations snapshot is returned

- **WHEN** Web returns heartbeat operations state
- **THEN** the response SHALL include bounded declaration, profile, run, count, trace, and audit fields
- **AND** the response SHALL NOT include raw manifests, raw `HEARTBEAT.md`, prompts, provider payloads, secrets, package bytes, or unbounded output
