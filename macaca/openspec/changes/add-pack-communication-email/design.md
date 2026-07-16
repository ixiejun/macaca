# Communication Email Pack Design

## Context

`pack.communication.email.v1` provides email operations through a serviceized
communication boundary. It must support user mailbox style providers and
transactional email providers while exposing one provider-neutral application
contract.

The pack is a communication capability, not an application business workflow.
Applications own copy, purpose, and user experience. Macaca owns declaration,
permission, policy, sender/recipient governance, trace, audit, provider
replacement, delivery diagnostics, and canonical execution.

## Supplier API Comparison

| Source API family | Relevant concepts | Macaca abstraction |
| --- | --- | --- |
| SMTP / MIME / IMAP | envelope recipients, RFC message headers, multipart body, attachments, mailboxes, flags | message DTO, envelope DTO, body parts, attachment refs, mailbox cursors, flags |
| Gmail API | messages, drafts, send, threads, labels, history ids, attachments, watch | draft/message/thread refs, label ops, sync cursor, attachment handle, watch event |
| Microsoft Graph Mail | sendMail, messages, folders, attachments, delta, subscriptions, permissions | send command, folder ops, delta cursor, subscription/event ingestion, permission scopes |
| SendGrid Mail Send | personalizations, templates, substitutions, categories, sandbox, webhooks | transactional send fields, template refs, sandbox/test mode, delivery events |
| Mailgun Messages | domain send, templates, variables, tags, test mode, attachments, events | sender domain identity, template refs, tags, test mode, delivery diagnostics |

Design conclusion: Macaca should expose message/draft/thread/delivery DTOs and
provider capability reports. It should not expose provider-native payloads or
transport credentials to applications.

## Goals

- Provide compose, validate recipient, save/update draft, send, schedule/cancel
  send, mailbox sync, list threads, fetch message, fetch attachment, label/folder
  mutation, mark read/unread, delivery status, and event ingestion operations.
- Support mailbox-style providers and transactional providers with capability
  diagnostics.
- Support artifact/file attachment references and size/type policy before send.
- Support external-recipient approval and rate limits.
- Support delivery evidence, bounce/failure diagnostics, idempotency, and replay.

## Non-Goals

- No provider-specific Gmail/Graph/SMTP/SendGrid/Mailgun payloads in SDK.
- No OS-owned application templates or campaign workflows.
- No raw OAuth tokens, SMTP credentials, provider webhooks secrets, or attachment
  bytes in logs/traces.
- No spam/phishing classifier ownership; security services may integrate later.
- No user inbox UI; shells render state only.

## Ownership And Boundaries

- Pack id: `pack.communication.email.v1`.
- Family: `communication`.
- Service owner: email communication/gateway service.
- Provider examples: SMTP/IMAP adapter, Gmail adapter, Microsoft Graph adapter,
  SendGrid adapter, Mailgun adapter, mock provider, unavailable provider.
- SDK surface: `sdk.packs.communication.email`.
- Command namespace: `email.*`.
- Microkernel ownership: identity, policy facade, service-call evidence,
  trace/audit primitives only.
- Runtime-host ownership: provider registration, webhook bridge, transport
  adapter lifecycle, redaction, unavailable provider composition.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, effective capability projection, WASM ABI import exposure.

## Command Surface

| Command | Supplier analogs | DTO notes | Side effects |
| --- | --- | --- | --- |
| `email.compose` | MIME message creation | sender, recipients, subject, body parts, attachment refs | No |
| `email.validate_recipients` | provider validation / policy preflight | recipients, domains, consent state, policy result | No |
| `email.save_draft` | Gmail/Graph draft | draft content, idempotency key | Yes |
| `email.update_draft` | draft update | draft id, revision, content patch | Yes |
| `email.send` | SMTP send, Gmail send, Graph sendMail, SendGrid/Mailgun send | message/draft ref, idempotency, approval ref | External send |
| `email.schedule_send` | provider scheduling when supported | message ref, send time, timezone, cancellation window | External scheduled side effect |
| `email.cancel_scheduled_send` | provider cancel | scheduled id, reason | Yes |
| `email.sync_mailbox` | IMAP sync, Gmail history, Graph delta | mailbox/folder, cursor, page size | Reads mailbox |
| `email.list_threads` | Gmail threads / mailbox conversations | folder/label, cursor, filters | Reads mailbox |
| `email.fetch_message` | message get | message id, body projection, header projection | Reads mailbox |
| `email.fetch_attachment` | attachment get | attachment id, artifact target, max bytes | Reads attachment |
| `email.apply_labels` | Gmail labels / folders | message ids, label/folder ops | Mutates mailbox |
| `email.mark_read` | IMAP flags / Graph read state | message ids, read/unread | Mutates mailbox |
| `email.delivery_status` | provider events / SMTP status | message id, provider event id, status projection | No |
| `email.ingest_event` | SendGrid/Mailgun webhooks, Graph subscription | provider event ref, signature status, normalized event | Records event |

## DTO Model

Core DTOs:

- `EmailSenderRef`: account, domain, verified identity, reply-to policy, provider
  class, and trace binding.
- `EmailRecipient`: to/cc/bcc kind, address, display name, consent status,
  domain policy result, redaction label.
- `EmailBodyPart`: text/html/markdown/reference body, language, encoding,
  redaction policy.
- `EmailAttachmentRef`: filesystem/artifact/document handle, content type, size,
  checksum, inline/cid metadata, scan policy.
- `EmailMessageRef`: provider-neutral message id, thread id, folder/label ids,
  revision, provider event refs.
- `EmailDraftRef`: draft id, revision, expires/retention metadata.
- `EmailSyncCursor`: mailbox/folder, cursor token hash, history/delta id,
  provider class.
- `EmailDeliveryState`: accepted, queued, scheduled, sent, delivered, bounced,
  complained, deferred, failed, cancelled, unknown.
- `EmailError`: denied, invalid_sender, invalid_recipient, consent_required,
  attachment_too_large, unsupported, rate_limited, provider_rejected,
  unavailable, provider_failure.

## Permission And Policy Model

Permission scopes:

- `email.send`
- `email.read`
- `email.draft`
- `email.attachment`
- `email.mailbox.sync`
- `email.mailbox.mutate`
- `email.delivery.read`
- `email.event.ingest`

Policy rules:

- Every command is scoped to tenant id, app id, session id, task id, sender id,
  message/draft/thread id, and trace id when available.
- External sends require sender verification, recipient validation, rate limits,
  idempotency key, and approval when policy requires it.
- Attachments require declared file/artifact handles, size/type bounds, and
  redaction/scan policy before send or fetch.
- Mailbox reads require read scope, page bounds, body projection bounds, and
  raw content redaction before observability.
- Webhook/event ingestion requires signature validation status and provider event
  id idempotency.
- Provider credentials enter through secret references only.

## SDK And Developer Documentation

SDK discovery returns command schemas, provider capabilities, mailbox versus
transactional support, sender identities, attachment limits, rate limits,
permission scopes, policy templates, health, examples, docs link, and unavailable
diagnostics.

Required developer guide:

- Path: `docs/developer-packs/communication/email.md`.
- Content: manifest declaration, sender identity, recipient model, compose/draft/
  send flow, mailbox sync, attachments, templates, delivery events, permissions,
  approval policy, idempotency, rate limits, unavailable diagnostics, provider
  replacement, trace/audit fields, and examples.
- Examples: compose/send with approval, save draft, send with artifact attachment,
  sync mailbox page, fetch attachment safely, delivery status, denied external
  recipient, unavailable provider, and webhook event ingestion.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `email_pack_declared`
- `email_pack_admission_validated`
- `email_pack_policy_decision`
- `email_pack_message_composed`
- `email_pack_draft_saved`
- `email_pack_send_requested`
- `email_pack_send_accepted`
- `email_pack_send_failed`
- `email_pack_mailbox_synced`
- `email_pack_attachment_fetched`
- `email_pack_event_ingested`
- `email_pack_delivery_status_changed`
- `email_pack_unavailable`

Events include pack id, service id, command name, trace id, app/session/task
identifiers, sender id hash, recipient count/domain summary, message id hash,
draft id hash, attachment count/size summary, delivery state, provider class,
latency, bounded resource counters, and bounded error code. Events must not
include raw credentials, raw provider payloads, full message bodies, raw
attachments, raw OAuth tokens, webhook secrets, prompts, or unbounded mailbox
content.

Health checks include provider registered state, sender identity availability,
send support, draft support, mailbox sync support, attachment support, event
ingestion support, rate-limit state, max attachment size, and unavailable
reasons.

Snapshots include descriptor version, provider class, sender identities summary,
capability flags, rate-limit counters, cursor summaries, policy template hash,
delivery state summaries, and sanitized replay references.

## Implementation Slices

1. Contract slice: descriptor, command schemas, message/draft/thread/delivery
   DTOs, result/error DTOs, health/snapshot DTOs, provider capability report.
2. Admission slice: email declarations, sender identity requirements,
   permissions, external recipient policy, attachment policy, service mapping.
3. Service slice: email service trait/provider interface, unavailable provider,
   mock provider, SMTP/IMAP adapter, Gmail adapter, Graph adapter, SendGrid/Mailgun
   adapter bridges.
4. SDK slice: discovery, typed command builders, compose/send helpers, mailbox
   sync helpers, attachment helpers, delivery/event helpers, docs link.
5. WASM/app-runtime slice: expose only declared callable email commands through
   service runtime; no raw credentials or provider payloads to WASM guests.
6. Observability slice: trace/audit events, redaction, webhook idempotency tests,
   replay tests, health snapshots.
7. Developer-docs slice: complete `docs/developer-packs/communication/email.md`
   and link it from catalog metadata.

## Design Patterns

- **Facade**: SDK exposes provider-neutral email helpers.
- **Command**: every operation is a typed command/result.
- **Adapter/Bridge**: SMTP/IMAP, Gmail, Graph, SendGrid, Mailgun, mock, and
  unavailable providers adapt to one contract.
- **Strategy**: provider selection, sender identity, delivery tracking, rate
  limit, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, consent, approval, attachment governance,
  rate-limit, and redaction wrap calls.
- **Specification**: recipient, sender, attachment, webhook, and permission rules
  are executable validators.
- **Observer**: delivery events, mailbox sync events, audit events, and service
  events are subscribable.
- **Memento**: sync cursors, delivery snapshots, and effective capability reports
  support replay.

## Risks And Mitigations

- Risk: email sends happen without user/policy approval.
  Mitigation: external sends require sender/recipient policy, idempotency, and
  approval when configured.
- Risk: provider payloads leak into SDK/audit.
  Mitigation: DTO normalization and redaction gates.
- Risk: attachments bypass file/artifact policy.
  Mitigation: attachments are handles with size/type/checksum governance.
- Risk: provider-specific mailbox semantics leak into app code.
  Mitigation: provider capability reports and provider-neutral folders/labels.
- Risk: webhook replay duplicates delivery events.
  Mitigation: provider event id idempotency and replay metadata.
