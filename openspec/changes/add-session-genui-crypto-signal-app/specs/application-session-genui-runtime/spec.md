## ADDED Requirements

### Requirement: Applications SHALL declare built-in session GenUI surfaces

Macaca SHALL support application manifests that declare a host-owned built-in UI kit for chat/session applications without requiring an app-owned web bundle.

#### Scenario: WASM app declares builtin session UI

- **GIVEN** an installed L2Wasm application manifest declares `ui.runtime` as `builtin_kit`
- **AND** it declares `ui.surface.mode` as `session`
- **WHEN** the application is opened in a shell
- **THEN** the shell SHALL keep the main thread, conversation stream, bottom chat composer, and AgentPanel available
- **AND** the shell SHALL use the generic GenUI renderer for any emitted UI surface
- **AND** the shell SHALL NOT branch on application id, app name, service id, workflow name, or business domain.

### Requirement: WASM session execution SHALL expose queryable GenUI surfaces

Macaca SHALL connect application `macaca:ui/render` host commands to a bounded, queryable GenUI surface scoped by application id, session id, and surface id.

#### Scenario: Guest emits render intent during session execution

- **GIVEN** a WASM application session dispatch carries application id and session id
- **AND** the guest emits an `ApplicationImport::UiRender` host command with a valid trace context
- **WHEN** the runtime host validates the render payload
- **THEN** it SHALL store the latest GenUI intent for that application/session/surface
- **AND** `APPLICATION_GENUI_SURFACE_COMMAND` SHALL return that intent through Application Service
- **AND** logs and audit metadata SHALL include trace id, app id, session id, surface id, and validation outcome.

#### Scenario: No surface exists for session

- **GIVEN** an application has not emitted a GenUI surface for the requested session
- **WHEN** the shell queries `APPLICATION_GENUI_SURFACE_COMMAND`
- **THEN** Application Service SHALL return a structured empty or unavailable result
- **AND** the frontend SHALL preserve the existing chat/session shell.

### Requirement: Session GenUI SHALL remain application-agnostic

Macaca SHALL render session GenUI surfaces from declarative component trees and SHALL NOT add business-specific renderers for individual applications.

#### Scenario: Crypto app emits signal cards

- **GIVEN** a crypto signal application emits a GenUI tree using supported component kinds such as card, table, list, markdown, and text
- **WHEN** the frontend receives the surface
- **THEN** the generic `GenUiRenderer` SHALL render it by component kind
- **AND** it SHALL NOT contain crypto-specific branches, custom React components, or direct service-provider calls.

### Requirement: Crypto signal app SHALL use generic service contracts

The crypto signal WASM app SHALL declare all external data dependencies through manifest service contracts and issue external calls only through `service.call`.

#### Scenario: Crypto signal app requests market analysis

- **GIVEN** the user asks for a crypto buy/sell signal analysis
- **WHEN** the WASM app executes
- **THEN** it SHALL normalize the symbol input
- **AND** it SHALL call declared host services such as `service.market_data`, `service.news_digest`, and `service.llm.analysis` through `service.call`
- **AND** it SHALL emit `analysis_only` and `not_financial_advice` metadata
- **AND** it SHALL NOT directly access the network, provider credentials, or application-specific host code.

### Requirement: Declared WASM host-command plans SHALL support generic result chaining

Macaca SHALL let a declared WASM host-command plan pass prior host-command outputs into later declared commands through provider-neutral template references.

#### Scenario: WASM app composes market, news, analysis, and UI render

- **GIVEN** a WASM application declares a sequence of host commands
- **AND** a later command payload references prior outputs using bounded placeholders such as `${host.results.0.output}` or `${host.results.2.output.analysis}`
- **WHEN** the Component Model runtime dispatches the declared plan
- **THEN** Macaca SHALL resolve those placeholders from earlier command results before dispatching the later command
- **AND** the application SHALL be able to feed market data and news results into an LLM analysis service
- **AND** the application SHALL be able to feed the analysis output into a generic `ui.render` intent
- **AND** Macaca SHALL NOT hardcode application names, symbols, service workflow names, or business-specific payload shapes.
