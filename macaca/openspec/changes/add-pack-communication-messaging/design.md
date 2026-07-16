# Communication Messaging Pack Design

## Context

`pack.communication.messaging.v1` provides provider-neutral messaging operations
through a communication service boundary. It must support workspace channels,
direct messages, group chats, bot chats, webhooks, and SMS/conversation providers
while keeping provider-specific payloads behind adapters.

Applications own message purpose, copy, and user experience. Macaca owns
capability declaration, permission, recipient/channel policy, provider
replacement, trace, audit, event ingestion, delivery diagnostics, and canonical
execution.

## Supplier API Comparison

| Source API family | Relevant concepts | Macaca abstraction |
| --- | --- | --- |
| Slack Web API | `chat.postMessage`, conversations, threads, reactions, blocks, rate limits | conversation refs, message refs, thread refs, reaction commands, formatting capability |
| Microsoft Graph Teams | chat/channel messages, replies, subscriptions, reactions, HTML restrictions, permissions | chat/channel refs, reply command, event subscriptions, formatting policy, permission diagnostics |
| Telegram Bot API | `sendMessage`, edit/delete, reply markup, parse modes, chat ids, updates | bot conversation refs, parse/format mode, inline action metadata, update ingestion |
| Discord API/Webhooks | channel messages, embeds, reactions, attachments, webhooks, interactions | channel/webhook refs, embed/body parts, reaction events, attachment refs, interaction event refs |
| Twilio Conversations/SMS | participants, messages, delivery receipts, webhooks, carrier states | participant refs, conversation refs, delivery/read receipts, phone/SMS provider state |

Design conclusion: Macaca should expose conversation/message/reaction/delivery
DTOs and provider capability reports. It should not expose Slack blocks, Teams
HTML, Telegram parse-mode payloads, Discord embeds, or Twilio payloads directly.

## Goals

- Provide conversation lookup/create, participant inspection, send, edit, delete,
  reply, list, fetch, reaction add/remove, mark read, attachment, delivery/read
  status, typing indicator, and event ingestion operations.
- Support channel, DM, group chat, bot chat, webhook, and SMS/conversation
  provider classes.
- Support provider formatting capability diagnostics and safe downgraded
  rendering when a provider cannot represent a requested content shape.
- Support recipient/channel approval, consent, rate limits, idempotency, and
  event replay.

## Non-Goals

- No provider-native Slack/Teams/Telegram/Discord/Twilio payloads in SDK.
- No OS-owned bot conversation workflows or autoresponder business logic.
- No raw access tokens, bot tokens, webhook secrets, raw provider payloads, full
  conversation exports, or raw attachments in logs/traces.
- No permanent chat UI; shells render state only.

## Ownership And Boundaries

- Pack id: `pack.communication.messaging.v1`.
- Family: `communication`.
- Service owner: messaging communication/gateway service.
- Provider examples: Slack adapter, Microsoft Teams/Graph adapter, Telegram bot
  adapter, Discord adapter, Twilio Conversations/SMS adapter, mock provider,
  unavailable provider.
- SDK surface: `sdk.packs.communication.messaging`.
- Command namespace: `messaging.*`.
- Microkernel ownership: identity, policy facade, service-call evidence,
  trace/audit primitives only.
- Runtime-host ownership: provider registration, webhook/update bridge, adapter
  lifecycle, redaction, unavailable provider composition.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, effective capability projection, WASM ABI import exposure.

## Command Surface

| Command | Supplier analogs | DTO notes | Side effects |
| --- | --- | --- | --- |
| `messaging.find_conversation` | Slack conversations, Teams chats, Telegram chat id | conversation selector, provider class, participant filter | No |
| `messaging.create_conversation` | Teams chat create, Twilio conversation create | participants, topic, policy, idempotency | Yes |
| `messaging.inspect_participants` | channel/chat members | conversation ref, page token, projection | No |
| `messaging.send_message` | Slack postMessage, Graph send, Telegram sendMessage, Discord create message | sender, conversation, content, idempotency, approval | External send |
| `messaging.reply_message` | Slack threads, Teams replies, Telegram reply_to, Discord reply | parent message ref, content, thread mode | External send |
| `messaging.edit_message` | provider edit APIs | message ref, revision, patch | Yes |
| `messaging.delete_message` | provider delete APIs | message ref, reason | Yes |
| `messaging.list_messages` | conversations.replies / channel messages / Graph list | conversation/thread, cursor, page size | Reads messages |
| `messaging.fetch_message` | message get | message ref, projection | Reads message |
| `messaging.add_reaction` | Slack/Teams/Discord reactions | message ref, reaction type, actor | Yes |
| `messaging.remove_reaction` | reaction removal | message ref, reaction type, actor | Yes |
| `messaging.mark_read` | read receipts / flags | conversation/message refs, read position | Yes |
| `messaging.attach_handle` | provider attachments/media | message/draft ref, artifact handle, content type | Yes |
| `messaging.delivery_status` | delivery/read receipts | message ref, provider event id | No |
| `messaging.send_typing` | typing indicators | conversation ref, ttl | Ephemeral side effect |
| `messaging.ingest_event` | Slack events, Graph subscriptions, Telegram updates, Discord/Twilio webhooks | provider event ref, signature status, normalized event | Records event |

## DTO Model

Core DTOs:

- `MessagingConversationRef`: provider-neutral id, provider class, channel/chat
  kind, tenant/workspace, topic, visibility, trace binding.
- `MessagingParticipantRef`: user/bot/service/phone/webhook participant,
  display metadata, consent state, redaction label.
- `MessagingSenderRef`: bot/user/service identity, verified status, provider
  class, secret-reference binding.
- `MessagingContent`: plain text, markdown, rich text blocks, embed/card ref,
  artifact reference, fallback text, formatting policy.
- `MessagingAttachmentRef`: filesystem/artifact/media handle, size, content type,
  checksum, inline metadata, scan/redaction policy.
- `MessagingMessageRef`: message id, conversation id, thread id, revision,
  provider event refs.
- `MessagingReaction`: normalized reaction type, provider representation, actor,
  timestamp.
- `MessagingDeliveryState`: accepted, queued, sent, delivered, read, failed,
  deleted, edited, rate_limited, unknown.
- `MessagingError`: denied, invalid_conversation, invalid_recipient,
  unsupported_format, unsupported_command, consent_required,
  attachment_too_large, rate_limited, provider_rejected, unavailable,
  provider_failure.

## Permission And Policy Model

Permission scopes:

- `messaging.send`
- `messaging.read`
- `messaging.conversation.manage`
- `messaging.edit`
- `messaging.delete`
- `messaging.reaction`
- `messaging.attachment`
- `messaging.read_receipt`
- `messaging.delivery.read`
- `messaging.typing`
- `messaging.event.ingest`

Policy rules:

- Every command is scoped to tenant id, app id, session id, task id, sender id,
  conversation id, message id, and trace id when available.
- External sends require sender identity, recipient/channel policy, consent, rate
  limits, idempotency, and approval when policy requires it.
- Provider formatting is validated before send; unsupported format returns
  structured diagnostics or uses explicit fallback text if policy allows.
- Attachments require declared handles, size/type bounds, scan/redaction policy,
  and provider capability.
- Event ingestion requires signature/trust status and provider event id
  idempotency.
- Provider credentials enter through secret references only.

## SDK And Developer Documentation

SDK discovery returns command schemas, provider capabilities, conversation kinds,
formatting capabilities, attachment limits, rate limits, permission scopes,
policy templates, health, examples, docs link, and unavailable diagnostics.

Required developer guide:

- Path: `docs/developer-packs/communication/messaging.md`.
- Content: manifest declaration, provider classes, conversation/participant
  model, content/formatting model, send/reply/edit/delete flow, reactions, read
  receipts, attachments, event ingestion, permissions, approval policy,
  idempotency, rate limits, unavailable diagnostics, provider replacement,
  trace/audit fields, and examples.
- Examples: send channel message with approval, DM participant, reply in thread,
  fallback formatting, add reaction, mark read, attach artifact, ingest provider
  event, denied external channel, and unavailable provider.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `messaging_pack_declared`
- `messaging_pack_admission_validated`
- `messaging_pack_policy_decision`
- `messaging_pack_send_requested`
- `messaging_pack_send_accepted`
- `messaging_pack_send_failed`
- `messaging_pack_message_edited`
- `messaging_pack_message_deleted`
- `messaging_pack_reaction_recorded`
- `messaging_pack_read_marked`
- `messaging_pack_event_ingested`
- `messaging_pack_delivery_status_changed`
- `messaging_pack_unavailable`

Events include pack id, service id, command name, trace id, app/session/task
identifiers, sender hash, conversation hash, participant summary, message hash,
attachment count/size summary, delivery state, provider class, latency, bounded
resource counters, and bounded error code. Events must not include raw
credentials, raw provider payloads, webhook secrets, full message bodies, raw
attachments, prompts, or unbounded conversation content.

Health checks include provider registered state, sender identity availability,
conversation support, send/read/edit/delete/reaction support, event ingestion
support, formatting capabilities, attachment limits, rate-limit state, and
unavailable reasons.

Snapshots include descriptor version, provider class, sender identities summary,
capability flags, conversation cursor summaries, delivery state summaries,
rate-limit counters, policy template hash, and sanitized replay references.

## Implementation Slices

1. Contract slice: descriptor, command schemas, conversation/message/reaction/
   delivery DTOs, result/error DTOs, health/snapshot DTOs, provider capability
   report.
2. Admission slice: messaging declarations, sender identity, conversation
   classes, permissions, recipient/channel policy, attachment policy, event
   ingestion policy, service mapping.
3. Service slice: messaging service trait/provider interface, unavailable
   provider, mock provider, Slack/Teams/Telegram/Discord/Twilio adapter bridges.
4. SDK slice: discovery, typed command builders, send/reply/edit/delete helpers,
   reaction helpers, attachment helpers, event helpers, docs link.
5. WASM/app-runtime slice: expose only declared callable messaging commands
   through service runtime; no raw credentials or provider payloads.
6. Observability slice: trace/audit events, redaction, event idempotency tests,
   replay tests, health snapshots.
7. Developer-docs slice: complete
   `docs/developer-packs/communication/messaging.md` and link it from catalog
   metadata.

## Design Patterns

- **Facade**: SDK exposes provider-neutral messaging helpers.
- **Command**: every operation is a typed command/result.
- **Adapter/Bridge**: Slack, Teams, Telegram, Discord, Twilio, mock, and
  unavailable providers adapt to one contract.
- **Strategy**: provider selection, formatting downgrade, sender identity, rate
  limit, delivery tracking, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, consent, approval, attachment governance,
  rate-limit, and redaction wrap calls.
- **Specification**: participant, conversation, formatting, attachment, event,
  and permission rules are executable validators.
- **Observer**: message events, reactions, delivery/read receipts, and service
  events are subscribable.
- **Memento**: cursors, delivery snapshots, and effective capability reports
  support replay.

## Risks And Mitigations

- Risk: provider-specific formatting leaks into application code.
  Mitigation: normalized content DTO plus capability reports and fallback text.
- Risk: messages are sent without approval or consent.
  Mitigation: sender/recipient/channel policy, idempotency, rate limits, and
  approval gates before provider send.
- Risk: event ingestion duplicates messages.
  Mitigation: provider event id idempotency and replay metadata.
- Risk: conversation exports leak sensitive content.
  Mitigation: bounded pages, projections, and redaction gates.
- Risk: webhooks bypass service runtime.
  Mitigation: webhook/update bridge normalizes events behind service runtime and
  trace/audit decorators.
