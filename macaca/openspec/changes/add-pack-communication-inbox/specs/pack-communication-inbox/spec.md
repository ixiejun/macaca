## ADDED Requirements

### Requirement: Macaca SHALL provide Communication Inbox Pack as a serviceized capability

Macaca SHALL provide `pack.communication.inbox.v1` as a provider-neutral
industrial pack for unified inbox source registration, incremental sync, event
ingestion, item/thread access, label/folder/read-state mutation, claim leases,
and unavailable diagnostics. Applications SHALL declare the pack in manifests,
admission SHALL resolve it into effective capabilities, and all operations SHALL
run through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.communication.inbox.v1` as required and an inbox aggregation service provider is registered, healthy, entitled, source-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, source capability metadata, permission scopes, policy templates, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing raw credentials, OAuth tokens, webhook secrets, raw provider payloads, raw bodies, or raw attachments

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.communication.inbox.v1` as required but provider, source support, permission, entitlement, credential reference, resource budget, or policy support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, or quota diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.communication.inbox.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Inbox commands SHALL use typed canonical service calls

Every `pack.communication.inbox.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace, policy, resource, entitlement, approval, health, snapshot, idempotency,
cursor, replay, and structured error behavior.

#### Scenario: Source is registered
- **WHEN** `inbox.register_source` is invoked with provider-neutral source metadata and secret references for credentials or webhook secrets
- **THEN** Macaca SHALL validate declaration, entitlement, credential references, provider capability, policy, and descriptor compatibility before registering the source
- **AND** the result SHALL contain an opaque source handle and sanitized capability metadata rather than raw credentials or provider payloads

#### Scenario: Sources are synchronized
- **WHEN** `inbox.sync_sources` or `inbox.resume_sync` is invoked with a source handle, cursor, page budget, and timeout budget
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and inbox aggregation provider
- **AND** it SHALL return typed sync results with item upserts, cursor checkpoints, watermarks, duplicate counts, reset-required state, partial state, and replay pointers

#### Scenario: Provider event is ingested
- **WHEN** `inbox.ingest_event` receives a webhook or event-feed change from a provider adapter
- **THEN** Macaca SHALL require source validation, idempotency key, provider event hash, bounded payload reference, and policy checks
- **AND** duplicate or out-of-order events SHALL be deduplicated or recorded as bounded conflict diagnostics without corrupting cursor state

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, resource, body-access, attachment-access, or credential-reference checks reject an inbox command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** audit evidence SHALL include a bounded reason code without raw body text, raw attachments, raw provider payloads, tokens, credentials, or webhook secrets

### Requirement: Inbox DTOs SHALL model sources, cursors, items, threads, labels, attachments, events, and claims

`pack.communication.inbox.v1` SHALL define portable DTOs for source metadata,
incremental sync cursors, inbox items, threads, labels/folders/flags, attachment
handles, provider events, sync checkpoints, claim leases, provider capability,
and diagnostics. Provider-specific fields SHALL remain bounded adapter metadata
and SHALL NOT become OS-layer routing branches.

#### Scenario: Developer lists inbox items
- **WHEN** `inbox.list_items` returns items
- **THEN** each `InboxItem` SHALL include a stable Macaca item handle, source handle, item kind, provider item reference, thread handle, actor/participant handles, redacted preview, timestamps, read state, labels, folder/stream, flags, sensitivity, attachment handles, and content hash
- **AND** the page SHALL be bounded and sanitized according to the active redaction profile

#### Scenario: Developer fetches body or attachment
- **WHEN** `inbox.fetch_body` or `inbox.fetch_attachment` is invoked
- **THEN** Macaca SHALL require stronger content permissions than metadata listing and SHALL return bounded content handles or redacted body parts
- **AND** raw full bodies and raw attachments SHALL NOT enter traces, audits, snapshots, SDK diagnostics, or examples

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active inbox provider
- **THEN** Macaca SHALL report source kinds, query support, label/folder support, read-state support, body/attachment support, watch/event support, mutation support, cursor expiry, page limits, rate limits, lifecycle, and health
- **AND** callers SHALL use this metadata rather than provider-name branches

### Requirement: Inbox Pack SHALL enforce permissions, redaction, idempotency, and sync recovery

`pack.communication.inbox.v1` SHALL define permission scopes for source
management, sync, event ingest, metadata read, body read, attachment read,
search, triage write, claim, and summarization. Policy SHALL run before side
effects and SHALL account for source ownership, provider capability, credential
references, webhook trust, content sensitivity, resource budget, cursor state,
and approval.

#### Scenario: Missing body permission blocks content access
- **WHEN** an application has `inbox.read.metadata` but invokes `inbox.fetch_body`
- **THEN** Macaca SHALL return a typed denied result and SHALL NOT fetch full body content from the provider
- **AND** trace/audit evidence SHALL identify the missing scope by stable code

#### Scenario: Cursor expires or becomes invalid
- **WHEN** a provider reports that an incremental sync cursor, history id, UID state, or event watermark is no longer valid
- **THEN** Macaca SHALL return a typed `reset_required` or partial-sync result with bounded recovery metadata
- **AND** full resync SHALL occur only through an explicit policy-admissible sync command with trace and resource budgets

#### Scenario: Item is claimed for processing
- **WHEN** `inbox.claim_item` is invoked by an agent, session, or task
- **THEN** Macaca SHALL create a bounded claim lease with owner identity, expiry, replay pointer, and recovery policy
- **AND** expired or conflicting claims SHALL return typed conflict diagnostics rather than silently overwriting assignment state

### Requirement: Inbox Pack SHALL expose industrial metadata and developer documentation

`pack.communication.inbox.v1` SHALL expose descriptor metadata for source
capabilities, command schemas, permission scopes, policy templates, sync limits,
event ingestion schema, query capabilities, mutation capabilities, resource
budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots,
unavailable diagnostics, redaction profiles, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.communication.inbox.v1`
- **THEN** it SHALL return command namespace `inbox.*`, source capabilities, supported commands, permissions, policy templates, sync limits, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, and documentation links
- **AND** examples SHALL use generic handles and synthetic data rather than application-specific workflows, provider names, credentials, or business routing

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/communication/inbox.md` SHALL document manifest declaration, source registration, permissions, DTOs, result handling, cursor handling, sync loops, event ingestion, deduplication, label/folder mapping, read-state mutation, attachment handling, claim leases, provider replacement, unavailable diagnostics, trace/audit interpretation, and operational limits
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Inbox Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.communication.inbox.v1` SHALL emit sanitized trace/audit events and
bounded snapshots for declaration, admission, source registration, source health,
sync requests, cursor checkpoints, event ingestion, item upserts, mutations,
claims, policy/resource decisions, provider calls, unavailable states, and
replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records an inbox pack snapshot
- **THEN** the snapshot SHALL include descriptor version, source capability hashes, source health, cursor checkpoints, watermark summaries, item counts, label mappings, claim leases, sync error aggregates, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, OAuth tokens, webhook secrets, raw provider payloads, raw full bodies, raw attachments, manifests, package bytes, private keys, signatures, and unbounded output

#### Scenario: Item mutation is audited
- **WHEN** an item is labeled, moved, archived, marked read, claimed, released, or summarized
- **THEN** Macaca SHALL emit a sanitized audit event with stable handles, mutation type, policy decision, idempotency key, result code, and replay pointer
- **AND** the event SHALL distinguish provider mutation from local overlay state

### Requirement: Inbox implementation SHALL preserve Macaca boundaries

The `pack.communication.inbox.v1` implementation SHALL remain owned by inbox
aggregation service providers behind the service runtime. The microkernel, SDK,
shells, and generic application framework SHALL remain provider-neutral and free
of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete inbox provider or connector imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.communication.inbox.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches
