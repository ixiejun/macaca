# Foundation Filesystem Pack Design

## Context

`pack.foundation.filesystem.v1` provides scoped filesystem operations for
Macaca applications. It is a microkernel-compatible service capability:
applications declare the pack, admission resolves permissions, SDK clients build
typed commands, service runtime decorators enforce policy and observability, and
replaceable providers perform host or virtual filesystem work.

The pack must serve multiple consumers: WASM apps, YAML apps, GenUI apps,
headless agents, developer tools, document packs, media packs, repository packs,
and workflow artifacts. The design must therefore avoid business-specific
folders, application names, provider names, and workflow semantics.

## Supplier API Comparison

| Source API family | Relevant concepts | Macaca abstraction |
| --- | --- | --- |
| POSIX/Open Group | `open`, descriptor-based read/write, access modes, file status, directory entries, `rename`, `unlink`, errno | Logical file handles, byte-range commands, metadata DTOs, atomic move/delete semantics, structured error codes |
| Node.js `fs/promises` | `FileHandle`, `readFile`, `writeFile`, `appendFile`, `readdir`, `stat`, `copyFile`, `cp`, `rename`, `rm`, `watch`, streams | Async command/result DTOs, bounded streaming result chunks, recursive copy/delete flags, watch event stream commands |
| WASI filesystem | preopened directories, descriptor rights, guest portability, sandboxed host imports | App-scoped roots, capability-limited handles, WASM ABI host calls that route through service runtime |
| Web File System / OPFS | user or origin-scoped handles, writable streams, private storage, permission checks | Handle grants, private app workspace roots, explicit availability/permission diagnostics, transactional writes |

Design conclusion: Macaca should expose neither raw POSIX descriptors nor raw
Node/Web/WASI APIs. It should expose a stable pack contract with provider
adapters beneath it.

## Goals

- Provide scoped open, close, read, write, append, list, stat, mkdir, copy,
  move, delete, temp file, watch, snapshot, and restore operations.
- Support both path-like logical references and durable scoped handles.
- Support byte and text modes without leaking unbounded file contents into
  traces or diagnostics.
- Support atomic write/rename where the provider can guarantee it, and report
  `unsupported` where it cannot.
- Support mock, unavailable, local-host, virtual workspace, WASM preopen, and
  remote artifact providers through the same contract.
- Emit replayable audit evidence for every declaration, policy decision,
  provider call, result, watch subscription, snapshot, and restore.

## Non-Goals

- No direct application access to arbitrary host paths.
- No kernel-owned filesystem provider.
- No shell-owned filesystem behavior.
- No provider-specific path syntax in SDK APIs.
- No raw file bytes, raw manifests, secrets, package bytes, credentials, private
  keys, raw provider payloads, or unbounded directory listings in logs/traces.
- No application-specific workspace naming or special-case workflow folders.

## Ownership And Boundaries

- Pack id: `pack.foundation.filesystem.v1`.
- Family: `foundation`.
- Service owner: filesystem system service.
- Provider examples: local scoped workspace provider, WASM preopen provider,
  in-memory mock provider, unavailable provider, remote artifact provider.
- SDK surface: `sdk.packs.foundation.filesystem`.
- Command namespace: `filesystem.*`.
- Microkernel ownership: identity, service-call evidence, policy facade,
  resource primitives, trace/audit primitives only.
- Application framework ownership: manifest declarations, app-scoped permission
  declarations, effective capability projection, WASM ABI import exposure.
- Runtime-host ownership: provider registration, host path sandboxing,
  decorators, unavailable provider composition.

## Command Surface

| Command | Supplier analogs | DTO notes | Side effects |
| --- | --- | --- | --- |
| `filesystem.open_handle` | POSIX `open`, Node `fs.open`, Web file handles, WASI descriptors | logical path/root id, access mode, create mode, conflict mode, handle ttl | Optional create/truncate |
| `filesystem.close_handle` | descriptor close / handle dispose | handle id, close reason | Releases handle lease |
| `filesystem.read_file` | POSIX `read`, Node `readFile`, Web `getFile` | handle or logical path, range, max bytes, encoding, checksum option | No |
| `filesystem.write_file` | POSIX `write`, Node `writeFile`, writable streams | handle/path, content reference, encoding, atomic flag, expected revision | Yes |
| `filesystem.append_file` | Node `appendFile`, POSIX append mode | handle/path, content reference, max append size | Yes |
| `filesystem.list_directory` | POSIX directory entries, Node `readdir` | path/handle, recursive flag, page token, max entries, metadata projection | No |
| `filesystem.stat_path` | POSIX `stat`, Node `stat` | path/handle, follow symlinks flag, revision fields | No |
| `filesystem.create_directory` | POSIX `mkdir`, Node `mkdir` | path, recursive flag, conflict mode | Yes |
| `filesystem.copy_path` | Node `copyFile` / `cp` | source, destination, recursive flag, preserve metadata, conflict mode | Yes |
| `filesystem.move_path` | POSIX `rename`, Node `rename` | source, destination, atomic preference, conflict mode | Yes |
| `filesystem.delete_path` | POSIX `unlink`, Node `rm` | path/handle, recursive flag, tombstone option, expected revision | Yes |
| `filesystem.create_temp` | Node `mkdtemp`, temp file APIs | namespace, ttl, size budget, cleanup policy | Yes |
| `filesystem.watch_path` | Node `watch`, file event APIs | path/handle, recursive flag, event filter, stream budget | Starts stream |
| `filesystem.snapshot_tree` | backup/snapshot APIs | root handle/path, include filters, max bytes, retention policy | Records snapshot |
| `filesystem.restore_snapshot` | restore APIs | snapshot id, target root, conflict mode, dry-run flag | Yes |

## DTO Model

Core DTOs:

- `FilesystemRootRef`: app workspace, session workspace, package artifact,
  temporary namespace, WASM preopen, user-granted handle, or remote artifact root.
- `FilesystemPathRef`: root ref plus normalized relative path. Absolute host
  paths are rejected outside provider-private DTOs.
- `FilesystemHandleRef`: opaque handle id, root id, access mode, expiry,
  revision, and trace binding.
- `FilesystemAccessMode`: read, write, append, create, truncate, metadata,
  list, delete, watch, snapshot, restore.
- `FilesystemConflictMode`: fail, overwrite, create_new, merge_directory,
  tombstone.
- `FilesystemContentRef`: inline bounded bytes, text, blob/artifact id, stream
  id, or provider-local temporary reference.
- `FilesystemMetadata`: file type, size, content hash when requested, revision,
  modified time, provider capability flags, and sanitized permission summary.
- `FilesystemError`: denied, not_found, already_exists, conflict,
  invalid_path, invalid_handle, quota_exceeded, too_large, unsupported,
  unavailable, provider_failure.

## Permission And Policy Model

Permission scopes:

- `filesystem.read`
- `filesystem.write`
- `filesystem.append`
- `filesystem.list`
- `filesystem.metadata`
- `filesystem.copy`
- `filesystem.move`
- `filesystem.delete`
- `filesystem.watch`
- `filesystem.temp`
- `filesystem.snapshot`
- `filesystem.restore`

Policy rules:

- Every command is scoped to app id, tenant id, session id, task id, root id,
  handle id, and trace id when available.
- Read operations require declared roots and max byte limits.
- Write/append/copy/move/delete/restore require side-effect policy and resource
  reservation before provider calls.
- Delete/overwrite/restore across non-temporary roots requires approval unless a
  policy explicitly marks the root as automation-safe.
- Watch commands require stream budget, timeout, and cancellation support.
- Snapshot and restore commands require retention policy and replay metadata.
- Provider errors must be normalized into Macaca error DTOs without leaking raw
  host paths or provider payloads.

## SDK And Developer Documentation

SDK discovery returns command schemas, root types, permission scopes, policy
templates, provider availability, health, examples, docs link, and unavailable
diagnostics.

Required developer guide:

- Path: `docs/developer-packs/foundation/filesystem.md`.
- Content: manifest declarations, permission scopes, root/handle model, command
  DTOs, result DTOs, error model, examples, watch streams, snapshots, unavailable
  diagnostics, provider replacement, trace/audit fields, and security guidance.
- Examples: minimal read/list/write flow, safe atomic write flow, unavailable
  provider diagnostics, denied delete flow, and WASM app handle usage.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `filesystem_pack_declared`
- `filesystem_pack_admission_validated`
- `filesystem_pack_policy_decision`
- `filesystem_pack_handle_opened`
- `filesystem_pack_handle_closed`
- `filesystem_pack_service_call_requested`
- `filesystem_pack_service_call_succeeded`
- `filesystem_pack_service_call_failed`
- `filesystem_pack_watch_started`
- `filesystem_pack_watch_stopped`
- `filesystem_pack_snapshot_recorded`
- `filesystem_pack_restore_requested`
- `filesystem_pack_unavailable`

Events include identifiers, command name, root kind, handle id hash, path hash,
policy decision, provider class, byte counters, entry counters, revision,
latency, and bounded error code. Events do not include raw paths when they would
expose user data; use normalized relative paths only when the root policy allows
it, otherwise use path hashes.

Health checks include provider registered state, root availability, max file
size, max directory page, stream support, watch support, snapshot support,
atomic write support, and unavailable reasons.

Snapshots include descriptor version, provider class, root availability, open
handle count, active watch count, policy template hash, resource counters, and
sanitized replay references.

## Implementation Slices

1. Contract slice: DTOs, descriptor, command schema, result schema, error schema,
   permission scopes, policy template, provider capability schema.
2. Admission slice: root declarations, required/optional pack behavior,
   permission validation, lifecycle validation, service mapping validation.
3. Service slice: filesystem service trait/provider interface, unavailable
   provider, mock provider, local scoped provider, lifecycle/health/snapshot.
4. SDK slice: discovery, command builders, handle helpers, stream helpers,
   unavailable diagnostics, docs link.
5. WASM/app-runtime slice: expose only declared root/handle operations through
   host imports that route to service runtime.
6. Observability slice: trace/audit events, redaction, replay tests, health
   snapshots.
7. Developer-docs slice: complete `docs/developer-packs/foundation/filesystem.md`
   and link it from catalog metadata.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders only.
- **Command**: every operation is a typed command/result.
- **Adapter/Bridge**: POSIX-like, Node-like, WASI-like, web-like, local, remote,
  mock, and unavailable providers adapt to one contract.
- **Strategy**: provider selection, conflict handling, unavailable behavior, and
  atomic-write strategy are replaceable.
- **Decorator**: policy, trace, resource, entitlement, approval, metering, and
  redaction wrap every call.
- **Specification**: root declarations, command schemas, permission scopes,
  provider capabilities, and path rules are executable validators.
- **Observer**: watch events, audit events, health changes, and service-call
  events are subscribable.
- **Memento**: snapshots and effective capability reports are replayable.

## Risks And Mitigations

- Risk: raw host paths leak into app or audit surfaces.
  Mitigation: apps use root/path refs and opaque handles; traces use root ids and
  path hashes unless policy allows normalized relative paths.
- Risk: filesystem pack becomes a generic escape hatch for any host access.
  Mitigation: declared roots, permission scopes, approval policy, and provider
  capability checks gate every command.
- Risk: SDK helpers bypass service runtime.
  Mitigation: helpers only build canonical service-call commands; no-direct
  provider gates cover SDK, shells, and application framework.
- Risk: recursive operations create unbounded output or side effects.
  Mitigation: page tokens, max bytes, max entries, resource reservations,
  dry-run restore, and idempotency keys.
- Risk: WASM apps bypass policy through host imports.
  Mitigation: WASM host imports route through the same service command path and
  require effective capability membership.
