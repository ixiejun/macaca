## ADDED Requirements

### Requirement: App-owned UI model selection bridge

Macaca SHALL let app-owned UI bundles discover LLM provider/model choices and
submit selected route hints through declared generic bridge capabilities instead
of application-specific shell code.

#### Scenario: UI renders a backend-owned model selector

- **GIVEN** an application manifest declares the required UI bridge capability and `service.llm` access
- **WHEN** the app-owned UI requests provider/model choices
- **THEN** the shell bridge routes the request through the generic service boundary
- **AND** the UI renders only sanitized catalog data returned by `service.llm`
- **AND** the shell does not branch on application id, provider name, model name, or workflow semantics

#### Scenario: UI starts execution with selected model hint

- **GIVEN** an app-owned UI selected a provider/model route from the service catalog
- **WHEN** the UI starts application execution
- **THEN** the execution request carries the selected provider/model as a provider-neutral hint
- **AND** model routing remains owned by `service.llm`
- **AND** execution events expose bounded route diagnostics when available

#### Scenario: UI lacks declared service capability

- **GIVEN** an application manifest does not declare the required `service.llm` bridge capability
- **WHEN** the UI requests a model catalog or route resolution
- **THEN** the bridge denies the call before routing
- **AND** the denial is recorded with trace id, application id, service id, command, and a sanitized reason code
