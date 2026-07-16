## ADDED Requirements

### Requirement: Macaca SHALL provide a supplier-grade Communication Messaging Pack

Macaca SHALL provide `pack.communication.messaging.v1` as a provider-neutral,
serviceized messaging pack for conversation lookup/create, participant
inspection, send, reply, edit, delete, list/fetch messages, reactions, read
receipts, attachments, delivery status, typing indicators, and provider event
ingestion.

#### Scenario: Application declares messaging access
- **WHEN** an application declares `pack.communication.messaging.v1` with sender
  identities, conversation classes, event ingestion needs, and permission scopes
- **THEN** admission SHALL validate pack id, lifecycle, sender identity,
  conversation class, permission scopes, policy bounds, service mappings, command
  schemas, and provider capability requirements
- **AND** admission SHALL produce an effective capability report with callable,
  denied, unsupported, and unavailable command states

#### Scenario: Required messaging provider is unavailable
- **WHEN** `pack.communication.messaging.v1` is required but no admitted provider
  can satisfy declared messaging commands
- **THEN** application readiness SHALL be blocked with structured unavailable
  diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to raw webhook access,
  or fake success

#### Scenario: Provider lacks reactions
- **WHEN** the active provider supports send/read but not reactions
- **THEN** admission and SDK discovery SHALL report reaction commands as
  unsupported
- **AND** SDK helpers SHALL refuse to build callable reaction service calls

### Requirement: Messaging commands SHALL use typed canonical service calls

Every `messaging.*` operation SHALL be represented as a typed command/result DTO
and SHALL traverse the canonical service runtime path with trace, policy,
resource, entitlement, approval, health, snapshot, redaction, idempotency, and
structured error behavior.

#### Scenario: Message send succeeds
- **WHEN** a declared and policy-allowed `messaging.send_message` command is
  invoked with a valid sender, conversation, content, and idempotency key
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the
  messaging service provider
- **AND** it SHALL emit sanitized policy, send-request, provider-result, and
  replay events with sender hash, conversation hash, participant summary, message
  hash, delivery state, provider class, and stable trace identifiers

#### Scenario: Formatting is unsupported
- **WHEN** requested rich content cannot be represented by the active provider
- **THEN** Macaca SHALL return `unsupported_format` unless an explicit fallback
  text is declared and policy allows downgraded rendering
- **AND** audit evidence SHALL record the formatting decision without raw provider
  payloads

#### Scenario: Provider event is ingested
- **WHEN** `messaging.ingest_event` receives a provider message/reaction/read
  event
- **THEN** Macaca SHALL validate signature or trust status, apply idempotency by
  provider event id, normalize event state, and emit sanitized audit evidence
- **AND** it SHALL not expose raw webhook secrets or raw provider payloads

### Requirement: Messaging data SHALL be provider-neutral and attachment-safe

`pack.communication.messaging.v1` SHALL expose explicit DTOs for conversations,
participants, sender identities, content, attachments, message refs, reactions,
cursors, delivery states, and provider event refs. It SHALL NOT expose provider-
native chat payloads or transport credentials to applications.

#### Scenario: Application sends an attachment
- **WHEN** an application attaches a filesystem/artifact/media handle to a
  message
- **THEN** Macaca SHALL validate declared handle access, content type, size,
  checksum, scan/redaction policy, and provider attachment support before send
- **AND** traces and audit records SHALL include only attachment count/size
  summaries and handle hashes

#### Scenario: Application lists messages
- **WHEN** `messaging.list_messages` reads conversation data
- **THEN** Macaca SHALL return bounded pages and provider-neutral cursors
- **AND** trace/audit evidence SHALL include cursor hashes, counts, and policy
  decisions without storing unbounded conversation content

### Requirement: Messaging trace, audit, health, snapshots, and replay SHALL be sanitized

Macaca SHALL bound and sanitize message bodies, participants, attachments,
conversation data, provider events, snapshots, traces, and audit records.

#### Scenario: Messaging service snapshot is recorded
- **WHEN** the messaging service records a snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, sender
  identity summary, conversation support, capability flags, rate-limit counters,
  cursor summaries, policy template hash, delivery summaries, and replay refs
- **AND** it SHALL exclude raw credentials, bot tokens, webhook secrets, raw
  provider payloads, full message bodies, raw attachments, prompts, manifests,
  and unbounded conversation content

#### Scenario: Replay reconstructs message send decision
- **WHEN** audit replay inspects a message send
- **THEN** replay evidence SHALL include command name, sender hash, conversation
  hash, participant summary, message hash, formatting decision, attachment
  summary, policy decision, provider result, and trace identifiers
- **AND** replay SHALL NOT require raw message body or raw attachment bytes

### Requirement: Messaging implementation SHALL preserve Macaca boundaries

The messaging implementation SHALL remain owned by the messaging communication
service and replaceable providers. The microkernel, SDK, shells, and generic
application framework SHALL remain provider-neutral and free of application-
specific messaging routing.

#### Scenario: Boundary gates scan messaging implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path
  gates scan the implementation
- **THEN** they SHALL find no concrete messaging provider imports in the
  microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned
  service registrations and typed service commands

#### Scenario: WASM app uses messaging host imports
- **WHEN** a WASM application invokes messaging host imports
- **THEN** the host imports SHALL route through the same `messaging.*` service
  command path used by SDK and YAML applications
- **AND** WASM code SHALL NOT receive raw credentials, raw provider payloads, or
  bypass policy

### Requirement: Messaging pack completion SHALL include developer documentation

The `pack.communication.messaging.v1` proposal SHALL NOT be marked complete until
the detailed developer guide exists and is linked from SDK discovery metadata.

#### Scenario: Developer reads messaging documentation
- **WHEN** a developer opens `docs/developer-packs/communication/messaging.md`
- **THEN** the guide SHALL document manifest declaration, provider classes,
  conversation and participant model, content and formatting model, send/reply/
  edit/delete flow, reactions, read receipts, attachments, event ingestion,
  permission scopes, approval policy, idempotency, rate limits, command DTOs,
  result DTOs, error DTOs, unavailable diagnostics, provider replacement,
  trace/audit fields, and examples
- **AND** examples SHALL use generic data and SHALL NOT hardcode application
  business logic, provider names, credentials, raw message bodies, or
  workflow-specific conversation ids
