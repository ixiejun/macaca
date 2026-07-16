# Foundation Filesystem Pack Research

## Purpose

This note records the supplier/API research required before implementing
`pack.foundation.filesystem.v1`. The pack must expose a Macaca-owned filesystem
contract rather than copying POSIX, Node.js, WASI, or browser API shapes into
the OS developer surface.

## Source Baseline

- POSIX / The Open Group Base Specifications Issue 8:
  <https://pubs.opengroup.org/onlinepubs/9799919799/>
- POSIX `rename`:
  <https://pubs.opengroup.org/onlinepubs/9799919799/functions/rename.html>
- POSIX `readdir`:
  <https://pubs.opengroup.org/onlinepubs/9699919799/functions/readdir.html>
- Node.js `node:fs` and `node:fs/promises`:
  <https://nodejs.org/api/fs.html>
- WASI interfaces:
  <https://wasi.dev/interfaces>
- WASI filesystem reference:
  <https://wa.dev/wasi%3Afilesystem>
- Node.js WASI security notes:
  <https://nodejs.org/api/wasi.html>
- MDN File System API:
  <https://developer.mozilla.org/en-US/docs/Web/API/File_System_API>
- MDN Origin Private File System:
  <https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system>
- MDN `FileSystemFileHandle.createWritable()`:
  <https://developer.mozilla.org/en-US/docs/Web/API/FileSystemFileHandle/createWritable>
- MDN `FileSystemWritableFileStream.write()`:
  <https://developer.mozilla.org/en-US/docs/Web/API/FileSystemWritableFileStream/write>

## POSIX / Open Group Summary

POSIX provides the durable baseline for path-oriented file operations:
open/read/write, metadata queries, directory iteration, rename, unlink, and
errno-style failures. Macaca should borrow the following semantics:

- Filesystem effects are side effects over a named namespace and must be
  guarded by permissions on the target path and, for many operations, the parent
  directory.
- `rename` is a mutation with replacement and directory-specific constraints;
  it needs conflict behavior, same-object no-op behavior, symlink policy, and
  parent-directory authorization.
- `readdir` returns paged directory entries and may observe concurrent directory
  mutation inconsistently, so Macaca must expose bounded paging and freshness
  diagnostics instead of promising a globally stable listing by default.
- `stat`-style metadata belongs in a normalized metadata DTO with file kind,
  size class, timestamps when allowed, permissions/capability class, and
  provider-specific details redacted behind bounded attribution fields.
- POSIX errno values should not leak directly. They map to Macaca result classes
  such as `not_found`, `already_exists`, `invalid_path`, `denied`, `conflict`,
  `too_large`, `quota_exceeded`, `unsupported`, `unavailable`, and
  `provider_failure`.

## Node.js `fs` / `fs/promises` Summary

Node.js is useful because it combines POSIX-like operations with asynchronous
IO, file handles, recursive helpers, streams, metadata APIs, and watch support.
Macaca should borrow the following concepts:

- Separate handle-oriented operations from one-shot path operations.
- Async execution must be modeled as canonical service commands with bounded
  streams or artifact/content references, not direct event-loop callbacks in the
  SDK.
- Recursive copy/delete and directory creation require explicit recursive
  bounds, conflict modes, symlink policy, and resource reservations.
- Watch operations are long-lived leases with cleanup requirements and
  provider-specific reliability limitations; Macaca should expose watch
  capabilities and watch lifecycle states, not promise identical host behavior.
- Streams should become paged content references, chunk cursors, or bounded
  stream leases so traces and SDK diagnostics never contain raw file bytes or
  unbounded output.

## WASI Filesystem Summary

WASI is the closest model for capability-scoped guest filesystem access.
Macaca should borrow the following concepts:

- Filesystem access is capability-based. Guests should receive only declared
  preopened roots or equivalent root references; no ambient host filesystem
  access should be available.
- The application-facing ABI should talk in provider-neutral root/path/handle
  references. Host paths are implementation details and must not appear in
  traces, audits, snapshots, or generic SDK diagnostics.
- Descriptor rights map naturally to Macaca permission scopes such as read,
  write, append, list, metadata, copy, move, delete, watch, temp, snapshot, and
  restore.
- WASI's guest portability goal supports Macaca's unified execution path:
  YAML, WASM, GenUI, and headless applications must invoke the same service
  command surface rather than direct host filesystem adapters.
- WASI security notes also show that not every runtime provides a strong
  sandbox; Macaca must treat provider capability claims as inspected service
  diagnostics and must enforce policy/resource checks in the service path.

## Web File System / OPFS Summary

The browser File System API and OPFS show a user/origin-scoped handle model with
permission diagnostics and writable streams. Macaca should borrow the following
concepts:

- User-granted handles and origin/private handles are different root classes.
  Macaca should represent roots by kind, scope, retention, and authorization
  state rather than raw paths.
- Write streams can stage data and commit on close, which maps to Macaca atomic
  write strategy, conflict mode, and cleanup-on-cancel behavior.
- Permission state must be observable as a structured diagnostic before use.
  Missing, revoked, or prompt-required permission should produce explicit
  `denied` or `unavailable` results, not empty reads or fake success.
- OPFS-style private storage supports application-scoped persistence, but it
  must still obey quota, retention, backup/export, and trace/audit redaction
  policies.

## Macaca-Owned Abstractions

`pack.foundation.filesystem.v1` should define these provider-neutral concepts:

- `FilesystemRootRef`: opaque root identifier, root kind, application/session/
  tenant scope, retention class, capability flags, and authorization state.
- `FilesystemPathRef`: root ref plus normalized relative path components,
  redaction class, symlink policy, and path validation evidence.
- `FilesystemHandleRef`: opaque handle id, access mode, lifecycle state,
  lease/cancellation metadata, and replay pointer.
- `FilesystemAccessMode`: read, write, append, list, metadata, copy, move,
  delete, watch, temp, snapshot, and restore.
- `FilesystemConflictMode`: fail if exists, overwrite, append, create unique,
  atomic replace, merge directory, and provider unsupported.
- `FilesystemContentRef`: bounded content handle, byte range, content class,
  checksum when allowed, expiry, and retention policy.
- `FilesystemMetadata`: file kind, size class, timestamp class, permission class,
  checksum class, provider capability class, freshness, and attribution.
- `FilesystemWatchLease`: watch id, root/path scope, event filter, max duration,
  backpressure policy, cleanup state, and provider reliability class.
- `FilesystemSnapshotRef`: snapshot id, tree hash, scope, retention, restore
  policy, and replay pointer.
- `FilesystemProviderCapability`: supported commands, root kinds, max sizes,
  recursive support, watch support, snapshot support, atomic write support,
  symlink policy, health, and unavailable reasons.

## Rejected API Leakage

Macaca must not expose these provider-native shapes as stable SDK/ABI contracts:

- POSIX file descriptors, raw errno values, host absolute paths, numeric mode
  bits, or direct symlink-following behavior.
- Node.js callbacks, `fs.Stats`, raw `Buffer` streams, `fs.watch` event names,
  or platform-specific recursive option behavior.
- WASI descriptor internals, raw preopen host mappings, runtime-specific
  sandbox assumptions, or direct WIT type leakage in non-WASM SDKs.
- Browser `FileSystemHandle` objects, origin-specific persistence semantics,
  user picker UI semantics, or direct writable-stream objects.

Instead, all operations must enter through typed Macaca service commands with
trace context, policy checks, resource reservations, structured result
envelopes, sanitized audit events, and provider replacement support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
