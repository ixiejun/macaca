## ADDED Requirements

### Requirement: Tool Capability Contracts Shall Be Provider-Neutral

Macaca SHALL define provider-neutral tool capability contracts that describe tool metadata, planning, diagnostics, policy, result classes, artifacts, provider health, and audit references without transferring runtime ownership away from the owning service.

#### Scenario: Descriptor identifies owner and route without provider leakage

- **GIVEN** a Driver, Skill, MCP, Memory, Task, Scheduler, Gateway, Store, or runtime tool is described
- **WHEN** a tool descriptor is serialized
- **THEN** it SHALL include stable owner, service, provider, family, schema, policy, lifecycle, result, artifact, and audit metadata
- **AND** it SHALL NOT include raw secrets, credentials, env values, raw provider payloads, prompts, private keys, headers, or unbounded output.

#### Scenario: Descriptor does not transfer lifecycle ownership

- **GIVEN** a descriptor represents an MCP tool owned by `service.mcp`
- **WHEN** `service.tool` stores or returns the descriptor
- **THEN** the descriptor SHALL remain metadata only
- **AND** concrete MCP lifecycle and invocation ownership SHALL remain with `service.mcp`.

### Requirement: Service Tool Commands Shall Be Typed And Trace-Required

Macaca SHALL expose `service.tool` operations as typed command/result DTOs and every command SHALL require trace context.

#### Scenario: Missing trace is rejected

- **WHEN** a caller submits a `service.tool` command without trace context
- **THEN** the command SHALL be rejected before side effects
- **AND** the result SHALL use a structured failure reason.

#### Scenario: Command surface covers the complete industrial flow

- **WHEN** a client uses `service.tool`
- **THEN** typed commands SHALL exist for planning, snapshots, toolset resolution, invocation, cancellation, status, result retrieval, artifact access, provider status, provider health, policy explanation, and audit query.

### Requirement: Tool Plans Shall Model Visible And Hidden Tools

Macaca SHALL define DTOs for deterministic tool plans that separate model-visible tool entries from hidden entries with diagnostics.

#### Scenario: Hidden tool preserves operator diagnostics

- **GIVEN** a tool is hidden because its provider is unavailable
- **WHEN** a `ToolPlan` is serialized
- **THEN** the tool SHALL NOT appear in the visible entries
- **AND** the hidden entry SHALL include a stable reason code and sanitized remediation hint.

### Requirement: Availability Expressions Shall Be Declarative

Macaca SHALL define declarative availability expression DTOs for composable service, config, auth, environment, binary, platform, resource, entitlement, plugin, manifest, agent-policy, and session-context checks.

#### Scenario: Availability expression remains data

- **GIVEN** a descriptor requires an auth provider and a service health signal
- **WHEN** the descriptor is returned to a planning service
- **THEN** those requirements SHALL be represented as availability expression data
- **AND** no application-specific code branch SHALL be required to understand them.

### Requirement: Tool Client Shall Provide Unavailable Behavior

The SDK SHALL provide `SystemToolClient` with service-backed and unavailable implementations.

#### Scenario: Tool service is absent

- **GIVEN** `service.tool` is not registered
- **WHEN** a shell, SDK caller, or application adapter requests a tool plan
- **THEN** the unavailable client SHALL return an explicit unavailable result
- **AND** it SHALL NOT crash, hang, silently fall back, or fake success.

### Requirement: Tool Contract Logs And Audit Refs Shall Be Sanitized

Tool capability contracts SHALL support structured logging and audit references without requiring raw inputs or raw outputs in observability surfaces.

#### Scenario: Audit reference is stable and bounded

- **WHEN** a command result includes audit evidence
- **THEN** it SHALL use stable refs, hashes, counts, timestamps, ids, and reason codes
- **AND** it SHALL NOT include raw provider payloads, raw prompts, raw secrets, or unbounded output.
