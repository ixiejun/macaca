## ADDED Requirements

### Requirement: Tool Planning Shall Produce Visible And Hidden Entries

Macaca SHALL convert service-owned tool descriptors, application policy, agent policy, availability expressions, provider health, and context signals into deterministic tool plans with visible entries and hidden diagnostics.

#### Scenario: Unavailable tool is hidden with reason

- **GIVEN** a tool descriptor requires a missing auth provider
- **WHEN** `tool.catalog.plan` runs
- **THEN** the tool SHALL be excluded from model-visible tools
- **AND** the hidden entry SHALL include a stable reason code such as `missing_auth`
- **AND** the diagnostic SHALL NOT expose secrets or raw provider configuration.

#### Scenario: Provider service is absent

- **GIVEN** a descriptor contributor depends on an unavailable provider service
- **WHEN** planning runs
- **THEN** the relevant tools SHALL be hidden or summarized as unavailable
- **AND** planning SHALL continue for other providers.

### Requirement: Toolsets Shall Be Data-Driven

Macaca SHALL resolve toolsets from declarative family and tool membership rules rather than application-specific code branches.

#### Scenario: Application declares research toolset

- **GIVEN** an application manifest declares the `research` toolset
- **WHEN** the plan is built for an agent
- **THEN** matching web, browser, memory, and document-capable tools SHALL be considered through data-driven rules
- **AND** no OS-layer branch SHALL depend on the application name.

#### Scenario: Exact allowed tools remain compatible

- **GIVEN** an existing application manifest declares exact `allowed_tools`
- **WHEN** the tool plan is built
- **THEN** Macaca SHALL preserve exact-name filtering semantics
- **AND** it SHALL also report hidden diagnostics for tools filtered by the compatibility allowlist.

### Requirement: Tool Families Shall Be Abstract Capability Categories

Macaca SHALL define tool families as abstract capability categories for planning and policy.

#### Scenario: Family policy filters tools across providers

- **GIVEN** an agent policy denies the `communication` family
- **WHEN** descriptors from Gateway and MCP communication tools are contributed
- **THEN** matching tools SHALL be hidden with a `policy_denied` reason
- **AND** the OS SHALL NOT branch on a specific gateway or application name.

### Requirement: Availability Evaluation Shall Be Bounded And Cacheable

Availability evaluation SHALL support bounded caching and explicit invalidation for config or provider changes.

#### Scenario: Binary availability is checked

- **GIVEN** a browser tool requires a local binary
- **WHEN** planning evaluates the descriptor
- **THEN** the binary check SHALL produce a visible or hidden decision
- **AND** repeated checks MAY be cached for a bounded TTL
- **AND** config/provider changes SHALL be able to invalidate the cache.

### Requirement: Context Shall Include Compact Tool Capability Index

The Context service SHALL expose compact tool capability information without injecting raw tool docs, raw MCP resources, raw provider payloads, or unbounded schemas by default.

#### Scenario: Context report records capability counts

- **WHEN** context composition completes
- **THEN** the report SHALL include selected, hidden, skipped, and conflicted tool counts
- **AND** model-visible context SHALL remain bounded and sanitized.

#### Scenario: Hidden diagnostics summarize unavailable capabilities

- **GIVEN** several requested tool families are unavailable
- **WHEN** context is composed
- **THEN** context MAY include aggregate unavailable counts and stable reason summaries
- **AND** it SHALL NOT reveal secret names, env values, headers, or credentials.

### Requirement: Planning Logs Shall Be Sanitized And Traceable

Tool planning SHALL emit structured logs and audit refs at key execution nodes.

#### Scenario: Plan completes

- **WHEN** `tool.catalog.plan` completes
- **THEN** logs or events SHALL include trace id, application id, session id, agent name, contributor count, visible count, hidden count, conflict count, and reason-code counts
- **AND** they SHALL NOT include raw provider payloads, prompts, secrets, or unbounded schemas.
