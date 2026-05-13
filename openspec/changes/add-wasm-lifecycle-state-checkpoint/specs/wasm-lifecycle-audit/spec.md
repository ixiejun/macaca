## ADDED Requirements
### Requirement: WASM lifecycle operations emit sanitized audit metadata
Macaca SHALL emit sanitized audit records and logs for lifecycle requested, completed, failed, unsupported, drained, checkpointed, restored, upgraded, and rolled-back operations.

#### Scenario: Lifecycle audit success
- **WHEN** a traced lifecycle operation completes
- **THEN** the runtime SHALL record an audit event with operation, from-state, to-state, session id, application id, ability id, trace id, reason code, and sanitized metadata.

#### Scenario: Lifecycle audit failure
- **WHEN** a lifecycle operation is rejected, unsupported, unavailable, or fails
- **THEN** the runtime SHALL record an audit event with a fail-closed reason code and SHALL NOT record raw command payload, raw guest memory, prompts, secrets, API keys, or environment values.
