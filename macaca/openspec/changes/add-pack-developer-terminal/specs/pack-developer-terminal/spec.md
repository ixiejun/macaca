## ADDED Requirements

### Requirement: Macaca SHALL provide the Developer Terminal Pack as a serviceized capability

Macaca SHALL provide `pack.developer.terminal.v1` as a provider-neutral industrial pack for bounded process/session execution, PTY interaction where supported, stdout/stderr streaming, stdin submission, terminal resize, process inspection, exit collection, cancellation, sanitized workdir snapshots, cleanup, and provider diagnostics. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.developer.terminal.v1` as required and terminal/process service provider is registered, healthy, entitled, workspace-scoped, host-capable, and policy-admissible
- **THEN** admission SHALL expose `pack.developer.terminal.v1` in the effective capability set with command schemas, permission scopes, workspace scope metadata, policy template hash, provider capability hash, health, and replay metadata
- **AND** SDK discovery SHALL mark callable `terminal.*` commands as available without exposing provider secrets, environment values, raw terminal output, private file content, raw provider payloads, or application-specific workflow names

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.developer.terminal.v1` as required but provider, host capability, workspace permission, entitlement, resource, approval, network, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, spawn a process, send stdin, cancel a process, snapshot a workdir, contact a network, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.developer.terminal.v1` as optional and the pack or a sub-capability is unavailable
- **THEN** admission SHALL produce a degraded effective capability memento naming unavailable commands and bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands while preserving discoverability and diagnostics

### Requirement: Terminal commands SHALL use typed canonical service calls

Every `pack.developer.terminal.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior. SDK, WASM ABI, shell, and application-framework helpers SHALL only build canonical service commands and SHALL NOT construct concrete terminal providers or call host process APIs directly.

#### Scenario: Read or inspect command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `terminal.inspect_provider`, `terminal.stream_output`, `terminal.inspect_process`, or `terminal.collect_exit` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and terminal/process service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers and bounded cursor metadata

#### Scenario: Spawn is planned before request
- **WHEN** an application wants to spawn a process or terminal session
- **THEN** Macaca SHALL require `terminal.plan_spawn` with command validation, shell-mode policy, cwd/workspace policy, environment policy, stdio policy, PTY profile, network policy, resource reservation, timeout, cancellation strategy, idempotency key, approval state where required, and provider capability validation
- **AND** `terminal.plan_spawn` SHALL be replay-addressable and SHALL NOT spawn a process

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, resource, quota, workspace, env, stream, stale-handle, provider capability, or timeout checks reject a `terminal.*` command
- **THEN** Macaca SHALL return a typed denied, validation, conflict, stale-handle, invalid-command, invalid-workdir, invalid-env, not-running, approval-required, quota, timeout, unavailable, or unsupported result before invoking the concrete provider
- **AND** the audit trail SHALL include only bounded reason codes and sanitized handles

### Requirement: Terminal DTOs SHALL model provider-neutral process and terminal concepts

`pack.developer.terminal.v1` SHALL define provider-neutral DTOs for terminal scope, provider capability, process spec, environment policy, workdir scope, PTY profile, spawn plan, terminal session, stream cursor, output chunk, stdin frame, signal intent, exit status, resource usage, snapshot handle, and diagnostics. Provider-specific fields SHALL be exposed only as bounded `adapter_metadata` guarded by capability hashes and SHALL NOT drive OS-layer routing branches.

#### Scenario: Provider capability is inspected
- **WHEN** `terminal.inspect_provider` is invoked for a provider or workspace scope
- **THEN** Macaca SHALL return provider-neutral `TerminalProviderCapability` metadata for spawn support, shell support, PTY support, stdin support, stream support, resize support, signal support, snapshot support, env support, cwd support, network modes, resource limits, lifecycle, health, and compatibility
- **AND** it SHALL include stable descriptor, provider capability, policy template, and compatibility hashes for validation and replay

#### Scenario: Process state is inspected
- **WHEN** `terminal.inspect_process` returns a running or completed session
- **THEN** the result SHALL use `TerminalSession`, stream handles, process state, started timestamp, resource counters, cancellation state, freshness metadata, and redaction class
- **AND** it SHALL NOT expose raw credentials, environment values, raw output, private file content, raw provider payloads, or host-specific private paths

#### Scenario: Provider-specific capability exists
- **WHEN** an active provider supports a process or terminal concept not present in the canonical DTO model
- **THEN** the provider MAY expose bounded `adapter_metadata` and compatibility diagnostics through `TerminalProviderCapability`
- **AND** the OS, SDK, shell, and generic application framework SHALL NOT branch on shell names, provider names, command names, container names, operating systems, or workflow-specific fields

### Requirement: Spawn, stdin, resize, cancellation, and snapshots SHALL be policy-safe and auditable

All terminal side effects SHALL use typed requests, policy checks, workspace scope validation, provider capability validation, resource reservations, idempotency where applicable, approval gates where required, and sanitized audit.

#### Scenario: Spawn request succeeds
- **WHEN** `terminal.plan_spawn` validates executable handle, argument vector, shell mode, cwd, env handles, stdio policy, PTY profile, timeout, cancellation strategy, network intent, resources, quota, and approvals
- **THEN** `terminal.spawn_request` MAY use the validated plan handle and idempotency key to request process/session creation
- **AND** Macaca SHALL record sanitized plan, request, process spec hash, provider capability hash, policy decision, audit reason, session handle, stream handles, and replay pointer

#### Scenario: Stdin is sent to a running process
- **WHEN** `terminal.send_stdin` is invoked with a process handle and stdin frame
- **THEN** Macaca SHALL validate stdin permission, process freshness, running state, payload bounds, sensitivity class, encoding, idempotency key, and provider capability before sending input
- **AND** traces, audits, snapshots, and SDK diagnostics SHALL use sanitized handles or bounded metadata rather than raw sensitive stdin payloads

#### Scenario: Terminal resize is requested
- **WHEN** `terminal.resize` is invoked for a PTY/TTY session
- **THEN** Macaca SHALL validate PTY support, resize support, process state, row/column bounds, provider capability, and permission before requesting resize
- **AND** it SHALL return typed unsupported diagnostics when the active provider or session does not support resize

#### Scenario: Cancellation escalates according to policy
- **WHEN** `terminal.cancel` requests graceful cancel, terminate, or force kill
- **THEN** Macaca SHALL validate cancellation permission, process state, grace period, signal class, escalation policy, approval state where required, and provider capability before requesting cancellation
- **AND** forceful cancellation escalation SHALL be approval-gated when policy requires approval

#### Scenario: Workdir snapshot handle is created
- **WHEN** `terminal.snapshot_workdir` is invoked
- **THEN** Macaca SHALL validate filesystem scope, workdir handle, retention, size class, file count class, redaction class, resource budget, and approval requirements
- **AND** it SHALL return a bounded `TerminalSnapshotHandle` rather than raw file contents in traces, audits, snapshots, examples, or diagnostics

### Requirement: Terminal streams SHALL be bounded, redacted, cursor-addressable, and replayable

`pack.developer.terminal.v1` SHALL model stdout, stderr, and combined output as bounded stream resources with cursors, chunk limits, retention policy, redaction, dropped-output counters, and replay pointers.

#### Scenario: Output is streamed
- **WHEN** `terminal.stream_output` is invoked for a running or completed process
- **THEN** Macaca SHALL return bounded `TerminalOutputChunk` records with stream kind, cursor, byte count, line count, sanitized payload handle or bounded snippet, redaction markers, truncation flags, dropped-output counters where applicable, and timestamp
- **AND** it SHALL enforce stream permission, page size, output byte quotas, retention, timeout, cancellation, and redaction

#### Scenario: Output exceeds retention or quota
- **WHEN** output exceeds configured stream byte, line, chunk, retention, or replay bounds
- **THEN** Macaca SHALL truncate, drop, or summarize according to policy and return typed stream-truncated or quota diagnostics
- **AND** it SHALL NOT expose unbounded output in logs, traces, audits, snapshots, SDK diagnostics, or examples

#### Scenario: Exit is collected
- **WHEN** `terminal.collect_exit` is invoked after process completion
- **THEN** Macaca SHALL return `TerminalExitStatus` with exit category, exit code or signal class, duration, resource usage, final stream cursors, diagnostics, and replay pointer
- **AND** it SHALL keep raw output behind stream handles and redaction policy

### Requirement: Terminal Pack SHALL enforce permissions, scopes, resources, entitlements, approvals, and redaction

`pack.developer.terminal.v1` SHALL enforce explicit permission scopes for provider inspection, spawn, stream reading, stdin writing, resize, process inspection, exit collection, cancellation, workdir snapshot, and session cleanup. Every command SHALL carry application id, tenant id, session id, task id, trace id, provider scope, workspace handle, process/session handle where applicable, and actor handle when available.

#### Scenario: Permission is missing
- **WHEN** an application invokes a `terminal.*` command without the required permission scope
- **THEN** Macaca SHALL return a typed denied result before provider invocation
- **AND** the denied result SHALL identify the missing permission scope using sanitized identifiers

#### Scenario: Resource budget is exceeded
- **WHEN** spawn, stream reading, stdin, resize, cancellation, snapshot, or cleanup exceeds process count, duration, CPU class, memory class, disk bytes, network bytes, stdout/stderr bytes, stdin bytes, stream retention, snapshot size, timeout, provider quota, or replay metadata budgets
- **THEN** Macaca SHALL return typed quota, timeout, cancellation, stream-truncated, or resource-denied diagnostics
- **AND** it SHALL preserve replayable audit evidence without raw output or provider payloads

#### Scenario: Sensitive operation requires approval
- **WHEN** policy marks secret-reference use, sensitive env keys, filesystem writes outside declared workspace scope, network access, privilege escalation, long-running processes, destructive commands, external side effects, terminal snapshots, or force-kill escalation as approval-required
- **THEN** Macaca SHALL return an approval-required result until a valid approval token is supplied
- **AND** no spawn, stdin send, resize, cancellation escalation, snapshot, network access, or filesystem mutation SHALL happen before approval

### Requirement: Terminal Pack SHALL expose industrial metadata and developer documentation

`pack.developer.terminal.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, approval rules, redaction profiles, provider capability hashes, SDK examples, lifecycle state, compatibility, health probes, snapshots, unavailable diagnostics, and documentation links. The implementation SHALL include detailed developer documentation at `docs/developer-packs/developer/terminal.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.developer.terminal.v1`
- **THEN** it SHALL return command namespace `terminal.*`, command schemas, permissions, provider support, PTY support, stream support, stdin support, resize support, signal support, snapshot support, examples, lifecycle, availability, health, diagnostics, compatibility metadata, redaction profiles, and documentation link
- **AND** examples SHALL use synthetic commands, workspaces, process handles, streams, snapshots, and exit states rather than provider names, real credentials, private environment values, host-specific paths, raw output, or application-specific workflows

#### Scenario: Developer documentation is complete
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/developer/terminal.md` SHALL document manifest declarations, required versus optional behavior, permissions, provider scopes, workspace scopes, process specs, shell mode, argument vectors, cwd, env, stdio, PTY profiles, streams, stdin, resize, cancellation, exit status, resource usage, snapshots, cleanup, command DTOs, result DTOs, idempotency, streaming/pagination, timeout/cancellation, redaction, approvals, unavailable diagnostics, provider replacement, trace/audit interpretation, conformance tests, and supplier/API mapping
- **AND** the guide SHALL be linked from SDK discovery metadata and the industrial pack catalog index

### Requirement: Terminal Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.developer.terminal.v1` SHALL emit sanitized trace and audit events for declaration, admission, provider inspection, spawn planning, spawn request, output streaming, stdin send, resize, process inspection, exit collection, cancellation, workdir snapshot creation, cleanup, policy decisions, service-call lifecycle, failures, unavailable states, and snapshots.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.developer.terminal.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, command availability, provider health, policy template hash, resource counters, bounded process/session summaries, stream cursor summaries, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, environment values, secret material, private file content, raw streams, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

#### Scenario: Replay reconstructs command evidence
- **WHEN** replay inspects a past `terminal.*` command
- **THEN** Macaca SHALL reconstruct descriptor version, command DTO hash, policy decision, resource decision, approval state, provider capability hash, spawn plan handle where applicable, stream cursor where applicable, result classification, and sanitized provider class metadata
- **AND** replay SHALL NOT require raw provider payloads, raw terminal output, private file content, credentials, environment values, or application-specific workflow code

### Requirement: Terminal implementation SHALL preserve Macaca boundaries

The `pack.developer.terminal.v1` implementation SHALL remain owned by terminal/process service providers and service-runtime contracts. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, supplier-specific, shell-specific, platform-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and serviceization gates scan the implementation
- **THEN** they SHALL find no concrete shell, PTY, SSH, Docker, IDE terminal, platform process, credential-manager, filesystem-provider, network-provider, or remote execution adapter imports in the microkernel, SDK helpers, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.developer.terminal.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, and bounded diagnostics rather than provider-specific business branches
