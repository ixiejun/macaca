## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, the umbrella industrial catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API comparison notes for Android Storage Access Framework, Apple security-scoped resources, Web File System Access, Windows file picker/capabilities, and HarmonyOS file management.
- [x] 1.3 Confirm boundaries with foundation filesystem, device camera, device sensors, media packs, office/document parsing packs, and application package runtime so local-files does not absorb unrelated storage or parsing semantics.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits, per the current refactor instruction.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define provider-neutral commands for `local_files.request_open_handle`, `local_files.request_save_handle`, `local_files.request_directory_handle`, `local_files.inspect_handle`, `local_files.list_handles`, `local_files.revoke_handle`, `local_files.read`, `local_files.write`, `local_files.append`, `local_files.truncate`, `local_files.list_directory`, `local_files.import_file`, `local_files.export_file`, `local_files.cancel_transfer`, and `local_files.inspect_host`.
- [x] 2.2 Define `LocalFileHandle`, `LocalFileGrant`, `LocalFileMetadata`, `LocalFileFilter`, `LocalFileChunk`, `LocalFileTransfer`, `LocalFileDirectoryEntry`, `LocalFileWritePlan`, `LocalFileHostStatus`, and `LocalFileError`.
- [x] 2.3 Define typed success, partial, denied, unavailable, unsupported, picker-cancelled, permission-prompt-required, foreground-required, grant-expired, handle-revoked, handle-not-found, read-only, write-conflict, destructive-approval-required, file-too-large, directory-traversal-denied, content-scan-blocked, transfer-cancelled, quota-exceeded, provider-failure, and conflict results.
- [x] 2.4 Define descriptor metadata for pack id, family, lifecycle, command schemas, picker capabilities, grant persistence, transfer limits, directory support, filters, permission scopes, policy template, resource budgets, SDK metadata, compatibility, diagnostics, and documentation URL.
- [x] 2.5 Add stable descriptor hashing, version compatibility checks, DTO snapshot fixtures, grant lifecycle fixtures, transfer fixtures, redaction fixtures, and schema migration tests.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for `device.local_files.open`, `device.local_files.save`, `device.local_files.directory`, `device.local_files.read`, `device.local_files.write`, and `device.local_files.grant.manage`.
- [ ] 3.2 Enforce picker, foreground, directory traversal, MIME/type filter, grant persistence, transfer size, content scanning, retention, and raw path redaction policies before dispatch.
- [x] 3.3 Require explicit `LocalFileWritePlan` for write, append, truncate, export, and overwrite operations.
- [ ] 3.4 Add resource reservation and quota checks for active grants, active transfers, bytes, chunks, directory entry count, traversal depth, memory, storage, retained snapshots, and replay metadata.
- [ ] 3.5 Add approval behavior for directory grants, destructive operations, large exports/imports, delegated/remote host file access, and sensitive file categories.
- [ ] 3.6 Add tests proving denied, unavailable, foreground-required, grant-expired, revoked, destructive-denied, directory-denied, content-scan-blocked, and quota paths do not call concrete providers or leak resources.

## 4. Service Provider, Grant, And Transfer Strategy

- [x] 4.1 Implement the device local file service provider contract behind the service runtime; do not construct providers from kernel, SDK, shells, or generic application-framework code.
- [x] 4.2 Add provider descriptor support for host-native, browser, remote-host, plugin, mock, and unavailable provider classes.
- [x] 4.3 Add grant and transfer state machines covering requested, granted, active, completed, cancelled, expired, revoked, failed, and unavailable states.
- [x] 4.4 Add mock and unavailable providers for deterministic tests; external or host-specific adapters must remain optional providers or plugin/remote modules.
- [ ] 4.5 Add provider conformance tests for picker commands, handle inspection/list/revoke, read/write/append/truncate, import/export, directory listing, cancellation, redaction, and unsupported-command reporting.
- [ ] 4.6 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, backpressure, partial transfer, content scanning, resource cleanup, and bounded output behavior.

## 5. SDK, Admission, Examples, And ABI

- [x] 5.1 Extend SDK discovery for `pack.device.local.files.v1` with command schemas, DTO schemas, permission scopes, examples, availability, host status, picker capabilities, grant persistence, transfer limits, diagnostics, compatibility, and documentation URL.
- [ ] 5.2 Extend application admission so required declarations block when unavailable/disabled and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls and never construct providers, expose raw paths, or branch on host/platform names.
- [ ] 5.4 Add WASM/application ABI exposure for local file commands using provider-neutral DTO schemas and canonical service-call dispatch.
- [x] 5.5 Add generic examples for open handle, save handle, directory handle, read, write, import, export, revoke, and unavailable-provider diagnostics using synthetic handles and no raw host paths.

## 6. Trace, Audit, Replay, And Boundary Gates

- [x] 6.1 Emit sanitized `local_files.pack_declared`, `local_files.admission_validated`, `local_files.policy_decision`, `local_files.entitlement_checked`, `local_files.resource_reserved`, `local_files.picker_requested`, `local_files.handle_granted`, `local_files.handle_revoked`, `local_files.transfer_started`, `local_files.transfer_progressed`, `local_files.transfer_completed`, `local_files.transfer_cancelled`, `local_files.command_failed`, `local_files.unavailable`, and `local_files.snapshot_recorded` events.
- [x] 6.2 Add replay tests proving every command is trace-addressable through the canonical service path after refresh/restart without raw file contents or raw host paths.
- [x] 6.3 Add dependency-boundary gates proving microkernel, SDK, shells, and generic application framework do not import concrete local file providers or host file APIs.
- [x] 6.4 Add no-direct-provider-call gates proving all local file commands enter through descriptor-owned service registrations and typed service runtime dispatch.
- [x] 6.5 Add redaction tests for raw host paths, raw file contents, file names when policy forbids them, provider payloads, credentials, transfer chunks, handles, snapshots, and diagnostics.
- [ ] 6.6 Run `openspec validate add-pack-device-local-files --strict`, DTO compatibility tests, grant lifecycle tests, bounded transfer tests, revocation tests, boundary gates, file-size gates, and audit replay checks before marking implementation tasks complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/device/local-files.md` with purpose, manifest declarations, required/optional behavior, scopes, command DTOs, result DTOs, picker handles, grants, revocation, directory traversal, read/write/import/export, content scanning, unavailable diagnostics, and trace/audit behavior.
- [x] 7.2 Add provider author documentation covering descriptor fields, host adapter responsibilities, grant/transfer state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy.
- [x] 7.3 Add minimal app-facing examples for open handle, save handle, directory listing, read, write, import, export, revoke, and unavailable-provider diagnostics using generic synthetic data.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-device-local-files` complete.
