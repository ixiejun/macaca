# Communication Calendar Pack Research

## Purpose

This note records supplier/API research and Macaca platform inventory for
`pack.communication.calendar.v1`. Calendar support must be a provider-neutral
service capability for calendar sources, events, recurrence, attendees,
availability, reminders, conferences, sync, watches, conflicts, and iCalendar
interop. It must not hardcode provider behavior, application scheduling
workflows, shell-owned conflict resolution, or raw provider payloads into OS
layers.

## Source Baseline

- Google Calendar API reference, events watch, and freebusy resources:
  <https://developers.google.com/workspace/calendar/api/v3/reference>
  and <https://developers.google.com/workspace/calendar/api/v3/reference/events/watch>
- Microsoft Graph calendar event resources, delta query, subscriptions, and
  findMeetingTimes:
  <https://learn.microsoft.com/en-us/graph/api/resources/event>,
  <https://learn.microsoft.com/en-us/graph/delta-query-events>, and
  <https://learn.microsoft.com/en-us/graph/api/user-findmeetingtimes>
- iCalendar RFC 5545 and CalDAV RFC 4791:
  <https://datatracker.ietf.org/doc/html/rfc5545>
  and <https://www.rfc-editor.org/info/rfc4791/>
- Apple EventKit:
  <https://developer.apple.com/documentation/eventkit>
- Android Calendar Provider:
  <https://developer.android.com/identity/providers/calendar-provider>

## Supplier API Notes

Google Calendar contributes source, event, availability, sync, and watch
concepts:

- CalendarList, Calendars, Events, Freebusy, Settings, and Channels split source
  metadata, event records, availability lookup, and watch registration.
- Event resources model recurrence, attendees, reminders, conference data,
  event status, etags, updated timestamps, and provider ids.
- Incremental sync and watch notifications map to cursor/watch DTOs with reset
  handling, idempotency, and replay references.
- Freebusy queries expose availability without full event details, which maps to
  a privacy-preserving availability command.

Microsoft Graph contributes mailbox/calendar integration concepts:

- Events, calendars, calendar groups, online meetings, response status, and
  attendees map to source, event, conference, attendee, and invite-response DTOs.
- Delta query supports incremental changes for calendar views and maps to
  Macaca sync cursors and reset-required diagnostics.
- Subscriptions and change notifications map to watch registration and event
  ingestion with provider event refs and signature/trust status.
- `findMeetingTimes` contributes scheduling suggestions without making Macaca
  own application-specific booking policy.

iCalendar and CalDAV contribute portable interchange and sync concepts:

- RFC 5545 models VEVENT, VTODO, VJOURNAL, VFREEBUSY, UID, SEQUENCE, RRULE,
  EXDATE/RDATE, organizer, attendee, alarms, and timezone data.
- CalDAV extends WebDAV for calendar access using iCalendar resources, calendar
  queries, ETags, collections, and sync/report behavior.
- Macaca should support import/export and conflict-version abstractions without
  exposing raw `.ics` blobs or WebDAV methods as the SDK contract.

Apple EventKit and Android Calendar Provider contribute host-local constraints:

- Host APIs provide permission-gated local calendar stores, events, reminders,
  attendees, recurrence, and local source metadata.
- Host permission and platform privacy levels should appear as provider
  capability and unavailable/denied diagnostics.
- Host-specific classes, content provider rows, and UI helpers must remain
  adapter details.

## Macaca-Owned Abstractions

`pack.communication.calendar.v1` should define these provider-neutral concepts:

- `CalendarSource`: source handle, display name, owner, timezone, access role,
  provider class, sync support, watch support, and health.
- `CalendarEvent`: event handle, source handle, UID, sequence/version, title,
  redacted description handle, location handle, time range, status, visibility,
  organizer, attendees, reminders, recurrence, conference handle, and
  sensitivity.
- `CalendarInstance`: expanded recurrence instance, original start, exception
  metadata, and instance status.
- `CalendarRecurrence`: RRULE-like frequency, interval, until/count, by-fields,
  exception dates, timezone, and expansion limit.
- `CalendarAttendee`: participant identity, role, required/optional flag,
  response state, delegated state, and privacy label.
- `CalendarAvailabilityQuery`: participants/resources, time range, timezone,
  granularity, privacy level, and maximum returned windows.
- `CalendarReminder`: method, offset, target channel, quiet-hours policy, and
  provider support state.
- `CalendarConference`: provider-neutral meeting handle, join metadata handle,
  secret-reference fields, and provider metadata hash.
- `CalendarCursor`, `CalendarWatch`, and `CalendarConflict`: sync cursor,
  watch/subscription handle, conflict version, reset policy, provider hash, and
  replay pointer.

## Existing Macaca Platform Inventory

Current repository capabilities that can back the calendar pack:

- Provider-neutral descriptors and domain-pack registration:
  `macaca-proto::ServiceDescriptor` and
  `macaca-kernel::domain_pack_registration` already provide descriptor metadata
  and boot-trace suffixes without provider parsing.
- Canonical service-call path: `macaca-kernel::service_call` enforces trace
  context through middleware and emits trace events around dispatch.
- SDK facade pattern: `macaca-sdk::SystemFacade` is composed from focused
  Strategy clients and already exposes unavailable/null-object client patterns.
- Unavailable behavior examples: Web3, tool, workbench, execution, entitlement,
  persistence, and finance service code show structured unavailable diagnostics
  instead of fake success.
- Scheduler and resource primitives: scheduler service DTOs already carry trace
  and can inform future reminder/scheduled-watch command boundaries without
  making calendar a kernel scheduler feature.
- Persistence/checkpoint support: kernel persistence and event-log lineage code
  provide Memento-style evidence patterns that can inspire cursor, conflict,
  and replay snapshots.
- Trace/audit schemas: trace service descriptor and service-call executor
  provide the Observer boundary for sanitized calendar declaration, admission,
  mutation, sync, watch, conflict, health, and snapshot events.

No existing code proves a calendar-specific provider, SDK, admission gate, DTO,
or developer guide is complete; those remain future tasks.

## Rejected Boundary Leakage

Macaca must reject:

- Google event JSON, Graph event JSON, CalDAV/WebDAV methods, raw iCalendar
  exports, EventKit classes, Android provider rows, provider etags as stable
  public ids, raw invite payloads, and conference secrets as SDK contracts.
- Application-specific booking, travel, support, sales, or staff-scheduling
  workflows in OS layers.
- Shell-owned conflict resolution, watch repair, recurrence expansion policy, or
  meeting-time scoring.
- Raw OAuth tokens, webhook secrets, conference credentials, private notes,
  unbounded descriptions, provider responses, prompts, manifests, WASM bytes, or
  package bytes in observability surfaces.

All operations must use typed calendar service commands with trace context,
policy checks, resource limits, approval for invite side effects, structured
results, sanitized audit, unavailable provider behavior, replay evidence, and
provider replacement support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
