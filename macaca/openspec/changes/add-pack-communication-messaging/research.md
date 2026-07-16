# Communication Messaging Pack Research

## Purpose

This note records supplier/API research for
`pack.communication.messaging.v1`. The pack must support channels, direct
messages, group chats, bot conversations, webhooks, and SMS/conversation
providers through one Macaca-owned service contract while keeping provider
payloads, credentials, formatting dialects, and application chat workflows out
of SDK, WASM ABI, shell, kernel, and generic application framework code.

## Source Baseline

- Slack Web API `chat.postMessage`, Conversations API, reactions, Events API,
  pagination, and rate limits:
  <https://docs.slack.dev/reference/methods/chat.postMessage>,
  <https://docs.slack.dev/reference/methods/conversations.history>,
  <https://docs.slack.dev/reference/methods/conversations.replies>,
  <https://docs.slack.dev/reference/methods/reactions.add>,
  <https://docs.slack.dev/apis/events-api/>,
  <https://docs.slack.dev/apis/web-api/pagination>, and
  <https://docs.slack.dev/apis/web-api/rate-limits>
- Microsoft Graph Teams channel/chat messages and permissions:
  <https://learn.microsoft.com/en-us/graph/api/channel-list-messages>
  and <https://learn.microsoft.com/en-us/graph/permissions-reference>
- Telegram Bot API:
  <https://core.telegram.org/bots/api>
- Discord Message and Interaction APIs:
  <https://docs.discord.com/developers/resources/message>
  and <https://docs.discord.com/developers/interactions/receiving-and-responding>
- Twilio Conversations and Messaging status callbacks:
  <https://www.twilio.com/docs/conversations-classic/api>,
  <https://www.twilio.com/docs/conversations-classic/delivery-receipts>,
  <https://www.twilio.com/docs/conversations-classic/conversations-webhooks>,
  and <https://www.twilio.com/docs/messaging/guides/track-outbound-message-status>

## Slack Web API Summary

Slack contributes workspace conversation and bot-message concepts:

- `chat.postMessage` posts to public channels, private channels, DMs, and IMs.
  Macaca should normalize these as conversation refs plus sender refs.
- Conversations history and replies use cursor pagination and thread refs.
  Macaca should expose bounded cursors, page limits, and replayable references.
- Reactions are represented by message location and reaction name. Macaca should
  expose normalized reaction commands and provider representation metadata.
- Events API deliveries may arrive through HTTP or Socket Mode. Macaca should
  ingest normalized events with provider event refs, signature/trust status, and
  idempotency.
- Slack rate limits and per-channel send limits require provider capability
  reports and structured rate_limited results.

## Microsoft Graph Teams Summary

Microsoft Graph Teams contributes tenant-scoped chat and channel concepts:

- Channel messages, replies, and chat messages map to conversation refs, message
  refs, reply commands, and thread-like parent-child relationships.
- Graph permissions distinguish delegated and application access and often
  require admin consent. Macaca should expose consent and permission diagnostics
  rather than hiding provider authorization constraints.
- Subscriptions/change notifications map to event ingestion and provider event
  refs with idempotency.
- Teams message content has HTML and formatting constraints. Macaca should model
  formatting capability, fallback text, and unsupported_format errors.
- Tenant, team, channel, and chat identifiers must remain provider details behind
  normalized conversation and participant refs.

## Telegram Bot API Summary

Telegram contributes bot-centered chat and update concepts:

- `sendMessage`, edit, delete, reply markup, parse modes, and chat identifiers
  map to send/edit/delete, content formatting policy, interaction metadata, and
  conversation refs.
- Updates represent incoming messages, callback queries, and bot interactions.
  Macaca should ingest updates through a normalized provider event bridge.
- Parse modes and message entities are provider formatting dialects. Macaca
  should expose content intent plus fallback behavior, not Telegram-specific
  parse payloads.
- Chat ids, forum topic ids, reply targets, inline keyboard structures, and bot
  token behavior must remain adapter details.
- Broadcast/rate behavior must be surfaced through rate-limit diagnostics and
  provider capability flags.

## Discord API Summary

Discord contributes channel message, reaction, webhook, attachment, and
interaction concepts:

- Message resources represent channel messages with content, embeds, attachments,
  reactions, author data, and timestamps. Macaca should normalize content,
  attachment refs, reaction refs, and message refs.
- Webhooks and interaction responses represent provider-specific ingress and
  response paths. Macaca should model them as event ingestion and response
  commands behind service policy.
- Attachments and embeds are provider-specific rich content structures. Macaca
  should expose artifact/media handles and formatting capability diagnostics.
- Reactions and message edits/deletes are side effects requiring permission,
  sender identity, trace, and audit.
- Discord tokens, webhook URLs, raw interaction payloads, and provider object
  shapes must not leak into stable SDK contracts.

## Twilio Conversations / SMS Summary

Twilio contributes participant, delivery, and carrier-state concepts:

- Conversations supports multiparty messaging across chat, WhatsApp, and SMS.
  Macaca should normalize conversation refs, participant refs, sender refs, and
  channel/provider class.
- SMS and conversation messages expose delivery status, error codes, status
  callbacks, and receipts. Macaca should map these to delivery/read states,
  provider event refs, and sanitized diagnostics.
- Webhooks provide pre-action and post-action event hooks. Macaca should require
  signature/trust status, event idempotency, and policy before ingestion-driven
  side effects.
- Phone identities and carrier states are provider-specific constraints that
  should appear as capability and delivery diagnostics, not business logic.
- Twilio credentials, phone numbers where sensitive, raw webhook payloads, and
  carrier payloads must remain protected behind adapter and audit redaction.

## Macaca-Owned Abstractions

`pack.communication.messaging.v1` should define these provider-neutral concepts:

- `MessagingConversationRef`: normalized id, provider class, conversation kind,
  tenant/workspace summary, visibility, topic, and trace binding.
- `MessagingParticipantRef`: user, bot, service, phone, webhook, or external
  participant identity with consent state and redaction label.
- `MessagingSenderRef`: verified sender identity, provider class,
  secret-reference binding, and capability constraints.
- `MessagingContent`: plain text, markdown, rich text, card/embed ref, artifact
  ref, fallback text, formatting policy, and unsupported-format behavior.
- `MessagingAttachmentRef`: filesystem/artifact/media handle, content type,
  size, checksum, scan state, and redaction policy.
- `MessagingMessageRef`: message id, conversation id, thread/parent id,
  revision, provider event refs, and replay binding.
- `MessagingReaction`: normalized reaction type, provider representation, actor
  ref, timestamp, and removal support.
- `MessagingCursor`: conversation/thread selector, page cursor hash, provider
  cursor class, rate-limit state, and replay reference.
- `MessagingDeliveryState`: accepted, queued, sent, delivered, read, edited,
  deleted, failed, rate_limited, provider_rejected, and unknown.
- `MessagingProviderCapability`: supported conversation kinds, send/edit/delete,
  reaction, attachment, typing, event ingestion, formatting modes, rate limits,
  sender identities, unavailable reasons, and health.

## Rejected Boundary Leakage

Macaca must not expose these provider-native or application-specific shapes as
stable SDK/ABI contracts:

- Slack block payloads, Slack timestamps as stable message ids, Teams HTML
  message bodies, Telegram parse-mode payloads, Discord embed/webhook objects,
  Twilio status callback forms, provider HTTP request/response bodies, or
  provider SDK models.
- Access tokens, bot tokens, webhook URLs, webhook secrets, signing secrets,
  provider account credentials, or raw authorization responses.
- Full raw conversation exports, raw attachments, unbounded message bodies,
  prompts, manifests, WASM bytes, package bytes, provider retry payloads, or raw
  webhook payloads in trace/audit/snapshot output.
- Application-specific bot workflows, autoresponders, customer-care scripts,
  moderation rules, campaign logic, or provider-specific channel routing in OS
  layers.

All operations must enter through typed Macaca messaging service commands with
trace context, policy checks, resource limits, approval where required,
structured result envelopes, sanitized audit events, unavailable provider
behavior, idempotency, replay evidence, and provider replacement support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
