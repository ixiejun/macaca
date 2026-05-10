## ADDED Requirements

### Requirement: System Application Client
The SDK SHALL expose a focused `SystemApplicationClient` for shell-facing Application Service operations.

#### Scenario: SDK calls service through focused client
- **WHEN** Web, CLI, or Gateway needs application lifecycle or status data
- **THEN** it SHALL call `SystemApplicationClient` or `SystemFacade` accessors rather than constructing application runtime or registry internals.

#### Scenario: SDK remains provider-neutral
- **WHEN** `SystemApplicationClient` is compiled
- **THEN** the SDK SHALL NOT depend on `macaca-runtime-host`, `macaca-web`, or runtime-host provider implementation types.

### Requirement: Service-Backed and Unavailable Clients
The SDK SHALL provide both a service-backed Application client over `SystemServiceClient` and a null-object unavailable client.

#### Scenario: Service-backed client dispatches traced command
- **WHEN** a service-backed application client starts an application
- **THEN** it SHALL serialize the typed command through `SystemServiceClient` with trace context and return a typed result.

#### Scenario: Missing service is explicit
- **WHEN** a shell uses the unavailable Application client
- **THEN** lifecycle operations SHALL return structured unavailable rather than panic, block, or pretend success.

### Requirement: Facade Accessor
The SDK `SystemFacade` SHALL provide an Application client accessor without constructing provider implementations.

#### Scenario: Facade exposes focused client
- **WHEN** an upper shell has a `SystemFacade`
- **THEN** it SHALL be able to borrow the focused Application client through a stable accessor.

#### Scenario: Facade logs dispatch safely
- **WHEN** the facade dispatches an application command helper
- **THEN** it SHALL log operation, trace id, and application/session scope without logging prompt bodies or raw manifests.

