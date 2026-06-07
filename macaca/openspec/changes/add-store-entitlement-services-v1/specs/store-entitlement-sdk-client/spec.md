## ADDED Requirements

### Requirement: Macaca SDK SHALL provide Store and Entitlement focused clients

Macaca SDK SHALL provide `SystemStoreClient` and `SystemEntitlementClient` focused client boundaries, with service-backed implementations over `SystemServiceClient` and unavailable/null-object implementations for missing services.

#### Scenario: SDK calls Store service without provider dependency

- **WHEN** Web, CLI, Gateway, or application-facing code calls `SystemStoreClient`
- **THEN** SDK SHALL dispatch through provider-neutral service commands
- **AND** SDK SHALL NOT construct Store providers, runtime-host providers, application runtimes, skill runtimes, or entitlement repositories

#### Scenario: SDK calls Entitlement service without provider dependency

- **WHEN** upper consumers call `SystemEntitlementClient`
- **THEN** SDK SHALL dispatch through provider-neutral service commands
- **AND** SDK SHALL NOT depend on `macaca-runtime-host`, `macaca-app`, `macaca-skill`, `macaca-web`, or `macaca-cli` concrete implementations for Store/Entitlement behavior

### Requirement: SDK package client SHALL become Store-service-backed

The existing SDK package client SHALL support Store-service-backed package inspection, install, and status where Store Service is available, while retaining structured unavailable behavior otherwise.

#### Scenario: Package inspection uses Store service

- **WHEN** a runtime-backed Store Service client is configured
- **THEN** package inspection SHALL call Store Service
- **AND** the result SHALL preserve sanitized package metadata and diagnostics

#### Scenario: Package inspection service is unavailable

- **WHEN** Store Service is missing
- **THEN** the SDK package client SHALL return structured unavailable or empty inspection with diagnostics
- **AND** it SHALL NOT panic or fabricate installed paid packages

### Requirement: SDK clients SHALL log traceable command lifecycle

SDK Store/Entitlement clients SHALL log command start, completion, and failure with safe identifiers.

#### Scenario: SDK command logging is safe

- **WHEN** a Store or Entitlement SDK command runs
- **THEN** logs SHALL include command name, service id, trace id when available, package id when available, developer id when available, and status
- **AND** logs SHALL omit raw package bodies, encrypted payloads, credentials, API keys, private keys, license secrets, prompt bodies, and raw manifest bodies
