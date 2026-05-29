## ADDED Requirements

### Requirement: Application-visible LLM catalog

The LLM Service SHALL expose a sanitized provider/model catalog for applications
through typed service commands that include application, session, agent, tenant,
trace, and policy context.

#### Scenario: Application reads available models

- **GIVEN** an application has declared access to `service.llm`
- **WHEN** the application UI requests model catalog data through the service boundary
- **THEN** `service.llm` returns provider id, health, default model, known model rows, protocol metadata, and sanitized diagnostics
- **AND** the response does not include API keys, provider base URLs, raw prompts, raw provider payloads, or unbounded output

#### Scenario: Configured provider is unavailable

- **GIVEN** a provider is configured but cannot be initialized because required runtime credentials or dependencies are absent
- **WHEN** the model catalog is read
- **THEN** `service.llm` returns a structured unavailable provider row with a stable sanitized reason code
- **AND** the catalog read does not crash, hide the entire service, or fake provider availability

### Requirement: Request-level route override

The LLM Service SHALL resolve request-level provider/model hints before agent,
application, and system defaults while preserving provider-neutral routing
semantics and structured diagnostics.

#### Scenario: Application selects a model for execution

- **GIVEN** an application submits a task with a selected provider/model hint
- **WHEN** application execution resolves the effective model route
- **THEN** `service.llm` evaluates the request hint before agent, app, and system defaults
- **AND** the selected or rejected route is recorded with trace id, scope, source, and sanitized diagnostics

#### Scenario: Selected model is unsupported

- **GIVEN** an application submits a provider/model hint that cannot be routed
- **WHEN** `service.llm` resolves the route
- **THEN** the service returns a structured unsupported or unavailable diagnostic
- **AND** fallback usage is explicit rather than silent when policy permits fallback

### Requirement: Route audit metadata

The LLM Service SHALL emit sanitized audit and replay metadata for catalog reads,
route resolution, chat dispatch, fallback selection, and route failures.

#### Scenario: Route is replayed after session reload

- **GIVEN** a session was started from an app-owned UI with a selected model
- **WHEN** the session is reloaded or diagnostics are queried
- **THEN** Macaca can show the requested route, effective route, route source, trace id, and bounded diagnostics
- **AND** the replay data excludes raw prompt text, raw provider response bodies, credentials, and provider secrets
