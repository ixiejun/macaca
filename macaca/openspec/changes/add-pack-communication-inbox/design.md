# Communication Inbox Pack Design

## Context

`pack.communication.inbox.v1` exposes unified inbox aggregation and inbound
communication sync as a Macaca OS serviceized capability. It lets applications
declare access to inbox sources without hardcoding Gmail, Graph, IMAP, Slack,
Teams, or host-feed logic into OS layers.

An inbox pack is stateful and audit-heavy. Sync sources can be eventually
consistent, provider ids can be unstable, history windows can expire, webhooks
can duplicate events, and item bodies can contain sensitive content. The design
therefore treats source registration, sync cursors, event ingestion, item
mutation, read state, and claim/lock operations as typed commands behind the
service runtime.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Gmail API | Messages, threads, labels, history id, watches, batch modify, attachments, query search | `InboxItem`, `InboxThread`, `InboxLabel`, `InboxCursor`, source watch, batch mutation, attachment handle, provider-query capability |
| Microsoft Graph Mail | Messages, folders, categories, flags, conversation id, delta query, change notifications, subscriptions, attachments | `InboxSource`, folder/category metadata, delta cursor, webhook subscription, thread id, attachment handle, read/flag state |
| IMAP | Mailboxes, UIDs, UIDVALIDITY, flags, search, fetch/bodystructure, copy/move, IDLE-style observation | Source checkpoint, stable id policy, sync reset diagnostics, provider flags, body part handles, mailbox moves |
| Slack/Teams conversations | Channels/conversations, messages, threads, reactions, history cursors, events | Non-email item kind, source stream, thread reference, reaction metadata, event cursor, subscription capability |
| Notification/activity feeds | Read/dismiss/archive state, action routing, display vs delivery distinction | Visibility state, acknowledgement/read state, action/event routing, shell-neutral inbox semantics |

## Goals

- Provide stable pack id `pack.communication.inbox.v1` and command namespace
  `inbox.*`.
- Support source registration, source sync, cursor resume, event ingestion,
  item/thread list, search, get, body/attachment fetch, label/folder/archive,
  mark read/unread, star/flag, claim/release, assignment, and bounded summary
  delegation.
- Preserve provider-neutral DTOs while allowing bounded provider capability and
  query metadata.
- Make sync cursors, watermarks, deduplication, source reset, read state, item
  mutation, and replay visible in trace/audit evidence.
- Require industrial developer documentation under
  `docs/developer-packs/communication/inbox.md`.

## Non-Goals

- Do not implement a concrete Gmail, Graph, IMAP, Slack, Teams, or ticketing
  connector in this proposal.
- Do not define application-specific triage, CRM, sales, support, or workflow
  semantics.
- Do not expose raw credentials, OAuth tokens, webhook secrets, raw provider
  payloads, raw attachments, unbounded message bodies, or provider-specific
  internal ids in logs, traces, snapshots, SDK diagnostics, or examples.
- Do not let shells own inbox sync, webhook ingestion, triage state, or review
  loops.

## Ownership And Boundaries

- Pack id: `pack.communication.inbox.v1`.
- Family: `communication`.
- Backing service owner: inbox aggregation service provider.
- SDK surface: `sdk.packs.communication.inbox`.
- Command namespace: `inbox.*`.
- Microkernel owns identity, policy facade, service-call evidence, resource
  primitives, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, service decorators, webhook
  bridge composition, and sanitized diagnostics through approved composition
  roots.
- Shells may render inbox state and approval surfaces but must not own sync,
  deduplication, claim, assignment, or triage semantics.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `inbox.register_source` | Register an inbox source using provider-neutral metadata and secret references | Requires entitlement, permission, credential reference validation, and descriptor hash |
| `inbox.update_source` | Update source policy, folders/streams, sync profile, or capability metadata | Requires compatibility and policy checks |
| `inbox.revoke_source` | Disable and optionally revoke a source/watch/subscription | Must preserve audit history and return partial states |
| `inbox.sync_sources` | Run bounded sync for one or more sources | Requires cursor, watermark, dedupe, timeout, quota, and snapshot metadata |
| `inbox.resume_sync` | Resume from a checkpoint or reset after provider cursor expiry | Must distinguish resumed, reset_required, and failed states |
| `inbox.ingest_event` | Accept webhook/event feed changes from provider adapters | Requires idempotency key, provider event hash, source validation, and replay evidence |
| `inbox.list_items` | Page through inbox items with filters, labels, folder, thread, read state, and time bounds | Must return bounded redacted summaries |
| `inbox.search_items` | Search using provider-neutral filters or declared provider query capability | Must report unsupported filters explicitly |
| `inbox.get_item` | Fetch item metadata and optional redacted body summary | Requires body permission for full content |
| `inbox.fetch_body` | Fetch bounded body parts by handle | Requires content redaction profile and separate body-read permission |
| `inbox.fetch_attachment` | Fetch attachment metadata or content handles | Requires attachment permission and malware/content policy hooks |
| `inbox.list_threads` | List provider-neutral thread summaries | Must preserve provider thread confidence metadata |
| `inbox.label_item` | Add/remove labels, categories, or provider flags | Requires write/triage permission and idempotency |
| `inbox.move_item` | Move between folders/mailboxes/streams when supported | Must report unsupported and partial moves |
| `inbox.archive_item` | Archive or hide an item according to provider capability | Must distinguish local triage from provider mutation |
| `inbox.mark_read` | Set read/unread state | Requires write permission and concurrency guard |
| `inbox.claim_item` | Claim an item for processing by an agent/task/session | Requires lock lease and recovery semantics |
| `inbox.release_item` | Release or complete a claim | Must handle expired leases and audit assignment state |
| `inbox.summarize_item` | Delegate bounded summary to an approved knowledge/AI capability | Requires explicit delegated capability and redaction policy |

## DTO Model

Core DTOs:

- `InboxSource`: source handle, source kind, provider class, tenant/app scope,
  credential secret reference, folders/streams, sync profile, event capability,
  health, and provider capability hash.
- `InboxCursor`: source handle, provider cursor, high-watermark, last stable
  event id, checkpoint version, expiry, reset policy, and replay pointer.
- `InboxItem`: stable Macaca item handle, provider item reference, item kind,
  source handle, thread handle, sender/actor handle, recipients/participants,
  subject/title, redacted preview, timestamps, read state, labels, folder,
  flags, sensitivity, attachment handles, and content hash.
- `InboxThread`: thread handle, source handles, participant handles, item count,
  latest timestamp, unread count, provider thread confidence, and redacted
  summary.
- `InboxLabel`: label/category/folder/flag id, display name, color token,
  provider mutability, system/custom kind, and sync mapping.
- `InboxAttachmentHandle`: item handle, part id, filename, mime type, size,
  content hash, malware scan state, storage handle, and redaction policy.
- `InboxEvent`: source handle, event id, event hash, provider timestamp,
  mutation type, affected item handles, idempotency key, and replay pointer.
- `InboxClaim`: item handle, owner agent/session/task, lease expiry, claim state,
  assignment metadata, and recovery policy.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `inbox.source.manage`
- `inbox.sync`
- `inbox.event.ingest`
- `inbox.read.metadata`
- `inbox.read.body`
- `inbox.read.attachment`
- `inbox.search`
- `inbox.write.triage`
- `inbox.claim`
- `inbox.summarize`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id, and
  trace id when available.
- Source registration accepts only secret references for credentials and webhook
  secrets.
- Sync and event ingestion require idempotency keys, provider event hashes,
  watermarks, bounded page sizes, timeout budgets, and retry metadata.
- Body and attachment access require stronger scopes than metadata listing.
- Triage mutations require concurrency guards and must distinguish provider
  mutation from local overlay state.
- Summarization is delegated to approved AI/knowledge packs and must not bypass
  inbox redaction policy.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, source capabilities,
command schemas, permission scopes, policy templates, sync limits, event
ingestion schema, provider capability metadata, redaction rules, examples,
unavailable diagnostics, health, compatibility, and documentation links.

The developer guide at `docs/developer-packs/communication/inbox.md` must cover
manifest declarations, source registration, credential references, cursor
handling, sync loops, event ingestion, deduplication, item DTOs, label/folder
mapping, read-state mutations, claim leases, attachment handling, redaction,
unavailable diagnostics, provider replacement, trace/audit interpretation, and
conformance tests.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `inbox_pack_declared`
- `inbox_pack_admission_validated`
- `inbox_source_registered`
- `inbox_source_health_changed`
- `inbox_sync_requested`
- `inbox_sync_checkpoint_recorded`
- `inbox_event_ingested`
- `inbox_item_upserted`
- `inbox_item_mutation_requested`
- `inbox_claim_created`
- `inbox_claim_released`
- `inbox_pack_policy_decision`
- `inbox_pack_service_call_requested`
- `inbox_pack_service_call_succeeded`
- `inbox_pack_service_call_failed`
- `inbox_pack_unavailable`
- `inbox_pack_snapshot_recorded`

Snapshots include descriptor version, source capability hashes, source health,
cursor checkpoints, watermark summaries, item counts, label mappings, claim
leases, sync error aggregates, policy template hash, resource counters, and
sanitized replay pointers. Snapshots must exclude raw credentials, raw tokens,
raw provider payloads, raw full bodies, raw attachments, webhook secrets, and
unbounded content.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider connectors, sync strategy, query support, mutation
  support, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  redaction, and malware/content checks wrap service calls.
- **Specification**: admission validates source declarations, permissions,
  descriptors, provider capability, sync profile, and compatibility.
- **Observer**: source events, item changes, claim changes, health, trace, and
  audit events are subscribable and replayable.
- **Memento**: cursors, checkpoints, source snapshots, and claim leases preserve
  recovery state.
- **Abstract Factory**: provider adapters are created only by approved runtime
  host composition roots.

## Risks And Mitigations

- Risk: inbox pack turns into a CRM/support workflow. Mitigation: expose generic
  source/item/label/claim primitives and leave business workflows to apps.
- Risk: provider webhooks produce duplicate or out-of-order events. Mitigation:
  require event hashes, idempotency keys, watermarks, and replayable cursors.
- Risk: provider cursors expire or become invalid. Mitigation: model
  `reset_required` and bounded full-resync plans explicitly.
- Risk: content leaks into observability. Mitigation: separate metadata, body,
  attachment permissions and enforce redaction for traces/snapshots/examples.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only
  build canonical service-call commands and are covered by no-direct-provider
  gates.
