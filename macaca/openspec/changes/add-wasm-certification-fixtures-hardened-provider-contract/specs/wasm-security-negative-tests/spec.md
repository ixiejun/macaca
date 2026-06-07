## ADDED Requirements

### Requirement: WASM Security Negative Cases
The runtime SHALL provide negative certification fixtures for raw env access, raw filesystem access, raw network access, missing trace, missing capability, oversized payload, and timeout/resource exhaustion.

#### Scenario: Negative cases fail closed
- **WHEN** hardened certification evaluates any security negative case
- **THEN** it SHALL return a failed report with a stable reason code before guest execution.

### Requirement: Negative Case Sanitization
Security negative diagnostics SHALL describe the rejected category without retaining unsafe input values.

#### Scenario: Unsafe negative inputs are redacted
- **WHEN** a negative case carries raw payload, secret, env, API key, or prompt markers
- **THEN** the report SHALL include only sanitized category labels and bounded reason codes.
