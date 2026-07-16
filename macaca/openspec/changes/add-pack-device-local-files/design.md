# Device Local Files Pack Design

## Context

`pack.device.local.files.v1` provides controlled access to user-selected host files and directories. The stable model across Android, Apple, Windows, browser, and HarmonyOS is not arbitrary path access; it is picker-mediated grants, scoped handles, streaming reads/writes, sandbox boundaries, and revocation. Macaca should expose those concepts as provider-neutral DTOs behind the service runtime.

This pack is separate from foundation filesystem. Foundation filesystem owns app-private and OS-managed storage abstractions. Device local files owns host/user-selected files, host picker UX, scoped grants, import/export, and revocation.

## Supplier Capability Matrix

| Platform/API | Borrowed capability | Macaca mapping |
| --- | --- | --- |
| Android Storage Access Framework | content URI handles, MIME filters, persistable grants, tree access, provider streams | `LocalFileHandle`, `LocalFileGrant`, picker filters, directory grants, stream transfer |
| Apple security-scoped resources | sandboxed document picker, bookmarks, scoped access begin/end | scoped grant lease, persistent handle metadata, revocation-aware reads/writes |
| Web File System Access | file/directory handles, permission query/request, writable streams | picker commands, handle inspection, read/write streams, permission state |
| Windows file picker/capabilities | picker-mediated files, libraries, broad access restrictions | host status, capability diagnostics, foreground picker requirement |
| HarmonyOS file management | user-selected access, sandbox/storage permissions | provider adapter, policy scopes, host permission diagnostics |

## Goals

- Provide picker-mediated file/directory handles, scoped grants, metadata inspection, bounded read/write streams, import/export, directory listing, recent handle discovery, revocation, and host status.
- Preserve privacy by using opaque handles and redacted metadata rather than raw host paths.
- Enforce permission, policy, approval, resource, content scanning, foreground, and revocation before and during transfers.
- Support host-native, browser, plugin, remote-host, mock, and unavailable providers through descriptors.
- Provide detailed developer documentation and provider conformance guidance.

## Non-Goals

- Do not provide unrestricted path-based host filesystem access.
- Do not own foundation/app-private filesystem, cloud storage connectors, document parsing, media decoding, backup/sync, or application-specific file formats.
- Do not expose raw host paths, raw contents, raw provider payloads, credentials, secrets, package bytes, or unbounded listings in observability surfaces.
- Do not branch on host OS, browser, provider name, path prefix, file extension as business logic, or application workflow in OS-layer code.

## Ownership And Boundaries

- Pack id: `pack.device.local.files.v1`.
- Capability family: `device`.
- Backing service: device local file service.
- SDK surface: `sdk.packs.device.local_files`.
- Command namespace: `local_files.*`.
- Application framework owns manifest declaration and app-scoped permission projection.
- Service runtime owns typed dispatch, decorators, grant lifecycle, transfer lifecycle, provider health, snapshots, and unavailable behavior.
- Runtime host owns concrete host/browser/provider adapters through approved composition roots.
- Shells may render picker diagnostics but must not implement file semantics.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `local_files.request_open_handle` | Request user-selected readable file handles | Requires foreground/approval policy, filters, max selection count, grant duration, and host picker availability |
| `local_files.request_save_handle` | Request a writable save destination | Requires suggested name/type metadata, overwrite policy, grant duration, and write scope |
| `local_files.request_directory_handle` | Request scoped directory access | Requires explicit directory policy, max traversal depth, filters, grant duration, and approval |
| `local_files.inspect_handle` | Inspect an opaque handle and grant state | Returns metadata, permission state, scope, expiry, redaction, and provider limitations |
| `local_files.list_handles` | List recent/persisted handles visible to the app/session | Returns redacted handle summaries and grant states, never raw paths |
| `local_files.revoke_handle` | Revoke a file/directory grant | Closes active transfers and marks the handle revoked |
| `local_files.read` | Read bounded bytes or chunks from a handle | Enforces grant, offset, length, content policy, and resource quota |
| `local_files.write` | Write bounded bytes/chunks to a handle | Enforces write grant, write plan, overwrite/append/truncate policy, content scanning, and resource quota |
| `local_files.append` | Append bounded data to a writable handle | Enforces append capability and content policy |
| `local_files.truncate` | Truncate writable file to requested size | Requires explicit destructive approval when policy requires it |
| `local_files.list_directory` | List entries under a directory handle | Enforces depth, filters, count limits, redaction, and traversal policy |
| `local_files.import_file` | Copy host-selected file into Macaca-managed storage/resource | Produces a provider-neutral resource reference with content policy evidence |
| `local_files.export_file` | Export a Macaca-managed resource to user-selected local destination | Uses save handle/export policy and emits transfer evidence |
| `local_files.cancel_transfer` | Cancel an active read/write/import/export transfer | Releases resources and emits cancellation audit evidence |
| `local_files.inspect_host` | Inspect host local-file capability status | Returns picker availability, permission state, foreground requirement, provider class, and diagnostics |

## DTO Model

- `LocalFileHandle`: opaque id, handle kind, grant id, redacted display name, MIME/type hints, size class, writable/readable flags, directory flag, provider class, expiry, and revoked state.
- `LocalFileGrant`: grant id, source command, scope, permissions, persistence class, expiry, foreground requirement, approval id, revocation state, and policy hash.
- `LocalFileMetadata`: redacted name, extension/type hint, MIME, size, modified time class, created time class, directory/file kind, symlink/alias warning, and provenance.
- `LocalFileFilter`: MIME types, extensions as hints, size limits, multiple selection, directory policy, and provider-supported filter metadata.
- `LocalFileChunk`: transfer id, offset, length, checksum/hash when allowed, content reference or bounded bytes, sequence number, and truncation state.
- `LocalFileTransfer`: transfer id, handle id, direction, state, bytes transferred, total size class, checksum policy, scan status, cancellation token, and resource counters.
- `LocalFileDirectoryEntry`: entry handle/reference, redacted name, kind, size class, type hint, child count class, and traversal warning.
- `LocalFileWritePlan`: create/overwrite/append/truncate mode, expected size, checksum policy, atomicity preference, conflict behavior, and destructive-operation flag.
- `LocalFileHostStatus`: provider class, picker availability, permission state, foreground requirement, supported commands, active grants, active transfers, disabled reason, and diagnostics.
- `LocalFileError`: denied, unavailable, unsupported, picker cancelled, permission prompt required, foreground required, grant expired, handle revoked, handle not found, read only, write conflict, destructive approval required, file too large, directory traversal denied, content scan blocked, transfer cancelled, quota exceeded, provider failure, or conflict.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `device.local_files.open`: picker-mediated readable handles.
- `device.local_files.save`: picker-mediated writable save handles.
- `device.local_files.directory`: scoped directory handles and listing.
- `device.local_files.read`: bounded reads/imports.
- `device.local_files.write`: bounded writes/exports/appends/truncates.
- `device.local_files.grant.manage`: list, inspect, and revoke handles/grants.

Policy requirements:

- Raw host paths are never exposed to applications or observability by default.
- Picker commands require foreground/user-mediated context unless host policy allows delegated flows.
- Directory handles require stricter approval, traversal depth limits, filters, and count limits.
- Reads/writes/imports/exports require bounded transfer limits and content scanning hooks when enabled.
- Destructive operations such as overwrite and truncate require explicit policy allowance and approval when configured.
- Grant revocation closes active transfers and invalidates future reads/writes.

## Service Runtime And Provider Strategy

Provider Strategy categories:

- Host-native provider: OS picker and security-scoped handles.
- Browser provider: File System Access/file input handles.
- Remote-host provider: delegated file picker/transfer from a trusted remote host.
- Plugin provider: specialized desktop/enterprise document provider.
- Mock provider: synthetic handles and transfers for tests/docs.
- Unavailable provider: explicit unavailable diagnostics.

Providers declare supported picker types, grant persistence, directory support, transfer limits, MIME/filter behavior, write capabilities, foreground requirements, and host permission state. Provider construction is allowed only in approved runtime composition roots.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, host status, picker/filter capabilities, grant persistence classes, transfer limits, policy templates, examples, diagnostics, compatibility, and documentation links.

The implementation SHALL create `docs/developer-packs/device/local-files.md` with:

- Manifest declarations for required and optional use.
- Permission scopes and foreground picker behavior.
- Command-by-command DTO reference.
- Handle/grant lifecycle, revocation, directory traversal, read/write/import/export, conflict, and content scanning guidance.
- Path redaction and observability rules.
- Error taxonomy and unavailable-provider troubleshooting.
- Trace/audit event reference and replay workflow.
- Provider author conformance checklist.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `local_files.pack_declared`
- `local_files.admission_validated`
- `local_files.policy_decision`
- `local_files.entitlement_checked`
- `local_files.resource_reserved`
- `local_files.picker_requested`
- `local_files.handle_granted`
- `local_files.handle_revoked`
- `local_files.transfer_started`
- `local_files.transfer_progressed`
- `local_files.transfer_completed`
- `local_files.transfer_cancelled`
- `local_files.command_failed`
- `local_files.unavailable`
- `local_files.snapshot_recorded`

Events include pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when present, provider class, handle kind, grant id hash, transfer id hash, direction, size class, policy decision, latency, and resource counters. Events exclude raw host paths, raw contents, raw provider payloads, secrets, credentials, package bytes, and unbounded listings.

Snapshots include provider health, host status, supported command matrix, active grant summaries, active transfer summaries, transfer limit classes, policy template hash, unavailable diagnostics, and sanitized replay pointers.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while `SystemFacade` carries canonical service calls.
- **Command**: every operation is a typed command/result DTO.
- **Adapter**: host, browser, remote, plugin, mock, and unavailable providers map into Macaca DTOs.
- **Strategy**: picker type, grant persistence, transfer mode, conflict behavior, and unavailable behavior are descriptor-driven.
- **Decorator**: trace, policy, resource, entitlement, approval, content scanning, metering, and redaction wrap every call.
- **State**: grants and transfers are explicit state machines.
- **Specification**: admission validates scopes, filters, transfer size, directory policy, foreground mode, and destructive operations.
- **Observer**: trace, audit, transfer, health, and service events are subscribable.
- **Memento**: snapshots record grants/transfers for replay without raw file contents.
- **Abstract Factory**: providers are created only in approved composition roots.

## Risks And Mitigations

- Risk: local-files becomes unrestricted filesystem access. Mitigation: opaque picker-mediated handles, grants, and no raw path API.
- Risk: traces leak file names/contents. Mitigation: redacted metadata, size classes, hashes only when allowed, and no raw content in observability.
- Risk: writes cause irreversible damage. Mitigation: write plans, conflict policy, destructive approval, and bounded transfers.
- Risk: directory grants overexpose host data. Mitigation: stricter scopes, depth/count limits, filters, and approval.
- Risk: active transfers continue after revocation. Mitigation: grant/transfer state machines close resources on revoke, cancellation, task stop, and shutdown.
