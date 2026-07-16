# Foundation Filesystem Pack

`pack.foundation.filesystem.v1` defines scoped filesystem access for Macaca
applications. It provides logical roots, opaque handles, bounded reads and
writes, directory listing, metadata, copy/move/delete, temporary storage,
watching, snapshots, restore dry-runs, and unavailable diagnostics without
exposing raw host paths or provider-native file descriptors.

## Manifest Declaration

Declare the pack in an application service contract:

```yaml
service_contract:
  optional_packs:
    - pack.foundation.filesystem.v1
```

Use `required_packs` only when the application cannot run without a registered
scoped filesystem provider. If no provider is installed, discovery returns
`filesystem_provider_not_installed`; it does not grant host access or fake file
operations.

## Root And Handle Model

Applications operate on logical roots such as app workspace, session workspace,
temporary namespace, package artifact, WASM preopen, user-granted handle, or
remote artifact root. `FilesystemPathRef` combines a root with a normalized
relative path. Absolute host paths remain provider-private and must not appear
in SDK DTOs, traces, audits, or diagnostics.

`FilesystemHandleRef` is an opaque lease for a provider-managed descriptor.
Handles have access modes, optional revisions, and lifecycle events. Closing or
expiring a handle releases provider resources through the service runtime.

## Permissions

- `filesystem.read`: read file content through bounded content references.
- `filesystem.write`: write or replace files.
- `filesystem.append`: append to existing files.
- `filesystem.list`: list directories with paging.
- `filesystem.metadata`: stat paths and inspect bounded metadata.
- `filesystem.copy`: copy files or directories.
- `filesystem.move`: move or rename paths.
- `filesystem.delete`: delete or tombstone paths.
- `filesystem.watch`: start bounded watch streams.
- `filesystem.temp`: create temporary paths.
- `filesystem.snapshot`: create redacted tree snapshots.
- `filesystem.restore`: restore snapshots, normally after dry-run approval.

## Commands

- `filesystem.open_handle`
- `filesystem.close_handle`
- `filesystem.read_file`
- `filesystem.write_file`
- `filesystem.append_file`
- `filesystem.list_directory`
- `filesystem.stat_path`
- `filesystem.create_directory`
- `filesystem.copy_path`
- `filesystem.move_path`
- `filesystem.delete_path`
- `filesystem.create_temp`
- `filesystem.watch_path`
- `filesystem.snapshot_tree`
- `filesystem.restore_snapshot`

Side-effect commands require policy and resource checks before provider calls.
Delete, overwrite, restore, and recursive operations may require approval.

## Result And Error DTOs

Commands return a bounded envelope with status, optional data, optional error,
trace id, and descriptor hash. Standard statuses are `success`,
`partial_stream_page`, `denied`, `not_found`, `already_exists`, `conflict`,
`invalid_path`, `invalid_handle`, `quota_exceeded`, `too_large`, `unsupported`,
`unavailable`, and `provider_failure`.

Unavailable diagnostic example:

```json
{
  "status": "unavailable",
  "error": {
    "code": "unavailable",
    "message": "filesystem provider is not installed",
    "retryable": false
  }
}
```

## Examples

List a directory:

```json
{
  "path": {
    "root": { "root_id": "workspace", "root_kind": "app_workspace" },
    "relative_path": "docs"
  },
  "recursive": false,
  "page_size": 100,
  "cursor": null
}
```

Atomic write:

```json
{
  "path": {
    "root": { "root_id": "workspace", "root_kind": "app_workspace" },
    "relative_path": "docs/readme.md"
  },
  "content": {
    "content_ref": "artifact:bounded-file-content",
    "encoding": "utf8",
    "expected_hash": "content-hash"
  },
  "conflict_mode": "overwrite",
  "atomic": true
}
```

Directory copy:

```json
{
  "source": {
    "root": { "root_id": "workspace", "root_kind": "app_workspace" },
    "relative_path": "docs"
  },
  "destination": {
    "root": { "root_id": "workspace", "root_kind": "app_workspace" },
    "relative_path": "docs-copy"
  },
  "recursive": true,
  "conflict_mode": "fail"
}
```

Denied delete operations should return `denied` without invoking a provider.
Watch streams must include cancellation and budget handling. WASM host imports
may expose only declared callable `filesystem.*` commands and must route through
the same traced service-runtime path as YAML, GenUI, and headless applications.

## Provider Replacement

Expected provider classes include `local-scoped-workspace`, `wasi-preopen`,
`remote-artifact`, `mock`, and `unavailable`. Providers must expose descriptor
metadata, root availability, command support, max file and directory sizes,
watch support, snapshot support, atomic-write support, health, snapshots, and
structured unavailable behavior. SDKs, shells, kernel code, and applications
must not instantiate provider filesystems directly.
