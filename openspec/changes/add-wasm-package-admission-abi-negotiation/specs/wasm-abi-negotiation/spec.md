## ADDED Requirements

### Requirement: WASM ABI negotiation is explicit and fail-closed

Macaca SHALL negotiate WASM ABI compatibility using declared ABI requirement, supported ABI versions, required imports, exported abilities, and runtime provider capabilities.

#### Scenario: ABI version matches
- **WHEN** the package requires an ABI version supported by the host
- **THEN** negotiation SHALL return a compatible result with stable supported version metadata.

#### Scenario: ABI version mismatch fails closed
- **WHEN** the package requires an ABI version unsupported by the host
- **THEN** negotiation SHALL return an incompatible result with a traceable `abi_version_mismatch` reason code
- **AND** SHALL NOT fall back to YAML, native, or other privileged non-WASM execution paths.

### Requirement: Runtime capabilities match package requirements

Macaca SHALL compare package runtime requirements against provider-neutral runtime capabilities before admission passes.

#### Scenario: Provider cannot execute
- **WHEN** the selected runtime provider cannot execute WASM components
- **THEN** admission SHALL fail closed or report unavailable with a traceable `runtime_capability_missing` reason code.
