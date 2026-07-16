## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries,
  serviceization allowlist, design-pattern guidance, and the industrial catalog
  umbrella proposal before implementation.
- [x] 1.2 Record API notes for Gmail messages/threads/labels/history/watch,
  Microsoft Graph mail/delta/change notifications, IMAP mailbox/UID/flags/search
  semantics, Slack/Teams conversation history/events, and host activity feeds.
- [x] 1.3 Map supplier concepts to provider-neutral source, cursor, item,
  thread, label, attachment, event, read-state, claim, and sync DTOs.
- [x] 1.4 Inventory existing service descriptors, SDK clients, admission paths,
  trace/audit schemas, optional providers, mock providers, unavailable providers,
  and storage/checkpoint primitives that can back inbox aggregation.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define provider-neutral DTOs for `InboxSource`, `InboxCursor`,
  `InboxItem`, `InboxThread`, `InboxLabel`, `InboxAttachmentHandle`,
  `InboxEvent`, `InboxClaim`, `InboxSyncCheckpoint`, and
  `InboxProviderCapability`.
- [x] 2.2 Define typed command DTOs for `inbox.register_source`,
  `inbox.update_source`, `inbox.revoke_source`, `inbox.sync_sources`,
  `inbox.resume_sync`, `inbox.ingest_event`, `inbox.list_items`,
  `inbox.search_items`, `inbox.get_item`, `inbox.fetch_body`,
  `inbox.fetch_attachment`, `inbox.list_threads`, `inbox.label_item`,
  `inbox.move_item`, `inbox.archive_item`, `inbox.mark_read`,
  `inbox.claim_item`, `inbox.release_item`, and `inbox.summarize_item`.
- [x] 2.3 Define typed success, page, partial-sync, reset-required, denied,
  unavailable, unsupported, conflict, quota, timeout, canceled, and provider
  failure result DTOs.
- [x] 2.4 Define descriptor metadata for pack id, source kinds, command schemas,
  permissions, policy templates, sync limits, event ingestion schema, query
  capabilities, mutation capabilities, redaction profiles, SDK metadata,
  compatibility, diagnostics, and documentation links.
- [x] 2.5 Add descriptor hash, cursor compatibility, provider capability,
  redaction-profile, and schema compatibility tests.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement declaration validation for scopes:
  `inbox.source.manage`, `inbox.sync`, `inbox.event.ingest`,
  `inbox.read.metadata`, `inbox.read.body`, `inbox.read.attachment`,
  `inbox.search`, `inbox.write.triage`, `inbox.claim`, and `inbox.summarize`.
- [x] 3.2 Enforce source ownership, credential secret references, webhook secret
  references, host/provider availability, provider capability, rate limit,
  timeout, page size, storage budget, body/attachment redaction, and approval
  checks before side effects.
- [x] 3.3 Reject raw credentials, OAuth tokens, webhook secrets, raw provider
  payloads, raw full bodies, raw attachments, and unbounded content at admission
  and observability boundaries.
- [x] 3.4 Model required declarations as readiness blockers and optional
  declarations as explicit degraded effective capabilities.
- [x] 3.5 Add tests proving denied, quota, unsupported, and unavailable paths do
  not call concrete source providers.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind inbox aggregation providers only through the service
  runtime and approved runtime-host composition roots.
- [x] 4.2 Add unavailable and mock providers with deterministic source, cursor,
  event, item, mutation, claim, and reset behavior.
- [ ] 4.3 Add lifecycle, health, snapshot, shutdown, timeout, cancellation,
  bounded pagination, sync checkpoint, cursor resume, cursor reset, idempotent
  event ingestion, and claim lease support.
- [ ] 4.4 Add provider capability reporting for source kinds, query support,
  label/folder support, read-state support, body/attachment support, watch/event
  support, mutation support, cursor expiry, page limits, and rate limits.
- [x] 4.5 Add canonical execution-path tests proving every inbox command
  traverses SDK/facade, service runtime decorators, and provider dispatch exactly
  once.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.communication.inbox.v1` with command
  schemas, source capability reports, examples, availability, diagnostics, docs
  metadata, policy templates, sync limits, and compatibility.
- [x] 5.2 Add focused SDK helper builders that only produce canonical traced
  service calls and return Null Object unavailable diagnostics when the pack is
  absent.
- [ ] 5.3 Extend WASM/application ABI metadata so applications can declare inbox
  source access, receive item events, claim items, and fetch content only through
  declared permissions.
- [x] 5.4 Add generic examples for source registration, sync, list/search/get,
  fetch body, fetch attachment, label/archive, mark read, claim/release, event
  ingestion, reset-required handling, and unavailable provider handling.

## 6. Trace, Audit, Replay, Security, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, source registration, source
  health, sync, checkpoint, event ingestion, item upsert, mutation, claim, policy,
  resource, entitlement, approval, service-call, provider-call, health,
  snapshot, and unavailable events.
- [x] 6.2 Add replay tests proving source registration, sync cursors, event
  ingestion, item mutations, claim leases, and reset-required flows are
  trace-addressable through the canonical service path.
- [ ] 6.3 Add sanitization tests proving traces, audits, snapshots, SDK
  diagnostics, and examples do not leak raw credentials, OAuth tokens, webhook
  secrets, raw provider payloads, raw full bodies, raw attachments, or unbounded
  content.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic
  application framework do not import concrete inbox providers or connector
  adapters.
- [x] 6.5 Run `openspec validate add-pack-communication-inbox --strict`,
  targeted cargo tests, boundary gates, file-size gates, canonical execution-path
  tests, and audit replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/communication/inbox.md` with pack purpose,
  platform comparison, manifest declaration, source registration, permission
  scopes, command DTOs, result DTOs, cursor handling, sync loops, event
  ingestion, deduplication, label/folder mapping, read-state mutation,
  attachment handling, claim leases, provider replacement, unavailable
  diagnostics, trace/audit interpretation, and operational limits.
- [x] 7.2 Include generic app-facing examples for register source, sync, list,
  search, get, fetch body, fetch attachment, label, archive, mark read, claim,
  release, ingest event, and handle cursor reset/unavailable provider results.
- [x] 7.3 Include provider-author guidance for descriptor metadata, cursor
  stability, event idempotency, source health, redaction, sync reset diagnostics,
  snapshots, quota reporting, and conformance tests.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial
  pack catalog index before marking `add-pack-communication-inbox` complete.
