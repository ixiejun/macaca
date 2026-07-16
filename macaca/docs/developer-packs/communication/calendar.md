# Communication Calendar Pack

`pack.communication.calendar.v1` defines calendar source, event, recurrence,
invite, availability, reminder, conference, sync, watch, iCalendar, and conflict
operations through a provider-neutral service boundary.

## Manifest Declaration

```yaml
service_contract:
  optional_packs:
    - pack.communication.calendar.v1
```

No installed provider returns `calendar_provider_not_installed`.

## Permissions

Use `calendar.read.metadata`, `calendar.read.details`, `calendar.write`,
`calendar.invite.send`, `calendar.invite.respond`, `calendar.availability`,
`calendar.reminder`, `calendar.conference`, `calendar.sync`, `calendar.watch`,
and `calendar.import_export`.

## Commands And DTOs

Core DTOs include `CalendarSource`, `CalendarEvent`, `CalendarInstance`,
`CalendarRecurrence`, `CalendarAttendee`, `CalendarAvailabilityQuery`,
`CalendarReminder`, `CalendarConference`, `CalendarCursor`, `CalendarWatch`,
`CalendarConflict`, and `CalendarProviderCapability`.

Commands are list calendars, query/get/create/update/delete event, respond to
invite, check availability, propose times, set reminder, manage conference,
import/export iCalendar, register watch, sync events, and inspect conflicts.

## Examples

List calendars:

```json
{"page_size": 100}
```

Create recurring event:

```json
{
  "event": {
    "event_id": "event",
    "source_id": "calendar",
    "title_ref": "artifact:title",
    "start_epoch_ms": 1800000000000,
    "end_epoch_ms": 1800003600000,
    "timezone_id": "UTC",
    "recurrence": {"frequency": "weekly", "interval": 1, "expansion_limit": 32}
  },
  "idempotency_key": "cal-001"
}
```

Respond to invite:

```json
{"event_id": "event", "attendee_id": "self", "response_state": "accepted"}
```

Check availability:

```json
{"query": {"participant_ids": ["user"], "start_epoch_ms": 1800000000000, "end_epoch_ms": 1800036000000, "timezone_id": "UTC"}}
```

Set reminder and conference handle:

```json
{"event_id": "event", "reminders": [{"reminder_id": "r1", "offset_minutes": -10, "channel_handle": "notification"}]}
```

Sync and conflict handling:

```json
{"source": {"source_id": "calendar", "timezone_id": "UTC"}, "cursor": {"cursor_hash": "cursor"}, "page_size": 100}
```

Unavailable provider:

```json
{"status": "unavailable", "error": {"code": "unavailable", "message": "calendar provider is not installed"}}
```

## App-Facing Example Coverage

Generic examples cover list calendars, query events, create recurring event,
update a recurring instance, delete or cancel an event, respond to an invite,
check availability, set reminders, attach conference handles, sync events,
import/export iCalendar handles, conflict handling, and unavailable provider
handling. All examples use synthetic calendar, event, recurrence, attendee,
availability, conference, cursor, and artifact refs; they must not encode
provider workflows, private titles, private attendees, conference secrets, or
application-specific scheduling policy.

## Provider Author Guidance

Provider classes are `calendar-sync`, `availability-bridge`, `event-store`,
`mock`, and `unavailable`. Providers must report timezone handling, recurrence
expansion limits, conflict versions, sync cursors, watch events, redaction,
snapshots, quota status, and conformance tests. Raw credentials, OAuth tokens,
webhook secrets, invite payloads, conference secrets, calendar exports, private
notes, and unbounded descriptions must stay out of observability surfaces.
