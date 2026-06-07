## ADDED Requirements

### Requirement: Macaca SHALL manage context providers through a provider runtime

Macaca SHALL provide a context provider runtime that registers, creates, orders, invokes, and diagnoses context providers through configuration-driven abstractions.

#### Scenario: Provider set is created from configuration

- **GIVEN** context configuration selects built-in or custom providers
- **WHEN** Macaca initializes context composition
- **THEN** the provider runtime SHALL create the provider set through registry or factory abstractions
- **AND** it SHALL NOT select providers by hardcoded app name, workflow name, driver name, or business name

#### Scenario: Provider metadata is reportable

- **GIVEN** providers participate in a model request
- **WHEN** `ContextReport` is produced
- **THEN** the report SHALL include provider ids, versions or source markers, enabled status, and relevant diagnostics

### Requirement: Context governance SHALL apply to every provider output

Macaca SHALL apply budget, redaction, trust, source policy, timeout, and fallback governance to provider outputs before they become model-visible context.

#### Scenario: Provider output exceeds budget

- **GIVEN** a provider returns content exceeding its budget
- **WHEN** governance processes the output
- **THEN** Macaca SHALL truncate or skip the content according to policy
- **AND** it SHALL record the decision in the context report

#### Scenario: Sensitive content is redacted

- **GIVEN** a provider returns content matching redaction policy
- **WHEN** the content is considered for model context
- **THEN** Macaca SHALL redact or skip the sensitive content according to policy
- **AND** the report SHALL include a redaction decision without leaking the sensitive value

### Requirement: Provider failures SHALL be isolated by timeout and fallback policy

Provider runtime SHALL prevent a slow or failing provider from indefinitely blocking model calls.

#### Scenario: Slow provider times out

- **GIVEN** a provider exceeds its configured timeout
- **WHEN** context composition is running
- **THEN** the provider runtime SHALL stop waiting for that provider according to timeout policy
- **AND** the model request SHALL continue with fallback or remaining providers unless policy requires fail-closed

#### Scenario: Provider error is diagnostic

- **GIVEN** a provider returns an error
- **WHEN** the composer builds a context plan
- **THEN** the error SHALL be recorded as diagnostics
- **AND** other providers SHALL continue according to fallback policy

### Requirement: Custom context systems SHALL remain behind anti-corruption boundaries

Custom providers or future external context systems SHALL be adapted into Macaca candidate/report models and SHALL NOT bypass governance.

#### Scenario: Custom provider returns invalid output

- **GIVEN** a custom provider returns output missing required source, trust, scope, or budget metadata
- **WHEN** provider runtime validates the output
- **THEN** the output SHALL be rejected or normalized according to policy
- **AND** the validation decision SHALL be reportable

#### Scenario: Custom context manager replaces default composer

- **GIVEN** a user registers a custom context manager implementation
- **WHEN** configuration selects it
- **THEN** runtime/framework SHALL still call the same context facade
- **AND** Macaca SHALL still enforce report, budget, trust, and fallback contracts at the boundary

### Requirement: Provider runtime SHALL be observable without leaking full context

Macaca SHALL expose provider runtime diagnostics for debugging while avoiding full prompt or sensitive context leakage by default.

#### Scenario: Diagnostics API returns provider summary

- **GIVEN** a user inspects context diagnostics for a session or request
- **WHEN** diagnostics are returned
- **THEN** Macaca SHALL show provider status, selected/skipped counts, warnings, latency, and policy decisions
- **AND** it SHALL NOT return full provider content by default

### Requirement: Runtime and framework SHALL not directly couple to provider internals

Runtime and framework model call paths SHALL depend on context facade abstractions, not concrete profile, memory, skill, MCP, or custom provider implementations.

#### Scenario: Runtime does not call memory provider for prompt injection

- **GIVEN** active memory context is enabled
- **WHEN** runtime assembles a model request
- **THEN** runtime SHALL call the context facade
- **AND** it SHALL NOT call concrete memory provider APIs to inject prompt text

#### Scenario: Framework does not call MCP provider for prompt injection

- **GIVEN** MCP capability context is enabled
- **WHEN** framework assembles a model request
- **THEN** framework SHALL call the context facade
- **AND** it SHALL NOT call concrete MCP transport or registry internals for prompt injection
