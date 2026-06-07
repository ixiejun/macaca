## ADDED Requirements

### Requirement: App host import exposes declared file service

Macaca SHALL route declared `service.file` calls from app-owned UI and WASM
application host imports to the registered file service provider when policy
allows the operation.

#### Scenario: Application writes a workspace file

- **GIVEN** an application manifest declares `service.file`
- **AND** policy allows a bounded workspace write
- **WHEN** the application calls `service.file/file.write` through the generic
  app host import path
- **THEN** Macaca routes the command to the file service provider
- **AND** the result includes trace, audit, and bounded file metadata
- **AND** Macaca does not use application-specific routing branches

#### Scenario: Application writes a nested workspace file

- **GIVEN** an application manifest declares `service.file`
- **AND** policy allows a bounded workspace write
- **AND** the requested relative path contains missing parent directories
- **WHEN** the application calls `service.file/file.write` with
  `create_parent_directories=true`
- **THEN** Macaca creates the missing parent directories under the registered
  application workspace
- **AND** Macaca writes the file through the registered file service provider
- **AND** Macaca rejects traversal, absolute paths, and disallowed symlink
  traversal before the side effect

#### Scenario: File service is intentionally unavailable

- **GIVEN** the runtime deployment has no file service provider or policy
  disables file writes
- **WHEN** an application calls `service.file/file.write`
- **THEN** Macaca returns a structured unavailable or denied result
- **AND** the result includes a sanitized reason code and trace id
- **AND** Macaca does not crash, silently fall back, or fake success
