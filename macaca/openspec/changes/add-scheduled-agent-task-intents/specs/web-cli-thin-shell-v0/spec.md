## ADDED Requirements

### Requirement: Web and frontend SHALL provide generic scheduled agent task entry surfaces

Macaca Web and frontend SHALL provide application-scoped scheduled-agent-task
entry surfaces for manual task creation and inspection while remaining
presentation shells and not owning Scheduler, prompt, or agent execution
semantics.

#### Scenario: User manually creates a scheduled agent task

- **GIVEN** a user is viewing an application operations surface
- **WHEN** the user submits target agent, task prompt, schedule, name, and safe metadata
- **THEN** the frontend SHALL call `/api/apps/{app_id}/autonomy/scheduled-agent-tasks`
- **AND** Web SHALL adapt the request into a typed Scheduled Agent Task client command
- **AND** neither Web nor frontend SHALL register Scheduler jobs directly.

#### Scenario: User browses scheduled agent task summaries

- **WHEN** the user opens the scheduled agent task list
- **THEN** the frontend SHALL render sanitized summaries including target agent, schedule summary, lifecycle, payload digest, trace id, audit id when available, and result status
- **AND** it SHALL NOT render raw task prompt text or raw delegated context in list or run-history responses.

#### Scenario: Web maps service failure

- **WHEN** Scheduled Agent Task, Scheduler, Agent Execution, Context, or policy services return unavailable, unsupported, denied, validation, conflict, timeout, or provider failure
- **THEN** Web SHALL return a safe structured HTTP response with trace id when available
- **AND** it SHALL NOT expose raw prompts, manifests, WASM bytes, package bytes, provider payloads, private keys, credentials, raw signatures, or unbounded output.

### Requirement: Web and frontend SHALL not encode application-specific scheduled task semantics

Web and frontend scheduled-agent-task code SHALL stay generic and SHALL NOT
encode application-specific templates, workflow names, provider names, model
names, driver names, gateway names, chain names, payment names, or business
domain branches.

#### Scenario: Frontend renders the manual editor

- **WHEN** frontend renders the scheduled-agent-task editor
- **THEN** it SHALL present generic fields for target agent, prompt, schedule, name, and metadata
- **AND** it SHALL NOT contain hardcoded application-specific task templates or business-domain branching.
