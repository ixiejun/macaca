## ADDED Requirements

### Requirement: Macaca SHALL provide a pluggable memory provider runtime

Macaca SHALL provide a provider runtime that allows builtin, remote, MCP, and future provider adapters to register memory capabilities without requiring upper application code to depend on concrete provider implementations.

#### Scenario: Default provider is builtin

- **GIVEN** no custom memory provider profile is configured
- **WHEN** memory facade is initialized
- **THEN** Macaca SHALL select builtin provider capabilities
- **AND** existing memory behavior SHALL remain available

#### Scenario: Provider registry selects by profile

- **GIVEN** a memory profile config selects provider ids for agent private and session shared memory
- **WHEN** the memory facade resolves routes
- **THEN** it SHALL use the provider registry/factory to create selected providers
- **AND** it SHALL NOT hardcode application, workflow, driver, or business names

### Requirement: Agent private and session shared providers SHALL be independently configurable

Macaca SHALL allow agent private memory provider and session/project shared memory provider to be configured independently.

#### Scenario: Agent private provider differs from shared provider

- **GIVEN** memory profile config sets `agent_private_provider = "lancedb"` and `session_shared_provider = "remote-company-rag"`
- **WHEN** an agent writes `AgentPrivate` memory
- **THEN** the private write SHALL route to `lancedb`
- **WHEN** the same agent writes `SessionShared` memory
- **THEN** the shared write SHALL route to `remote-company-rag`

#### Scenario: Agent override selects custom private provider

- **GIVEN** the default profile uses builtin private memory
- **AND** agent `coder` overrides private provider
- **WHEN** coder writes private memory
- **THEN** the coder override SHALL be used
- **AND** other agents SHALL continue using the default private provider unless they also override it

### Requirement: Remote memory providers SHALL use macaca-memory-v1 protocol

Macaca SHALL define a remote HTTP provider protocol for users who want to replace memory systems without writing Rust.

#### Scenario: Remote provider exposes required endpoints

- **GIVEN** a remote provider is configured
- **WHEN** Macaca initializes it
- **THEN** it SHALL use `GET /memory/v1/status` for health/status
- **AND** it SHALL use `POST /memory/v1/search`, `/get`, `/write`, `/delete`, and `/events` for memory operations when supported

#### Scenario: Remote requests include scope

- **GIVEN** Macaca sends a remote memory request
- **WHEN** the request is serialized
- **THEN** it SHALL include `MemoryScope`
- **AND** it SHALL include operation id or trace id, timeout budget, and request metadata

#### Scenario: Remote response is validated

- **GIVEN** a remote provider returns a response
- **WHEN** Macaca receives it
- **THEN** Macaca SHALL validate the response schema
- **AND** invalid responses SHALL produce diagnostics and graceful fallback

### Requirement: MCP memory providers SHALL be adapted through standard operations

Macaca SHALL support MCP memory providers by mapping standard memory operations to configured MCP tools.

#### Scenario: MCP search maps to configured tool

- **GIVEN** MCP memory provider config defines a search tool
- **WHEN** Macaca performs memory search through that provider
- **THEN** it SHALL call the configured MCP search tool
- **AND** it SHALL convert the MCP result into standard `MemoryHit` values

#### Scenario: MCP output is untrusted by default

- **GIVEN** an MCP provider returns memory content
- **WHEN** the content enters active recall or context reporting
- **THEN** it SHALL be marked as external/untrusted unless a policy explicitly upgrades it

### Requirement: External provider calls SHALL be resilient

Macaca SHALL wrap external provider calls with timeout, circuit breaker, diagnostics, and secret redaction.

#### Scenario: Provider timeout does not crash agent run

- **GIVEN** a remote provider does not respond before timeout
- **WHEN** memory search is requested
- **THEN** Macaca SHALL record timeout diagnostics
- **AND** it SHALL continue with fallback memory routes when available
- **AND** it SHALL not terminate the agent run solely because of the timeout

#### Scenario: Circuit breaker opens after repeated failures

- **GIVEN** an external provider repeatedly fails
- **WHEN** failure threshold is reached
- **THEN** Macaca SHALL open a circuit breaker for that provider
- **AND** subsequent calls during cooldown SHALL fail fast with diagnostics

#### Scenario: Secrets are redacted in diagnostics

- **GIVEN** provider config contains endpoint headers, API keys, tokens, or auth env names
- **WHEN** diagnostics or trace reports are produced
- **THEN** secret values SHALL be redacted

### Requirement: Provider tools SHALL be registered safely

Memory providers MAY expose tools, but tool registration SHALL avoid schema conflicts and preserve standard memory tool behavior.

#### Scenario: Standard tools are available

- **GIVEN** builtin memory provider is active
- **WHEN** tools are listed for an agent
- **THEN** standard tools such as memory search/get/store/delete SHOULD be available according to policy

#### Scenario: Tool name conflict is detected

- **GIVEN** two providers attempt to register the same tool name
- **WHEN** provider registry builds the tool set
- **THEN** Macaca SHALL detect the conflict
- **AND** it SHALL either reject the duplicate, namespace it, or report diagnostics according to policy
