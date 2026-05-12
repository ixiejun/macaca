## ADDED Requirements

### Requirement: WASM Component Application ABI Schema
The system SHALL provide a WASM Component Application ABI schema or WIT file aligned with the provider-neutral Application ABI imports and exports.

#### Scenario: ABI schema matches Rust DTOs
- **WHEN** ABI alignment tests run
- **THEN** the schema import/export names SHALL match the canonical `ApplicationImport` and `ApplicationExport` names used by Rust protocol DTOs.

#### Scenario: ABI remains provider-neutral
- **WHEN** a WASM application declares imports
- **THEN** imports SHALL represent Macaca host capabilities and service commands, not concrete provider, Web, Kernel, or runtime-host implementation types.

### Requirement: WASM Guest SDK Scaffold
The SDK SHALL provide scaffold helpers or fixtures that show how WASM applications declare ABI metadata, host imports, required permissions, and runtime profile without executing guest code.

#### Scenario: WASM fixture is contract-valid
- **WHEN** a WASM skeleton application fixture is validated by SDK TestKit
- **THEN** it SHALL pass manifest, ability, ABI, permission, service, and trace contract checks.

#### Scenario: Scaffold is not a real runtime
- **WHEN** developers inspect the WASM scaffold
- **THEN** documentation and diagnostics SHALL clearly state that execution is unavailable until a later real WASM runtime proposal is implemented.

### Requirement: Unavailable-Safe WASM Application Host
The runtime host SHALL provide an unavailable-safe WASM application host skeleton for metadata-admitted WASM applications.

#### Scenario: WASM execution is unavailable
- **WHEN** a WASM application host operation is invoked before a real WASM runtime exists
- **THEN** the host SHALL return structured runtime-unavailable with trace id, application/package id when known, runtime kind, status, and reason code.

#### Scenario: Missing WASM runtime does not break base OS
- **WHEN** the base OS runs without a WASM runtime module
- **THEN** YAML and other non-WASM applications SHALL remain unaffected, and WASM execution requests SHALL fail safely rather than panic, hang, or pretend success.

### Requirement: No Heavy WASM Runtime Dependency
This proposal SHALL NOT introduce a real heavy WASM runtime dependency or execute third-party WASM code.

#### Scenario: Runtime dependency is deferred
- **WHEN** dependency boundaries are checked
- **THEN** no new real WASM execution engine dependency SHALL be required unless a later OpenSpec explicitly approves it.
