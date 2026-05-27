## ADDED Requirements

### Requirement: Provider-backed Filesystem Service
The system SHALL provide `service.file` for workspace-scoped file read, write,
patch, copy, remove, metadata, directory listing, watch, unwatch, and diff
operations.

#### Scenario: Policy before file write
- **WHEN** an application requests a file write or patch
- **THEN** `service.file` SHALL run path, workspace, permission, resource, and
  audit decorators before side effects
- **AND** denied calls SHALL return structured denied results without modifying
  the filesystem

#### Scenario: File watch notification
- **WHEN** a watched file or directory changes
- **THEN** `service.file` SHALL emit bounded change notifications with watch id,
  sanitized paths, and trace refs

### Requirement: Provider-backed Process and PTY Service
The system SHALL provide `service.process` for command execution, process
spawning, PTY sessions, stdin writes, resize, termination, output subscription,
status, and background cleanup.

#### Scenario: Sandboxed command execution
- **WHEN** a model-initiated command is executed
- **THEN** `service.process` SHALL resolve sandbox and permission profile before
  spawning the process
- **AND** stdout/stderr SHALL stream as bounded output deltas or artifact refs

#### Scenario: Background process cleanup
- **WHEN** a thread requests background process cleanup
- **THEN** the service SHALL terminate owned processes, emit lifecycle events,
  and audit the cleanup result

### Requirement: Sandbox and Permission Profile Service
The system SHALL provide `service.sandbox` for permission profile resolution,
environment prepare, environment health, cleanup, and policy explanation.

#### Scenario: Optional sandbox provider absent
- **WHEN** Docker, SSH, remote, or OS-specific sandbox providers are not
  installed
- **THEN** `service.sandbox` SHALL return structured unavailable diagnostics
- **AND** base OS startup and unrelated workflows SHALL continue

#### Scenario: Runtime environment cleanup
- **WHEN** a sandboxed run completes or is cancelled
- **THEN** the service SHALL release resource leases, record cleanup status, and
  emit sanitized audit evidence
