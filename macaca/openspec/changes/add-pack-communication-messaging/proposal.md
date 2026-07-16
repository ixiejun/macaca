# Change: Add Communication Messaging Pack

## Why

Developers need `pack.communication.messaging.v1` as a provider-neutral
messaging capability for channel, direct, group, bot, webhook, and conversation
messaging. Applications need to send, receive, thread, edit, delete, react,
mark read, attach artifacts, and inspect delivery without embedding Slack,
Teams, Telegram, Discord, Twilio, or provider-specific chat payloads into OS
layers.

Messaging is an external-communication surface. It requires recipient/channel
policy, identity and consent, rate limits, formatting restrictions, attachment
governance, event idempotency, delivery/read diagnostics, trace, audit, and
structured unavailable behavior.

## Supplier And Platform API Research

The proposal is derived from a capability-by-capability comparison of mature
messaging APIs:

- Slack Web API: `chat.postMessage`, conversation ids, channels/DMs, threads,
  reactions, message formatting, blocks, cursor pagination, and rate limits.
- Microsoft Graph Teams messaging: chat/channel messages, replies,
  subscriptions for new/edited/deleted messages and reactions, HTML restrictions,
  delegated/app permission constraints.
- Telegram Bot API: `sendMessage`, edit/delete, reply markup, parse modes,
  chat ids, bot permissions, updates, and rate/broadcast behavior.
- Discord API and webhooks: channel messages, message objects, reactions,
  embeds, attachments, webhooks, interaction responses, and permission scopes.
- Twilio Conversations/SMS style APIs: conversations, participants, messages,
  delivery receipts, webhooks, phone-number identity, and carrier delivery state.

Macaca borrows the stable concepts, not provider APIs:

- model conversations, channels, participants, messages, threads, reactions, and
  delivery states explicitly;
- separate send/edit/delete/react/read/event-ingest commands;
- represent attachments as handles, not raw bytes;
- require approval/policy for external messages;
- normalize provider formatting limits, rate limits, and delivery diagnostics.

## What Changes

- Define `pack.communication.messaging.v1` as the canonical app-facing messaging
  pack.
- Add an industrial command surface covering conversation lookup/create, send,
  edit, delete, reply, list messages, fetch message, add/remove reaction, mark
  read, attach handle, delivery/read status, typing indicator, and provider event
  ingestion.
- Define provider-neutral DTO requirements for conversation refs, participant
  refs, sender identity, message content, formatting, attachments, reactions,
  cursors, delivery/read receipts, provider event refs, idempotency, and
  unavailable diagnostics.
- Define permission scopes for send, read, conversation manage, edit, delete,
  reaction, attachment, read receipt, delivery read, typing, and event ingestion.
- Require a detailed developer guide under
  `docs/developer-packs/communication/messaging.md` before completion.
- Keep implementation ownership in a messaging/communication gateway service;
  kernel, SDK, shells, and application framework remain provider-neutral.

## Impact

- Affected specs: `pack-communication-messaging`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs, descriptor validators, application
  admission, SDK discovery, SDK command helpers, messaging service/gateway
  provider, webhook/update bridge, mock/unavailable providers, trace/audit event
  schema, replay tests, and dependency-boundary gates.
- Non-goals: provider-specific chat payloads in SDK, OS-owned bot workflows,
  app-specific message templates, raw credentials in app code, or shell-owned
  messaging semantics.
