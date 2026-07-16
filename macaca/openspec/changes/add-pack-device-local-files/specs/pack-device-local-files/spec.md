## ADDED Requirements

### Requirement: Macaca SHALL provide Device Local Files as a serviceized industrial pack

Macaca SHALL provide `pack.device.local.files.v1` as a provider-neutral industrial pack for picker-mediated file and directory handles, scoped grants, metadata inspection, bounded reads, bounded writes, imports, exports, directory listing, transfer cancellation, revocation, and host status. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.device.local.files.v1` as required and the device local file service is registered, healthy, entitled, policy-admissible, host-enabled, and command-compatible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, host status, picker capabilities, grant persistence classes, transfer limits, policy template, availability, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, credentials, raw host paths, raw file contents, raw provider payloads, package bytes, or unbounded directory listings

#### Scenario: Required declaration is unavailable or disabled
- **WHEN** an application declares `pack.device.local.files.v1` as required but provider, command support, permission, entitlement, resource, host support, foreground state, picker availability, or host permission is absent
- **THEN** admission SHALL block readiness with structured unavailable, disabled, foreground-required, permission-prompt-required, or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to another provider, or fake success

#### Scenario: Optional declaration is degraded
- **WHEN** an application declares `pack.device.local.files.v1` as optional and the pack is unavailable, disabled, or command-limited
- **THEN** admission SHALL produce an explicit degraded effective capability report with bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Device Local Files SHALL expose supplier-grade provider-neutral commands

`pack.device.local.files.v1` SHALL expose typed commands for `local_files.request_open_handle`, `local_files.request_save_handle`, `local_files.request_directory_handle`, `local_files.inspect_handle`, `local_files.list_handles`, `local_files.revoke_handle`, `local_files.read`, `local_files.write`, `local_files.append`, `local_files.truncate`, `local_files.list_directory`, `local_files.import_file`, `local_files.export_file`, `local_files.cancel_transfer`, and `local_files.inspect_host`.

#### Scenario: Open picker returns scoped readable handles
- **WHEN** a declared and policy-allowed caller invokes `local_files.request_open_handle` with filters, max selection count, grant duration, and trace context
- **THEN** Macaca SHALL route the command through SDK/facade helpers into service runtime and the active local file provider
- **AND** the result SHALL contain opaque `LocalFileHandle` and `LocalFileGrant` DTOs without raw host paths

#### Scenario: Save picker returns writable destination
- **WHEN** a caller invokes `local_files.request_save_handle`
- **THEN** Macaca SHALL require suggested type/name metadata, write scope, overwrite policy, grant duration, and foreground picker policy
- **AND** it SHALL return a writable handle only after policy, approval, and resource checks succeed

#### Scenario: Directory picker enforces stricter policy
- **WHEN** a caller invokes `local_files.request_directory_handle`
- **THEN** Macaca SHALL require directory scope, max traversal depth, filters, grant duration, and explicit policy allowance
- **AND** denied directory access SHALL return typed denied diagnostics before provider dispatch when policy rejects it

#### Scenario: Handle inspection reveals grant state
- **WHEN** a caller invokes `local_files.inspect_handle`
- **THEN** Macaca SHALL return redacted metadata, permission state, grant scope, expiry, provider class, revoked state, and provider limitations
- **AND** it SHALL not expose raw path, raw provider payload, or unbounded file metadata

#### Scenario: Recent handle listing is redacted
- **WHEN** a caller invokes `local_files.list_handles`
- **THEN** Macaca SHALL return only app/session-visible handle summaries and grant states
- **AND** handles revoked, expired, or outside scope SHALL be omitted or reported with bounded diagnostics according to policy

#### Scenario: Revocation closes active transfers
- **WHEN** a caller invokes `local_files.revoke_handle`
- **THEN** Macaca SHALL mark the grant revoked, close active transfers, release resources, and emit sanitized audit evidence
- **AND** future operations with the handle SHALL return handle-revoked diagnostics

#### Scenario: Read returns bounded chunks
- **WHEN** a caller invokes `local_files.read`
- **THEN** Macaca SHALL enforce readable grant, offset, length, transfer budget, content policy, and redaction before provider dispatch
- **AND** the result SHALL return bounded bytes or content references with transfer metadata

#### Scenario: Write uses explicit write plan
- **WHEN** a caller invokes `local_files.write`, `local_files.append`, or `local_files.truncate`
- **THEN** Macaca SHALL require `LocalFileWritePlan` with mode, expected size, conflict behavior, checksum policy, and destructive-operation flag
- **AND** destructive or conflicting writes SHALL require policy allowance and approval when configured

#### Scenario: Directory listing is bounded
- **WHEN** a caller invokes `local_files.list_directory`
- **THEN** Macaca SHALL enforce depth, entry count, filters, symlink/alias policy, and redaction
- **AND** it SHALL return bounded `LocalFileDirectoryEntry` DTOs without raw path traversal output

#### Scenario: Import and export produce transfer evidence
- **WHEN** a caller invokes `local_files.import_file` or `local_files.export_file`
- **THEN** Macaca SHALL create a bounded `LocalFileTransfer` with direction, bytes transferred, size class, content scan status, checksum policy, and resource counters
- **AND** the operation SHALL be cancellable and replayable without storing raw contents in trace/audit

#### Scenario: Host status explains picker availability
- **WHEN** a caller invokes `local_files.inspect_host`
- **THEN** Macaca SHALL return provider class, picker availability, permission state, foreground requirement, supported commands, active grants, active transfers, disabled reason, and diagnostics
- **AND** disabled local file support SHALL not appear as fake empty success when explicit diagnostics are required

### Requirement: Device Local Files DTOs SHALL model handles, grants, transfers, and redaction safely

The pack SHALL define provider-neutral DTOs for local file handles, grants, metadata, filters, chunks, transfers, directory entries, write plans, host status, and structured errors. Provider adapters SHALL translate host-specific APIs into these DTOs and SHALL redact host paths and sensitive metadata by default.

#### Scenario: Handle is opaque and scoped
- **WHEN** a picker command succeeds
- **THEN** `LocalFileHandle` SHALL include opaque id, handle kind, grant id, redacted display name, type hints, size class, readable/writable flags, directory flag, provider class, expiry, and revoked state
- **AND** it SHALL not include raw host path by default

#### Scenario: Grant records lifecycle and policy
- **WHEN** a file grant is created or updated
- **THEN** `LocalFileGrant` SHALL include source command, scope, permissions, persistence class, expiry, foreground requirement, approval id, revocation state, and policy hash
- **AND** grant state SHALL be replayable without raw file contents

#### Scenario: Transfer records bounded progress
- **WHEN** a read/write/import/export transfer progresses
- **THEN** `LocalFileTransfer` SHALL record direction, state, bytes transferred, total size class, checksum policy, scan status, cancellation token, and resource counters
- **AND** raw file contents SHALL remain outside generic trace, audit, and snapshot records

#### Scenario: Structured errors are stable across providers
- **WHEN** providers return picker cancelled, permission prompt, foreground required, grant expired, handle revoked, read only, write conflict, file too large, directory traversal denied, content scan blocked, transfer cancelled, quota, or provider failure states
- **THEN** Macaca SHALL map them to stable `LocalFileError` variants
- **AND** provider-specific diagnostics SHALL be sanitized and bounded

### Requirement: Device Local Files SHALL enforce permission, policy, resource, entitlement, approval, scanning, and revocation

Every command in `pack.device.local.files.v1` SHALL run through permission, policy, resource, entitlement, content-scanning, approval, and revocation decorators before and during provider use.

#### Scenario: Missing permission denies before provider dispatch
- **WHEN** an application invokes a command without required scope such as `device.local_files.open`, `device.local_files.save`, `device.local_files.directory`, `device.local_files.read`, `device.local_files.write`, or `device.local_files.grant.manage`
- **THEN** Macaca SHALL return a typed denied result before invoking the concrete provider
- **AND** the audit event SHALL include the bounded missing-scope code

#### Scenario: Foreground picker is required
- **WHEN** a picker command is requested while the host/application is not foreground-visible and delegated file picking is not allowed
- **THEN** Macaca SHALL return foreground-required diagnostics before provider dispatch
- **AND** the result SHALL include host status and policy reason codes

#### Scenario: Content scan blocks transfer
- **WHEN** content scanning policy blocks an import, export, read, or write transfer
- **THEN** Macaca SHALL cancel the transfer, release resources, and return content-scan-blocked diagnostics
- **AND** the audit trail SHALL include bounded scan status without raw file content

#### Scenario: Destructive write requires approval
- **WHEN** a write plan overwrites, truncates, or otherwise destructively changes a file
- **THEN** Macaca SHALL require policy allowance and approval evidence when configured
- **AND** missing approval SHALL return destructive-approval-required diagnostics before provider write

#### Scenario: Revocation invalidates future operations
- **WHEN** permission, policy, session, task, or user action revokes a file grant
- **THEN** Macaca SHALL close active transfers, mark grants revoked, release resources, and reject future handle operations
- **AND** subsequent commands SHALL return handle-revoked diagnostics

### Requirement: Device Local Files SHALL preserve canonical service runtime execution

All callable operations SHALL traverse the canonical Macaca service path: application declaration, admission/effective capability projection, SDK/facade command construction, service runtime dispatch, decorators, provider adapter, structured result, trace/audit evidence, and replayable snapshot. SDK helpers SHALL NOT construct providers, expose raw paths, or create alternate execution paths.

#### Scenario: Command succeeds through the canonical path
- **WHEN** a declared and policy-allowed command is invoked
- **THEN** Macaca SHALL route it through SDK/facade helpers into service runtime dispatch and the active local file provider adapter
- **AND** trace evidence SHALL show declaration, admission, policy, entitlement, resource, provider selection, grant or transfer state if applicable, command result, and replay pointer events

#### Scenario: Provider is absent
- **WHEN** no provider is registered for `pack.device.local.files.v1`
- **THEN** the unavailable provider SHALL return structured unavailable diagnostics
- **AND** SDK discovery SHALL report unavailable state while preserving the same provider-neutral command/result contract

#### Scenario: Provider supports only a subset
- **WHEN** the active provider supports open handles but not directory handles, persistent grants, or writable streams
- **THEN** SDK discovery SHALL mark unsupported commands/features as non-callable
- **AND** direct invocation SHALL return typed unsupported diagnostics without falling through to application-specific logic

#### Scenario: Provider is replaced
- **WHEN** a host-native, browser, remote-host, plugin, mock, or unavailable provider is selected
- **THEN** callers SHALL observe the same provider-neutral DTO contract
- **AND** OS-layer code SHALL identify only provider class, descriptor version, grant class, and capability metadata in traces rather than branching on provider names

### Requirement: Device Local Files SHALL expose industrial SDK discovery and developer documentation

SDK discovery for `pack.device.local.files.v1` SHALL expose pack metadata, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, host status, picker capabilities, grant persistence classes, transfer limits, policy templates, examples, diagnostics, compatibility, and documentation links. The implementation SHALL provide detailed developer documentation under `docs/developer-packs/device/local-files.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.device.local.files.v1`
- **THEN** it SHALL return command namespace `local_files.*`, supported commands, required scopes, host status, picker capabilities, grant persistence classes, transfer limits, policy templates, examples, lifecycle, health, diagnostics, compatibility metadata, and documentation URL
- **AND** examples SHALL use generic synthetic handles rather than raw paths, application-specific workflows, or provider-name routing

#### Scenario: Documentation covers app developer usage
- **WHEN** a developer opens `docs/developer-packs/device/local-files.md`
- **THEN** the guide SHALL explain manifest declarations, required versus optional behavior, scopes, command DTOs, result DTOs, picker handles, grants, revocation, directory traversal, read/write/import/export, content scanning, unavailable diagnostics, trace/audit behavior, and replay workflow
- **AND** it SHALL include minimal app-facing examples that use synthetic handles and canonical SDK calls

#### Scenario: Documentation covers provider authors
- **WHEN** a provider author reads the guide
- **THEN** it SHALL document descriptor fields, host adapter responsibilities, grant/transfer state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy
- **AND** it SHALL forbid raw path exposure and application-specific business routing in provider-neutral layers

### Requirement: Device Local Files observability SHALL be sanitized, replayable, and auditable

The pack SHALL emit sanitized trace, audit, health, transfer, grant, snapshot, and replay evidence for declaration, admission, policy, entitlement, resource reservation, picker request, handle grant, handle revocation, transfer lifecycle, command failure, unavailable state, and snapshot recording.

#### Scenario: Successful command emits bounded evidence
- **WHEN** a local file command succeeds
- **THEN** Macaca SHALL emit sanitized events containing pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when available, provider class, handle kind, grant id hash, transfer id hash, direction, size class, policy decision, latency, and resource counters
- **AND** it SHALL exclude raw host paths, raw file contents, raw provider payloads, secrets, credentials, package bytes, and unbounded directory listings

#### Scenario: Transfer progress event is aggregated
- **WHEN** a transfer progresses
- **THEN** Macaca SHALL emit only bounded counters, direction, size class, scan status, transfer id hash, and resource counters
- **AND** raw bytes and raw file names SHALL remain outside generic trace/audit records unless explicitly permitted and bounded by policy

#### Scenario: Snapshot records grant summaries
- **WHEN** the service runtime records a local file snapshot
- **THEN** the snapshot SHALL include provider health, host status, supported command matrix, active grant summaries, active transfer summaries, transfer limit classes, policy template hash, unavailable diagnostics, and sanitized replay pointers
- **AND** it SHALL exclude raw paths, raw file contents, raw provider payloads, credentials, package bytes, and unbounded output

#### Scenario: Replay verifies grant lifecycle
- **WHEN** a session or task is replayed after refresh or restart
- **THEN** Macaca SHALL reconstruct the local file command, grant, and transfer chain from bounded trace/audit evidence
- **AND** replay diagnostics SHALL prove the commands used the canonical service runtime path without raw host paths or raw contents

### Requirement: Device Local Files implementation SHALL preserve Macaca architecture boundaries

The `pack.device.local.files.v1` implementation SHALL keep concrete host/browser/remote providers behind service/runtime provider adapters. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, provider-specific, host-specific, path-specific, or file-format-specific routing branches.

#### Scenario: Boundary gates scan imports
- **WHEN** dependency-boundary gates scan the implementation
- **THEN** they SHALL find no concrete local file provider, host file API, browser file API, path-based host filesystem API, or remote file client in the microkernel, SDK, shells, or generic application framework
- **AND** provider construction SHALL appear only in approved runtime composition roots or plugin/remote provider registration paths

#### Scenario: No-direct-provider-call gate scans commands
- **WHEN** no-direct-provider-call gates scan local file commands
- **THEN** every callable operation SHALL be reachable only through descriptor-owned service registrations and typed service runtime dispatch
- **AND** SDK helpers SHALL only build canonical service commands and opaque handles

#### Scenario: Pack remains separate from neighboring storage capabilities
- **WHEN** architecture review compares storage and file packs
- **THEN** local-files SHALL own host-selected handles, grants, transfers, imports, exports, directory listing, revocation, and host local-file status
- **AND** foundation filesystem, cloud storage, media processing, office parsing, package runtime, and application-specific file formats SHALL remain owned by their respective packs or services
