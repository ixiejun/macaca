## ADDED Requirements

### Requirement: Macaca SHALL provide a supplier-grade Communication Email Pack

Macaca SHALL provide `pack.communication.email.v1` as a provider-neutral,
serviceized email pack for composing, validating recipients, drafting, sending,
scheduling, mailbox sync, thread/message fetch, attachment fetch, mailbox
mutation, delivery status, and provider event ingestion.

#### Scenario: Application declares email access
- **WHEN** an application declares `pack.communication.email.v1` with sender
  identities, mailbox access, event ingestion needs, and permission scopes
- **THEN** admission SHALL validate pack id, lifecycle, sender identity,
  permission scopes, policy bounds, service mappings, command schemas, and
  provider capability requirements
- **AND** admission SHALL produce an effective capability report with callable,
  denied, unsupported, and unavailable command states

#### Scenario: Required email provider is unavailable
- **WHEN** `pack.communication.email.v1` is required but no admitted provider can
  satisfy declared email commands
- **THEN** application readiness SHALL be blocked with structured unavailable
  diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to raw SMTP/sendmail
  access, or fake success

#### Scenario: Transactional provider lacks mailbox sync
- **WHEN** the active provider supports send but not mailbox sync
- **THEN** admission and SDK discovery SHALL report mailbox commands as
  unsupported
- **AND** SDK helpers SHALL refuse to build callable mailbox service calls

### Requirement: Email commands SHALL use typed canonical service calls

Every `email.*` operation SHALL be represented as a typed command/result DTO and
SHALL traverse the canonical service runtime path with trace, policy, resource,
entitlement, approval, health, snapshot, redaction, idempotency, and structured
error behavior.

#### Scenario: Email send succeeds
- **WHEN** a declared and policy-allowed `email.send` command is invoked with a
  valid sender, recipients, message or draft ref, and idempotency key
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the
  email service provider
- **AND** it SHALL emit sanitized policy, send-request, provider-result, and
  replay events with sender hash, recipient summary, message hash, delivery
  state, provider class, and stable trace identifiers

#### Scenario: External recipient is denied before send
- **WHEN** recipient policy, consent, approval, rate limit, sender identity, or
  attachment policy rejects `email.send`
- **THEN** Macaca SHALL return a typed denied, consent_required, rate_limited, or
  attachment_too_large result before provider send
- **AND** no provider call that sends external email SHALL occur

#### Scenario: Provider delivery event is ingested
- **WHEN** `email.ingest_event` receives a provider delivery event
- **THEN** Macaca SHALL validate signature status or mark the event untrusted,
  apply idempotency by provider event id, normalize delivery state, and emit
  sanitized audit evidence
- **AND** it SHALL not expose raw webhook secrets or raw provider payloads

### Requirement: Email messages SHALL be provider-neutral and attachment-safe

`pack.communication.email.v1` SHALL expose explicit DTOs for sender refs,
recipients, body parts, attachment refs, message refs, draft refs, sync cursors,
delivery states, and provider event refs. It SHALL NOT expose provider-native
message payloads or transport credentials to applications.

#### Scenario: Application sends an attachment
- **WHEN** an application attaches a filesystem/artifact/document handle to an
  email
- **THEN** Macaca SHALL validate declared handle access, content type, size,
  checksum, scan/redaction policy, and provider attachment support before send
- **AND** traces and audit records SHALL include only attachment count/size
  summaries and handle hashes

#### Scenario: Application syncs mailbox page
- **WHEN** `email.sync_mailbox` reads mailbox data
- **THEN** Macaca SHALL return bounded pages and provider-neutral cursors
- **AND** trace/audit evidence SHALL include cursor hashes, counts, and policy
  decisions without storing unbounded mailbox content

### Requirement: Email trace, audit, health, snapshots, and replay SHALL be sanitized

Macaca SHALL bound and sanitize email messages, recipients, attachments, mailbox
sync data, delivery events, snapshots, traces, and audit records.

#### Scenario: Email service snapshot is recorded
- **WHEN** the email service records a snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, sender
  identity summary, capability flags, rate-limit counters, cursor summaries,
  policy template hash, delivery summaries, and replay references
- **AND** it SHALL exclude raw credentials, OAuth tokens, webhook secrets, raw
  provider payloads, full message bodies, raw attachments, prompts, manifests,
  and unbounded mailbox content

#### Scenario: Replay reconstructs send decision
- **WHEN** audit replay inspects an email send
- **THEN** replay evidence SHALL include command name, sender id hash, recipient
  summary, message hash, attachment summary, policy decision, approval id when
  required, provider result, and trace identifiers
- **AND** replay SHALL NOT require raw message body or raw attachment bytes

### Requirement: Email implementation SHALL preserve Macaca boundaries

The email implementation SHALL remain owned by the email communication/gateway
service and replaceable providers. The microkernel, SDK, shells, and generic
application framework SHALL remain provider-neutral and free of application-
specific email routing.

#### Scenario: Boundary gates scan email implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path
  gates scan the implementation
- **THEN** they SHALL find no concrete email provider imports in the microkernel,
  SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned
  service registrations and typed service commands

#### Scenario: WASM app uses email host imports
- **WHEN** a WASM application invokes email host imports
- **THEN** the host imports SHALL route through the same `email.*` service command
  path used by SDK and YAML applications
- **AND** WASM code SHALL NOT receive raw credentials, raw provider payloads, or
  bypass policy

### Requirement: Email pack completion SHALL include developer documentation

The `pack.communication.email.v1` proposal SHALL NOT be marked complete until
the detailed developer guide exists and is linked from SDK discovery metadata.

#### Scenario: Developer reads email documentation
- **WHEN** a developer opens `docs/developer-packs/communication/email.md`
- **THEN** the guide SHALL document manifest declaration, sender identity,
  recipient model, compose/draft/send flow, mailbox sync, attachments, templates,
  delivery events, permission scopes, approval policy, idempotency, rate limits,
  command DTOs, result DTOs, error DTOs, unavailable diagnostics, provider
  replacement, trace/audit fields, and examples
- **AND** examples SHALL use generic data and SHALL NOT hardcode application
  business logic, provider names, credentials, raw message bodies, or
  workflow-specific email templates
