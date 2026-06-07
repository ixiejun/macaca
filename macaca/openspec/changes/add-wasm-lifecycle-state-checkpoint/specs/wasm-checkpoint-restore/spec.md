## ADDED Requirements
### Requirement: WASM checkpoint and restore use sanitized mementos
Macaca SHALL represent checkpoint and restore through bounded mementos that contain lifecycle, artifact, ABI, and compatibility metadata without raw guest memory dumps.

#### Scenario: Checkpoint creation
- **WHEN** a traced WASM session requests a checkpoint
- **THEN** the runtime SHALL return a checkpoint memento containing application id, ability id, lifecycle state, artifact id, artifact hash prefix, ABI version, timestamp, and sanitized metadata.

#### Scenario: Restore compatibility check
- **WHEN** a restore request carries an ABI version or artifact hash that is incompatible with the active session
- **THEN** the runtime SHALL fail closed with a structured `abi_mismatch` result and SHALL NOT change lifecycle state.

#### Scenario: Raw memory exclusion
- **WHEN** checkpoint metadata or restore metadata contains raw memory, prompt, payload, secret, API key, or environment markers
- **THEN** the runtime SHALL redact or omit that material from returned mementos and logs.
