## ADDED Requirements

### Requirement: Industrial Tool Families Shall Cover Real Complex Work

Macaca SHALL provide application-neutral provider-backed tool families for file, shell, browser, web, memory, knowledge, task, scheduler, skill, MCP, media, document, communication, enterprise API, code execution, computer use, and payment/entitlement.

#### Scenario: Multi-family task completes through generic tools

- **GIVEN** an application-neutral agent task requires research, browser or web access, file work, shell or code execution, memory recall, document or artifact handling, and scheduled follow-up
- **WHEN** the task runs through Macaca
- **THEN** the agent SHALL complete the task through planned visible tools
- **AND** every invoked tool SHALL pass service-owned policy, trace, result, telemetry, and audit handling.

#### Scenario: Family provider contributes descriptors

- **GIVEN** a provider implements a tool family
- **WHEN** the provider is available
- **THEN** it SHALL contribute descriptors through the Tool Capability Plane
- **AND** the descriptor SHALL identify family, provider, owner service, availability, policy, result class, and audit metadata.

### Requirement: Missing Optional Families Shall Be Explicit

Optional providers SHALL return structured unavailable, disabled, unsupported, or denied states when absent.

#### Scenario: Document provider is absent

- **GIVEN** the document family is requested
- **AND** no document provider, plugin, MCP server, or gateway route is available
- **WHEN** the tool plan is built
- **THEN** document tools SHALL appear as hidden diagnostics or unavailable provider summaries
- **AND** Macaca SHALL NOT fake success.

#### Scenario: Computer use provider is unsupported on platform

- **GIVEN** an agent requests computer-use tools
- **AND** the host platform does not support the configured provider
- **WHEN** the tool plan is built
- **THEN** computer-use tools SHALL be hidden with an `unsupported_platform` reason
- **AND** other tool families SHALL remain available.

### Requirement: Provider Selection Shall Be Data-Driven

Macaca SHALL select providers through descriptor, config, policy, and availability data rather than hardcoded provider-specific routing branches.

#### Scenario: Web family has direct and gateway providers

- **GIVEN** a web search capability is available through a direct provider and a managed gateway
- **WHEN** planning and invocation evaluate provider selection
- **THEN** routing SHALL follow policy and descriptor data
- **AND** OS code SHALL NOT branch on a concrete provider product name.

### Requirement: Industrial Proof Shall Use Stable Evidence

The final industrial Tools validation SHALL summarize stable refs and aggregate counts only.

#### Scenario: Live proof report is generated

- **WHEN** a multi-family industrial task completes
- **THEN** the report SHALL include stable session refs, tool plan counts, invocation audit refs, artifact refs, provider health summaries, and reason-code counts
- **AND** it SHALL NOT include raw model output, raw provider payloads, secrets, credentials, or unbounded tool output.

### Requirement: Family Providers Shall Preserve Service Ownership

Every family provider SHALL enter through an owning service, MCP, plugin, gateway, runtime adapter, or unavailable provider.

#### Scenario: Skill family is planned

- **GIVEN** the skill family is enabled
- **WHEN** descriptors are contributed
- **THEN** skill descriptors SHALL come through `service.skill` ownership
- **AND** `service.tool` SHALL NOT read or mutate skill package internals directly.
