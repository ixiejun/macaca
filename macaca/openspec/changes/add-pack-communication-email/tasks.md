## 1. Supplier API Research And Scope

- [x] 1.1 Read and summarize SMTP, MIME, and IMAP concepts for envelopes, RFC
  message structure, attachments, folders, flags, and delivery errors.
- [x] 1.2 Read and summarize Gmail API messages, drafts, threads, labels, send,
  attachments, history ids, watch notifications, and OAuth scopes.
- [x] 1.3 Read and summarize Microsoft Graph Mail APIs for sendMail, messages,
  drafts, folders, attachments, delta queries, subscriptions, and permissions.
- [x] 1.4 Read and summarize SendGrid Mail Send API for personalizations,
  templates, substitutions, attachments, sandbox mode, categories, and event
  webhooks.
- [x] 1.5 Read and summarize Mailgun Messages API for domain sending, templates,
  variables, tags, test mode, attachments, and delivery events.
- [x] 1.6 Convert the supplier comparison into Macaca-owned abstractions and
  explicitly reject provider-native message payloads and credentials in SDK.
- [x] 1.7 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.communication.email.v1` descriptor metadata: lifecycle,
  stability, service ids, command namespace, command schemas, permission scopes,
  policy template, resource template, SDK metadata, docs link, health, snapshot,
  and unavailable diagnostics.
- [x] 2.2 Define command DTOs for `email.compose`, `email.validate_recipients`,
  `email.save_draft`, `email.update_draft`, `email.send`,
  `email.schedule_send`, `email.cancel_scheduled_send`, `email.sync_mailbox`,
  `email.list_threads`, `email.fetch_message`, `email.fetch_attachment`,
  `email.apply_labels`, `email.mark_read`, `email.delivery_status`, and
  `email.ingest_event`.
- [x] 2.3 Define shared DTOs for sender ref, recipients, body parts, attachment
  refs, message refs, draft refs, sync cursors, delivery states, provider event
  refs, consent status, rate-limit status, provider capability report, and stable
  descriptor hashes.
- [x] 2.4 Define result/error DTOs for success, partial page, denied,
  invalid_sender, invalid_recipient, consent_required, attachment_too_large,
  unsupported, rate_limited, provider_rejected, unavailable, and
  provider_failure.
- [x] 2.5 Add schema compatibility tests and stable hash tests for command,
  result, health, snapshot, provider capability, and unavailable DTOs.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement manifest declaration validation for required/optional
  `pack.communication.email.v1`, sender identities, mailbox access, and event
  ingestion endpoints.
- [x] 3.2 Validate scopes: `email.send`, `email.read`, `email.draft`,
  `email.attachment`, `email.mailbox.sync`, `email.mailbox.mutate`,
  `email.delivery.read`, and `email.event.ingest`.
- [x] 3.3 Add policy checks for sender verification, recipient/domain policy,
  consent, external recipient approval, message size, attachment count/size/type,
  rate limits, webhook signature state, idempotency, and provider capability.
- [x] 3.4 Add approval behavior for external sends, scheduled sends, broad mailbox
  sync, attachment fetch/export, and mailbox mutation.
- [x] 3.5 Require provider credentials through secret references only.
- [x] 3.6 Add tests proving denied, unavailable, invalid_recipient,
  consent_required, attachment_too_large, rate_limited, and provider_rejected
  paths do not send email.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Define the email service trait/provider interface behind the service
  runtime.
- [x] 4.2 Implement unavailable provider behavior for absent email service,
  missing sender identity, unsupported draft/schedule/mailbox/event behavior,
  missing entitlement, and provider health failure.
- [x] 4.3 Implement deterministic mock provider for contract, replay, and delivery
  event tests.
- [x] 4.4 Implement adapter bridge points for SMTP/IMAP, Gmail, Microsoft Graph,
  SendGrid, and Mailgun without leaking provider-native APIs to SDK callers.
- [x] 4.5 Add webhook/event ingestion bridge with signature status, idempotency,
  provider event refs, and normalized delivery states.
- [x] 4.6 Add lifecycle, health, snapshot, shutdown, sync cursor management,
  attachment handling, rate-limit reporting, redaction, and provider capability
  reports.

## 5. SDK, WASM ABI, And Application Framework

- [x] 5.1 Extend SDK discovery with pack metadata, command schemas, sender
  identities, provider capabilities, mailbox/transactional support, attachment
  limits, permissions, policy templates, health, diagnostics, and docs link.
- [x] 5.2 Add SDK command builders for every `email.*` command; builders must only
  produce canonical traced service calls.
- [x] 5.3 Add SDK helpers for compose, draft, send with approval, attachment ref
  binding, mailbox sync, message fetch, delivery status, event ingestion, and
  unavailable diagnostics.
- [x] 5.4 Extend effective capability projection so applications can inspect
  callable commands, denied commands, unavailable providers, sender identities,
  provider capability flags, rate limits, and replay references.
- [x] 5.5 Expose WASM host imports only for declared callable email commands and
  route every import through the service runtime path.
- [x] 5.6 Add app-framework tests proving YAML, WASM, GenUI, and headless apps all
  use the same email execution path.

## 6. Trace, Audit, Replay, And Gates

- [x] 6.1 Emit sanitized events for declaration, admission, policy, compose, draft,
  send request, send accepted/failed, mailbox sync, attachment fetch, delivery
  event ingestion, success, failure, denied, and unavailable states.
- [x] 6.2 Add audit redaction tests proving raw OAuth tokens, SMTP credentials,
  webhook secrets, raw provider payloads, full message bodies, raw attachments,
  prompts, manifests, and unbounded mailbox content do not enter observability
  surfaces.
- [x] 6.3 Add replay tests proving email commands and delivery events are
  trace-addressable and can reconstruct send/delivery decisions without raw
  message bodies or attachments.
- [x] 6.4 Add dependency-boundary tests proving kernel, SDK, shells, and
  application framework do not import concrete email providers.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers and WASM host
  imports cannot bypass service runtime.
- [x] 6.6 Run `openspec validate add-pack-communication-email --strict`, targeted
  cargo tests, dependency-boundary gates, file-size gates, and audit replay
  checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/communication/email.md`.
- [x] 7.2 Document purpose, manifest declaration, sender identity, recipient
  model, compose/draft/send flow, mailbox sync, attachments, templates, delivery
  events, permissions, approval policy, idempotency, rate limits, command DTOs,
  result DTOs, error DTOs, unavailable diagnostics, and provider replacement.
- [x] 7.3 Add minimal examples for compose/send with approval, save draft, send
  with artifact attachment, sync mailbox page, fetch attachment safely, delivery
  status, denied external recipient, unavailable provider, and webhook event
  ingestion.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack
  catalog index before marking this proposal complete.
