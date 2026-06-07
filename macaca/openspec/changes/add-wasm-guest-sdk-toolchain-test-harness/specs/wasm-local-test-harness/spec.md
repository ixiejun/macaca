## ADDED Requirements
### Requirement: Local mock host imports share runtime contract vocabulary
Macaca SHALL provide local mock host import outcomes for success, denied, unavailable, and unsupported results using the same provider-neutral Application ABI and WASM host import metadata vocabulary as the real runtime host import bridge.

#### Scenario: Mock import success
- **WHEN** a traced command matches an allowed mock host import
- **THEN** the harness SHALL return `Ok` with sanitized output, trace metadata, and reason code `mock_import_completed`.

#### Scenario: Mock import failure
- **WHEN** a command is denied, unavailable, unsupported, or missing trace
- **THEN** the harness SHALL fail closed with structured status and sanitized reason metadata.
