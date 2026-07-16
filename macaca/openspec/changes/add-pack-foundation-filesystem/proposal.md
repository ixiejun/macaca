# Change: Add Foundation Filesystem Pack

## Why

Developers need `pack.foundation.filesystem.v1` as a real filesystem capability,
not a catalog label. Applications must be able to declare scoped filesystem
access, inspect the callable command surface, read/write/list/stat/copy/move
files through typed service calls, receive explicit denied/unavailable
diagnostics, and audit every side effect.

This pack is foundational because many future packs depend on file handles,
artifact storage, document conversion, media processing, repository operations,
and app-owned workspace state. If filesystem access is shallow or bypasses the
service runtime, every higher-level pack will inherit unsafe direct host access.

## Supplier And Platform API Research

The proposal is derived from a capability-by-capability comparison of mature
filesystem APIs:

- POSIX/Open Group system interfaces: `open`, read/write by descriptor, file
  status flags, file descriptors, directory traversal, metadata, rename,
  unlink, permissions, and errno-style structured failures.
- Node.js `fs` and `fs/promises`: async file handles, `readFile`, `writeFile`,
  `appendFile`, `readdir`, `stat`, `copyFile`, `cp`, `rename`, `rm`, `watch`,
  streams, and callback/promise separation.
- WASI filesystem model: sandboxed preopened directories, descriptor-oriented
  host calls, capability-based access, and portable WASM guest filesystem
  imports.
- Web File System / Origin Private File System: user-mediated handles,
  origin-scoped private storage, file/directory handles, writable streams, and
  explicit permission/availability behavior.

Macaca borrows the stable concepts, not the provider APIs directly:

- use scoped logical handles instead of raw host paths;
- make every operation a typed command/result;
- model errors as structured `denied`, `not_found`, `already_exists`,
  `conflict`, `quota_exceeded`, `unsupported`, `unavailable`, and `failure`;
- require trace, policy, resource, entitlement, and optional approval before
  side effects;
- expose provider capability metadata without exposing host implementation
  details.

## What Changes

- Define `pack.foundation.filesystem.v1` as the canonical app-facing filesystem
  pack.
- Add an industrial command surface covering handle open/close, read/write,
  append, directory list, metadata, create directory, copy, move, delete,
  temporary files, snapshots, restore, and watch.
- Define provider-neutral DTO requirements for paths, handles, byte ranges,
  encodings, metadata, recursive operations, conflict strategy, idempotency,
  watch streams, and snapshot references.
- Define permission scopes for read, write, list, metadata, delete, copy, move,
  watch, temporary storage, snapshot, and restore.
- Require a detailed developer guide under `docs/developer-packs/foundation/filesystem.md`
  before this proposal can be marked complete.
- Keep implementation ownership in a filesystem system service or optional host
  filesystem provider; kernel, SDK, shells, and application framework remain
  provider-neutral.

## Impact

- Affected specs: `pack-foundation-filesystem`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs, descriptor validators, application
  admission, SDK discovery, SDK command helpers, service provider implementation,
  mock/unavailable providers, trace/audit event schema, replay tests, and
  dependency-boundary gates.
- Non-goals: raw unrestricted host path access, shell-owned filesystem
  semantics, SDK provider construction, provider-specific command branches, or
  application-specific workspace behavior.
