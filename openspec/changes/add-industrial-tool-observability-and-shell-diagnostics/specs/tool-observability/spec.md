## ADDED Requirements

### Requirement: Tool Events Shall Be Observable And Sanitized

Macaca SHALL emit bounded sanitized events for tool planning, hidden diagnostics, policy, approvals, resource leases, invocation lifecycle, artifacts, and provider health.

#### Scenario: Invocation is visible live

- **WHEN** a tool invocation starts, progresses, completes, fails, or is cancelled
- **THEN** EventLog and live SSE SHALL expose a sanitized lifecycle event
- **AND** raw secrets, prompts, raw provider payloads, credentials, headers, env values, and unbounded output SHALL NOT be emitted.

#### Scenario: Provider health changes

- **WHEN** a tool provider moves from ready to degraded or unavailable
- **THEN** EventLog and diagnostic APIs SHALL expose the provider id, status, stable reason code, and captured timestamp
- **AND** they SHALL NOT expose raw provider configuration or credentials.

### Requirement: Tool Audit Shall Be Replayable

Macaca SHALL provide replayable audit records for tool planning and invocation.

#### Scenario: Plan audit is queried

- **GIVEN** a tool plan has been built
- **WHEN** an operator queries tool audit by trace id
- **THEN** the response SHALL include stable refs, visible count, hidden count, conflict count, policy refs, reason-code counts, and captured timestamp
- **AND** it SHALL NOT include raw model output or raw provider payloads.

#### Scenario: Invocation audit is queried

- **GIVEN** a tool invocation completed or failed
- **WHEN** an operator queries the invocation audit record
- **THEN** the response SHALL include trace id, application id, session id, agent name, service id, provider id, tool id, status, reason code, input hash, output hash, artifact refs, and latency
- **AND** it SHALL NOT include raw input, raw output, prompts, secrets, credentials, or unbounded output.

### Requirement: Shells Shall Render Diagnostics Without Owning Semantics

Web, CLI, and frontend SHALL render tool plans, hidden diagnostics, provider health, policy explanations, approval state, artifacts, and audit refs through SDK/service clients only.

#### Scenario: Web shows hidden tool reason

- **GIVEN** a tool is hidden because its provider is unavailable
- **WHEN** the user opens tool diagnostics
- **THEN** Web SHALL display the stable reason and remediation hint
- **AND** Web SHALL NOT contain provider lifecycle or policy decision logic.

#### Scenario: Frontend shows invocation trace

- **GIVEN** an invocation has audit refs and artifact refs
- **WHEN** the frontend renders the invocation trace panel
- **THEN** it SHALL show stable refs, status, timestamps, and bounded summaries
- **AND** it SHALL NOT render raw provider payloads or raw secrets.

### Requirement: Approval UI Shall Be Adapter-Only

Approval UI SHALL display and resolve approval requests through service commands without owning policy semantics.

#### Scenario: Approval is required

- **GIVEN** a write-capable tool requires human approval
- **WHEN** Web displays the approval request
- **THEN** the request SHALL include sanitized tool identity, policy reason, trace ref, and bounded argument summary
- **AND** the final policy decision SHALL be recorded by service-owned approval handling, not frontend logic.
