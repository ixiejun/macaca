# Autonomy Scheduler and Heartbeat Services

## Purpose

`service.scheduler` and `service.heartbeat` provide the generic autonomous
runtime loop capabilities required by a 24/7 Agent OS. They are system services,
not kernel implementations and not application workflows. Scheduler owns durable
time-based job intent and run mementos. Heartbeat owns wake-loop coalescing,
gate evaluation, and wake-run evidence.

These services let upper applications request recurring or event-driven
autonomy without adding application-specific code to Macaca OS.

## Local Autonomy Runtime Activation

Macaca keeps autonomy execution disabled by default. In the default mode,
runtime-host registers fail-closed unavailable providers for
`service.scheduler` and `service.heartbeat`; no background supervisor loop is
started, and application-facing clients receive structured unavailable results.
This preserves optional-module semantics and prevents hidden global side
effects during ordinary host startup.

When runtime-host receives explicit provider-neutral local autonomy
configuration, it may register the built-in local Scheduler and Heartbeat
providers and create an `AutonomySupervisor`. This supervisor is lifecycle
managed by runtime-host only. It owns bounded timer-loop coordination, lease
acquisition, dispatch timeout handling, recovery wake requests, scheduled
heartbeat ticks, shutdown cancellation, and safe logs. It does not parse cron
expressions, evaluate heartbeat gates, inspect application payloads, or branch
on application, workflow, provider, driver, model, gateway, chain, payment, or
business-domain names.

The current local dispatch strategy supports generic `ServiceCommand` targets
through `ServiceRuntime` and `HeartbeatWakeCommand` targets through
`service.heartbeat`. Agent execution, application capability, and plugin target
categories remain provider-neutral strategy slots; until their downstream
service dispatch adapters are activated, the supervisor records explicit
skipped/unsupported outcomes rather than panicking or faking success.

The local Web host now enables this path explicitly through
`macaca/config/default.toml`:

```toml
[autonomy]
provider_mode = "local"
supervisor_enabled = true
```

Applications can request recurring work through the existing application route
namespace without constructing providers:

- `POST /api/apps/{app_id}/autonomy/schedules` registers a serviceized
  Scheduler job through the SDK Scheduler client.
- `GET /api/apps/{app_id}/autonomy/scheduler/runs` returns bounded,
  sanitized run history for monitoring.

If no target is supplied, the route creates a generic Heartbeat wake target.
That default proves the 24/7 runtime path without embedding application
business logic in Web or runtime-host.

## Ownership Diagram

```text
Application / WASM / Plugin
        |
        | declared capability + typed command
        v
SDK SystemFacade / focused clients
        |
        | ServiceCallCommand + TraceContext
        v
ServiceRuntime decorators
        |
        | trace / policy / resource / entitlement / metering / audit
        v
Runtime-host service provider adapter
        |
        | typed SchedulerService / HeartbeatService trait
        v
Built-in, plugin, remote, mock, or unavailable provider
```

The microkernel may know service identities, trace evidence, policy decisions,
resource reservations, and service lifecycle state. It must not parse cron
expressions, construct provider engines, run heartbeat gates, dispatch
application business logic, or branch on provider/application names.

## Scheduler Extension Points

- `SchedulerService` is the replaceable provider trait.
- `SchedulerScheduleSpec` is the provider-neutral schedule contract.
- `SchedulerTargetCommand` stores generic dispatch intent only.
- `SchedulerRunSummary` is the bounded run-history memento.
- `SchedulerServiceSnapshot` is the sanitized diagnostic view.

A plugin or remote scheduler provider must:

- Register through runtime-host or an approved service-provider factory.
- Accept typed scheduler commands through `ServiceRuntime`.
- Require `TraceContext` on every call.
- Emit sanitized logs for due calculation, lease acquisition, dispatch boundary,
  retry, skip, failure, completion, and snapshot reads.
- Return structured unavailable, unsupported, denied, conflict, timeout, or
  provider-failure states instead of panics or silent fallback.
- Store only safe payload references in snapshots and audit records.

## Heartbeat Extension Points

- `HeartbeatService` is the replaceable provider trait.
- `HeartbeatWakeCommand` is the generic wake entrypoint.
- `HeartbeatGateDecision` records bounded gate evidence.
- `HeartbeatRunSummary` is the wake-run memento.
- `HeartbeatServiceSnapshot` is the sanitized diagnostic view.

A plugin or remote heartbeat provider must:

- Coalesce by scope without inspecting application-specific workflow names.
- Gate wakes through generic active-hours, cooldown, busy, resource, budget,
  provider-health, policy, and extension gates.
- Treat scheduled ticks from `service.scheduler` as typed wake intent, not as
  a special application workflow.
- Log wake accepted, coalesced, gated, delayed, dispatched, failed, skipped, and
  completed events with trace ids and safe reason codes only.
- Keep raw prompts, manifests, package bytes, provider payloads, credentials,
  and secrets out of snapshots and logs.

## Application Usage Rule

Applications must not import scheduler or heartbeat providers. They may request
autonomous behavior only through declared capabilities and service calls:

```text
application capability declaration
    -> SDK focused client or WASM host import
    -> service.scheduler / service.heartbeat command
    -> provider-neutral result or unavailable state
```

Examples and demos must stay outside OS-layer code. If an example needs a
recurring job, it should declare the capability and call the Scheduler client
with provider-neutral command data. If it needs a wake loop, it should call the
Heartbeat client with a generic wake scope and reason code. Macaca OS must not
gain branches for that example's product domain.

Applications never enable the local autonomy runtime directly. Operators,
runtime-host configuration, or future policy-controlled module activation select
whether local autonomy is unavailable or active. Applications continue to use
declared capabilities and SystemFacade clients in both modes, so provider
activation remains replaceable and auditable.

## Audit and Trace Requirements

Every mutation must carry trace context and create replayable evidence. Local
providers currently retain bounded sanitized audit identifiers in snapshots.
Durable providers may replace that in-memory memento with a persistent audit
repository, but the public DTO shape must remain provider-neutral.

Audit records must never include raw command payloads, prompt bodies, manifests,
WASM bytes, package bytes, private keys, credentials, raw signatures, or
unbounded provider output.

## Boundary Gates

The following executable gates protect the boundary:

- `serviceization_escape_hatches` rejects scheduler or heartbeat provider
  construction outside approved service/provider/facade surfaces.
- `route_c_dependency_boundaries` classifies scheduler and heartbeat crates as
  service providers and rejects presentation-shell or kernel ownership leaks.
- `autonomy_scheduler_heartbeat_services` proves runtime-host registration,
  typed service calls, trace propagation, cron timezone handling, and sanitized
  audit snapshot evidence.
