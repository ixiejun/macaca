## ADDED Requirements
### Requirement: Runtime harness validates WIT and ABI label consistency
Macaca SHALL provide deterministic toolchain fixtures that compare WIT canonical labels with provider-neutral Application ABI imports and exports.

#### Scenario: WIT labels match ABI
- **WHEN** the harness generates a WIT label report
- **THEN** every Application ABI v0 import/export SHALL have a matching canonical WIT label.

#### Scenario: Deterministic artifact fixture
- **WHEN** the harness builds a WASM artifact fixture
- **THEN** it SHALL include artifact id, artifact reference, digest metadata, ABI version, required imports, permissions, and service dependencies without raw WASM bytes.
