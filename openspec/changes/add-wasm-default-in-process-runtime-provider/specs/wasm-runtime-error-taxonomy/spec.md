## ADDED Requirements

### Requirement: WASM runtime errors are provider-neutral and sanitized

Macaca SHALL map in-process engine compile, instantiate, invoke, trap, timeout, policy, and resource failures into provider-neutral error kinds and sanitized reports.

#### Scenario: Compile failure maps to compile_failed
- **WHEN** artifact bytes cannot be compiled
- **THEN** the provider SHALL return or record `compile_failed` without exposing raw engine output or raw bytes.

#### Scenario: Trap failure maps to trap
- **WHEN** an invoked export traps
- **THEN** the provider SHALL return a provider-neutral rejected result with `trap` reason code and sanitized diagnostics.

#### Scenario: Missing trace is rejected
- **WHEN** a session request or command lacks trace context
- **THEN** the provider SHALL fail closed with `missing_trace` and SHALL NOT compile, instantiate, or invoke guest code.
