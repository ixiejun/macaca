## ADDED Requirements

### Requirement: WASM sandbox policy is deny-by-default
Macaca SHALL define a sandbox policy that denies raw environment, raw filesystem, raw network, unrestricted clocks, and unrestricted randomness by default.

#### Scenario: Raw host access is not enabled
- **WHEN** a default WASM runtime session is created
- **THEN** raw env, raw filesystem, and raw network access SHALL be disabled
- **AND** the policy state SHALL be auditable through sanitized metadata and logs.

### Requirement: Runtime guards are composable
Macaca SHALL enforce sandbox and resource decisions through composable runtime guards rather than application-specific conditionals.

#### Scenario: Guard rejects before execution
- **WHEN** a guard denies a dispatch request
- **THEN** guest code SHALL NOT be invoked
- **AND** the result SHALL include only stable identifiers and reason codes.
