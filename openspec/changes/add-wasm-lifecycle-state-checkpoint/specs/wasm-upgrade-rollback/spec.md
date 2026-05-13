## ADDED Requirements
### Requirement: WASM upgrade and rollback decisions are compatibility based
Macaca SHALL decide WASM upgrade and rollback requests from artifact id, artifact hash, ABI version, and compatibility metadata rather than application names, driver names, workflow names, or business-specific branches.

#### Scenario: Compatible upgrade
- **WHEN** a traced upgrade request references a new artifact with a compatible ABI version
- **THEN** the runtime SHALL return an upgrade report containing source artifact metadata, target artifact metadata, ABI compatibility, trace id, and sanitized metadata.

#### Scenario: Incompatible upgrade
- **WHEN** a traced upgrade request references a target artifact whose ABI version is incompatible with the active session
- **THEN** the runtime SHALL fail closed with reason code `abi_mismatch`.

#### Scenario: Rollback report
- **WHEN** a traced rollback request references a prior checkpoint memento
- **THEN** the runtime SHALL return a rollback report tied to checkpoint metadata and SHALL NOT include raw guest memory.
