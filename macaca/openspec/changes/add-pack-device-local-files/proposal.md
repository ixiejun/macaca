# Change: Add Industrial Device Local Files Pack

## Why

Macaca applications need `pack.device.local.files.v1` for safe interaction with host-selected local files and directories. The capability must support picker-mediated handles, scoped grants, imports, exports, reads, writes, metadata inspection, recent handle discovery, revocation, and unavailable diagnostics without exposing arbitrary host paths or letting applications bypass policy.

Local files are a high-risk device capability because file names, paths, metadata, and contents can contain sensitive personal, business, credential, and regulated data. A supplier-grade pack must therefore use explicit handle grants, bounded transfers, malware/content policy hooks, redacted observability, and revocation semantics.

## Supplier/API Baseline

The design borrows from mature file-access platforms:

- Android Storage Access Framework: document picker, content URIs, persistable URI permissions, tree access, MIME filters, and provider-mediated streams. Official docs: https://developer.android.com/guide/topics/providers/document-provider
- Apple document picker / security-scoped resources: sandboxed user-selected files, scoped access, bookmarks, coordination, and revocation-sensitive handling. Official docs: https://developer.apple.com/documentation/uikit/providing-access-to-directories and https://developer.apple.com/documentation/foundation/nsurl/1417051-startaccessingsecurityscopedreso
- Web File System Access API: file/directory handles, picker UX, permission querying/requesting, streams, writable handles, and origin-scoped grants. Official docs: https://developer.mozilla.org/docs/Web/API/File_System_API and https://wicg.github.io/file-system-access/
- Windows file picker and broadFileSystem restrictions: user-mediated file selection, storage libraries, capabilities, and permissioned file streams. Official docs: https://learn.microsoft.com/windows/apps/develop/files/ and https://learn.microsoft.com/windows/uwp/files/file-access-permissions
- HarmonyOS file picker/storage permissions: user-selected file access, sandboxed app files, and permission-mediated storage operations. Official docs: https://developer.huawei.com/consumer/en/doc/harmonyos-guides/file-management-overview

## Macaca Provider-Neutral Mapping

Macaca SHALL expose local files as revocable scoped handles:

- Picker UX becomes `local_files.request_open_handle`, `local_files.request_save_handle`, and `local_files.request_directory_handle`.
- Grant lifecycle becomes `local_files.inspect_handle`, `local_files.list_handles`, and `local_files.revoke_handle`.
- Safe reads/writes become `local_files.read`, `local_files.write`, `local_files.append`, and `local_files.truncate`.
- Import/export workflows become `local_files.import_file` and `local_files.export_file`, backed by Macaca-managed file/resource references.
- Directory traversal becomes `local_files.list_directory` with policy-limited depth and filters.
- Data governance becomes `LocalFileGrant`, `LocalFileHandle`, `LocalFileTransfer`, `LocalFilePolicy`, and sanitized trace/audit events.

The pack SHALL not own the foundation filesystem pack's app-private virtual filesystem, cloud storage connectors, document parsing, media decoding, or application-specific file formats.

## What Changes

- Add `pack.device.local.files.v1` as a service-backed industrial pack under the device family.
- Define commands for picker-mediated open/save/directory handles, handle inspection/list/revocation, metadata, read/write/append/truncate, import/export, directory listing, transfer cancellation, and host status.
- Define DTOs for `LocalFileHandle`, `LocalFileGrant`, `LocalFileMetadata`, `LocalFileFilter`, `LocalFileTransfer`, `LocalFileChunk`, `LocalFileDirectoryEntry`, `LocalFileWritePlan`, `LocalFileHostStatus`, and structured errors.
- Define permission scopes, policy/approval rules, resource budgets, content scanning hooks, path redaction, grant persistence, foreground requirements, and unavailable-provider behavior.
- Require detailed developer documentation under `docs/developer-packs/device/local-files.md`.

## Impact

- Affected specs: `pack-device-local-files`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Later affected code: protocol DTOs, descriptor/admission validators, SDK pack client, local file service provider contract, host picker adapters, stream transfer manager, mock/unavailable providers, trace/audit schemas, and boundary gates.
- Validation: `openspec validate add-pack-device-local-files --strict`, grant lifecycle tests, picker-denial tests, path redaction tests, bounded transfer tests, revocation tests, no-direct-provider-call gates, and docs coverage checks.

## Non-Goals

- This pack does not provide unrestricted host filesystem access, app-private virtual filesystem semantics, cloud drive integration, document parsing, media processing, or application-specific file format logic.
- This pack does not hardcode Android, Apple, Windows, browser, HarmonyOS, path prefixes, file extensions as business rules, or application workflows into OS-layer routing.
- This pack does not expose raw host paths, credentials, secrets, raw file contents, package bytes, unbounded directory listings, or raw provider payloads in traces, audits, snapshots, logs, or examples.
