# Communication Messaging Pack

`pack.communication.messaging.v1` defines provider-neutral conversation and
message operations for channels, direct messages, group chats, bot conversations,
webhooks, and SMS-like providers. It keeps provider-native chat payloads behind
service adapters and exposes only Macaca-owned DTOs.

## Manifest Declaration

```yaml
service_contract:
  optional_packs:
    - pack.communication.messaging.v1
```

No installed provider returns `messaging_provider_not_installed`. Required
declarations block readiness; optional declarations become degraded effective
capabilities.

## Permissions

Scopes are `messaging.send`, `messaging.read`,
`messaging.conversation.manage`, `messaging.edit`, `messaging.delete`,
`messaging.reaction`, `messaging.attachment`, `messaging.read_receipt`,
`messaging.delivery.read`, `messaging.typing`, and
`messaging.event.ingest`.

## DTOs And Commands

Core DTOs include `MessagingConversationRef`, `MessagingParticipantRef`,
`MessagingSenderRef`, `MessagingContent`, `MessagingAttachmentRef`,
`MessagingMessageRef`, `MessagingReaction`, `MessagingCursor`,
`MessagingDeliveryState`, `MessagingProviderEventRef`, and
`MessagingProviderCapability`.

Commands cover find/create conversation, inspect participants, send, reply,
edit, delete, list/fetch messages, add/remove reactions, mark read, attach
handles, delivery status, typing indicators, and event ingestion.

## Examples

Send a channel message with approval:

```json
{
  "conversation": {"conversation_id": "channel", "kind": "channel"},
  "content": {"fallback_text_ref": "artifact:text", "format": "markdown"},
  "approval_ref": "approval:external-channel",
  "idempotency_key": "msg-001"
}
```

DM a participant:

```json
{"participants": [{"participant_id": "user", "consent_state": "granted"}], "idempotency_key": "dm-001"}
```

Reply in a thread:

```json
{"parent": {"message_id": "m1", "thread_id": "t1"}, "send": {"content": {"fallback_text_ref": "artifact:reply"}}}
```

Formatting fallback:

```json
{"content": {"format": "rich", "fallback_text_ref": "artifact:fallback", "formatting_policy": "fallback_allowed"}}
```

Reaction and read receipt:

```json
{"message": {"message_id": "m1"}, "reaction": {"reaction_key": "ack", "actor_id": "agent"}}
```

Attach artifact:

```json
{"message": {"message_id": "m1"}, "attachment": {"content_ref": "artifact:file", "size_bytes": 1024}}
```

Provider event ingestion:

```json
{"event": {"event_id_hash": "event", "signature_status": "verified"}, "state": "delivered"}
```

Denied external channel:

```json
{"status": "denied", "error": {"code": "denied", "message": "channel policy denied"}}
```

Unavailable provider:

```json
{"status": "unavailable", "error": {"code": "unavailable", "message": "messaging provider is not installed"}}
```

## Provider Replacement

Provider classes are `conversation-bridge`, `delivery-bridge`, `event-ingest`,
`mock`, and `unavailable`. Adapters must provide health, snapshots, rate limits,
formatting capability, attachment limits, delivery evidence, and sanitized audit
events through the service runtime.
