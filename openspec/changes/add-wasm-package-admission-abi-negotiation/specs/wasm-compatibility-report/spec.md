## ADDED Requirements

### Requirement: WASM admission reports are sanitized mementos

Macaca SHALL produce a sanitized admission and compatibility report that can be displayed by Web/CLI, SDK, Store tooling, and certification workflows.

#### Scenario: Report captures audit evidence
- **WHEN** admission completes
- **THEN** the report SHALL include package id, runtime kind, ABI version, artifact id, status, reason codes, trace id when available, and diagnostics.

#### Scenario: Report excludes sensitive raw material
- **WHEN** admission produces diagnostics
- **THEN** diagnostics SHALL NOT contain raw WASM bytes, raw manifest bodies, raw host payloads, secrets, env values, API keys, private keys, prompts, raw signatures, or unbounded provider output.
