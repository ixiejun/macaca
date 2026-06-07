## ADDED Requirements

### Requirement: Macaca SHALL compose context from provider candidates

Macaca SHALL define a context composition pipeline where all context sources contribute `ContextCandidate` values through `ContextProvider` implementations, and only `ContextComposer` may select, order, truncate, and render those candidates into model-visible context.

#### Scenario: Provider contributes candidates without mutating prompt

- **GIVEN** a profile, memory, skill, MCP, tool, trace, or workspace provider is registered
- **WHEN** a model request is assembled
- **THEN** the provider SHALL return bounded `ContextCandidate` values
- **AND** the provider SHALL NOT directly mutate the LLM request messages
- **AND** the provider SHALL NOT write dynamic content into the canonical transcript

#### Scenario: Composer selects candidates deterministically

- **GIVEN** multiple providers return candidates in non-deterministic collection order
- **WHEN** `ContextComposer` builds a `ContextPlan`
- **THEN** candidates SHALL be ordered by deterministic stage, priority, source id, and stable tie-breakers
- **AND** equivalent inputs SHALL produce equivalent plans and stable hashes

### Requirement: Context candidates SHALL carry source, trust, scope, target, cache, and budget metadata

Every `ContextCandidate` SHALL carry enough metadata for policy, rendering, audit, and debugging without requiring the composer to know concrete provider internals.

#### Scenario: Candidate includes required metadata

- **GIVEN** any provider returns a candidate
- **WHEN** the composer validates the candidate
- **THEN** the candidate SHALL include source id, kind, scope, priority, trust level, cache class, target, bounded content, and diagnostics
- **AND** invalid or incomplete candidates SHALL be skipped with a reportable reason

#### Scenario: Unknown or request-specific candidate is dynamic

- **GIVEN** a candidate cannot prove it is stable across requests
- **WHEN** the composer classifies cache behavior
- **THEN** the candidate SHALL be treated as dynamic
- **AND** it SHALL NOT enter the stable prefix

### Requirement: Context planning SHALL be explicit and reportable

Macaca SHALL build an explicit `ContextPlan` before rendering final model context. The plan SHALL record selected and skipped candidates with reasons.

#### Scenario: Budget excludes candidates

- **GIVEN** providers return more content than the configured token or character budget
- **WHEN** the composer builds a plan
- **THEN** it SHALL select only candidates fitting policy
- **AND** it SHALL record skipped candidates with budget-related reasons

#### Scenario: Plan can be inspected without full prompt leakage

- **GIVEN** a model request has been composed
- **WHEN** diagnostics are requested
- **THEN** Macaca SHALL expose source ids, source kinds, selected/skipped status, estimates, hashes, and decisions
- **AND** it SHALL NOT expose full prompt or full sensitive content by default

### Requirement: ContextFacade SHALL be the upper-layer integration boundary

Runtime and framework code SHALL call a narrow `ContextFacade` or equivalent contract for model-request context assembly instead of directly invoking concrete providers or prompt builders.

#### Scenario: Runtime uses facade

- **GIVEN** a runtime loop is about to call an LLM provider
- **WHEN** it needs model-visible context
- **THEN** it SHALL call the context facade
- **AND** it SHALL NOT call concrete memory, skill, MCP, or profile providers directly for prompt injection

#### Scenario: Framework uses facade

- **GIVEN** a framework agent is about to call an LLM provider
- **WHEN** it needs model-visible context
- **THEN** it SHALL call the context facade
- **AND** it SHALL receive a compiled context and report through stable abstractions

### Requirement: Legacy context entry points SHALL remain deprecated and searchable

Macaca SHALL keep legacy prompt/context construction entry points during migration, mark replaced entry points as deprecated, and prohibit new production calls to them.

#### Scenario: Replaced legacy entry remains discoverable

- **GIVEN** a direct prompt construction function has been replaced by context facade usage
- **WHEN** migration finishes
- **THEN** the old function SHALL remain in the codebase with a deprecated marker and replacement guidance
- **AND** new production code SHALL NOT call it

#### Scenario: Legacy behavior remains available for rollback

- **GIVEN** context provider composition is disabled
- **WHEN** a model request is assembled
- **THEN** Macaca SHALL preserve legacy-compatible request behavior
- **AND** the compatibility path SHALL still emit a minimal context report
