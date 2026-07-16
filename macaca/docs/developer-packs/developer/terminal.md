# Developer Terminal Pack

`pack.developer.terminal.v1` provides provider-neutral terminal provider
inspection, spawn planning, spawn requests, output streaming, stdin writes,
resize, process inspection, exit collection, cancellation, workdir snapshots,
and session cleanup.

The pack is intentionally split into plan and request commands for side effects.
Applications work with process, stream, cursor, exit, usage, and snapshot
handles; traces never expose raw terminal output or filesystem content.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.developer.terminal.v1"]
```

Unavailable optional declarations report
`developer_terminal_provider_not_installed`. Required declarations block
readiness until a descriptor-compatible terminal provider is installed.

## Permission Scopes

- `terminal.provider.inspect`, `terminal.spawn`, `terminal.stream.read`,
  `terminal.stdin.write`, and `terminal.resize`.
- `terminal.process.inspect`, `terminal.exit.collect`, `terminal.cancel`,
  `terminal.workdir.snapshot`, and `terminal.session.cleanup`.

## Commands

- `terminal.inspect_provider`, `terminal.plan_spawn`,
  `terminal.spawn_request`, and `terminal.stream_output`.
- `terminal.send_stdin`, `terminal.resize`, `terminal.inspect_process`,
  `terminal.collect_exit`, `terminal.cancel`,
  `terminal.snapshot_workdir`, and `terminal.cleanup_session`.

## DTOs And Results

Core DTOs include `TerminalScope`, `TerminalProviderCapability`,
`TerminalProcessSpec`, `TerminalEnvironmentPolicy`, `TerminalWorkdirScope`,
`TerminalPtyProfile`, `TerminalSpawnPlan`, `TerminalSession`,
`TerminalStreamCursor`, `TerminalOutputChunk`, `TerminalStdinFrame`,
`TerminalSignalIntent`, `TerminalExitStatus`, `TerminalResourceUsage`, and
`TerminalSnapshotHandle`. Result statuses cover success, streaming, paging,
partial results, denied, unavailable, unsupported, conflict, not running, stale
handles, invalid commands, invalid workdirs, invalid environments, stream
truncation, quota, timeout, cancellation, approval required, and provider
failure.

## Command DTO Details

Every command wrapper carries a `DeveloperCommandEnvelope`:

- `subject_ref`: provider scope, process spec, spawn plan, session, stream
  cursor, stdin frame, signal intent, exit status, resource usage, or snapshot
  subject.
- `parameters`: reference-only arguments such as `process_spec_ref`,
  `spawn_plan_ref`, `session_ref`, `cursor_ref`, `stdin_ref`, `signal_ref`,
  `workdir_scope_ref`, `snapshot_ref`, and `approval_ref`.
- `cursor` and `page_size`: bounded output streaming and historical stream
  reads.
- `idempotency_key`: stable key for spawn, stdin, resize, cancel, snapshot, and
  cleanup requests.

Result envelopes return `status`, optional `data`, optional paged data, and a
trace-safe error. Spawn is split into planning and request phases; raw output is
returned only through bounded chunks and cannot enter traces. Cleanup commands
are idempotent.

## Supplier/API Mapping

- POSIX process, signal, exit status, working directory, environment, resource
  usage, and PTY concepts map to terminal process, signal, exit, usage, and PTY
  DTOs.
- Windows process, environment block, console session, signal-like control
  events, and exit status concepts map to the same provider-neutral handles.
- Container, sandbox, and remote-executor concepts map to process-runtime and
  stream-runtime provider classes without exposing provider-specific APIs.
- Shell command pass-through, raw credentials, host-specific shell behavior,
  application workflows, and raw filesystem snapshots are not OS semantics.

## Examples

Plan a spawn:

```json
{
  "subject_ref": "terminal-scope:demo",
  "parameters": {
    "process_spec_ref": "process-spec:demo",
    "workdir_scope_ref": "workdir:workspace"
  },
  "idempotency_key": "terminal-demo-plan-spawn"
}
```

Read output from a bounded cursor:

```json
{
  "subject_ref": "terminal-session:demo",
  "cursor": "stream-cursor:demo",
  "page_size": 50
}
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.developer.terminal.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "developer_terminal_provider_not_installed"
}
```

## App-Facing Example Matrix

Generic examples cover provider capability inspection, spawn planning, spawn
request planning, bounded output streaming, stdin request planning, PTY resize
planning, process-state inspection, exit-status collection, cancellation,
snapshot-handle creation, and session cleanup. All examples use synthetic
terminal scope, process spec, session, stream cursor, snapshot, and workdir
scope refs.

Diagnostic examples cover unavailable provider, missing workspace permission,
invalid command, invalid environment, invalid workdir, stream truncated,
stdin denied, resize unsupported, cancellation approval, provider quota,
timeout, network denied, and snapshot denied outcomes. Diagnostics must use
provider-neutral reason codes and must not include provider names, credentials,
private env values, private file content, raw output, stdin payloads, or
workflow-specific conventions.

## Provider Conformance

Provider authors must prove descriptor completeness, process lifecycle state,
spawn/request separation, environment redaction, cwd scope enforcement, stream
truncation, stdin policy, signal safety, resource metering, snapshot handles,
cleanup idempotency, policy hooks, sanitized trace/audit events, unavailable
behavior, snapshot/replay metadata, and no raw output, stdin payload,
environment secret, raw file content, credential, or provider payload leakage.

## Trace And Audit

Trace and audit events may include process refs, stream cursor refs, bounded
output counters, workdir scope refs, exit metadata, snapshot handles, status,
and trace-safe error codes. They must not include raw output, stdin payloads,
raw environment values, secrets, raw file contents, credentials, or provider
payloads.

## Provider Replacement

Provider classes are descriptor labels such as `process-runtime`,
`stream-runtime`, `snapshot-runtime`, `mock`, and `unavailable`. Concrete shell,
PTY, process, stream, and filesystem snapshot implementations stay behind
service adapters.
