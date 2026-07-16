# Communication Inbox Pack Research

## Purpose

This note records supplier/API research and Macaca platform inventory for
`pack.communication.inbox.v1`. Inbox support must aggregate inbound
communication sources through provider-neutral source, cursor, item, thread,
label, attachment, event, read-state, claim, and sync DTOs. It must not become a
CRM, support queue, shell-owned triage loop, or provider-specific mailbox
adapter embedded into OS layers.

## Source Baseline

- Gmail API messages, threads, labels, history, watch, and attachments:
  <https://developers.google.com/workspace/gmail/api/reference/rest>
- Microsoft Graph Mail API, delta queries, and change notifications:
  <https://learn.microsoft.com/en-us/graph/api/resources/mail-api-overview>
  and <https://learn.microsoft.com/en-us/graph/delta-query-overview>
- IMAP RFC 3501:
  <https://datatracker.ietf.org/doc/html/rfc3501>
- Slack conversation history, replies, Events API, pagination, and rate limits:
  <https://docs.slack.dev/reference/methods/conversations.history>,
  <https://docs.slack.dev/reference/methods/conversations.replies>,
  <https://docs.slack.dev/apis/events-api/>,
  <https://docs.slack.dev/apis/web-api/pagination>, and
  <https://docs.slack.dev/apis/web-api/rate-limits>
- Microsoft Graph Teams chat/channel messages:
  <https://learn.microsoft.com/en-us/graph/api/channel-list-messages>

## Supplier API Notes

Gmail contributes mailbox aggregation primitives:

- Messages, threads, labels, history ids, watches, search query syntax, batch
  mutation, and attachments map to inbox items, threads, labels, sync cursors,
  source watches, query capability, mutations, and attachment handles.
- History-window expiry and watch expiration require reset-required and
  degraded-capability diagnostics.
- Raw message bodies, Gmail raw/base64 payloads, and provider search dialects
  must remain behind adapters.

Microsoft Graph Mail contributes enterprise mailbox sync concepts:

- Messages, folders, categories, flags, conversation ids, attachments, delta
  queries, and change notifications map to items, labels/folders, read/flag
  state, thread refs, attachment handles, sync cursors, and event ingestion.
- Delegated/application permissions and admin consent map to explicit
  unavailable, denied, or consent_required diagnostics.
- Delta links and subscription ids must be treated as provider cursors or event
  refs, not stable SDK identities.

IMAP contributes low-level mailbox sync semantics:

- Mailboxes, UIDs, UIDVALIDITY, flags, search, fetch/bodystructure, copy/move,
  and IDLE-style observation map to source checkpoints, stable-id policy,
  provider flags, body-part handles, move/archive commands, and watch support.
- UIDVALIDITY changes require cursor reset and replay evidence.
- IMAP commands, raw BODYSTRUCTURE, and server-specific flags must not leak to
  app-facing DTOs.

Slack and Teams conversations contribute non-email inbound streams:

- Channels/chats, messages, threads, reactions, history cursors, and events map
  to inbox source streams, items, thread refs, reaction metadata, event cursors,
  and subscription capability.
- Conversation providers have different message formatting, deletion, edit,
  reaction, and pagination behavior; Macaca should expose provider capability
  diagnostics instead of making inbox item semantics provider-specific.

Host activity feeds contribute acknowledgement and action state:

- Read/dismiss/archive/acknowledge state maps to provider-neutral visibility and
  read-state commands.
- Action routing must stay an application capability concern; inbox provides
  item metadata, claim leases, and redacted fetch handles.

## Macaca-Owned Abstractions

`pack.communication.inbox.v1` should define these provider-neutral concepts:

- `InboxSource`: source kind, provider class, tenant/app scope, credential
  secret reference, folders/streams, sync profile, event support, and health.
- `InboxCursor`: source handle, provider cursor hash, high-watermark,
  checkpoint version, expiry, reset policy, and replay pointer.
- `InboxItem`: stable item handle, provider reference hash, item kind, source,
  thread, sender/actor, recipients/participants, subject/title, redacted
  preview, timestamps, read state, labels, folder, flags, sensitivity, content
  hash, and attachment handles.
- `InboxThread`: cross-item thread handle, participants, latest timestamp,
  unread count, provider confidence, and redacted summary.
- `InboxLabel`: label/category/folder/flag mapping, display name, provider
  mutability, and system/custom kind.
- `InboxAttachmentHandle`: item handle, part id, filename, content type, size,
  content hash, scan state, storage handle, and redaction policy.
- `InboxEvent`: source handle, event id hash, mutation type, affected items,
  idempotency key, provider timestamp, and replay pointer.
- `InboxClaim`: item handle, owning agent/session/task, lease expiry, claim
  state, recovery policy, and audit metadata.

## Existing Macaca Platform Inventory

Current repository capabilities that can back inbox aggregation:

- `macaca-proto::ServiceDescriptor` and domain-pack registration already provide
  descriptor identity and bounded boot trace metadata for pack services.
- `macaca-kernel::service_call` enforces trace-required dispatch and is the
  canonical path that future inbox commands must traverse.
- `macaca-sdk::SystemFacade` demonstrates focused Facade clients and
  null-object unavailable behavior, which inbox SDK helpers must follow.
- Existing unavailable clients and providers show how optional services return
  explicit unavailable diagnostics without constructing concrete providers in
  SDK or shell code.
- Persistence/event-log lineage code provides reusable Memento concepts for
  sync checkpoints, cursor replay, source reset, and claim lease recovery.
- Trace service descriptors and service-call trace emission provide the Observer
  path for source registration, sync, event ingestion, item mutation, claim, and
  unavailable events.
- Permission-contract command objects in runtime/framework code demonstrate the
  Specification/Command boundary for policy checks before side effects.

No current evidence proves inbox-specific DTOs, service providers, admission
logic, SDK helpers, WASM ABI imports, or developer documentation; those remain
unchecked tasks.

## Rejected Boundary Leakage

Macaca must reject:

- Gmail raw payloads, Graph message JSON, IMAP commands/body structures, Slack
  event payloads, Teams message HTML, raw provider ids, provider query strings as
  canonical filters, and host-feed SDK objects as stable SDK contracts.
- CRM, sales, support, moderation, or application triage workflows in OS-layer
  inbox code.
- Shell-owned sync loops, webhook repair, item assignment semantics, or review
  state machines.
- Raw credentials, OAuth tokens, webhook secrets, raw full bodies, raw
  attachments, unbounded content, prompts, manifests, WASM bytes, provider
  responses, or package bytes in trace/audit/snapshot output.

All operations must enter through typed inbox service commands with trace
context, policy/resource checks, structured results, sanitized audit,
idempotency, cursor/replay evidence, unavailable provider behavior, and provider
replacement support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
