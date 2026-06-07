## ADDED Requirements

### Requirement: SDK Scheduler client SHALL expose focused job-management commands

The SDK SHALL expose focused Scheduler client methods for serviceized job
management so Web, CLI, application runtimes, and future plugins do not call
concrete providers or legacy presentation-owned schedule paths.

#### Scenario: Web lists Scheduler jobs through SDK client

- **WHEN** Macaca Web needs to list Scheduler jobs for an application
- **THEN** it SHALL construct a typed Scheduler list-jobs command with trace context and application scope
- **AND** it SHALL call the focused Scheduler client
- **AND** it SHALL NOT construct `LocalSchedulerProvider`, `TaskScheduler`, or any concrete scheduler provider

#### Scenario: Scheduler client preserves structured unavailable behavior

- **WHEN** the focused Scheduler client receives a job-management command but the Scheduler service is unavailable
- **THEN** it SHALL return a structured unavailable or unsupported result
- **AND** it SHALL log operation, trace id, app scope, job id when available, and safe error code
- **AND** it SHALL NOT fake a successful mutation

#### Scenario: Scheduler command remains provider-neutral

- **WHEN** upper layers construct list, get, update, delete, pause, or resume commands
- **THEN** the command schema SHALL use provider-neutral scope, schedule spec, target kind, metadata, and lifecycle operation fields
- **AND** it SHALL NOT require application names, workflow names, provider names, driver names, model names, gateway names, chain names, payment names, or business-domain branches
