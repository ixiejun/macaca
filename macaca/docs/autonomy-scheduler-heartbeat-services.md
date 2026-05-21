# Autonomy Scheduler and Heartbeat Services

## Purpose

`service.scheduler` and `service.heartbeat` provide the generic autonomous
runtime loop capabilities required by a 24/7 Agent OS. They are sibling system
services, not kernel implementations and not application workflows. Scheduler
owns durable time-based job intent and run mementos. Heartbeat owns native
agent/system cadence, wake-loop coalescing, gate evaluation, heartbeat profiles,
and wake-run evidence.

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
managed by runtime-host only. It owns two sibling lanes:

- `SchedulerLane` coordinates Scheduler due-run materialization, lease
  acquisition, generic scheduled target dispatch, retry outcome mapping, and
  Scheduler run mementos.
- `HeartbeatLane` coordinates Heartbeat native cadence ticks, recovery wake
  requests, heartbeat profile evaluation, and heartbeat run mementos.

The lanes share runtime-host lifecycle and sanitized observability, but they do
not own each other's semantics. Scheduler does not own heartbeat cadence, and
Heartbeat does not own scheduled job calculation. Runtime-host does not parse
cron expressions, evaluate heartbeat gates, inspect application payloads, or
branch on application, workflow, provider, driver, model, gateway, chain,
payment, or business-domain names.

The current local dispatch strategy supports generic `ServiceCommand` targets
through `ServiceRuntime`. Scheduler `HeartbeatWakeCommand` targets are retained
only as internal/runtime compatibility for migration; application-facing
schedule management must not present Heartbeat native cadence as a Scheduler
target. Agent execution, application capability, and plugin target categories
remain provider-neutral strategy slots; until their downstream service dispatch
adapters are activated, the supervisor records explicit skipped/unsupported
outcomes rather than panicking or faking success.

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
- `GET /api/apps/{app_id}/autonomy/schedules` lists application-scoped
  serviceized Scheduler job summaries.
- `GET /api/apps/{app_id}/autonomy/schedules/{job_id}` returns one sanitized
  job summary.
- `PATCH /api/apps/{app_id}/autonomy/schedules/{job_id}` updates a job by
  submitting a provider-neutral replacement definition through the Scheduler
  service client.
- `PUT /api/apps/{app_id}/autonomy/schedules/{job_id}/lifecycle` pauses or
  resumes a job through Scheduler-owned lifecycle semantics.
- `DELETE /api/apps/{app_id}/autonomy/schedules/{job_id}` deletes a job through
  the Scheduler service while preserving bounded run/audit evidence.
- `GET /api/apps/{app_id}/autonomy/scheduler/runs` returns bounded,
  sanitized run history for monitoring.

If no target is supplied, the route rejects the request with a structured
adapter error. Heartbeat native cadence is managed by `service.heartbeat`
profiles through HeartbeatLane, not by default Web-created Scheduler jobs.

## Web Schedule Management Surface

The frontend exposes an application-scoped `AUTONOMY` workspace tab for
operators. The tab is a presentation shell only: it loads sanitized Scheduler
job summaries, submits provider-neutral create/update/delete/lifecycle
commands, renders bounded run mementos, and displays safe trace/audit
identifiers. It does not construct providers, parse cron semantics, inspect
application business payloads, or call the legacy `/api/apps/{app_id}/schedules`
compatibility routes.

The schedule editor intentionally exposes Scheduler-owned generic fields only:

- schedule name metadata;
- interval seconds;
- service id and command name for service targets;
- safe metadata key/value pairs.

This keeps the UI useful for every application while preserving Macaca OS as
generic infrastructure rather than application-specific automation code.

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
- `HeartbeatProfile` is the native cadence/profile contract.
- `HeartbeatGateDecision` records bounded gate evidence.
- `HeartbeatRunSummary` is the wake-run memento.
- `HeartbeatServiceSnapshot` is the sanitized diagnostic view.

A plugin or remote heartbeat provider must:

- Coalesce by scope without inspecting application-specific workflow names.
- Gate wakes through generic active-hours, cooldown, busy, resource, budget,
  provider-health, policy, and extension gates.
- Treat scheduled ticks from `service.scheduler` as typed wake intent, not as
  a special application workflow or the owner of native heartbeat cadence.
- Evaluate native heartbeat cadence from Heartbeat profiles without requiring
  Scheduler jobs or Scheduler due-run materialization.
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
