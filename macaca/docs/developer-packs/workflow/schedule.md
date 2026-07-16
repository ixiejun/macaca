# Workflow Schedule Pack

`pack.workflow.schedule.v1` describes provider-neutral schedule and trigger
capabilities for autonomous applications. The pack is descriptor-only until a
schedule provider is registered through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when scheduling is mandatory for readiness.
Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.workflow.schedule.v1"]
```

## Permissions

Use the narrowest scope: `workflow.schedule.read`,
`workflow.schedule.write`, `workflow.schedule.control`,
`workflow.schedule.fire`, `workflow.schedule.backfill`, and
`workflow.schedule.admin`.

## Capability Model

Macaca models schedules as specs, recurrence references, timezone policies,
misfire policies, overlap policies, action references, trigger records,
bounded backfill requests, snapshots, and trigger history. Raw action payloads,
private metadata, prompts, provider payloads, credentials, and unbounded trigger
histories stay behind provider adapters and must not appear in traces,
snapshots, or SDK diagnostics.

## Platform Comparison

Unix cron, systemd timers, Temporal schedules, Quartz, AWS EventBridge,
Windows Task Scheduler, Android WorkManager periodic work, macOS LaunchAgent,
and OpenHarmony background task scheduling map to recurrence, timezone, misfire,
overlap, trigger, and backfill DTOs. Provider-native expressions and action
payloads remain implementation details.

## Commands

`workflow_schedule.create`, `workflow_schedule.update`,
`workflow_schedule.pause`, `workflow_schedule.resume`,
`workflow_schedule.delete`, `workflow_schedule.inspect`,
`workflow_schedule.preview`, `workflow_schedule.next_occurrences`,
`workflow_schedule.fire_due`, `workflow_schedule.backfill`,
`workflow_schedule.cancel_trigger`, `workflow_schedule.get_history`,
`workflow_schedule.snapshot`, and `workflow_schedule.inspect_provider` are
descriptor-owned schema names. SDK helpers build canonical traced service calls;
providers execute behind the service runtime.

## App-Facing Examples

- Create a schedule with recurrence, timezone, misfire, overlap, action, and
  jitter references.
- Express one-shot, interval, cron, and RRULE schedules through provider-neutral
  recurrence descriptors instead of provider-native expression strings.
- Preview next occurrences before activation and reject invalid recurrence or
  unresolved DST cases before side effects.
- Pause, resume, or delete schedules through explicit control commands.
- Fire due triggers with idempotency keys and inspect bounded trigger history.
- Run backfills only with explicit start, end, max-trigger, and approval
  references.
- Route task integration through typed action references and preserve misfire,
  overlap-blocked, backfill-too-large, paused, trigger-conflict, quota, and
  unavailable-provider diagnostics as structured results.

## Trace And Audit

Traces should record declaration, admission decision, command name, schedule
ref, recurrence ref, timezone ref, trigger ref, backfill ref, provider class,
capability hash, result status, and trigger count. They must not record raw
action payloads, prompts, private metadata, credentials, provider payloads, or
unbounded trigger histories.

## Provider Authors

Descriptors must report recurrence support, timezone database metadata, DST
strategy support, misfire limits, overlap limits, backfill limits, trigger
history bounds, health, and snapshot metadata. Providers must return structured
denied, unavailable, unsupported, invalid-recurrence, invalid-timezone,
dst-unresolved, misfire-blocked, overlap-blocked, backfill-too-large,
schedule-paused, trigger-conflict, quota, provider-failure, and
version-mismatch results.

Conformance tests should cover descriptor completeness, recurrence validation,
timezone and DST behavior, misfire handling, overlap gates, bounded backfill,
idempotent trigger firing, pause/resume/delete state, policy hooks, trace and
audit events, unavailable behavior, snapshot/replay, and restart recovery.
