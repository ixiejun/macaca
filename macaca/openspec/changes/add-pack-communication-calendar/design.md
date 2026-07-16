# Communication Calendar Pack Design

## Context

`pack.communication.calendar.v1` exposes calendar scheduling and availability as
a Macaca OS serviceized capability. Calendar integrations combine structured
time data, external communication, recurrence, synchronization, invitations,
reminders, and shared-state conflicts. The pack must provide a portable,
provider-neutral contract while letting provider adapters handle Google
Calendar, Microsoft Graph, CalDAV, local EventKit/Android stores, or future
calendar backends.

The design keeps calendar semantics behind typed service commands. The
microkernel owns identity, policy, resource, trace, audit, and service-call
evidence only. Providers are replaceable strategies registered through runtime
host composition roots.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Google Calendar | Calendar list, events, recurring events, attendees, reminders, conference data, freebusy, sync tokens, etags, watches | Calendar set, event/series/instance, attendee, reminder, conference handle, availability query, cursor, conflict version, watch |
| Microsoft Graph Calendar | Calendars/groups, events, recurrence, response status, online meetings, delta query, subscriptions, findMeetingTimes | Calendar source, recurrence, attendee response, online meeting handle, delta cursor, watch, scheduling suggestion |
| iCalendar/CalDAV | VEVENT, VALARM, RRULE, EXDATE, UID, SEQUENCE, organizer/attendee, ETag, calendar-query, sync REPORT | Portable event, alarm, recurrence, exception, import/export, conflict version, sync checkpoint |
| Apple EventKit | Local event store, calendars, events, recurrence, alarms, attendees, permission-gated access | Host calendar provider, local source handle, host permission diagnostics, event/reminder handles |
| Android Calendar Provider | Calendars, events, reminders, attendees, instances, availability, permissions | Local host source, event instances, reminders, attendee state, host capability report |

## Goals

- Provide stable pack id `pack.communication.calendar.v1` and command namespace
  `calendar.*`.
- Support calendar source listing, event querying, event CRUD, recurrence,
  invitations, attendee responses, availability, scheduling suggestions,
  reminders, conference metadata, iCalendar import/export, watches, incremental
  sync, and conflict inspection.
- Model timezone, recurrence expansion, event instances, exceptions, etags/
  sequence versions, idempotency, and provider sync cursors explicitly.
- Require trace/audit evidence for every declaration, admission decision,
  command, provider call, invite, reminder, sync checkpoint, conflict, snapshot,
  and unavailable state.
- Require developer documentation under
  `docs/developer-packs/communication/calendar.md`.

## Non-Goals

- Do not implement a concrete Google, Graph, CalDAV, EventKit, Android, or
  conference provider in this proposal.
- Do not create application-specific meeting booking, sales, support, travel, or
  scheduling workflow logic.
- Do not expose raw OAuth tokens, webhook secrets, invite provider payloads,
  conference credentials, raw calendar exports, or unbounded event descriptions
  in logs, traces, snapshots, SDK diagnostics, or examples.
- Do not make shell UI own calendar conflict resolution or scheduling semantics.

## Ownership And Boundaries

- Pack id: `pack.communication.calendar.v1`.
- Family: `communication`.
- Backing service owner: calendar service provider.
- SDK surface: `sdk.packs.communication.calendar`.
- Command namespace: `calendar.*`.
- Microkernel owns identity, policy facade, scheduler/resource primitives,
  service-call evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions,
  lifecycle, and effective capability projection.
- Runtime host owns provider adapter registration, service decorators, watch
  bridge composition, and sanitized diagnostics through approved composition
  roots.
- Shells render availability, approvals, and trace evidence but do not own
  scheduling policy or provider semantics.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `calendar.list_calendars` | List accessible calendar sources/sets | Returns bounded metadata and permission/capability state |
| `calendar.query_events` | Query events, instances, or series by time range/filter | Requires timezone, expansion limit, page limit, and redaction profile |
| `calendar.get_event` | Fetch event detail by handle | Requires read-detail permission for sensitive fields |
| `calendar.create_event` | Create event or recurring series | Requires idempotency key, conflict policy, invite policy, and version handle |
| `calendar.update_event` | Replace or patch event/series/instance | Requires optimistic concurrency or explicit overwrite policy |
| `calendar.delete_event` | Delete/cancel event, series, or instance | Must distinguish delete, cancel with notice, and provider unsupported |
| `calendar.respond_invite` | Accept/tentative/decline/propose new time | Requires attendee identity scope and audit reason |
| `calendar.check_availability` | Query free/busy windows for participants/resources | Requires availability permission and redacted result shape |
| `calendar.propose_times` | Ask provider or local strategy for candidate meeting times | Must report provider support and scoring metadata |
| `calendar.set_reminder` | Create/update/remove event reminders | Requires reminder permission and host/provider capability check |
| `calendar.manage_conference` | Attach, inspect, or remove conference metadata by handle | Must not expose raw conference secrets |
| `calendar.import_icalendar` | Import iCalendar data into provider-neutral DTOs or provider calendar | Requires validation, redaction, and conflict policy |
| `calendar.export_icalendar` | Export event/calendar data as bounded iCalendar representation | Requires export permission and redaction policy |
| `calendar.register_watch` | Register source watch/subscription for changes | Requires secret-reference callback config and provider capability |
| `calendar.sync_events` | Incrementally sync events using provider cursor | Requires cursor, watermark, reset handling, and snapshot metadata |
| `calendar.inspect_conflicts` | Inspect provider conflict, recurrence, or invite errors | Returns bounded conflict diagnostics and replay pointer |

## DTO Model

Core DTOs:

- `CalendarSource`: source handle, calendar id, display name, owner handle,
  timezone, access role, color token, provider class, capability hash, sync
  support, and health.
- `CalendarEvent`: event handle, source handle, UID, sequence/version, title,
  description handle/redacted description, location, time range, timezone,
  transparency, visibility, status, organizer, attendees, reminders,
  recurrence, exceptions, conference handle, attachments, sensitivity, and
  provider metadata hash.
- `CalendarInstance`: event handle, instance handle, original start, expanded
  time range, recurrence exception metadata, and status.
- `CalendarRecurrence`: RRULE-like frequency, interval, count/until, by-day,
  by-month, EXDATE/RDATE handles, timezone, and expansion limit.
- `CalendarAttendee`: participant handle, role, optional/required state,
  response state, comment handle, delegated-from/to handles, and identity scope.
- `CalendarAvailabilityQuery`: participant/resource handles, time range,
  timezone, granularity, working-hours policy, privacy level, and max windows.
- `CalendarReminder`: method, offset, channel handle, quiet-hours policy, and
  provider support metadata.
- `CalendarConference`: provider-neutral conference handle, join-url handle,
  dial-in metadata handle, provider metadata hash, and secret-reference fields.
- `CalendarCursor`: source handle, sync token/provider cursor, watermark,
  expiry, reset policy, and replay pointer.
- `CalendarConflict`: event handle, conflict version, reason code, provider
  etag/sequence hash, recommended action, and replay pointer.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `calendar.read.metadata`
- `calendar.read.details`
- `calendar.write`
- `calendar.invite.send`
- `calendar.invite.respond`
- `calendar.availability`
- `calendar.reminder`
- `calendar.conference`
- `calendar.sync`
- `calendar.watch`
- `calendar.import_export`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id, and
  trace id when available.
- Event writes require idempotency key, target calendar scope, conflict policy,
  timezone validation, recurrence expansion budget, and optimistic concurrency.
- Sending or canceling external invites may require explicit approval and must
  emit audit evidence.
- Availability results must respect privacy policy and return busy/free windows
  rather than event details unless `calendar.read.details` is granted.
- Conference metadata must use handles and secret references; raw meeting
  secrets or provider credentials are forbidden.
- iCalendar import/export must enforce size limits, recurrence expansion limits,
  redaction rules, and validation diagnostics.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
permission scopes, policy templates, provider capabilities, recurrence limits,
timezone support, sync/watch support, import/export support, availability
support, examples, unavailable diagnostics, health, compatibility, redaction
profiles, and documentation links.

The developer guide at `docs/developer-packs/communication/calendar.md` must
cover manifest declarations, permissions, DTOs, timezone rules, recurrence,
event CRUD, invites, RSVP, free/busy, reminders, conference handles, sync
cursors, watches, iCalendar import/export, conflict handling, unavailable
diagnostics, provider replacement, trace/audit interpretation, and conformance
tests.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `calendar_pack_declared`
- `calendar_pack_admission_validated`
- `calendar_pack_policy_decision`
- `calendar_source_listed`
- `calendar_event_query_requested`
- `calendar_event_mutation_requested`
- `calendar_invite_action_requested`
- `calendar_availability_checked`
- `calendar_reminder_mutated`
- `calendar_watch_registered`
- `calendar_sync_checkpoint_recorded`
- `calendar_conflict_detected`
- `calendar_pack_service_call_requested`
- `calendar_pack_service_call_succeeded`
- `calendar_pack_service_call_failed`
- `calendar_pack_unavailable`
- `calendar_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, source health,
calendar source summaries, sync cursors, watch handles, recurrence expansion
limits, conflict aggregates, policy template hash, resource counters, and
sanitized replay pointers. Snapshots must exclude raw credentials, raw invite
payloads, raw calendar export content, conference secrets, provider responses,
private notes, and unbounded descriptions.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, recurrence expansion strategy, availability
  strategy, scheduling suggestion strategy, and unavailable behavior are
  replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  redaction, and recurrence-budget checks wrap service calls.
- **Specification**: admission validates declarations, permissions, provider
  capabilities, timezone support, recurrence limits, and compatibility.
- **Observer**: watches, event changes, invite responses, health, trace, and
  audit events are subscribable.
- **Memento**: sync cursors, conflict versions, event handles, watch handles,
  and snapshots preserve recovery state.
- **Abstract Factory**: provider adapters are created only by approved runtime
  host composition roots.

## Risks And Mitigations

- Risk: recurrence expansion causes unbounded work. Mitigation: require explicit
  expansion limits, time ranges, and resource budgets.
- Risk: external invite side effects occur without approval. Mitigation: invite
  send/cancel commands pass through approval-capable policy gates.
- Risk: providers have incompatible conflict/version semantics. Mitigation:
  expose conflict versions, etag/sequence hashes, and overwrite policy
  explicitly.
- Risk: timezone errors create wrong schedules. Mitigation: every time-bearing
  DTO requires timezone or explicit floating-time policy.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only
  build canonical service-call commands and are covered by no-direct-provider
  gates.
