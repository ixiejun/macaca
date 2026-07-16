# Workflow Schedule Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
boundary decisions, and GitNexus memo evidence for `pack.workflow.schedule.v1`.
The schedule pack owns one-shot, interval, cron, RRULE, preview, next
occurrence, trigger firing, misfire/catch-up/overlap, backfill, pause/resume,
history, snapshots, freshness, and redaction. It must not own task state,
approval decisions, delegation, review outcomes, recovery repair, calendar
messaging, low-level clock primitives, timezone lookup internals, or shell UI.

## Source Baseline

- RFC 5545 iCalendar recurrence:
  <https://datatracker.ietf.org/doc/html/rfc5545>
- Temporal Schedules:
  <https://docs.temporal.io/schedules>
- Quartz triggers, calendars, and misfire instructions:
  <https://www.quartz-scheduler.org/documentation/quartz-2.3.0/tutorials/tutorial-lesson-06.html>
  and
  <https://www.quartz-scheduler.org/documentation/quartz-2.3.0/tutorials/tutorial-lesson-04.html>
- Apache Airflow timetables, catchup, and backfill:
  <https://airflow.apache.org/docs/apache-airflow/stable/authoring-and-scheduling/timetable.html>,
  <https://airflow.apache.org/docs/apache-airflow/stable/core-concepts/dag-run.html>,
  and
  <https://airflow.apache.org/docs/apache-airflow/stable/core-concepts/backfill.html>

## Supplier API Notes

- RFC 5545 contributes RRULE, recurrence expansion, exclusions, durations,
  calendars, and timezone-sensitive recurrence. Macaca should support RRULE-like
  recurrence through provider-neutral DTOs and explicit compatibility tests.
- Temporal Schedules contribute interval/calendar specs, overlap policy, pause,
  trigger, backfill, and history. Macaca should normalize schedule state and
  generated trigger evidence without exposing Temporal payloads.
- Quartz contributes trigger types, cron semantics, calendars, and misfire
  instructions. Macaca should model misfire and overlap policies explicitly.
- Airflow contributes timetables, catch-up, backfill, logical time, and DAG run
  creation. Macaca should use these as scheduling patterns without owning
  Airflow DAG semantics or direct task creation outside canonical service calls.

## Macaca-Owned Abstractions

`pack.workflow.schedule.v1` should define `WorkflowSchedule`,
`WorkflowScheduleSpec`, `ScheduleRecurrence`, `ScheduleTimezonePolicy`,
`ScheduleMisfirePolicy`, `ScheduleOverlapPolicy`, `ScheduleTriggerRecord`,
`ScheduleBackfillRequest`, and `WorkflowScheduleError`.

The DTOs must carry recurrence type, timezone policy, DST resolver, logical
time, scheduled time, action reference, idempotency key, misfire policy,
catch-up policy, overlap policy, backfill bounds, paused state, history cursor,
redaction class, and replay pointers. Raw action payloads, raw prompts,
provider histories, application schedule names, and shell rendering state are
rejected.

## Boundary Decisions And Non-Goals

- Workflow task owns task creation/state after a trigger is authorized.
- Approval owns decision gates.
- Delegation owns assignment.
- Review owns review outcomes.
- Recovery owns repair/resume/replay.
- Communication calendar owns user-visible calendar workflows.
- Foundation time owns clocks/timers/instants; location timezone owns zone
  lookup and DST gap/fold resolution primitives.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  schedule SDK helpers should only build canonical traced service calls.
- Generic policy, resource, entitlement, trace, audit, mock-provider,
  unavailable-provider, and time primitives are reusable, but current evidence
  does not prove schedule-specific DTOs, descriptors, providers, SDK helpers,
  ABI metadata, tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
