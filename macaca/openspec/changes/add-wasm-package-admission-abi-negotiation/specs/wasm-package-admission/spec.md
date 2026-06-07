## ADDED Requirements

### Requirement: WASM package admission validates artifact metadata

Macaca SHALL admit WASM component packages only through structured artifact descriptors that reference artifacts by id, location reference, digest, and signature metadata rather than raw bytes.

#### Scenario: Missing digest fails closed
- **WHEN** a WASM package admission request lacks an artifact digest
- **THEN** admission SHALL fail closed with a traceable `artifact_digest_missing` reason code
- **AND** the report SHALL NOT contain raw WASM bytes or raw manifest bodies.

#### Scenario: Artifact reference is metadata-only
- **WHEN** admission succeeds or fails
- **THEN** reports and logs SHALL include bounded artifact id/reference/digest algorithm metadata
- **AND** SHALL NOT include raw artifact bytes, raw payloads, secrets, env values, API keys, prompts, private keys, or unbounded provider output.

### Requirement: Required imports match declared permissions

Macaca SHALL verify that every non-optional WASM host import requirement has a matching declared permission before package admission can pass.

#### Scenario: Missing permission fails closed
- **WHEN** a WASM import requirement needs `macaca:storage/set` and the manifest does not declare the matching permission
- **THEN** admission SHALL fail closed with a traceable `import_permission_missing` reason code.
