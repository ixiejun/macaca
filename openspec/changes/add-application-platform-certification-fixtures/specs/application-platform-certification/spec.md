## ADDED Requirements

### Requirement: Application Platform CertificationKit
The system SHALL provide certification checks for Application Platform packages and fixtures using reusable Visitor and Specification-style rules.

#### Scenario: Certification traverses full application contract
- **WHEN** CertificationKit evaluates an application fixture
- **THEN** it SHALL inspect manifest metadata, abilities, permissions, services, capabilities, UI declarations, commerce declarations, plugin dependencies, ABI declarations, and runtime availability expectations.

#### Scenario: Certification report is auditable
- **WHEN** certification completes
- **THEN** the report SHALL be serializable and include fixture id, app id, ability ids, service ids, capability ids, operation, status, reason codes, and trace id when supplied.

### Requirement: Ecosystem Shape Fixtures
The system SHALL include generic fixtures for declarative YAML/AgentAbility, GenUI, headless, Store-entitled, Plugin-enhanced, and WASM skeleton applications.

#### Scenario: Fixtures validate platform breadth
- **WHEN** integration tests run certification fixtures
- **THEN** each supported application shape SHALL pass its expected contract checks without relying on real network, real Store, real Payment, real Plugin execution, real Web3/EVM, or real WASM runtime.

#### Scenario: Fixtures avoid business hardcoding
- **WHEN** fixtures are inspected
- **THEN** they SHALL use generic fixture ids and SHALL NOT hardcode business app names, workflow names, provider names, driver names, gateway names, chain names, or application-specific behavior.

### Requirement: Fail-Closed and Unavailable Certification
Certification tests SHALL prove that missing permissions, service dependencies, plugin dependencies, optional runtimes, or optional modules fail closed or return structured unavailable.

#### Scenario: Missing dependency is rejected
- **WHEN** a fixture declares capability usage without the required permission, service, plugin, or runtime declaration
- **THEN** certification SHALL return structured diagnostics before execution.

#### Scenario: Optional runtime is unavailable-safe
- **WHEN** a fixture requires an unavailable optional runtime such as WASM execution, Store entitlement, Plugin execution, Web3, or EVM
- **THEN** tests SHALL verify structured unavailable behavior rather than panic, hang, or silent success.

### Requirement: Certification Redaction
Certification reports, fixture diagnostics, logs, and snapshots SHALL be sanitized.

#### Scenario: Unsafe data is excluded
- **WHEN** certification emits reports, diagnostics, logs, or snapshots
- **THEN** they SHALL NOT include prompt bodies, raw full manifest bodies, raw agent configs, raw WASM bytes, secrets, env values, API keys, private keys, raw signatures, raw host payloads, or unbounded user input.
