# Communication Email Pack Research

## Purpose

This note records supplier/API research for
`pack.communication.email.v1`. The pack must support mailbox-style and
transactional email providers through one Macaca-owned service contract without
leaking provider payloads, credentials, templates, webhook bodies, or mailbox
state semantics into SDK, WASM ABI, shell, kernel, or generic application code.

## Source Baseline

- IETF RFC 5321 SMTP:
  <https://datatracker.ietf.org/doc/html/rfc5321>
- IETF RFC 5322 Internet Message Format:
  <https://www.rfc-editor.org/info/rfc5322>
- Gmail API messages, drafts, labels, history, and watch:
  <https://developers.google.com/workspace/gmail/api/reference/rest>
- Microsoft Graph Mail API overview and permissions:
  <https://learn.microsoft.com/en-us/graph/api/resources/mail-api-overview>
  and <https://learn.microsoft.com/en-us/graph/permissions-reference>
- SendGrid Mail Send API personalizations and event webhook concepts:
  <https://www.twilio.com/docs/sendgrid/for-developers/sending-email/personalizations>
- Mailgun Messages API and HTTP sending guides:
  <https://documentation.mailgun.com/docs/mailgun/api-reference/send/mailgun/messages>
  and <https://documentation.mailgun.com/docs/mailgun/user-manual/sending-messages/send-http>

## SMTP, MIME, And IMAP Summary

SMTP and message-format standards establish the difference between transport
delivery and message content:

- SMTP models transport envelopes, sender/recipient routing, delivery attempts,
  and failure replies. Macaca should keep envelope recipient validation separate
  from rendered message body and delivery state.
- RFC 5322 models message headers and body syntax. Macaca should treat the
  RFC-shaped message as a provider artifact behind adapters, not as the stable
  SDK object.
- MIME-style multipart bodies and attachments map to Macaca body-part and
  attachment-reference DTOs with bounded size, content type, checksum, and
  redaction policy.
- IMAP mailbox folders, flags, and server state map to provider-neutral folder,
  label, read-state, sync-cursor, and message-reference abstractions.
- SMTP, MIME, and IMAP commands must not be exposed as application-facing
  service commands because they carry transport-specific semantics.

## Gmail API Summary

Gmail contributes a mature mailbox API model:

- Messages, drafts, threads, and labels separate content, editable pending
  state, conversation grouping, and mailbox organization.
- Send, import, insert, draft-create, draft-update, and draft-send operations
  imply separate Macaca commands for compose, draft, send, and mailbox ingest.
- Attachments are fetched by message/attachment references, so Macaca should
  return artifact/file handles instead of raw attachment bytes by default.
- History ids and watch notifications provide mailbox synchronization and
  change-event concepts that map to sync cursors and event ingestion.
- OAuth scopes and token state must be represented as permissions and secret
  references, never as SDK-visible credentials.

## Microsoft Graph Mail Summary

Microsoft Graph contributes Outlook mailbox and tenant-permission concepts:

- `sendMail`, messages, drafts, folders, and attachments map to command-level
  send, read, draft, folder/label mutation, and attachment operations.
- Delta queries and change notifications/subscriptions map to sync cursors,
  event subscriptions, provider event refs, and idempotent event ingestion.
- Delegated and application permissions require Macaca to expose explicit scope
  diagnostics and consent state rather than assuming a single user-token model.
- Graph mailbox folders and message resources remain provider-native details.
  Macaca should expose normalized folders/labels and provider capability reports.
- Tenant and mailbox authorization failure must return structured denied,
  consent_required, unavailable, or provider_failure results.

## SendGrid Mail Send API Summary

SendGrid contributes transactional email concepts:

- Personalizations model per-recipient delivery fields and substitutions. Macaca
  should normalize these as recipient sets, template refs, and bounded variables
  while keeping application-owned template content outside OS code.
- Dynamic templates and substitutions map to template references and variable
  DTOs. The OS must not own campaign or business-template semantics.
- Attachments, categories, custom args, sandbox mode, and scheduling map to
  attachment refs, tagging metadata, test-mode policy, idempotency, and
  provider capability flags.
- Event webhooks provide accepted, delivered, bounced, deferred, dropped,
  spam/complaint, and engagement-style delivery evidence. Macaca should ingest
  only normalized delivery events with signature/trust status.
- SendGrid API keys and webhook secrets must be secret references only.

## Mailgun Messages API Summary

Mailgun contributes domain-based transactional sending and event diagnostics:

- Domain sending and sender verification map to Macaca sender identities,
  provider-domain capability, and policy checks before side effects.
- Templates and variables map to provider template refs and bounded variable
  DTOs. The provider-native MIME headers and variable carrier formats must not
  become SDK contracts.
- Tags, test mode, scheduling, and attachments map to normalized metadata,
  dry-run/test policy, scheduled-send capability, and attachment references.
- Delivery events and event webhooks map to delivery-state records,
  provider-event refs, idempotency keys, and sanitized audit events.
- Provider-specific HTTP form fields, webhook payloads, and credentials must
  remain inside adapter boundaries.

## Macaca-Owned Abstractions

`pack.communication.email.v1` should define these provider-neutral concepts:

- `EmailSenderRef`: verified account/domain identity, reply-to policy,
  provider class, consent state, and secret-reference binding.
- `EmailRecipient`: to/cc/bcc role, address, display name, consent status,
  domain policy result, and redaction label.
- `EmailEnvelope`: sender, recipients, external-recipient classification,
  idempotency key, approval ref, and provider capability requirements.
- `EmailBodyPart`: plain text, HTML, markdown, template ref, artifact ref,
  language, encoding, and redaction policy.
- `EmailAttachmentRef`: filesystem/artifact/document handle, content type,
  size, checksum, inline content-id metadata, scan state, and redaction policy.
- `EmailDraftRef`, `EmailMessageRef`, and `EmailThreadRef`: provider-neutral
  references with revision, folder/label summaries, and trace binding.
- `EmailSyncCursor`: mailbox/folder selector, cursor token hash, history/delta
  id, provider class, and replay reference.
- `EmailDeliveryState`: accepted, queued, scheduled, sent, delivered, bounced,
  complained, deferred, failed, cancelled, unknown, and provider_rejected.
- `EmailProviderCapability`: mailbox support, transactional support, draft
  support, schedule support, attachment limits, event ingestion support,
  rate-limit state, sender identities, unavailable reasons, and health.

## Rejected Boundary Leakage

Macaca must not expose these provider-native or application-specific shapes as
stable SDK/ABI contracts:

- SMTP commands, IMAP commands, raw MIME messages, provider HTTP forms, Graph
  message JSON, Gmail raw/base64 message payloads, SendGrid personalizations, or
  Mailgun-specific message fields.
- OAuth tokens, SMTP credentials, API keys, webhook secrets, provider signature
  keys, or raw provider authorization responses.
- Full message bodies, raw attachments, unbounded mailbox pages, raw provider
  webhook bodies, prompts, manifests, WASM bytes, package bytes, or provider
  retry payloads in trace/audit/snapshot output.
- Application-specific email templates, campaign rules, marketing workflows,
  sender business policy, or mailbox UI semantics in OS-layer code.

All operations must enter through typed Macaca email service commands with trace
context, policy checks, resource limits, approval where required, structured
result envelopes, sanitized audit events, unavailable provider behavior,
idempotency, replay evidence, and provider replacement support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
