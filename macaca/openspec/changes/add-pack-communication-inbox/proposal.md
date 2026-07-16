# Change: Add Industrial Communication Inbox Pack

## Why

Applications need a unified inbox capability that can ingest, synchronize,
triage, search, label, archive, and route inbound communication items across
mailboxes, chats, ticket-like streams, notification centers, and provider event
feeds. A production inbox pack must not be a UI list or application workflow; it
must be a serviceized data-access and event-ingestion capability with source
connectors, cursors, sync checkpoints, item handles, read state, label state,
locks, replayable events, and provider replacement.

Without this pack, each application will reinvent inbox sync, provider webhooks,
deduplication, unread state, and triage semantics. That would create multiple
execution paths and application-specific business logic inside generic OS code.

## Supplier And Platform API Research

This proposal uses mature provider APIs as supplier-grade input and maps their
common concepts into Macaca abstractions:

- Gmail API exposes messages, threads, labels, history ids, watches, batch
  modify, attachments, and query syntax. Macaca maps these to `InboxItem`,
  `InboxThread`, `InboxLabel`, `InboxCursor`, source watches, batch mutation,
  attachment handles, and provider-query capability metadata.
- Microsoft Graph Mail exposes messages, mail folders, categories, delta query,
  change notifications, subscriptions, conversation ids, internet message ids,
  attachments, and flags. Macaca maps these to source/folder handles,
  incremental sync cursors, webhook subscriptions, provider-stable item ids,
  thread ids, attachment handles, and flag/read-state DTOs.
- IMAP defines mailboxes, message sequence numbers, UIDs, flags, search,
  fetch/bodystructure, append/copy/move, IDLE/NOTIFY-like change observation,
  and UIDVALIDITY. Macaca maps these to source checkpoints, stable/unstable id
  semantics, provider flags, search capabilities, body-part handles, and
  sync-reset diagnostics.
- Slack and Teams-style conversation APIs expose channels, conversations,
  messages, reactions, threads, history pagination, event subscriptions, and
  cursor-based sync. Macaca maps these to non-email inbox sources, item kind,
  thread references, reaction metadata, event cursors, and source capability
  descriptors.
- Platform notification centers and activity feeds separate display state from
  delivery/acknowledgement. Macaca maps this to inbox item visibility, read
  state, dismissal, archive, and action routing without making shell UI the
  semantic owner.

The Macaca contract is provider-neutral. Provider query syntax, labels,
folders, categories, and event types are represented as declared capabilities
and bounded provider options, not as OS-layer branches on provider names.

## What Changes

- Add provider-neutral `pack.communication.inbox.v1` under the `communication`
  family.
- Define source, item, thread, label, folder, attachment, cursor, checkpoint,
  watch/subscription, read-state, triage-state, and lock DTOs.
- Define commands for source registration, sync, cursor resume, list/search/get,
  fetch body/attachments, label/folder/archive, read-state mutation, watch
  registration, event ingestion, item claiming, assignment, and release.
- Define permission scopes for source management, read, body read, attachment
  read, write/triage, sync, watch, event ingest, lock/claim, and summarization
  delegation.
- Require idempotent event ingestion, deduplication, watermarks, replayable
  snapshots, redacted content handling, source reset diagnostics, and developer
  documentation.

## Impact

- Affected specs: `pack-communication-inbox`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected future code: provider-neutral proto DTOs, source descriptors,
  admission validators, SDK discovery metadata, focused SDK clients, inbox
  aggregation service providers, connector adapters, unavailable/mock providers,
  trace/audit schemas, replay tests, and dependency-boundary gates.
- Non-goals: no application-specific triage workflow, no provider-name routing
  in OS layers, no shell-owned inbox semantics, no concrete provider construction
  in kernel/SDK/shells, and no fake success when sync sources or providers are
  unavailable.

## References

- Gmail API messages: https://developers.google.com/gmail/api/reference/rest/v1/users.messages
- Gmail API threads: https://developers.google.com/gmail/api/reference/rest/v1/users.threads
- Gmail API labels: https://developers.google.com/gmail/api/reference/rest/v1/users.labels
- Gmail API history/watch:
  https://developers.google.com/gmail/api/guides/push
- Microsoft Graph Mail message:
  https://learn.microsoft.com/en-us/graph/api/resources/message
- Microsoft Graph delta query:
  https://learn.microsoft.com/en-us/graph/delta-query-messages
- Microsoft Graph change notifications:
  https://learn.microsoft.com/en-us/graph/change-notifications-overview
- IMAP RFC 9051: https://www.rfc-editor.org/rfc/rfc9051
- Slack Conversations API: https://api.slack.com/apis/conversations-api
- Slack Events API: https://api.slack.com/apis/events-api
