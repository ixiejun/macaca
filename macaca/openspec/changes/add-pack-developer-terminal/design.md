# Developer Terminal Pack Design

## Context

`pack.developer.terminal.v1` exposes terminal and process execution as a Macaca
OS serviceized capability. It lets applications request bounded command
execution, stream output, send input, resize PTY sessions, inspect state, cancel
or terminate work, collect exit diagnostics, and capture sanitized workdir
snapshots without embedding shell names, container runtimes, IDE APIs, platform
syscalls, or workflow-specific command semantics into generic OS layers.

Terminal execution is intentionally high-risk. It can mutate repositories,
access credentials, fork long-running processes, use large streams, contact
networks, and perform irreversible host side effects. The pack therefore treats
spawning as a planned side effect and all interactions as typed service commands
decorated by policy, entitlement, resource control, approval, redaction, trace,
audit, replay, and provider replacement.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| VS Code Terminal/Pseudoterminal | Terminal creation, PTY-like IO, dimensions, close events, shell integration metadata | Terminal session, PTY profile, stream cursor, resize command, process lifecycle event |
| Node.js `child_process` | Spawn/exec/execFile/fork, stdio streams, cwd, env, shell mode, signals, exit events, timeout/abort | Process spec, stdio policy, workdir scope, environment policy, signal intent, exit status |
| Python `subprocess` | `Popen`, args, stdin/stdout/stderr pipes, env, cwd, return codes, timeout, terminate/kill | Process handle, stream handles, stdin frame, exit diagnostics, cancellation policy |
| Docker Engine Exec API | Exec create, start/attach stream, resize TTY, inspect exec state | Provider-scoped terminal session, attach stream, resize, inspect, provider capability |

The pack exposes provider-neutral contracts. Provider adapters translate to host
process APIs, remote execution APIs, container exec APIs, or IDE terminals. OS
layers must not branch on shell names, provider names, command names, container
names, operating systems, or application workflows.

## Goals

- Provide stable pack id `pack.developer.terminal.v1` and command namespace
  `terminal.*`.
- Support provider inspection, spawn planning, spawn requests, output streaming,
  stdin frames, resize, process inspection, exit collection, cancellation,
  workdir snapshot handles, cleanup, health, snapshot, and replay.
- Preserve safety with command allowlist strategy, argument-vector validation,
  shell-mode policy, workdir scope, environment redaction, filesystem and network
  permissions, resource budgets, stream bounds, approval, and sanitized audit.
- Keep concrete terminal/process providers behind replaceable service providers.
- Require developer documentation at
  `docs/developer-packs/developer/terminal.md`.

## Non-Goals

- Do not implement concrete host shell, PTY, SSH, Docker, IDE, platform syscall,
  or remote execution providers in this proposal.
- Do not define application-specific build, test, deploy, package, repository,
  CI, support, release, or incident workflows.
- Do not execute filesystem, repository, CI, browser automation, or notification
  semantics directly; those belong to separate packs/services and may be linked
  through handles.
- Do not expose raw credentials, environment values, private file content, raw
  terminal streams, prompts, manifests, package bytes, private keys, signatures,
  or unbounded output in observability.
- Do not silently spawn commands, select shells, escalate privileges, contact
  networks, or mutate host resources without typed request, policy checks, and
  approval where required.

## Ownership And Boundaries

- Pack id: `pack.developer.terminal.v1`.
- Family: `developer`.
- Backing service owner: terminal/process service provider.
- SDK surface: `sdk.packs.developer.terminal`.
- Command namespace: `terminal.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, host process bridges,
  terminal/PTY bridges, optional remote/container bridges, decorators, and
  sanitized diagnostics through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `terminal.inspect_provider` | Inspect provider and host terminal/process capability | Returns sanitized command, PTY, stream, signal, snapshot, quota, and health metadata |
| `terminal.plan_spawn` | Plan a process/session spawn | Validates command policy, args, shell mode, cwd, env, stdio, PTY, resource, network, and approvals |
| `terminal.spawn_request` | Request spawn from a validated plan | Requires idempotency key, permission, provider state, resource reservation, and audit |
| `terminal.stream_output` | Subscribe to or fetch bounded stdout/stderr/combined output chunks | Requires stream permission, cursors, redaction, paging, and retention limits |
| `terminal.send_stdin` | Send bounded stdin frame to a running process/session | Requires stdin permission, process state validation, content bounds, and audit |
| `terminal.resize` | Resize a PTY/TTY session where supported | Requires PTY support, dimensions validation, and provider capability |
| `terminal.inspect_process` | Inspect process/session state | Returns state, started time, resource counters, stream cursors, and freshness |
| `terminal.collect_exit` | Collect exit status and final diagnostics | Returns exit code/signal/category, duration, resource usage, output handles, and replay metadata |
| `terminal.cancel` | Request graceful cancel, terminate, or kill according to policy | Requires cancellation strategy, grace period, approval where needed, and audit |
| `terminal.snapshot_workdir` | Create sanitized working-directory snapshot handle | Requires filesystem scope, retention, redaction, size limits, and approval where needed |
| `terminal.cleanup_session` | Release process/session resources and retained stream handles | Requires lifecycle state validation and snapshot/audit update |

Every command must define typed command DTOs, typed success results, typed
streaming/paged results, typed denied/unavailable/unsupported/conflict/
not-running/stale-handle/quota/timeout/cancellation/approval-required/failure
results, redaction profile, idempotency semantics for side effects, and replay
metadata.

## DTO Model

Core DTOs:

- `TerminalScope`: provider scope handle, workspace handle, credential reference,
  network policy, filesystem policy, permission state, rate-limit profile, and
  health.
- `TerminalProviderCapability`: provider class, spawn support, shell support,
  PTY support, stdin support, stream support, resize support, signal support,
  snapshot support, env support, cwd support, network modes, resource limits,
  lifecycle, and health.
- `TerminalProcessSpec`: executable handle, argument vector, shell mode, cwd
  handle, environment handles, stdio policy, PTY profile, timeout, cancellation
  strategy, network intent, and resource budget.
- `TerminalEnvironmentPolicy`: allowed env keys, secret-reference handles,
  inherited env policy, redaction class, and provider mapping hash.
- `TerminalWorkdirScope`: workspace handle, cwd handle, read/write class,
  mount/volume metadata, snapshot policy, and redaction class.
- `TerminalPtyProfile`: terminal kind, rows, columns, encoding, interactive flag,
  resize support, and compatibility hash.
- `TerminalSpawnPlan`: plan handle, process spec hash, policy decisions,
  resource reservation, required approvals, idempotency key, validation
  diagnostics, and provider capability hash.
- `TerminalSession`: session handle, process handle, state, provider scope,
  stream handles, started timestamp, resource counters, cancellation state,
  freshness, and redaction class.
- `TerminalStreamCursor`: stream handle, stream kind, offset/cursor, retention,
  chunk bounds, dropped-output counters, and replay pointer.
- `TerminalOutputChunk`: stream kind, cursor, byte count, line count, sanitized
  payload handle or bounded snippet, redaction markers, truncation flags, and
  timestamp.
- `TerminalStdinFrame`: process handle, payload handle, encoding, byte count,
  newline policy, sensitivity class, and idempotency key.
- `TerminalSignalIntent`: process handle, cancel mode, grace period, signal class,
  escalation policy, and approval state.
- `TerminalExitStatus`: process handle, exit category, exit code, signal class,
  duration, resource usage, stream final cursors, and diagnostics.
- `TerminalResourceUsage`: cpu time class, memory class, disk bytes class,
  network bytes class, process count, stream bytes, duration, and quota state.
- `TerminalSnapshotHandle`: workdir snapshot handle, size class, file count
  class, checksum handle, retention, redaction class, and replay pointer.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `terminal.provider.inspect`
- `terminal.spawn`
- `terminal.stream.read`
- `terminal.stdin.write`
- `terminal.resize`
- `terminal.process.inspect`
- `terminal.exit.collect`
- `terminal.cancel`
- `terminal.workdir.snapshot`
- `terminal.session.cleanup`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, workspace handle, process/session handle, and actor
  handle when available.
- Spawn requires plan/request separation, argument-vector validation,
  shell-mode policy, cwd/workspace policy, environment policy, filesystem
  policy, network policy, resource reservation, timeout, cancellation strategy,
  credential reference, idempotency key, and audit reason.
- Sensitive env keys, secret references, filesystem writes outside declared
  scope, network access, privilege escalation, long-running processes, terminal
  snapshots, and destructive or external side effects may require approval.
- Streaming requires bounded chunks, redaction, retention, dropped-output
  counters, cursor semantics, and output-size quotas.
- Remote operations require network permission, provider quota, timeout,
  cancellation, and structured unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
provider support, PTY support, stream support, stdin support, resize support,
signal support, snapshot support, permission scopes, policy templates, resource
limits, approval rules, provider capability hashes, health, compatibility,
diagnostics, examples, redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/developer/terminal.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, workspace scopes, process specs, shell mode, argument vectors,
  cwd, env, stdio, PTY profiles, streams, stdin, resize, cancellation, exit
  status, resource usage, snapshots, cleanup, provider capabilities, and
  unavailable states
- spawn plan/request lifecycle, command allowlist strategy, idempotency,
  version/freshness conflicts, output redaction, stream retention, timeout,
  cancellation, network policy, approvals, quotas, provider replacement,
  trace/audit interpretation, and conformance tests

Examples must use synthetic commands and handles. They must not include
application names, provider names, real credentials, private env values, private
file content, host-specific paths, or workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `terminal_pack_declared`
- `terminal_pack_admission_validated`
- `terminal_provider_inspected`
- `terminal_spawn_planned`
- `terminal_spawn_requested`
- `terminal_output_streamed`
- `terminal_stdin_sent`
- `terminal_resized`
- `terminal_process_inspected`
- `terminal_exit_collected`
- `terminal_cancel_requested`
- `terminal_workdir_snapshot_created`
- `terminal_session_cleaned_up`
- `terminal_pack_policy_decision`
- `terminal_pack_service_call_requested`
- `terminal_pack_service_call_succeeded`
- `terminal_pack_service_call_failed`
- `terminal_pack_unavailable`
- `terminal_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, command
availability, provider health, policy template hash, resource counters, bounded
process/session summaries, stream cursor summaries, and sanitized replay
pointers. Snapshots must exclude raw credentials, env values, secret material,
private file content, raw streams, prompts, manifests, package bytes, private
keys, signatures, raw provider payloads, and unbounded output.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: process providers, command validators, shell-mode policy,
  stream redaction, cancellation strategy, snapshot strategy, and unavailable
  behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  network policy, filesystem policy, env redaction, stream redaction, and
  mutation safety wrap service calls.
- **Specification**: admission validates provider scope, workspace support,
  command availability, permissions, command policy, cwd/env/schema, provider
  state, quota, and compatibility.
- **Observer**: process state, streams, exit events, health, trace, and audit
  events are subscribable.
- **Memento**: spawn plans, stream cursors, exit statuses, snapshots, and replay
  pointers preserve recovery state.
- **Abstract Factory**: concrete terminal/process providers are created only by
  approved runtime-host composition roots.

## Risks And Mitigations

- Risk: terminal pack becomes a hardcoded shell wrapper. Mitigation:
  provider-neutral process/session DTOs and Strategy adapters.
- Risk: command execution leaks secrets or private output. Mitigation: env
  handles, stream redaction, bounded chunks, and strict observability exclusions.
- Risk: unbounded processes consume host resources. Mitigation: resource
  reservation, timeout, cancellation, quotas, and cleanup lifecycle.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call process APIs directly.
- Risk: provider capability differences are hidden. Mitigation: explicit
  capability DTO, compatibility hashes, unavailable diagnostics, and conformance
  tests.
