# Change: Add Industrial Workflow Schedule Pack

## Why

Macaca applications need `pack.workflow.schedule.v1` for durable, auditable scheduling of workflow tasks and service commands: one-shot timers, intervals, cron-like expressions, calendar recurrence, exclusions, time zones, daylight-saving behavior, catch-up, misfire handling, pause/resume, backfill, jitter, concurrency policy, trigger history, and replay evidence.

The current template is too shallow because production scheduling is not just "run later"; it must define recurrence grammar, time-zone database behavior, missed-fire policy, idempotent trigger generation, bounded catch-up, and interaction with the workflow task pack without hardcoding business workflows.

## Supplier/API Baseline

- RFC 5545 iCalendar recurrence: RRULE/RDATE/EXDATE recurrence sets, DTSTART alignment, calendar-aware recurrence, and interoperable recurrence semantics. Official RFC: https://datatracker.ietf.org/doc/html/rfc5545
- Temporal Schedules: schedule identity independent from workflow execution, actions, overlap policy, catch-up window, pause, backfill, and next-run introspection. Official docs: https://docs.temporal.io/schedule
- Quartz Scheduler: cron triggers, simple triggers, calendars/exclusions, misfire instructions, and trigger state. Official docs: https://www.quartz-scheduler.org/documentation/quartz-2.3.0/tutorials/tutorial-lesson-06.html
- Apache Airflow Timetables: schedule calculation, logical date, data interval, catch-up/backfill, and DAG run generation. Official docs: https://airflow.apache.org/docs/apache-airflow/stable/authoring-and-scheduling/timetable.html

## Macaca Provider-Neutral Mapping

Macaca SHALL expose schedules as service-owned trigger plans:

- Schedule CRUD becomes `workflow_schedule.create`, `workflow_schedule.update`, `workflow_schedule.pause`, `workflow_schedule.resume`, and `workflow_schedule.delete`.
- Trigger computation becomes `workflow_schedule.preview`, `workflow_schedule.next_occurrences`, and `workflow_schedule.inspect`.
- Execution generation becomes `workflow_schedule.fire_due`, `workflow_schedule.backfill`, and canonical integration with `pack.workflow.task.v1`.
- Misfire/catch-up handling becomes explicit policy DTOs.
- Audit/replay becomes `workflow_schedule.get_history` and `workflow_schedule.snapshot`.

## What Changes

- Add `pack.workflow.schedule.v1` as a service-backed industrial pack under the workflow family.
- Define command DTOs for schedule CRUD, preview, due firing, backfill, pause/resume, deletion, history, and snapshots.
- Define DTOs for schedule specs, recurrence, timezone/DST policy, exclusions, trigger windows, misfire/catch-up policy, overlap policy, jitter, generated trigger records, and structured errors.
- Define permission scopes, policy/resource/entitlement rules, idempotent trigger keys, concurrency integration, trace/audit events, and unavailable diagnostics.
- Require detailed developer documentation under `docs/developer-packs/workflow/schedule.md`.

## Impact

- Affected specs: `pack-workflow-schedule`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Later affected code: protocol DTOs, descriptor/admission validators, SDK pack client, schedule service provider contract, recurrence evaluator, mock/unavailable providers, trace/audit schemas, and boundary gates.
- Validation: `openspec validate add-pack-workflow-schedule --strict`, recurrence tests, DST tests, misfire/catch-up tests, backfill tests, task integration tests, no-direct-provider-call gates, and docs coverage checks.

## Non-Goals

- This pack does not execute tasks directly, approve work, delegate workers, review outcomes, recover failures, own calendar invitations, or encode application-specific schedules.
- This pack does not hardcode cron strings, timezone names as business rules, task names, provider names, app names, or workflow names into OS-layer routing.
- This pack does not expose raw payloads, prompts, secrets, provider payloads, unbounded schedule history, or application-specific trigger bodies in traces, audits, snapshots, logs, or examples.
