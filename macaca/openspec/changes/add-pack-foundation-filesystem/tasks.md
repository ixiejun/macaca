## 1. Supplier API Research And Scope

- [x] 1.1 Read and summarize POSIX/Open Group filesystem operations relevant to
  open/read/write/stat/readdir/rename/unlink/error semantics.
- [x] 1.2 Read and summarize Node.js `fs` and `fs/promises` operations relevant
  to file handles, async IO, recursive copy/delete, watch, streams, and metadata.
- [x] 1.3 Read and summarize WASI filesystem concepts relevant to preopened
  directories, descriptor rights, guest portability, and host import boundaries.
- [x] 1.4 Read and summarize Web File System / Origin Private File System concepts
  relevant to user/origin-scoped handles, writable streams, and permission
  diagnostics.
- [x] 1.5 Convert the supplier comparison into Macaca-owned abstractions and
  explicitly reject any API shape that would leak provider-specific behavior.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.foundation.filesystem.v1` descriptor metadata: lifecycle,
  stability, service ids, command namespace, command schemas, permission scopes,
  policy template, resource template, SDK metadata, docs link, health, snapshot,
  and unavailable diagnostics.
- [x] 2.2 Define command DTOs for `filesystem.open_handle`,
  `filesystem.close_handle`, `filesystem.read_file`, `filesystem.write_file`,
  `filesystem.append_file`, `filesystem.list_directory`,
  `filesystem.stat_path`, `filesystem.create_directory`,
  `filesystem.copy_path`, `filesystem.move_path`, `filesystem.delete_path`,
  `filesystem.create_temp`, `filesystem.watch_path`,
  `filesystem.snapshot_tree`, and `filesystem.restore_snapshot`.
- [x] 2.3 Define shared DTOs for root refs, path refs, handle refs, access modes,
  conflict modes, content refs, metadata, watch events, snapshot refs, provider
  capability reports, and stable descriptor hashes.
- [x] 2.4 Define result/error DTOs for success, partial stream page, denied,
  not_found, already_exists, conflict, invalid_path, invalid_handle,
  quota_exceeded, too_large, unsupported, unavailable, and provider_failure.
- [x] 2.5 Add schema compatibility tests and stable hash tests for all command,
  result, health, snapshot, and unavailable DTOs.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement manifest declaration validation for required/optional
  `pack.foundation.filesystem.v1` and app-scoped filesystem roots.
- [x] 3.2 Validate scopes: `filesystem.read`, `filesystem.write`,
  `filesystem.append`, `filesystem.list`, `filesystem.metadata`,
  `filesystem.copy`, `filesystem.move`, `filesystem.delete`,
  `filesystem.watch`, `filesystem.temp`, `filesystem.snapshot`,
  `filesystem.restore`.
- [x] 3.3 Add policy checks for root scope, max bytes, max entries, recursive
  bounds, stream budget, retention, conflict mode, and provider capability.
- [x] 3.4 Add side-effect approval behavior for delete, overwrite, restore,
  recursive copy/move/delete, and non-temporary root mutation.
- [x] 3.5 Add resource reservations before side-effect provider calls and release
  resources on success, failure, timeout, and cancellation.
- [x] 3.6 Add tests proving denied, unavailable, and quota paths do not invoke a
  concrete provider.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Define the filesystem service trait/provider interface behind the
  service runtime.
- [x] 4.2 Implement unavailable provider behavior for absent filesystem service,
  disabled roots, unsupported watch/snapshot/atomic-write capability, and missing
  entitlement.
- [x] 4.3 Implement deterministic mock provider for contract and replay tests.
- [x] 4.4 Implement or bind a local scoped workspace provider with root sandboxing,
  path normalization, symlink policy, bounded directory paging, and atomic write
  strategy.
- [ ] 4.5 Add lifecycle, health, snapshot, shutdown, timeout, cancellation,
  bounded streaming, and watch cleanup.
- [x] 4.6 Add provider capability reporting for root kinds, maximum sizes,
  recursive support, watch support, snapshot support, atomic write support, and
  unavailable reasons.

## 5. SDK, WASM ABI, And Application Framework

- [x] 5.1 Extend SDK discovery with pack metadata, command schemas, root kinds,
  permissions, policy templates, examples, provider availability, health,
  diagnostics, and docs link.
- [x] 5.2 Add SDK command builders for every `filesystem.*` command; builders
  must only produce canonical traced service calls.
- [x] 5.3 Extend effective capability projection so applications can inspect
  callable commands, denied commands, unavailable roots, provider capability
  flags, and replay references.
- [x] 5.4 Expose WASM host imports only for declared callable filesystem commands,
  and route every import through the service runtime path.
- [x] 5.5 Add app-framework tests proving YAML, WASM, GenUI, and headless apps all
  use the same filesystem execution path.

## 6. Trace, Audit, Replay, And Gates

- [ ] 6.1 Emit sanitized events for declaration, admission, policy, resource,
  entitlement, handle lifecycle, service calls, watch lifecycle, snapshots,
  restore, success, failure, denied, and unavailable states.
- [x] 6.2 Add audit redaction tests proving raw host paths, file bytes, secrets,
  manifests, package bytes, credentials, private keys, provider payloads, and
  unbounded output do not enter observability surfaces.
- [x] 6.3 Add replay tests proving every filesystem command is trace-addressable
  and can reconstruct the decision path without replaying raw file content.
- [x] 6.4 Add dependency-boundary tests proving kernel, SDK, shells, and
  application framework do not import concrete filesystem providers.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers and WASM host
  imports cannot bypass service runtime.
- [ ] 6.6 Run `openspec validate add-pack-foundation-filesystem --strict`,
  targeted cargo tests, dependency-boundary gates, file-size gates, and audit
  replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/foundation/filesystem.md`.
- [x] 7.2 Document purpose, manifest declaration, required/optional declaration
  behavior, root model, handle model, permissions, policy defaults, resource
  limits, approval cases, command DTOs, result DTOs, error DTOs, watch streams,
  snapshots, restore, unavailable diagnostics, and provider replacement.
- [x] 7.3 Add minimal examples for list/read, atomic write, directory copy,
  unavailable provider diagnostics, denied delete, watch stream cancellation, and
  WASM host import usage.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack
  catalog index before marking this proposal complete.
