# Device Local Files Pack

`pack.device.local.files.v1` provides provider-neutral, picker-mediated local
file and directory access through opaque handles, scoped grants, metadata,
bounded reads/writes, append/truncate plans, directory listing, import/export,
transfer cancellation, and host status.

The pack is not arbitrary host path access and does not replace foundation
filesystem storage, package runtime storage, media parsing, or document parsing.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.device.local.files.v1"]
```

Unavailable optional declarations report
`device_local_files_provider_not_installed`.

## Commands

- `local_files.request_open_handle`, `request_save_handle`, and
  `request_directory_handle`: request host picker grants.
- `local_files.inspect_handle`, `list_handles`, and `revoke_handle`: manage
  `LocalFileHandle` and `LocalFileGrant`.
- `local_files.read`, `write`, `append`, and `truncate`: use bounded
  `LocalFileChunk` and explicit `LocalFileWritePlan`.
- `local_files.list_directory`: returns redacted `LocalFileDirectoryEntry`.
- `local_files.import_file`, `export_file`, and `cancel_transfer`: manage
  `LocalFileTransfer`.
- `local_files.inspect_host`: returns `LocalFileHostStatus`.

## DTOs And Results

Core DTOs include `LocalFileHandle`, `LocalFileGrant`, `LocalFileMetadata`,
`LocalFileFilter`, `LocalFileChunk`, `LocalFileTransfer`,
`LocalFileDirectoryEntry`, `LocalFileWritePlan`, `LocalFileHostStatus`, and
`LocalFileError`. Result statuses include success, partial, denied,
unavailable, unsupported, picker-cancelled, permission-prompt-required,
foreground-required, grant-expired, handle-revoked, handle-not-found,
read-only, write-conflict, destructive-approval-required, file-too-large,
directory-traversal-denied, content-scan-blocked, transfer-cancelled,
quota-exceeded, provider-failure, and conflict.

## Provider Mapping

Android Storage Access Framework, Apple security-scoped resources, Web File
System Access, Windows file picker/capabilities, and HarmonyOS file management
map into picker handles, grants, transfer state, directory entries, filters,
write plans, and host status. Raw host paths, raw file contents, provider
payloads, credentials, unbounded listings, and forbidden filenames are excluded
from observability surfaces.

## App-Facing Examples

Applications call the pack through picker-mediated typed commands and receive
opaque handles rather than raw host paths. Each example assumes the app already
declared `pack.device.local.files.v1` and every command carries trace, session,
tenant, and capability context through the SDK facade.

- Request an open grant with `local_files.request_open_handle` using synthetic
  filter metadata such as `text/*` or `application/pdf`, then persist only the
  opaque `handle_id`.
- Request a save grant with `local_files.request_save_handle` and write through
  an explicit `LocalFileWritePlan`.
- Request a directory grant with `local_files.request_directory_handle` and list
  redacted entries through `local_files.list_directory`.
- Read bounded chunks with `local_files.read` and avoid storing raw content in
  trace, audit, or diagnostics.
- Write, append, or truncate through `local_files.write`, `append`, and
  `truncate` after policy confirms the handle is writable.
- Import or export data through `local_files.import_file` and
  `local_files.export_file`, tracking only `transfer_id` and sanitized status.
- Revoke access with `local_files.revoke_handle` when the user removes a grant.
- Display unavailable diagnostics from
  `device_local_files_provider_not_installed` without falling back to host path
  access.

## Conformance

Provider authors must cover descriptor fields, host adapter responsibilities,
grant and transfer state machines, picker cancellation, content scanning,
unsupported behavior, redaction, health/snapshot behavior, replacement
strategy, unavailable behavior, and no raw path or content leakage.
