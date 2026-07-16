## 1. Supplier API Research And Scope

- [x] 1.1 Read and summarize Slack Web API chat/conversations/reactions/events
  behavior, message formatting, cursor pagination, and rate limits.
- [x] 1.2 Read and summarize Microsoft Graph Teams chat/channel messages,
  replies, subscriptions, reactions, HTML restrictions, and permission constraints.
- [x] 1.3 Read and summarize Telegram Bot API send/edit/delete/reply markup,
  parse modes, updates, chat ids, and broadcast/rate behavior.
- [x] 1.4 Read and summarize Discord message, reaction, webhook, attachment, and
  interaction response APIs.
- [x] 1.5 Read and summarize Twilio Conversations/SMS concepts for participants,
  messages, delivery receipts, webhooks, phone identities, and carrier states.
- [x] 1.6 Convert the supplier comparison into Macaca-owned abstractions and
  explicitly reject provider-native chat payloads and credentials in SDK.
- [x] 1.7 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.communication.messaging.v1` descriptor metadata:
  lifecycle, stability, service ids, command namespace, command schemas,
  permission scopes, policy template, resource template, SDK metadata, docs
  link, health, snapshot, and unavailable diagnostics.
- [x] 2.2 Define command DTOs for `messaging.find_conversation`,
  `messaging.create_conversation`, `messaging.inspect_participants`,
  `messaging.send_message`, `messaging.reply_message`,
  `messaging.edit_message`, `messaging.delete_message`,
  `messaging.list_messages`, `messaging.fetch_message`,
  `messaging.add_reaction`, `messaging.remove_reaction`,
  `messaging.mark_read`, `messaging.attach_handle`,
  `messaging.delivery_status`, `messaging.send_typing`, and
  `messaging.ingest_event`.
- [x] 2.3 Define shared DTOs for conversation refs, participant refs, sender refs,
  content, attachment refs, message refs, reactions, cursors, delivery states,
  provider event refs, rate-limit status, provider capability report, and stable
  descriptor hashes.
- [x] 2.4 Define result/error DTOs for success, partial page, denied,
  invalid_conversation, invalid_recipient, unsupported_format,
  unsupported_command, consent_required, attachment_too_large, rate_limited,
  provider_rejected, unavailable, and provider_failure.
- [x] 2.5 Add schema compatibility tests and stable hash tests for command,
  result, health, snapshot, provider capability, and unavailable DTOs.

## 3. Admission, Permission, Policy, Resource, And Approval

- [ ] 3.1 Implement manifest declaration validation for required/optional
  `pack.communication.messaging.v1`, sender identities, conversation classes,
  event ingestion endpoints, and attachment support.
- [x] 3.2 Validate scopes: `messaging.send`, `messaging.read`,
  `messaging.conversation.manage`, `messaging.edit`, `messaging.delete`,
  `messaging.reaction`, `messaging.attachment`, `messaging.read_receipt`,
  `messaging.delivery.read`, `messaging.typing`, and `messaging.event.ingest`.
- [x] 3.3 Add policy checks for sender verification, participant/channel policy,
  consent, external recipient approval, message size, formatting support,
  attachment count/size/type, rate limits, event signature state, idempotency,
  and provider capability.
- [x] 3.4 Add approval behavior for external sends, new conversation creation,
  destructive delete, broad conversation sync, attachment fetch/export, and
  event ingestion from untrusted signatures.
- [x] 3.5 Require provider credentials through secret references only.
- [x] 3.6 Add tests proving denied, unavailable, invalid_recipient,
  unsupported_format, consent_required, attachment_too_large, rate_limited, and
  provider_rejected paths do not send messages.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Define the messaging service trait/provider interface behind the service
  runtime.
- [x] 4.2 Implement unavailable provider behavior for absent messaging service,
  missing sender identity, unsupported conversation/edit/delete/reaction/event
  behavior, missing entitlement, and provider health failure.
- [x] 4.3 Implement deterministic mock provider for contract, replay, delivery,
  and event ingestion tests.
- [ ] 4.4 Implement adapter bridge points for Slack, Microsoft Teams/Graph,
  Telegram Bot API, Discord, and Twilio Conversations/SMS without leaking
  provider-native APIs to SDK callers.
- [ ] 4.5 Add webhook/update event bridge with signature status, idempotency,
  provider event refs, normalized delivery/read/reaction states.
- [ ] 4.6 Add lifecycle, health, snapshot, shutdown, cursor management, attachment
  handling, rate-limit reporting, formatting capability reporting, redaction, and
  provider capability reports.

## 5. SDK, WASM ABI, And Application Framework

- [x] 5.1 Extend SDK discovery with pack metadata, command schemas, sender
  identities, conversation kinds, formatting capabilities, attachment limits,
  permissions, policy templates, health, diagnostics, and docs link.
- [x] 5.2 Add SDK command builders for every `messaging.*` command; builders must
  only produce canonical traced service calls.
- [ ] 5.3 Add SDK helpers for find/create conversation, send/reply/edit/delete,
  reaction, mark read, attachment, typing, delivery status, event ingestion, and
  unavailable diagnostics.
- [ ] 5.4 Extend effective capability projection so applications can inspect
  callable commands, denied commands, unavailable providers, sender identities,
  provider capability flags, rate limits, and replay references.
- [ ] 5.5 Expose WASM host imports only for declared callable messaging commands
  and route every import through the service runtime path.
- [ ] 5.6 Add app-framework tests proving YAML, WASM, GenUI, and headless apps all
  use the same messaging execution path.

## 6. Trace, Audit, Replay, And Gates

- [ ] 6.1 Emit sanitized events for declaration, admission, policy, send request,
  send accepted/failed, edit/delete, reaction, read receipt, event ingestion,
  delivery status, success, failure, denied, and unavailable states.
- [x] 6.2 Add audit redaction tests proving raw access tokens, bot tokens, webhook
  secrets, raw provider payloads, full message bodies, raw attachments, prompts,
  manifests, and unbounded conversation content do not enter observability
  surfaces.
- [x] 6.3 Add replay tests proving messaging commands and provider events are
  trace-addressable and can reconstruct decisions without raw message bodies or
  attachments.
- [x] 6.4 Add dependency-boundary tests proving kernel, SDK, shells, and
  application framework do not import concrete messaging providers.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers and WASM host
  imports cannot bypass service runtime.
- [x] 6.6 Run `openspec validate add-pack-communication-messaging --strict`,
  targeted cargo tests, dependency-boundary gates, file-size gates, and audit
  replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/communication/messaging.md`.
- [x] 7.2 Document purpose, manifest declaration, provider classes, conversation
  and participant model, content/formatting model, send/reply/edit/delete flow,
  reactions, read receipts, attachments, event ingestion, permissions, approval
  policy, idempotency, rate limits, command DTOs, result DTOs, error DTOs,
  unavailable diagnostics, and provider replacement.
- [x] 7.3 Add minimal examples for sending a channel message with approval, DMing
  a participant, replying in a thread, formatting fallback, adding a reaction,
  marking read, attaching an artifact, ingesting a provider event, denied
  external channel, and unavailable provider diagnostics.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack
  catalog index before marking this proposal complete.
