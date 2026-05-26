## ADDED Requirements

### Requirement: Production Tool Invocation Shall Route Through Service Tool

Macaca SHALL route production framework tool invocation through `service.tool/tool.invoke` and then to the owning service or provider adapter.

#### Scenario: MCP tool invokes through owning MCP service

- **GIVEN** a visible MCP tool is selected in a `ToolPlan`
- **WHEN** the model calls the tool
- **THEN** the framework adapter SHALL call `SystemToolClient::invoke`
- **AND** `service.tool` SHALL route to `service.mcp/mcp.tool.invoke`
- **AND** Web SHALL NOT own the MCP protocol client.

#### Scenario: Skill tool invokes through owning Skill service

- **GIVEN** a visible executable Skill tool is selected in a `ToolPlan`
- **WHEN** the model calls the tool
- **THEN** `service.tool` SHALL route to `service.skill/skill.tool.invoke`
- **AND** Skill service SHALL remain the owner of skill runtime and policy semantics.

#### Scenario: Driver tool invokes through owning Driver service

- **GIVEN** a visible Driver tool is selected in a `ToolPlan`
- **WHEN** the model calls the tool
- **THEN** `service.tool` SHALL route to `service.driver/driver.tool.invoke`
- **AND** Driver service SHALL remain the owner of driver lifecycle and concrete execution.

### Requirement: Policy Shall Run Before Side Effects

Macaca SHALL run policy, approval, resource, entitlement, timeout, and budget gates before privileged tool side effects.

#### Scenario: Write tool requires approval

- **GIVEN** a tool is classified as write-capable
- **WHEN** session policy requires approval for writes
- **THEN** `tool.invoke` SHALL return an approval request before executing the tool
- **AND** the audit log SHALL record the approval requirement without raw input leakage.

#### Scenario: Denied tool does not dispatch

- **GIVEN** policy denies a tool family
- **WHEN** a caller invokes a tool in that family
- **THEN** `service.tool` SHALL return a structured denied result
- **AND** it SHALL NOT dispatch to the owning provider service.

### Requirement: Tool Results Shall Be Bounded And Artifact-Aware

Macaca SHALL normalize tool results into bounded inline responses, multimodal responses, artifact references, background handles, approval requests, or structured failures.

#### Scenario: Oversized result becomes artifact

- **GIVEN** a tool returns output larger than the configured inline result budget
- **WHEN** result normalization runs
- **THEN** the output SHALL be persisted as an artifact
- **AND** the model-visible result SHALL include a stable artifact ref and bounded summary.

#### Scenario: Binary result becomes artifact

- **GIVEN** a tool returns a binary document, image, audio, video, or archive
- **WHEN** result normalization runs
- **THEN** the binary output SHALL be stored as an artifact
- **AND** logs and model-visible text SHALL contain only a stable ref and sanitized metadata.

### Requirement: Invocation Lifecycle Shall Be Traceable And Auditable

Macaca SHALL emit sanitized trace, event, telemetry, and audit evidence for invocation lifecycle nodes.

#### Scenario: Invocation completes

- **WHEN** a tool invocation succeeds
- **THEN** audit evidence SHALL include trace id, application id, session id, agent name, service id, provider id, tool id, visible name, policy decision ref, resource scope, input hash, output hash, result class, latency, status, and stable reason code
- **AND** it SHALL NOT include raw input, raw output, prompts, credentials, raw provider payloads, headers, env values, or unbounded output.

#### Scenario: Invocation fails

- **WHEN** a tool invocation fails, times out, is cancelled, or reaches an unavailable provider
- **THEN** `service.tool` SHALL return a structured failure state
- **AND** audit evidence SHALL include a sanitized error summary and stable reason code.

### Requirement: Compatibility Paths Shall Not Become Production Ownership

Legacy direct toolkit or provider paths MAY remain during migration, but production framework invocation SHALL use `SystemToolClient`.

#### Scenario: Compatibility path remains for tests

- **GIVEN** a low-level compatibility test needs a direct provider primitive
- **WHEN** it uses that primitive
- **THEN** the primitive MAY remain available
- **AND** it SHALL NOT be treated as production tool invocation ownership.
