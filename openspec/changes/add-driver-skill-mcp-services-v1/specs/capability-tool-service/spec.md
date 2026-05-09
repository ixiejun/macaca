## ADDED Requirements

### Requirement: Capability Tool Descriptor

The system SHALL define a shared sanitized `CapabilityToolDescriptor` for tool metadata produced by Driver, Skill, and MCP services.

#### Scenario: Tool metadata is exposed to upper layers

- **WHEN** any capability service returns tool catalog metadata
- **THEN** each tool SHALL be represented by a sanitized descriptor containing service id, provider id, capability id, tool name, description, JSON schema, origin kind, permission hints, resource scope hints, conflict namespace, and display name
- **AND** the descriptor SHALL NOT transfer lifecycle or invocation ownership away from the producing service.

#### Scenario: Sensitive metadata is present in provider configuration

- **WHEN** provider configuration contains env, headers, API keys, credentials, full command lines, or workspace secrets
- **THEN** the descriptor SHALL omit or redact those values before returning metadata to SDK, Web, CLI, framework, snapshots, logs, or events.

### Requirement: Capability Tool Invocation DTO

The system SHALL define a shared invocation DTO shape for capability tool calls while preserving service-specific dispatch ownership.

#### Scenario: Tool invocation command is built

- **WHEN** SDK or Web builds a tool invocation request
- **THEN** the request SHALL include trace, application id, session id, agent name, tool name, JSON input, policy hints, and resource scope
- **AND** the producing service SHALL validate scope and policy before dispatch.

#### Scenario: Tool invocation result is reported

- **WHEN** a capability tool invocation completes or fails
- **THEN** the service SHALL return a structured result with status, sanitized output or error summary, trace id, and service origin metadata
- **AND** logs/events SHALL not include raw secrets or unsanitized payloads.

### Requirement: Capability Tool Conflict Metadata

The shared descriptor SHALL support conflict namespace and display metadata so Driver, Skill, and MCP tools can coexist without hardcoded names.

#### Scenario: Two services expose tools with the same public name

- **WHEN** the framework toolkit receives two descriptors with the same public tool name
- **THEN** the toolkit/service adapter SHALL apply the configured conflict policy using descriptor metadata
- **AND** the policy SHALL NOT hardcode application names, workflow names, driver names, skill names, MCP server names, or provider names.

### Requirement: Capability Tool Trace and Audit

The shared tool DTOs SHALL carry enough metadata for trace correlation, policy admission, and audit without exposing sensitive implementation details.

#### Scenario: Trace viewer displays a tool call

- **WHEN** a Driver, Skill, or MCP tool call is shown in trace output
- **THEN** the event SHALL include trace id, service id, operation, sanitized tool name, origin kind, scope, status, and timing/error summary
- **AND** the event SHALL NOT include env, headers, API keys, raw provider credentials, or full unsanitized input/output payloads.
