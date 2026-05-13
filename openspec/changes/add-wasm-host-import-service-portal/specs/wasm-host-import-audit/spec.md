## ADDED Requirements

### Requirement: WASM host import audit is sanitized
Macaca SHALL emit sanitized audit metadata for host import requested, allowed, denied, unavailable, completed, and failed decisions.

#### Scenario: Denied import is audited
- **WHEN** an import is denied by trace, capability, policy, payload, or service availability checks
- **THEN** logs and result metadata SHALL include trace id when available, application id, ability id, import name, reason code, and bounded byte counts
- **AND** SHALL NOT include raw prompts, raw payloads, raw guest memory, raw WASM bytes, backend responses, secrets, environment values, file paths, network targets, or private keys.
