# Change: Add Industrial Communication Calendar Pack

## Why

Applications need calendar capability for scheduling, availability, invites,
recurrence, reminders, conflict detection, and calendar synchronization. A
production calendar pack must do more than create a single event: it must model
calendar sets, event instances, recurring series, attendees, RSVP state,
free/busy queries, conference metadata, reminders, incremental sync, provider
conflict versions, timezone correctness, iCalendar import/export, trace, audit,
and provider replacement.

Calendar operations are user-sensitive and often send external invitations or
modify shared schedules. They must be serviceized behind policy, permission,
resource, approval, entitlement, redaction, and replay gates instead of being
implemented as application-specific workflow code.

## Supplier And Platform API Research

This proposal maps common concepts from mature calendar platforms into Macaca
provider-neutral abstractions:

- Google Calendar API exposes calendar lists, events, recurring events,
  attendees, reminders, conference data, freebusy queries, sync tokens, etags,
  ACLs, watch channels, and import/export behavior. Macaca maps these to
  `CalendarSet`, `CalendarEvent`, `CalendarRecurrence`, `CalendarAttendee`,
  `CalendarReminder`, `CalendarConference`, `CalendarAvailabilityQuery`,
  `CalendarCursor`, optimistic concurrency, and event watches.
- Microsoft Graph Calendar exposes calendars, events, calendar groups,
  attendees, response status, online meeting metadata, reminders, recurrence,
  delta query, subscriptions, and findMeetingTimes. Macaca maps these to
  availability windows, attendee response state, online meeting handles,
  incremental cursors, source subscriptions, and provider scheduling assistance
  capability metadata.
- iCalendar RFC 5545 defines interoperable VEVENT, VTODO, VALARM, RRULE,
  EXDATE, timezone, organizer, attendee, sequence, UID, and status semantics.
  CalDAV RFC 4791 defines calendar collections, calendar-query, sync, scheduling
  extensions, ETag, and REPORT flows. Macaca maps these standards to portable
  recurrence, exception, alarm, organizer/attendee, import/export, sync cursor,
  and conflict-version DTOs.
- Apple EventKit and Android Calendar Provider expose local calendars, event
  stores, recurrence rules, alarms/reminders, availability, attendees, and
  permission-gated read/write access. Macaca maps these to local-host provider
  capabilities, host permission diagnostics, event handles, reminders, and
  calendar-source metadata.

The Macaca contract keeps provider-specific fields in bounded adapter metadata.
OS layers do not branch on Google, Graph, CalDAV, Apple, Android, or any
provider-specific event type.

## What Changes

- Add provider-neutral `pack.communication.calendar.v1` under the
  `communication` family.
- Define DTOs for calendar sets, events, event instances, recurring series,
  recurrence rules, exceptions, attendees, organizers, reminders, availability
  windows, conference handles, sync cursors, watches, conflict versions, and
  iCalendar interchange.
- Define commands for list calendars, query events, get event, create/update/
  patch/delete event, respond to invite, check availability, propose times,
  manage reminders, import/export iCalendar, register watches, sync, and inspect
  conflicts.
- Define permission scopes for read metadata, read details, write, invite,
  availability, reminder, conference metadata, sync/watch, and import/export.
- Require timezone correctness, recurrence expansion limits, optimistic
  concurrency, idempotency, external invite approval, redaction, replayable
  snapshots, and a detailed developer guide.

## Impact

- Affected specs: `pack-communication-calendar`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected future code: provider-neutral proto DTOs, calendar descriptors,
  admission validators, SDK discovery metadata, focused SDK clients, calendar
  service providers, unavailable/mock providers, trace/audit schemas, replay
  tests, timezone/recurrence compatibility tests, and dependency-boundary gates.
- Non-goals: no application-specific scheduling workflow, no provider-name
  routing in OS layers, no raw credential/provider payload exposure, no concrete
  provider construction in kernel/SDK/shells, and no fake success when calendar
  providers are unavailable.

## References

- Google Calendar API:
  https://developers.google.com/calendar/api/guides/overview
- Google Calendar Events:
  https://developers.google.com/calendar/api/v3/reference/events
- Google Calendar Freebusy:
  https://developers.google.com/calendar/api/v3/reference/freebusy/query
- Microsoft Graph Calendar:
  https://learn.microsoft.com/en-us/graph/api/resources/calendar
- Microsoft Graph Event:
  https://learn.microsoft.com/en-us/graph/api/resources/event
- Microsoft Graph findMeetingTimes:
  https://learn.microsoft.com/en-us/graph/api/user-findmeetingtimes
- iCalendar RFC 5545: https://www.rfc-editor.org/rfc/rfc5545
- CalDAV RFC 4791: https://www.rfc-editor.org/rfc/rfc4791
- Apple EventKit: https://developer.apple.com/documentation/eventkit
- Android Calendar Provider:
  https://developer.android.com/guide/topics/providers/calendar-provider
