# Workflow Schedule Pack Design

## Context

`pack.workflow.schedule.v1` is the durable scheduling capability for Macaca workflow tasks and generic service commands. It computes when work should be triggered; it does not execute the work itself. Execution is delegated through `pack.workflow.task.v1` or another declared service command, preserving the canonical service path.

Production scheduling must be explicit about recurrence, time zones, DST gaps/folds, misfires, catch-up/backfill, overlap, jitter, idempotency, and trigger history. These semantics belong in the workflow schedule service, not in shells or application code.

## Supplier Capability Matrix

| Platform/API | Borrowed capability | Macaca mapping |
| --- | --- | --- |
| RFC 5545 | RRULE/RDATE/EXDATE, DTSTART, recurrence sets | calendar recurrence spec and exclusion/include sets |
| Temporal Schedules | independent schedule id, action, overlap, catch-up, pause, backfill | schedule identity, trigger action, overlap/misfire/catch-up policies |
| Quartz | cron/simple triggers, calendars, misfire instructions | cron/interval triggers, exclusions, misfire policy |
| Airflow Timetables | logical date, data interval, catch-up/backfill | trigger window, logical time, data interval, backfill policy |

## Goals

- Provide durable schedule create/update/delete, pause/resume, preview, next occurrences, due firing, backfill, history, and snapshot commands.
- Support one-shot, interval, cron-like, RFC 5545 recurrence, event-triggered external references, inclusions, exclusions, time zones, DST policy, jitter, and bounded catch-up.
- Generate idempotent trigger records that can create workflow tasks or service commands through declared capabilities.
- Emit sanitized replayable trace/audit evidence for every computed and fired occurrence.
- Provide detailed developer documentation and provider conformance guidance.

## Non-Goals

- Do not own task execution, approval, delegation, review, recovery, calendar invitations, notifications, or application-specific reminders.
- Do not branch on provider names, application names, workflow names, cron strings, or time-zone names in OS-layer code.
- Do not store raw prompts, application payloads, secrets, or unbounded history in generic observability.

## Ownership And Boundaries

- Pack id: `pack.workflow.schedule.v1`.
- Capability family: `workflow`.
- Backing service: workflow schedule service.
- SDK surface: `sdk.packs.workflow.schedule`.
- Command namespace: `workflow_schedule.*`.
- Application framework owns manifest declaration and app-scoped permission projection.
- Service runtime owns typed dispatch, decorators, schedule lifecycle, recurrence evaluation, trigger generation, health, snapshots, and unavailable behavior.
- Runtime host owns concrete provider adapters through approved composition roots.
- Shells render schedule state and diagnostics only from service events.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `workflow_schedule.create` | Create durable schedule | Requires idempotency key, spec, action reference, policy, and owner scope |
| `workflow_schedule.update` | Update mutable schedule fields | Validates version, state, recurrence compatibility, and policy |
| `workflow_schedule.pause` | Pause schedule | Stops future firing while preserving definition |
| `workflow_schedule.resume` | Resume schedule | Applies catch-up/misfire policy |
| `workflow_schedule.delete` | Delete schedule | Cancels future triggers and records terminal state |
| `workflow_schedule.inspect` | Inspect one schedule | Returns state, spec, next occurrence, policy, history pointer |
| `workflow_schedule.preview` | Preview occurrence set | Computes bounded future/past occurrences without firing |
| `workflow_schedule.next_occurrences` | Return next N due times | Enforces count/time range limits |
| `workflow_schedule.fire_due` | Generate due trigger records | Applies misfire, overlap, catch-up, jitter, and idempotency |
| `workflow_schedule.backfill` | Generate historical trigger records | Requires bounded range and policy allowance |
| `workflow_schedule.cancel_trigger` | Cancel pending trigger | Idempotently cancels generated trigger |
| `workflow_schedule.get_history` | Read schedule history | Returns bounded events and evidence ids |
| `workflow_schedule.snapshot` | Record service snapshot | Captures state summaries and replay pointers |

## DTO Model

- `WorkflowSchedule`: id, version, spec, state, owner scope, action reference, policy, next occurrence, trigger history pointer, and timestamps.
- `WorkflowScheduleSpec`: trigger type, start/end, timezone, recurrence, cron, interval, inclusions, exclusions, jitter, action, overlap policy, misfire policy, catch-up policy, and redaction policy.
- `ScheduleRecurrence`: RFC 5545-compatible RRULE subset/superset declaration, RDATE/EXDATE sets, DTSTART, count/until, and validation diagnostics.
- `ScheduleTimezonePolicy`: IANA zone id, tzdb version, DST gap/fold strategy, local-time resolution, stale-database behavior, and provenance.
- `ScheduleMisfirePolicy`: fire now, skip, reschedule next, coalesce, bounded catch-up, fail schedule, or provider-specific mapped unsupported.
- `ScheduleOverlapPolicy`: allow all, skip if active, buffer one, cancel previous, replace previous, or fail on overlap.
- `ScheduleTriggerRecord`: trigger id, schedule id, scheduled time, actual fire time, logical time, data interval, misfire class, jitter, idempotency key, action reference, and state.
- `ScheduleBackfillRequest`: range, max triggers, catch-up mode, concurrency/overlap behavior, and approval id.
- `WorkflowScheduleError`: denied, unavailable, unsupported, invalid recurrence, invalid timezone, DST unresolved, misfire blocked, overlap blocked, backfill too large, schedule paused, trigger conflict, quota exceeded, provider failure, or version mismatch.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `workflow.schedule.read`: inspect/list/history/preview.
- `workflow.schedule.write`: create/update/delete.
- `workflow.schedule.control`: pause/resume/cancel trigger.
- `workflow.schedule.fire`: fire due triggers.
- `workflow.schedule.backfill`: generate historical triggers.
- `workflow.schedule.admin`: snapshots and repair.

Policy requirements:

- Every schedule must declare action target capability and idempotency key derivation.
- Time zones must use provider-neutral timezone DTOs; DST gap/fold behavior must be explicit.
- Preview/backfill ranges and next-occurrence counts are bounded.
- Misfire/catch-up/overlap policies are explicit and cannot silently default to unsafe duplicate firing.
- Generated triggers create tasks or service commands through declared pack/service capabilities only.

## Service Runtime And Provider Strategy

Provider Strategy categories:

- Built-in schedule provider with durable recurrence evaluation.
- Remote workflow engine schedule adapter.
- Plugin recurrence/provider adapter.
- Mock provider for deterministic tests/docs.
- Unavailable provider for absent capability.

Providers declare recurrence support, cron syntax support, timezone/DST support, misfire policies, overlap policies, backfill limits, trigger idempotency, snapshot behavior, and health. Provider construction is allowed only in approved composition roots.

## SDK Discovery And Developer Documentation

SDK discovery returns pack metadata, command schemas, DTO schemas, permission scopes, schedule state machine, recurrence support matrix, timezone policy support, misfire/catch-up/overlap policies, examples, diagnostics, compatibility, and documentation links.

The implementation SHALL create `docs/developer-packs/workflow/schedule.md` with manifest declarations, scopes, command reference, one-shot/interval/cron/RRULE examples, timezone/DST behavior, preview, misfire/catch-up/backfill, overlap, task integration, history/replay, unavailable diagnostics, and provider conformance checklist.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `workflow_schedule.pack_declared`
- `workflow_schedule.admission_validated`
- `workflow_schedule.created`
- `workflow_schedule.updated`
- `workflow_schedule.paused`
- `workflow_schedule.resumed`
- `workflow_schedule.previewed`
- `workflow_schedule.trigger_computed`
- `workflow_schedule.trigger_fired`
- `workflow_schedule.trigger_cancelled`
- `workflow_schedule.backfill_started`
- `workflow_schedule.backfill_completed`
- `workflow_schedule.misfire_handled`
- `workflow_schedule.deleted`
- `workflow_schedule.snapshot_recorded`

Events include pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when present, schedule id, version, recurrence hash, timezone id/hash, scheduled time, logical time, trigger id hash, misfire class, overlap policy, catch-up class, policy decision, latency, and resource counters. Events exclude raw payloads, prompts, secrets, provider payloads, and unbounded history.

Snapshots include schedule state counts, next occurrence summaries, paused schedules, backfill summaries, misfire summaries, policy hash, provider health, unavailable diagnostics, and replay pointers.

## Design Patterns

- **Facade**: SDK exposes schedule clients while `SystemFacade` carries canonical service calls.
- **Command**: every operation is a typed command/result DTO.
- **State**: schedule, trigger, backfill, pause/resume, and deletion are explicit state machines.
- **Strategy**: recurrence evaluator, timezone policy, misfire policy, overlap policy, and provider replacement are descriptor-driven.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **Specification**: admission validates recurrence, timezone, action target, idempotency, ranges, and policy.
- **Observer**: schedule lifecycle, trigger, audit, health, and service events are subscribable.
- **Memento**: history and snapshots enable replay.
- **Abstract Factory**: providers are created only in approved composition roots.

## Risks And Mitigations

- Risk: DST or timezone changes produce duplicate/missed triggers. Mitigation: explicit timezone policy, tzdb evidence, DST strategy, and trigger idempotency.
- Risk: catch-up floods the system. Mitigation: bounded backfill/catch-up, quotas, approval, and overlap policy.
- Risk: schedule executes work directly. Mitigation: schedule only generates trigger records and delegates through declared task/service capabilities.
- Risk: recurrence grammar is ambiguous. Mitigation: support matrix, validation diagnostics, and provider conformance fixtures.
- Risk: shell owns scheduling semantics. Mitigation: all schedule transitions and firing are service commands.
