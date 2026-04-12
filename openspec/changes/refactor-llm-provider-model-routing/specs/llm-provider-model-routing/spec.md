## ADDED Requirements

### Requirement: Provider Registry Driven Initialization

The system SHALL initialize LLM providers through a registry/router owned by `macaca-llm`, rather than hardcoding provider construction in `macaca-web`.

#### Scenario: Bootstrap registers configured providers
- **GIVEN** the system configuration declares multiple providers under `llm.providers`
- **WHEN** the web server starts
- **THEN** the configured providers are registered into a shared provider registry/router
- **AND** `macaca-web` does not branch on provider names to construct runtime providers directly

#### Scenario: Adding a compatible provider does not require bootstrap branching
- **GIVEN** a new OpenAI-compatible provider is added to config
- **WHEN** the server initializes the LLM layer
- **THEN** the provider is available to framework agents through the registry/router
- **AND** no new provider-specific branch is required in `macaca-web` bootstrap code

### Requirement: Unified Model Selection Resolution

The system SHALL resolve the effective provider/model for every framework agent through a single model selection resolver.

#### Scenario: Agent override beats app default
- **GIVEN** an app default model is configured
- **AND** a specific agent declares its own model override
- **WHEN** the framework runner builds that agent
- **THEN** the agent uses the resolved override model
- **AND** the app default remains unchanged for other agents

#### Scenario: Explicit provider-qualified model is honored
- **GIVEN** a model reference explicitly identifies a provider-qualified target
- **WHEN** the resolver builds the effective route plan
- **THEN** the selected provider and model match the explicit reference
- **AND** the route plan preserves that selection for execution and tracing

### Requirement: Routed Framework Model Execution

All framework-based agents SHALL execute through a routed `ChatModel` adapter backed by the shared provider registry/router.

#### Scenario: Coordinator and worker use the same routing path
- **GIVEN** a coordinator agent and a worker agent are built in the same app
- **WHEN** they issue model calls
- **THEN** both go through the same routed framework adapter
- **AND** neither path bypasses the shared provider/model resolver

#### Scenario: Fallback chain is part of the route plan
- **GIVEN** a primary model and one or more fallback targets are configured
- **WHEN** the primary model fails
- **THEN** the runtime retries according to the resolved fallback chain
- **AND** the actual provider/model used can be observed in logs or trace metadata
