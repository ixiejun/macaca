# Communication Inbox Pack

`pack.communication.inbox.v1` defines generic inbox source aggregation. It
normalizes source registration, cursor-based sync, event ingestion, item and
thread listing, body/attachment fetch, labels, triage mutations, read state,
claim leases, and delegated summarization.

## Manifest Declaration

```yaml
service_contract:
  optional_packs:
    - pack.communication.inbox.v1
```

No installed provider returns `inbox_provider_not_installed`.

## Permissions

Scopes are `inbox.source.manage`, `inbox.sync`, `inbox.event.ingest`,
`inbox.read.metadata`, `inbox.read.body`, `inbox.read.attachment`,
`inbox.search`, `inbox.write.triage`, `inbox.claim`, and
`inbox.summarize`.

## Commands And DTOs

Core DTOs include `InboxSource`, `InboxCursor`, `InboxItem`, `InboxThread`,
`InboxLabel`, `InboxAttachmentHandle`, `InboxEvent`, `InboxClaim`,
`InboxSyncCheckpoint`, and `InboxProviderCapability`.

Commands cover source register/update/revoke, sync/resume, event ingestion,
list/search/get/fetch item data, list threads, label/move/archive/mark read,
claim/release, and summarize.

## Examples

Register source:

```json
{"source": {"source_id": "primary", "source_kind": "mailbox", "credential_secret_ref": "secret:source"}, "idempotency_key": "source-001"}
```

Sync and list:

```json
{"source_ids": ["primary"], "page_size": 100}
```

Search/get/fetch:

```json
{"source_id": "primary", "query_ref": "artifact:query", "page_size": 50}
```

Fetch attachment:

```json
{"attachment": {"item_id": "item", "part_id": "p1", "size_bytes": 4096}, "max_bytes": 1048576}
```

Triage and claim:

```json
{"item_id": "item", "add_labels": [{"label_id": "follow-up", "display_name": "Follow up"}]}
```

Cursor reset/unavailable:

```json
{"status": "reset_required", "error": {"code": "reset_required", "message": "source cursor expired"}}
```

## App-Facing Example Coverage

Generic examples cover source registration, sync, resume, list/search/get,
fetch body, fetch attachment, label/archive, mark read, claim/release, event
ingestion, reset-required handling, and unavailable provider handling. All
examples use synthetic source, cursor, item, thread, label, claim, event, and
artifact refs; they must not expose credentials, provider payloads, full bodies,
attachments, or application-specific triage workflows.

## Provider Author Guidance

Provider classes are `source-sync`, `event-ingest`, `aggregation-store`,
`mock`, and `unavailable`. Providers must report cursor stability, event
idempotency, source health, redaction behavior, sync reset diagnostics,
snapshots, quota status, and conformance evidence. Raw credentials, OAuth
tokens, webhook secrets, provider payloads, full bodies, attachments, and
unbounded content must not enter traces, audits, snapshots, SDK diagnostics, or
examples.
