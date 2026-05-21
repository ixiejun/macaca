## ADDED Requirements

### Requirement: Scheduler service SHALL provide provider-neutral application-scoped job management

The Scheduler service SHALL expose provider-neutral commands and results for
listing, reading, updating, deleting, and transitioning application-scoped
Scheduler jobs without requiring callers to know whether the provider is local,
plugin-backed, remote, mock, or unavailable.

#### Scenario: Application-scoped job list is returned

- **GIVEN** a traced Scheduler job-list command scoped to one application
- **WHEN** the Scheduler provider is active
- **THEN** the service SHALL return only jobs owned by that application scope
- **AND** each job summary SHALL include bounded sanitized fields such as job id, lifecycle, schedule spec, target kind, metadata, trace id, audit id when available, and timestamps
- **AND** the response SHALL NOT include raw provider payloads, prompts, manifests, WASM bytes, package bytes, credentials, private keys, raw signatures, or unbounded output

#### Scenario: Job mutation is traceable and auditable

- **GIVEN** a traced update, delete, pause, or resume command for an application-scoped job
- **WHEN** the Scheduler provider accepts the mutation
- **THEN** the service SHALL update the job through its provider-owned lifecycle state machine
- **AND** the result SHALL include a trace id and audit-compatible evidence id when available
- **AND** the service SHALL emit structured logs for request, validation, provider delegation, success, rejection, and failure

#### Scenario: Provider is unavailable

- **GIVEN** a traced Scheduler job-management command
- **WHEN** no active Scheduler provider is installed
- **THEN** the Scheduler client/service SHALL return a structured unavailable result
- **AND** it SHALL NOT panic, hang, silently succeed, or construct a concrete provider outside an approved composition root

### Requirement: Scheduler service SHALL keep lifecycle semantics out of Web and frontend

Scheduler job lifecycle semantics SHALL be owned by the Scheduler service and
its provider implementation rather than by Web routes or frontend components.

#### Scenario: Web requests a pause transition

- **GIVEN** a Web route receives a pause request for a Scheduler job
- **WHEN** the route constructs the Scheduler lifecycle command
- **THEN** the route SHALL pass the command to the Scheduler client
- **AND** the Scheduler provider SHALL decide how the lifecycle state changes due-run eligibility
- **AND** Web/frontend SHALL only render the resulting provider-neutral lifecycle state
