## ADDED Requirements

### Requirement: WASI is denied by default
Macaca SHALL disable WASI raw env, filesystem, and network access by default for WASM applications.

#### Scenario: No approved preopen exists
- **WHEN** a WASM application has no approved capability-scoped virtual preopen
- **THEN** the runtime policy SHALL deny raw WASI access
- **AND** logs SHALL NOT include raw paths, environment values, network targets, or secrets.

### Requirement: Preopen grants are capability scoped
Macaca SHALL model future WASI preopen grants as scoped virtual labels approved by policy, not as raw host paths.

#### Scenario: Grant is logged safely
- **WHEN** a preopen grant is evaluated
- **THEN** audit data SHALL include only the virtual label, capability scope, and reason code.
