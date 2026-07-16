## ADDED Requirements

### Requirement: Macaca SHALL provide Communication Calendar Pack as a serviceized capability

Macaca SHALL provide `pack.communication.calendar.v1` as a provider-neutral
industrial pack for calendar source access, event read/write, recurrence,
availability, invites, reminders, conference metadata, sync, iCalendar
interchange, and conflict inspection. Applications SHALL declare the pack in
manifests, admission SHALL resolve it into effective capabilities, and all
operations SHALL run through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.communication.calendar.v1` as required and a calendar service provider is registered, healthy, entitled, source-compatible, timezone-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, provider capability metadata, permission scopes, policy templates, recurrence limits, sync/watch support, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing raw credentials, OAuth tokens, webhook secrets, conference secrets, raw provider payloads, or raw calendar export content

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.communication.calendar.v1` as required but provider, source support, permission, entitlement, credential reference, timezone support, resource budget, or policy support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, validation, conflict, or quota diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.communication.calendar.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Calendar commands SHALL use typed canonical service calls

Every `pack.communication.calendar.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace, policy, resource, entitlement, approval, health, snapshot, idempotency,
optimistic concurrency, cursor, replay, and structured error behavior.

#### Scenario: Event is created
- **WHEN** `calendar.create_event` is invoked with a calendar source, timezone-aware time range, event body, attendees, conflict policy, and idempotency key
- **THEN** Macaca SHALL validate declaration, permissions, timezone, recurrence budget, invite policy, provider capability, and conflict policy before invoking the provider
- **AND** it SHALL return a typed event handle, version/conflict metadata, and sanitized replay evidence

#### Scenario: Recurring event is queried
- **WHEN** `calendar.query_events` requests recurring event instances over a bounded time range
- **THEN** Macaca SHALL apply recurrence expansion limits, timezone policy, page limits, and redaction rules
- **AND** the result SHALL distinguish series, instances, exceptions, canceled instances, and provider unsupported states

#### Scenario: Availability is checked
- **WHEN** `calendar.check_availability` is invoked for participants or resources over a time window
- **THEN** Macaca SHALL require `calendar.availability` permission and privacy policy checks
- **AND** it SHALL return free/busy windows or availability summaries without exposing event details unless read-detail permission permits it

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, resource, timezone, recurrence, invite, or conflict checks reject a calendar command
- **THEN** Macaca SHALL return a typed denied, validation, conflict, or quota result before invoking the concrete provider
- **AND** audit evidence SHALL include a bounded reason code without raw event descriptions, raw invite payloads, raw provider payloads, tokens, credentials, or conference secrets

### Requirement: Calendar DTOs SHALL model sources, events, recurrence, attendees, availability, reminders, conferences, cursors, and conflicts

`pack.communication.calendar.v1` SHALL define portable DTOs for calendar
sources, events, instances, recurrence rules, attendees, availability queries,
reminders, conference handles, sync cursors, watches, conflict versions,
iCalendar interchange, provider capability, and diagnostics. Provider-specific
fields SHALL remain bounded adapter metadata and SHALL NOT become OS-layer
routing branches.

#### Scenario: Developer inspects event schema
- **WHEN** SDK schemas expose `CalendarEvent`
- **THEN** the schema SHALL include event handle, source handle, UID, sequence/version, title, redacted description handle, location, timezone-aware time range, transparency, visibility, status, organizer, attendees, reminders, recurrence, exceptions, conference handle, attachments, sensitivity, and provider metadata hash
- **AND** unbounded descriptions, raw provider payloads, conference secrets, and raw credentials SHALL be rejected or redacted before observability

#### Scenario: Developer imports iCalendar data
- **WHEN** `calendar.import_icalendar` receives iCalendar content or a content handle
- **THEN** Macaca SHALL validate size limits, timezone references, recurrence rules, alarms, organizer/attendee fields, UID/sequence semantics, and redaction policy
- **AND** invalid or unsupported constructs SHALL return typed validation diagnostics without bypassing provider-neutral DTOs

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active calendar provider
- **THEN** Macaca SHALL report event CRUD support, recurrence support, attendee/RSVP support, reminder support, conference metadata support, availability support, scheduling suggestion support, sync/watch support, iCalendar import/export support, timezone support, page limits, rate limits, lifecycle, and health
- **AND** callers SHALL use this metadata rather than provider-name branches

### Requirement: Calendar Pack SHALL enforce permissions, timezone correctness, recurrence limits, and conflict policy

`pack.communication.calendar.v1` SHALL define permission scopes for metadata
read, detail read, write, invite send, invite response, availability, reminders,
conference metadata, sync, watch, and import/export. Policy SHALL run before
side effects and SHALL account for source ownership, credential references,
timezone validity, recurrence expansion, external invite approvals, provider
capability, resource budgets, and conflict versions.

#### Scenario: Missing detail permission blocks private event fields
- **WHEN** an application has `calendar.read.metadata` but invokes `calendar.get_event` for private details
- **THEN** Macaca SHALL return redacted metadata or a typed denied result according to policy
- **AND** trace/audit evidence SHALL identify the missing scope by stable code

#### Scenario: External invite requires approval
- **WHEN** event creation, update, deletion, or RSVP would send an external invite or cancellation notice
- **THEN** Macaca SHALL require `calendar.invite.send` or `calendar.invite.respond` and any configured approval policy before provider side effects
- **AND** the audit trail SHALL distinguish local event mutation from external communication

#### Scenario: Conflict version mismatch is detected
- **WHEN** `calendar.update_event` provides an outdated conflict version, etag, or sequence hash
- **THEN** Macaca SHALL return a typed conflict result with bounded diagnostics and suggested resolution policy
- **AND** it SHALL NOT silently overwrite shared calendar state unless explicit overwrite policy is granted

### Requirement: Calendar Pack SHALL expose industrial metadata and developer documentation

`pack.communication.calendar.v1` SHALL expose descriptor metadata for provider
capabilities, command schemas, permission scopes, policy templates, timezone
support, recurrence limits, sync/watch support, import/export support, resource
budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots,
unavailable diagnostics, redaction profiles, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.communication.calendar.v1`
- **THEN** it SHALL return command namespace `calendar.*`, provider capabilities, supported commands, permissions, policy templates, timezone/recurrence limits, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, and documentation links
- **AND** examples SHALL use generic handles and synthetic data rather than application-specific workflows, provider names, credentials, or business routing

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/communication/calendar.md` SHALL document manifest declaration, permissions, DTOs, timezone rules, recurrence, event CRUD, invite/RSVP, free/busy, reminders, conference handles, sync/watch, iCalendar import/export, conflict handling, provider replacement, unavailable diagnostics, trace/audit interpretation, and operational limits
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Calendar Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.communication.calendar.v1` SHALL emit sanitized trace/audit events and
bounded snapshots for declaration, admission, source listing, event queries,
event mutations, invite actions, availability checks, reminders, conference
changes, sync checkpoints, watches, conflicts, policy/resource decisions,
provider calls, unavailable states, and replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a calendar pack snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, source health, calendar source summaries, sync cursors, watch handles, recurrence expansion limits, conflict aggregates, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, OAuth tokens, webhook secrets, raw invite payloads, raw calendar export content, conference secrets, raw provider responses, private notes, and unbounded descriptions

#### Scenario: Calendar mutation is audited
- **WHEN** an event is created, updated, deleted, canceled, imported, exported, invited, responded to, or modified with a reminder/conference handle
- **THEN** Macaca SHALL emit a sanitized audit event with stable handles, mutation type, policy decision, idempotency key, conflict version, result code, and replay pointer
- **AND** the event SHALL distinguish local mutation, provider mutation, and external communication side effects

### Requirement: Calendar implementation SHALL preserve Macaca boundaries

The `pack.communication.calendar.v1` implementation SHALL remain owned by
calendar service providers behind the service runtime. The microkernel, SDK,
shells, and generic application framework SHALL remain provider-neutral and free
of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete calendar provider or connector imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.communication.calendar.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches
