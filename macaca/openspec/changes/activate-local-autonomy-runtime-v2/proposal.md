# Change: Activate Local Autonomy Runtime V2

## Why

Scheduler and Heartbeat services are now defined and implemented as
provider-neutral service capabilities, but the default runtime-host path still
registers unavailable providers and no lifecycle-managed autonomous runner
continuously leases and dispatches scheduled work. Macaca needs an explicitly
enabled production-active local autonomy module so applications can use
scheduler and heartbeat capabilities through service boundaries without moving
cron, wake, or execution-loop semantics into the kernel, shells, SDK, or
application-specific code.

## What Changes

- Add a runtime-host-owned local autonomy activation path guarded by generic
  configuration.
- Keep fail-closed unavailable Scheduler and Heartbeat providers as the default
  mode.
- Register `LocalSchedulerProvider` and `LocalHeartbeatProvider` only when the
  local autonomy module is explicitly enabled.
- Add a lifecycle-managed `AutonomySupervisor` that owns bounded background
  ticks, lease acquisition, generic dispatch, heartbeat wake integration,
  recovery wakes, shutdown, logs, and sanitized audit evidence.
- Dispatch scheduled work only through provider-neutral target commands and
  existing service/application/task/plugin boundaries.
- Strengthen SDK, service-runtime, and serviceization gates so applications see
  production-active capabilities when enabled while forbidden ownership remains
  blocked.

## Impact

- Affected specs: `autonomous-runtime`, `service-runtime`,
  `sdk-system-facade`, `serviceization-escape-hatches`.
- Affected code areas: runtime-host autonomy bootstrap, service runtime
  provider registration, scheduler/heartbeat local-provider integration,
  SDK/SystemFacade tests, integration tests, dependency-boundary tests,
  architecture docs.
- Compatibility: default behavior remains fail-closed unavailable until local
  autonomy is explicitly enabled.
- Security and governance: all implementation must comply with
  `macaca-os-architecture-governance.md`,
  `macaca-os-microkernel-boundaries.md`, and
  `macaca-os-serviceization-allowlist.md`.
