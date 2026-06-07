## ADDED Requirements

### Requirement: WASM resource audit events are sanitized
Macaca SHALL emit sanitized resource audit reports for policy allow, deny, throttle, timeout, and exhaustion decisions.

#### Scenario: Deny event is emitted
- **WHEN** runtime policy denies a session or dispatch request
- **THEN** the audit report SHALL include trace id when available, application id, ability id, runtime kind, scope, reason code, and bounded metadata
- **AND** the report SHALL NOT include raw WASM bytes, raw guest memory, raw command payloads, raw env values, raw filesystem paths, raw network addresses, secrets, prompts, private keys, or engine internals.
