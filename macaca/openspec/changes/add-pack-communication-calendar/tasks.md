## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries,
  serviceization allowlist, design-pattern guidance, and the industrial catalog
  umbrella proposal before implementation.
- [x] 1.2 Record API notes for Google Calendar events/freebusy/sync/watch,
  Microsoft Graph calendar/events/delta/subscriptions/findMeetingTimes,
  iCalendar RFC 5545, CalDAV RFC 4791, Apple EventKit, and Android Calendar
  Provider.
- [x] 1.3 Map supplier concepts to provider-neutral calendar source, event,
  instance, recurrence, attendee, reminder, conference, availability, cursor,
  watch, conflict, and iCalendar DTOs.
- [x] 1.4 Inventory existing service descriptors, SDK clients, admission paths,
  trace/audit schemas, optional providers, mock providers, unavailable providers,
  timezone helpers, scheduler/resource primitives, and sync/checkpoint
  primitives that can back calendar service providers.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define provider-neutral DTOs for `CalendarSource`, `CalendarEvent`,
  `CalendarInstance`, `CalendarRecurrence`, `CalendarAttendee`,
  `CalendarAvailabilityQuery`, `CalendarReminder`, `CalendarConference`,
  `CalendarCursor`, `CalendarWatch`, `CalendarConflict`, and
  `CalendarProviderCapability`.
- [x] 2.2 Define typed command DTOs for `calendar.list_calendars`,
  `calendar.query_events`, `calendar.get_event`, `calendar.create_event`,
  `calendar.update_event`, `calendar.delete_event`, `calendar.respond_invite`,
  `calendar.check_availability`, `calendar.propose_times`,
  `calendar.set_reminder`, `calendar.manage_conference`,
  `calendar.import_icalendar`, `calendar.export_icalendar`,
  `calendar.register_watch`, `calendar.sync_events`, and
  `calendar.inspect_conflicts`.
- [x] 2.3 Define typed success, page, partial-sync, reset-required, denied,
  unavailable, unsupported, conflict, quota, timeout, canceled, validation, and
  provider-failure result DTOs.
- [x] 2.4 Define descriptor metadata for pack id, source types, command schemas,
  permissions, policy templates, timezone support, recurrence limits, invite
  policy, availability capability, sync/watch capability, import/export support,
  redaction profiles, SDK metadata, compatibility, diagnostics, and
  documentation links.
- [x] 2.5 Add descriptor hash, timezone validation, recurrence compatibility,
  conflict-version, redaction-profile, and schema compatibility tests.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement declaration validation for scopes:
  `calendar.read.metadata`, `calendar.read.details`, `calendar.write`,
  `calendar.invite.send`, `calendar.invite.respond`, `calendar.availability`,
  `calendar.reminder`, `calendar.conference`, `calendar.sync`,
  `calendar.watch`, and `calendar.import_export`.
- [x] 3.2 Enforce calendar source ownership, credential secret references,
  timezone validation, recurrence expansion limits, event write idempotency,
  conflict policy, external invite approval, availability privacy,
  import/export size limits, provider capability, rate limit, timeout, and
  resource budget checks before side effects.
- [x] 3.3 Reject raw credentials, OAuth tokens, webhook secrets, raw provider
  payloads, raw calendar export content, conference secrets, raw invite payloads,
  and unbounded event descriptions at admission and observability boundaries.
- [x] 3.4 Model required declarations as readiness blockers and optional
  declarations as explicit degraded effective capabilities.
- [x] 3.5 Add tests proving denied, quota, unsupported, validation, conflict, and
  unavailable paths do not call concrete calendar providers.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind calendar providers only through the service runtime
  and approved runtime-host composition roots.
- [x] 4.2 Add unavailable and mock providers with deterministic source, event,
  recurrence, availability, reminder, invite, conference, sync, watch, and
  conflict behavior.
- [x] 4.3 Add lifecycle, health, snapshot, shutdown, timeout, cancellation,
  bounded pagination, recurrence expansion, sync checkpoint, cursor resume,
  cursor reset, idempotency, optimistic concurrency, and watch support.
- [x] 4.4 Add provider capability reporting for event CRUD, recurrence,
  attendees, RSVP, reminders, conference metadata, availability, scheduling
  suggestions, sync/watch, iCalendar import/export, timezone support, page
  limits, and rate limits.
- [x] 4.5 Add canonical execution-path tests proving every calendar command
  traverses SDK/facade, service runtime decorators, and provider dispatch exactly
  once.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.communication.calendar.v1` with command
  schemas, provider capability reports, examples, availability, diagnostics,
  docs metadata, policy templates, timezone/recurrence limits, sync/watch
  support, and compatibility.
- [x] 5.2 Add focused SDK helper builders that only produce canonical traced
  service calls and return Null Object unavailable diagnostics when the pack is
  absent.
- [x] 5.3 Extend WASM/application ABI metadata so applications can declare
  calendar access, receive calendar change events, inspect availability, mutate
  events, and respond to invites only through declared permissions.
- [x] 5.4 Add generic examples for list calendars, query events, create recurring
  event, update instance, delete/cancel event, respond to invite, check
  availability, set reminder, attach conference handle, sync events,
  import/export iCalendar, conflict handling, and unavailable provider handling.

## 6. Trace, Audit, Replay, Security, And Gates

- [x] 6.1 Emit sanitized declaration, admission, source listing, event query,
  event mutation, invite action, availability, reminder, conference, sync,
  watch, conflict, policy, resource, entitlement, approval, service-call,
  provider-call, health, snapshot, and unavailable events.
- [x] 6.2 Add replay tests proving event CRUD, recurrence expansion, invite
  responses, availability checks, reminders, conference handles, sync cursors,
  watch events, and conflict handling are trace-addressable through the canonical
  service path.
- [x] 6.3 Add sanitization tests proving traces, audits, snapshots, SDK
  diagnostics, and examples do not leak raw credentials, OAuth tokens, webhook
  secrets, raw provider payloads, raw calendar exports, conference secrets, raw
  invite payloads, private notes, or unbounded descriptions.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic
  application framework do not import concrete calendar providers or connector
  adapters.
- [x] 6.5 Run `openspec validate add-pack-communication-calendar --strict`,
  targeted cargo tests, boundary gates, file-size gates, canonical execution-path
  tests, and audit replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/communication/calendar.md` with pack
  purpose, platform comparison, manifest declaration, permission scopes, command
  DTOs, result DTOs, timezone rules, recurrence, event CRUD, invite/RSVP,
  free/busy, reminders, conference handles, sync/watch, iCalendar import/export,
  conflict handling, provider replacement, unavailable diagnostics, trace/audit
  interpretation, and operational limits.
- [x] 7.2 Include generic app-facing examples for list calendars, query events,
  create/update/delete event, respond to invite, check availability, set
  reminder, manage conference handle, sync events, import/export iCalendar,
  inspect conflicts, and handle unavailable provider results.
- [x] 7.3 Include provider-author guidance for descriptor metadata, timezone
  handling, recurrence expansion, conflict versions, sync cursors, watch events,
  redaction, snapshots, quota reporting, and conformance tests.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial
  pack catalog index before marking `add-pack-communication-calendar` complete.
