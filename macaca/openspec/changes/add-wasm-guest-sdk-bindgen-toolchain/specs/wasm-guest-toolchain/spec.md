## ADDED Requirements

### Requirement: WIT-Driven Guest SDK Toolchain
The system SHALL provide a provider-neutral guest SDK toolchain that validates
WIT inputs, generates Rust guest binding scaffolds, emits package fixtures,
registers mock host imports, and runs local certification feedback without
depending on a concrete runtime engine.

#### Scenario: Guest scaffold is generated
- **WHEN** a developer provides valid WIT metadata, package identity, ABI version, declared imports, and declared exports
- **THEN** the SDK SHALL generate a deterministic Rust guest scaffold and admission-ready package fixture

#### Scenario: Guest scaffold input is rejected
- **WHEN** WIT metadata is malformed, missing required imports, declares unsupported ABI, or attempts to request raw host resources
- **THEN** the SDK SHALL reject generation with sanitized diagnostics and stable reason codes

### Requirement: SDK Provider-Neutral Boundary
The system SHALL keep guest SDK bindgen outputs provider-neutral and SHALL NOT
construct runtime-host providers, daemon transports, engine adapters, Web state,
CLI state, or application-specific workflows.

#### Scenario: SDK output is provider-neutral
- **WHEN** SDK bindgen emits a scaffold, package fixture, or local test harness descriptor
- **THEN** the output SHALL reference Macaca ABI and host import contracts rather than concrete engine or daemon types
