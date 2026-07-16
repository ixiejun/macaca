# Device Local Files Pack Research

## Purpose

This note records supplier/API comparison, Macaca provider-neutral mapping,
boundary decisions, existing platform inventory, and GitNexus memo evidence for
`pack.device.local.files.v1`. The local-files pack must expose picker-mediated
handles, scoped grants, reads, writes, imports, exports, directory listing,
transfer lifecycle, revocation, host status, and redaction through typed service
commands. It must not provide unrestricted host filesystem access, cloud drive
integration, document parsing, media processing, or application-specific file
format logic.

## Source Baseline

- Android Storage Access Framework:
  <https://developer.android.com/guide/topics/providers/document-provider>
- Apple document picker and security-scoped resources:
  <https://developer.apple.com/documentation/uikit/providing-access-to-directories>
  and
  <https://developer.apple.com/documentation/foundation/nsurl/1417051-startaccessingsecurityscopedreso>
- Web File System Access API and File System API:
  <https://developer.mozilla.org/docs/Web/API/File_System_API> and
  <https://wicg.github.io/file-system-access/>
- Windows file picker and file access permissions:
  <https://learn.microsoft.com/windows/apps/develop/files/> and
  <https://learn.microsoft.com/windows/uwp/files/file-access-permissions>
- HarmonyOS file management:
  <https://developer.huawei.com/consumer/en/doc/harmonyos-guides/file-management-overview>

## Supplier API Notes

- Android Storage Access Framework contributes document pickers, content URIs,
  MIME filters, persistable grants, tree/document access, and provider-mediated
  streams. Macaca should normalize content URIs as opaque handles and never
  expose raw provider paths.
- Apple security-scoped resources contribute user-selected files/directories,
  sandboxed access, scoped bookmarks, coordination, and revocation-sensitive
  access windows. Macaca should model grants as explicit stateful leases.
- Web File System Access contributes handles, picker UX, permission
  query/request, streams, writable handles, and origin-scoped grants. Macaca
  should model prompt eligibility and foreground requirements generically.
- Windows file APIs contribute picker-mediated selection, storage libraries,
  capability declarations, streams, and restricted broad filesystem access.
  Macaca should reject unrestricted path access and use scoped handles.
- HarmonyOS file management contributes sandboxed app files, user-selected
  files, storage permissions, and provider-mediated operations. Macaca should
  keep host-specific file managers behind service providers.

## Macaca-Owned Abstractions

`pack.device.local.files.v1` should define `LocalFileHandle`,
`LocalFileGrant`, `LocalFileMetadata`, `LocalFileFilter`,
`LocalFileChunk`, `LocalFileTransfer`, `LocalFileDirectoryEntry`,
`LocalFileWritePlan`, `LocalFileHostStatus`, and `LocalFileError`.

The DTOs must carry opaque handle ids, grant state, permission class,
foreground requirement, MIME/type filters, redacted names, content class,
metadata policy, chunk bounds, transfer state, write/overwrite plan, directory
depth, content-scan status, resource reservation, redaction class, bounded
provider reason codes, and replay pointers. Raw host paths, credentials,
secrets, raw file contents, package bytes, unbounded directory listings, and raw
provider payloads are rejected.

## Boundary Decisions

- Foundation filesystem owns app-private virtual filesystem semantics and
  provider-neutral storage primitives; device local-files owns host/user-selected
  local file grants and transfer mediation.
- Device camera may produce media references; local-files only imports/exports
  through approved handles and does not own capture.
- Device sensors and notifications are separate host capabilities and do not
  route through local file handles.
- Media and office/document parsing packs own decoding, rendering,
  transcoding, extraction, and parsing after a file reference is approved.
- Application package runtime owns package bytes and runtime assets; local-files
  must not expose package internals or raw host paths in app examples.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor, lifecycle, availability, diagnostics, policy, SDK metadata, and
  unavailable diagnostic structures.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern
  for app-facing discovery and command construction; local-file SDK helpers
  should only build canonical traced service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics.
- Kernel policy, audit, trace, and redaction modules provide reusable
  enforcement and observability substrate, but current evidence does not prove
  local-file-specific DTOs, descriptors, providers, SDK helpers, ABI, tests, or
  docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
