## ADDED Requirements

### Requirement: SDK SHALL expose a focused Payment client

Macaca SDK SHALL expose a `SystemPaymentClient` Facade for quote, intent creation, policy evaluation, approval, settlement, receipt query, transition query, proof query, and snapshot operations.

#### Scenario: Upper consumer calls Payment through SDK client

- **WHEN** Web, CLI, Gateway, Application Framework, or future agent-facing code needs Payment / A2A capability
- **THEN** it SHALL call `SystemPaymentClient` or `SystemFacade::payment_client()`
- **AND** it SHALL NOT construct runtime-host payment providers, payment stores, kernel A2A coordinators, or concrete payment adapter strategies

#### Scenario: Service-backed client dispatches typed payment command

- **WHEN** `ServiceBackedPaymentClient` receives a typed payment command
- **THEN** it SHALL serialize the command into a `ServiceCallCommand` targeting Payment Service
- **AND** it SHALL preserve the command trace context and provider-neutral payload

### Requirement: SDK SHALL provide Payment unavailable behavior

Macaca SDK SHALL provide an unavailable Payment client that preserves safe Null Object behavior when Payment Service is absent, disabled, or not wired.

#### Scenario: Unavailable client fails closed for mutating payment calls

- **WHEN** quote, create intent, policy evaluate, approve, or settle is invoked through an unavailable Payment client
- **THEN** the client SHALL return structured unavailable or denied error
- **AND** it SHALL NOT pretend that payment succeeded

#### Scenario: Unavailable client returns diagnostic snapshot for read-only query

- **WHEN** snapshot or read-only receipt query is invoked through an unavailable Payment client
- **THEN** the client MAY return empty unavailable diagnostics
- **AND** the response SHALL make absence of Payment Service explicit

### Requirement: SDK SHALL remain provider-neutral

The Payment SDK client SHALL depend only on provider-neutral service contracts and generic service-client dispatch.

#### Scenario: SDK does not depend on runtime-host or provider implementations

- **WHEN** SDK Payment client code is compiled
- **THEN** it SHALL NOT require `macaca-runtime-host`, `macaca-web`, concrete payment provider crates, wallet providers, chain providers, Store implementations, Entitlement implementations, or business-specific application code
- **AND** provider replacement SHALL NOT require SDK API changes

