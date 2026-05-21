## ADDED Requirements

### Requirement: Frontend SHALL provide generic application-scoped autonomy schedule management

Macaca frontend SHALL provide a generic application-scoped schedule-management
surface for serviceized autonomy Scheduler jobs while remaining a presentation
shell and not owning Scheduler semantics.

#### Scenario: Operator browses schedules from an application workspace

- **GIVEN** an operator is viewing an application workspace
- **WHEN** the operator opens the autonomy schedule-management surface
- **THEN** the frontend SHALL call `/api/apps/{app_id}/autonomy/schedules`
- **AND** it SHALL render sanitized job summaries, lifecycle state, target kind, trace id, audit id when available, and safe metadata
- **AND** it SHALL NOT call legacy `/api/apps/{app_id}/schedules` routes for this feature

#### Scenario: Operator mutates a schedule

- **GIVEN** an operator creates, edits, deletes, pauses, or resumes a Scheduler job
- **WHEN** the frontend submits the operation
- **THEN** it SHALL send a provider-neutral request to `/api/apps/{app_id}/autonomy/*`
- **AND** it SHALL render success, unavailable, unsupported, denied, conflict, validation, and provider-failure states from structured responses
- **AND** it SHALL NOT encode application-specific schedule templates, workflow names, provider names, driver names, model names, gateway names, chain names, payment names, or business-domain branches

### Requirement: Web autonomy schedule routes SHALL remain shell command adapters

Macaca Web SHALL adapt HTTP autonomy schedule-management requests into typed
Scheduler client commands and SHALL NOT own Scheduler lifecycle semantics.

#### Scenario: Web route handles schedule update

- **WHEN** Web receives a schedule update request under `/api/apps/{app_id}/autonomy/schedules/{job_id}`
- **THEN** it SHALL validate HTTP scope and payload bounds
- **AND** it SHALL create trace context
- **AND** it SHALL construct a typed Scheduler client command
- **AND** it SHALL delegate to the Scheduler client
- **AND** it SHALL emit structured logs for request receipt, command construction, service delegation, success, rejection, and failure

#### Scenario: Scheduler service rejects a request

- **WHEN** the Scheduler client returns unavailable, unsupported, denied, conflict, validation error, or provider failure
- **THEN** Web SHALL map the result to a safe HTTP response
- **AND** it SHALL include trace id when available
- **AND** it SHALL NOT expose raw prompts, manifests, WASM bytes, package bytes, provider payloads, private keys, credentials, raw signatures, or unbounded output
