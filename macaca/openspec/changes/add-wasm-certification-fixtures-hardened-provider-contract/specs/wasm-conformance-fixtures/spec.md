## ADDED Requirements

### Requirement: WASM Conformance Fixture Matrix
The runtime SHALL provide deterministic conformance fixtures for valid minimal, GenUI render, host import permission, resource exhausted, ABI mismatch, and unavailable provider scenarios.

#### Scenario: Fixture matrix covers positive and unavailable paths
- **WHEN** the runtime conformance harness builds the fixture matrix
- **THEN** it SHALL include all required fixture kinds with provider-neutral manifest, artifact, command, expected status, and reason code metadata.

### Requirement: Fixture Evaluation Uses Existing Certification Contracts
Conformance fixture evaluation SHALL reuse existing application certification and WASM admission contracts instead of inventing a second certification semantic.

#### Scenario: Fixture maps to application certification
- **WHEN** a conformance fixture is evaluated
- **THEN** the runtime harness SHALL adapt it into existing certification DTOs and include application certification status in the WASM conformance report.
