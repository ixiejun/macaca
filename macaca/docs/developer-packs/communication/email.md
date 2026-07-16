# Communication Email Pack

`pack.communication.email.v1` defines provider-neutral email operations for
Macaca applications. It covers compose, recipient validation, drafts, send,
scheduled send, mailbox sync, thread listing, message and attachment fetch,
label mutation, read-state mutation, delivery status, and provider event
ingestion without exposing SMTP, mailbox, transactional, webhook, or provider
credential payloads.

## Manifest Declaration

```yaml
service_contract:
  optional_packs:
    - pack.communication.email.v1
```

Use `required_packs` only when the application cannot run without a registered
email provider. When no provider is installed, discovery and admission return
`email_provider_not_installed` rather than pretending email was sent.

## Permissions

- `email.send`: compose, send, schedule, and cancel sends.
- `email.read`: read mailbox metadata, threads, and messages.
- `email.draft`: save and update provider-neutral drafts.
- `email.attachment`: bind or fetch attachment handles.
- `email.mailbox.sync`: page mailbox state with cursors.
- `email.mailbox.mutate`: labels, folders, and read state.
- `email.delivery.read`: inspect delivery status.
- `email.event.ingest`: ingest signed provider delivery events.

## Commands And DTOs

Core DTOs include `EmailSenderRef`, `EmailRecipient`, `EmailBodyPart`,
`EmailAttachmentRef`, `EmailMessageRef`, `EmailDraftRef`, `EmailSyncCursor`,
`EmailDeliveryState`, `EmailProviderEventRef`, `EmailRateLimitStatus`, and
`EmailProviderCapability`.

Commands are named `email.compose`, `email.validate_recipients`,
`email.save_draft`, `email.update_draft`, `email.send`,
`email.schedule_send`, `email.cancel_scheduled_send`, `email.sync_mailbox`,
`email.list_threads`, `email.fetch_message`, `email.fetch_attachment`,
`email.apply_labels`, `email.mark_read`, `email.delivery_status`, and
`email.ingest_event`.

Result statuses are `success`, `partial_page`, `denied`, `invalid_sender`,
`invalid_recipient`, `consent_required`, `attachment_too_large`,
`unsupported`, `rate_limited`, `provider_rejected`, `unavailable`, and
`provider_failure`.

## Examples

Compose and send with approval:

```json
{
  "sender": {"sender_id": "sender", "address_hash": "hash", "verified": true},
  "recipients": [{"kind": "to", "address_hash": "recipient", "consent": "granted"}],
  "subject_ref": "artifact:subject",
  "body_parts": [{"kind": "markdown", "content_ref": "artifact:body"}],
  "attachments": [],
  "approval_ref": "approval:external-send"
}
```

Save draft:

```json
{"compose_ref": "artifact:email-compose", "idempotency_key": "draft-001"}
```

Send with artifact attachment:

```json
{
  "message": {"message_id": "msg", "thread_id": "thread"},
  "attachments": [{"attachment_id": "a1", "content_ref": "artifact:file", "size_bytes": 4096}]
}
```

Sync mailbox page:

```json
{"mailbox_id": "inbox", "cursor": {"cursor_hash": "cursor"}, "page_size": 100}
```

Fetch attachment safely:

```json
{"attachment": {"attachment_id": "a1", "content_ref": "artifact:file"}, "max_bytes": 1048576}
```

Delivery status:

```json
{"message": {"message_id": "msg"}, "event": {"event_id_hash": "event"}}
```

Denied external recipient:

```json
{"status": "denied", "error": {"code": "denied", "message": "recipient policy denied"}}
```

Unavailable provider:

```json
{"status": "unavailable", "error": {"code": "unavailable", "message": "email provider is not installed"}}
```

Webhook event ingestion:

```json
{"event": {"event_id_hash": "evt", "signature_status": "verified"}, "normalized_state": "delivered"}
```

## Provider Replacement

Descriptor provider classes are `transactional-mail`, `mailbox-sync`,
`event-ingest`, `mock`, and `unavailable`. Provider adapters must live behind
the service runtime, use secret references for credentials, emit bounded
health/snapshot metadata, and redact raw tokens, message bodies, attachments,
provider payloads, prompts, manifests, and unbounded mailbox content.
