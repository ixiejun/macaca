## ADDED Requirements

### Requirement: Macaca SHALL provide a supplier-grade Foundation Filesystem Pack

Macaca SHALL provide `pack.foundation.filesystem.v1` as a provider-neutral,
serviceized filesystem pack for scoped open, close, read, write, append, list,
metadata, directory creation, copy, move, delete, temporary storage, watch,
snapshot, and restore operations.

#### Scenario: Application declares scoped filesystem access
- **WHEN** an application declares `pack.foundation.filesystem.v1` with required
  filesystem roots and permission scopes
- **THEN** admission SHALL validate pack id, lifecycle, root declarations,
  permission scopes, policy bounds, service mappings, command schemas, and
  provider capability requirements
- **AND** admission SHALL produce an effective capability report with callable,
  denied, unsupported, and unavailable command states

#### Scenario: Required filesystem provider is unavailable
- **WHEN** `pack.foundation.filesystem.v1` is required but no admitted provider
  can satisfy the declared roots and commands
- **THEN** application readiness SHALL be blocked with structured unavailable
  diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to raw host paths, or
  fake success

#### Scenario: Optional filesystem provider is unavailable
- **WHEN** `pack.foundation.filesystem.v1` is optional and unavailable
- **THEN** admission SHALL mark the effective capability set as degraded
- **AND** SDK helpers and WASM host imports SHALL refuse to build callable
  service calls for unavailable commands

### Requirement: Filesystem commands SHALL use typed canonical service calls

Every `filesystem.*` operation SHALL be represented as a typed command/result
DTO and SHALL traverse the canonical service runtime path with trace, policy,
resource, entitlement, approval, health, snapshot, and structured error
behavior.

#### Scenario: Read command succeeds
- **WHEN** a declared and policy-allowed `filesystem.read_file` command is
  invoked with a valid root/path or handle reference
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the
  filesystem service provider
- **AND** it SHALL emit sanitized policy, service-call, result, and replay events
  with stable trace identifiers and bounded byte counters

#### Scenario: Write command is denied before side effects
- **WHEN** `filesystem.write_file`, `filesystem.append_file`,
  `filesystem.copy_path`, `filesystem.move_path`, `filesystem.delete_path`, or
  `filesystem.restore_snapshot` is rejected by permission, policy, approval,
  entitlement, or resource checks
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the
  concrete provider
- **AND** audit evidence SHALL include only bounded reason codes, root ids,
  handle hashes, path hashes, and policy decision metadata

#### Scenario: Provider lacks a requested filesystem feature
- **WHEN** a provider supports basic filesystem access but does not support
  `filesystem.watch_path`, `filesystem.snapshot_tree`, atomic writes, or
  recursive operations
- **THEN** Macaca SHALL return a typed unsupported result for that command
- **AND** SDK discovery SHALL report the command as non-callable for the current
  effective capability set

### Requirement: Filesystem access SHALL use scoped roots and opaque handles

`pack.foundation.filesystem.v1` SHALL expose app-scoped roots, normalized
relative path references, and opaque handle references. It SHALL NOT expose raw
unrestricted host paths to applications.

#### Scenario: Application opens a handle
- **WHEN** an application invokes `filesystem.open_handle` for a declared root
  and normalized relative path
- **THEN** Macaca SHALL return an opaque handle reference with access mode,
  expiry, revision, and trace binding
- **AND** subsequent handle-based calls SHALL still require policy and resource
  checks before provider execution

#### Scenario: Application sends an invalid path
- **WHEN** an application sends an absolute host path, path traversal, provider
  private path syntax, or a path outside the declared root
- **THEN** Macaca SHALL return a typed `invalid_path` or `denied` result before
  provider execution
- **AND** audit evidence SHALL avoid logging the raw rejected path when it may
  contain user-sensitive data

### Requirement: Filesystem metadata, watches, snapshots, and replay SHALL be bounded and sanitized

Macaca SHALL bound and sanitize filesystem metadata, directory listings, watch
events, snapshots, restore diagnostics, traces, and audit records.

#### Scenario: Directory listing is paged
- **WHEN** `filesystem.list_directory` is invoked on a large directory
- **THEN** Macaca SHALL return bounded pages with continuation tokens
- **AND** trace/audit evidence SHALL include entry counters and latency without
  storing unbounded directory output

#### Scenario: Watch stream is started and cancelled
- **WHEN** `filesystem.watch_path` starts a watch stream
- **THEN** Macaca SHALL reserve stream resources, emit a watch-start event, and
  return bounded watch events
- **AND** cancellation, timeout, provider failure, and session shutdown SHALL
  release resources and emit terminal watch events

#### Scenario: Snapshot is recorded
- **WHEN** `filesystem.snapshot_tree` records a snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, root
  id, policy template hash, bounded counters, and sanitized replay pointers
- **AND** it SHALL exclude raw file bytes, raw host paths, secrets, manifests,
  package bytes, credentials, private keys, raw provider payloads, and unbounded
  output

### Requirement: Filesystem implementation SHALL preserve Macaca boundaries

The filesystem implementation SHALL remain owned by the filesystem system
service and replaceable providers. The microkernel, SDK, shells, and generic
application framework SHALL remain provider-neutral and free of
application-specific filesystem routing.

#### Scenario: Boundary gates scan filesystem implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path
  gates scan the implementation
- **THEN** they SHALL find no concrete filesystem provider imports in the
  microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned
  service registrations and typed service commands

#### Scenario: WASM app uses filesystem host imports
- **WHEN** a WASM application invokes filesystem host imports
- **THEN** the host imports SHALL route through the same `filesystem.*` service
  command path used by SDK and YAML applications
- **AND** WASM code SHALL NOT receive direct host path access or bypass policy

### Requirement: Filesystem pack completion SHALL include developer documentation

The `pack.foundation.filesystem.v1` proposal SHALL NOT be marked complete until
the detailed developer guide exists and is linked from SDK discovery metadata.

#### Scenario: Developer reads filesystem pack documentation
- **WHEN** a developer opens `docs/developer-packs/foundation/filesystem.md`
- **THEN** the guide SHALL document manifest declaration, root and handle model,
  permission scopes, policy defaults, resource limits, approval cases, command
  DTOs, result DTOs, error DTOs, watch streams, snapshots, restore behavior,
  unavailable diagnostics, provider replacement, trace/audit fields, and examples
- **AND** examples SHALL use generic data and SHALL NOT hardcode application
  business logic, provider names, credentials, or workflow-specific paths
