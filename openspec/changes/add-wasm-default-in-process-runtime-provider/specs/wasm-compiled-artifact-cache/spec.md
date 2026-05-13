## ADDED Requirements

### Requirement: Compiled artifact cache uses deterministic safe keys

Macaca SHALL compute compiled artifact cache keys from artifact digest, ABI version, engine capability fingerprint, and execution profile fingerprint without storing raw WASM bytes.

#### Scenario: Cache key is deterministic
- **WHEN** the same artifact digest, ABI version, engine capabilities, and execution profile are used twice
- **THEN** the compiled artifact cache key SHALL be identical.

#### Scenario: Cache records hit and miss
- **WHEN** a provider compiles an artifact
- **THEN** it SHALL record cache hit or miss as sanitized diagnostics/log metadata
- **AND** cache reports SHALL NOT include raw WASM bytes or raw payloads.
