# Change: Decouple Heartbeat From Scheduler Runtime

## Why

Heartbeat cadence is an agent/system autonomy primitive and should not require
application-facing Scheduler jobs. The current design allows Scheduler jobs to
target heartbeat wakes, which is useful for compatibility but makes heartbeat
look like a scheduled task and risks coupling agent heartbeat liveness to
Scheduler due-run materialization.

## What Changes

- Make Heartbeat own native heartbeat cadence, heartbeat profiles, heartbeat
  scopes, coalescing, gates, and heartbeat run mementos.
- Make runtime-host `AutonomySupervisor` coordinate sibling `SchedulerLane` and
  `HeartbeatLane` loops instead of treating heartbeat as a Scheduler target.
- Clarify that Scheduler owns scheduled jobs, due-run materialization, leases,
  and scheduled run history, not heartbeat cadence.
- Remove or hide heartbeat wake targets from application-facing schedule
  management UI.
- Add serviceization gates that reject scheduler-owned heartbeat cadence and
  shell-owned heartbeat timers.

## Impact

- Affected specs:
  - `heartbeat-service`
  - `autonomous-runtime`
  - `scheduler-service`
  - `web-cli-thin-shell-v0`
  - `serviceization-escape-hatches`
- Affected code areas:
  - `macaca-proto` heartbeat and scheduler contracts.
  - `macaca-heartbeat` local provider.
  - `macaca-scheduler` local provider and target DTO handling.
  - `macaca-runtime-host` autonomy supervisor.
  - `macaca-web` autonomy routes.
  - `frontend/components/autonomy/*` schedule-management UI.
  - integration boundary tests and autonomy docs.

## Constitutional Fit

- The microkernel does not construct heartbeat or scheduler providers.
- Runtime-host remains the approved composition root for local autonomy loops.
- Scheduler and Heartbeat remain provider-neutral system services.
- Web, CLI, frontend, and applications do not own timers, coalescing, gates, or
  heartbeat lifecycle semantics.
- No implementation may branch on application, workflow, provider, model,
  driver, gateway, chain, payment, or business-domain names.
