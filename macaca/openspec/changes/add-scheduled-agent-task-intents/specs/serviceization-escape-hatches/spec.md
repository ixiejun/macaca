## ADDED Requirements

### Requirement: Scheduled Agent Task escape-hatch gates SHALL reject legacy and specialized paths

Serviceization escape-hatch gates SHALL reject new scheduled-agent-task code
that bypasses the Scheduled Agent Task service, uses legacy direct Scheduler
routes, stores raw prompts in Scheduler, or hardcodes application-specific task
semantics.

#### Scenario: New Web or frontend code uses legacy schedule routes

- **GIVEN** production Web or frontend code added for scheduled agent task creation
- **WHEN** the code calls `/api/apps/{app_id}/schedules` or another legacy direct schedule-management route
- **THEN** the escape-hatch gate SHALL fail with replacement guidance pointing to `/api/apps/{app_id}/autonomy/scheduled-agent-tasks`.

#### Scenario: New OS-layer code hardcodes business task semantics

- **WHEN** production OS-layer scheduled-agent-task code branches on application name, workflow name, provider name, model name, driver name, gateway name, chain name, payment name, or business-domain keywords
- **THEN** the escape-hatch gate SHALL fail
- **AND** the replacement guidance SHALL point to application manifests, capability declarations, policy, and provider-neutral service commands.

#### Scenario: Prompt appears in safe observability surfaces

- **WHEN** tests serialize Scheduler jobs, Scheduler runs, Scheduled Agent Task summaries, Web list responses, frontend-safe summaries, or audit-safe records
- **THEN** raw prompt fixture strings SHALL be absent
- **AND** payload digest, redacted summary, trace id, and audit id SHALL remain available for replay.
