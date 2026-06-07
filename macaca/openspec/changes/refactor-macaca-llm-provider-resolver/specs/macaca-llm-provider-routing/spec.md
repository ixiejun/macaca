## ADDED Requirements

### Requirement: Chain-Based Provider Resolution

`macaca-llm` SHALL resolve inferred providers through an ordered resolver chain rather than embedding provider prefix decisions directly in router selection code.

#### Scenario: Known model families preserve current provider mapping
- **GIVEN** a model reference using an existing known family prefix
- **WHEN** the provider resolver chain evaluates the model
- **THEN** the resolved provider matches the pre-refactor mapping for that family

#### Scenario: Aggregator-style model references preserve openrouter mapping
- **GIVEN** a model reference containing `/`
- **WHEN** the provider resolver chain evaluates the model
- **THEN** the resolved provider is `openrouter`

#### Scenario: Unknown model references remain extensible
- **GIVEN** a model reference that matches no built-in resolver rule
- **WHEN** the provider resolver chain evaluates the model
- **THEN** the resolved provider is the original model reference

### Requirement: Router Uses Provider Resolver

`LlmRouter` SHALL use the provider resolver chain when resolving model references without an explicit registered provider or provider hint.

#### Scenario: Router resolves provider through the default resolver chain
- **GIVEN** a router with providers registered for an existing model family
- **WHEN** a chat request uses a model from that family
- **THEN** the router dispatches the request to the provider selected by the resolver chain

#### Scenario: Explicit registered provider remains authoritative
- **GIVEN** a model reference in `provider:model` form where `provider` is registered
- **WHEN** `LlmRouter` resolves the target
- **THEN** the target provider is the explicit registered provider
- **AND** the model is the part after `:`

### Requirement: Deprecated Provider Name Compatibility

The old router provider-name inference helper SHALL remain callable but deprecated to support grep-based migration.

#### Scenario: Deprecated helper delegates to resolver behavior
- **GIVEN** existing code calls the deprecated provider-name helper
- **WHEN** it passes a model reference covered by the default resolver
- **THEN** the helper returns the same provider that the resolver chain returns
