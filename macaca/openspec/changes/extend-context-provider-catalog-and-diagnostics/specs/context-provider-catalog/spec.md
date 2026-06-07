## ADDED Requirements

### Requirement: Provider families SHALL be selected exclusively via configuration

Macaca SHALL build the active `ContextProvider` list from operator configuration (`provider_families`) and neutral dependency injection — never from application names, workflow labels, or business concepts.

#### Scenario: Default family order when configuration is empty

- **GIVEN** `context.provider_families` is empty
- **WHEN** the catalog assembler runs
- **THEN** Macaca SHALL apply the documented built-in default ordering (same neutrality guarantees as explicit configuration)
- **AND** it SHALL skip families whose dependencies are unavailable with diagnostics (not hard failures) when running in constrained kernels

#### Scenario: Explicit family ordering

- **GIVEN** `context.provider_families` lists families in a specific order
- **WHEN** the catalog assembler runs
- **THEN** providers SHALL be instantiated in that order (subject to composer stage sorting rules)

### Requirement: Provider metadata SHALL be observable without leaking prompt bodies

Macaca SHALL expose implementation identity (semver, capability tags) and last-known health derived from invocation summaries.

#### Scenario: Diagnostics endpoint returns summaries only

- **GIVEN** an operator queries provider-runtime diagnostics
- **WHEN** the response is returned
- **THEN** it SHALL include family ids, versions/tags, and last outcomes
- **AND** it SHALL NOT include raw candidate text by default

### Requirement: Trust governance SHALL be configurable and policy-driven

Macaca SHALL support optional trust promotion rules applied to `ContextCandidate` values before composer budgeting, without referencing application-specific concepts.

#### Scenario: Promotion rule matches source id prefix

- **GIVEN** a trust promotion rule matches a candidate `source_id` prefix
- **AND** the candidate trust is at most the configured ceiling
- **WHEN** governance processes the candidate
- **THEN** the candidate trust SHALL be promoted to the configured target
- **AND** the decision SHALL be recorded in the context report when it changed the trust level

### Requirement: External context payloads SHALL pass an anti-corruption boundary

Macaca SHALL accept opaque transport payloads and validate only structural limits (size, required ids, declared schema version) without coupling to a specific remote protocol.

#### Scenario: Oversized opaque payload is rejected

- **GIVEN** an external adapter hands an opaque payload exceeding configured byte limits
- **WHEN** validation runs
- **THEN** Macaca SHALL reject with structured diagnostics
- **AND** it SHALL NOT parse protocol-specific frames beyond the opaque string body

### Requirement: Runtime execution paths SHALL reuse the facade and catalog abstractions

Macaca runtime SHALL not construct profile, skill, MCP, memory, or custom providers directly for prompt injection; it SHALL use the same catalog assembly entry when context configuration is available.

#### Scenario: Kernel path without optional catalogs

- **GIVEN** kernel assembly has no skill/MCP/memory dependencies
- **WHEN** providers are assembled
- **THEN** unavailable families SHALL be omitted
- **AND** model calls SHALL still proceed via `ContextFacade`
