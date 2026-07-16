# Change: Add Communication Email Pack

## Why

Developers need `pack.communication.email.v1` as a provider-neutral email
capability for composing, drafting, sending, syncing, reading, threading,
attaching files, and inspecting delivery state. Applications should not embed
Gmail, Graph, SMTP, SendGrid, Mailgun, or provider-specific message payloads into
OS layers.

Email is a sensitive external-communication channel. It needs recipient policy,
sender identity, consent, rate limits, attachment governance, audit evidence,
delivery diagnostics, and unavailable behavior before it can be industrial-grade.

## Supplier And Platform API Research

The proposal is derived from a capability-by-capability comparison of mature
email APIs:

- SMTP/MIME/IMAP: envelope versus message headers, RFC 5322 message structure,
  MIME bodies and attachments, mailbox folders, flags, and delivery errors.
- Gmail API: messages, drafts, threads, labels, send/import/insert, attachments,
  history ids, watch notifications, and OAuth scopes.
- Microsoft Graph Mail: messages, sendMail, drafts, folders, attachments,
  subscriptions/change notifications, delta queries, and delegated/app
  permissions.
- SendGrid Mail Send API: transactional send, personalizations, templates,
  substitutions, attachments, categories, sandbox mode, and event webhooks.
- Mailgun Messages API: domain-based sending, templates, variables, test mode,
  attachments, tags, and delivery events.

Macaca borrows the stable concepts, not provider APIs:

- separate compose/draft/send/read/sync/delivery commands;
- model recipients, sender identity, body parts, and attachments explicitly;
- represent attachments as artifact/file handles, not raw unbounded bytes;
- require approval/policy for external recipient side effects;
- normalize delivery, bounce, provider rate limit, and unavailable diagnostics.

## What Changes

- Define `pack.communication.email.v1` as the canonical app-facing email pack.
- Add an industrial command surface covering compose, validate recipients, save
  draft, update draft, send, schedule send, cancel scheduled send, sync mailbox,
  list threads, fetch message, fetch attachment, apply labels/folders, mark read,
  delivery status, and provider webhook/event ingestion.
- Define provider-neutral DTO requirements for sender identity, recipients,
  message body, MIME parts, attachments, drafts, threads, mailbox cursors,
  delivery state, provider event ids, idempotency keys, and unavailable
  diagnostics.
- Define permission scopes for send, read, draft, attachment, mailbox sync,
  label/folder mutation, delivery inspection, and external event ingestion.
- Require a detailed developer guide under
  `docs/developer-packs/communication/email.md` before this proposal can be
  marked complete.
- Keep implementation ownership in an email/communication gateway service; kernel,
  SDK, shells, and application framework remain provider-neutral.

## Impact

- Affected specs: `pack-communication-email`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs, descriptor validators, application
  admission, SDK discovery, SDK command helpers, email service/gateway provider,
  webhook/event bridge, mock/unavailable providers, trace/audit event schema,
  replay tests, and dependency-boundary gates.
- Non-goals: provider-specific Gmail/Graph/SMTP/SendGrid/Mailgun payloads in SDK,
  raw credentials in app code, app-specific email templates in OS layers, or
  shell-owned delivery semantics.
