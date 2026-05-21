# Autonomy Runtime Activation V2 Design

Date: 2026-05-21

Status: brainstorm approved, OpenSpec proposal pending implementation approval

## Problem

Macaca now has provider-neutral Scheduler and Heartbeat service contracts,
focused SDK clients, runtime-host adapters, local providers, unavailable
providers, audit snapshots, and boundary tests. That V1 foundation makes the
capabilities visible, but it does not yet make them production-active for
applications because the default runtime-host path still registers unavailable
providers and no system-owned autonomy loop continuously leases and dispatches
scheduled runs.

The next step is to activate the local autonomy runtime without weakening the
microkernel boundary. The system must support true 24/7 autonomous operation,
but only through explicit configuration, service runtime dispatch, trace,
policy, audit, resource gates, and provider-neutral commands.

## Selected Approach

Use an explicitly enabled runtime-host local autonomy module.

By default, runtime-host SHALL keep registering fail-closed unavailable
Scheduler and Heartbeat providers. When generic autonomy configuration enables
the local module, runtime-host SHALL register `LocalSchedulerProvider`,
`LocalHeartbeatProvider`, and a lifecycle-managed `AutonomySupervisor`.

The supervisor is not an application workflow engine. It is a narrow host-side
daemon that wakes on a bounded tick interval, asks Scheduler for due work,
leases queued runs, dispatches only provider-neutral target commands through
approved service boundaries, records sanitized trace and audit evidence, and
transitions runs according to Scheduler results. A separate heartbeat lane uses
Heartbeat wake intents for scheduled, recovery, event, immediate, and manual
wakes, while Heartbeat remains responsible for coalescing and gate decisions.

## Alternatives Considered

### Option A: Explicit local autonomy module in runtime-host

This is the selected design. It preserves fail-closed defaults while allowing
operators or tests to opt into production-active local providers. Runtime-host
remains the Abstract Factory composition root, applications keep using
SystemFacade clients, and background loops are observable and stoppable.

Trade-off: implementation needs configuration, bootstrap wiring, supervisor
lifecycle, and tests for both disabled and enabled modes.

### Option B: Separate `service.autonomy_runtime`

This would make the runner itself a third replaceable service. It is elegant
for future distributed autonomy, but it adds a large new service before local
provider activation has proven the minimum viable operational loop.

Trade-off: more extensible later, but too much surface area for the current
activation step.

### Option C: Embed the loop inside Scheduler local provider

This is the fastest path, but it expands Scheduler from a scheduling service
into a daemon, dispatcher, and lifecycle owner. It would make Heartbeat
recovery and runtime-host shutdown behavior harder to reason about.

Trade-off: lower short-term code volume, higher long-term architecture debt.

## Architecture

```text
runtime-host startup
        |
        v
AutonomyRuntimeConfig
        |
        +-- disabled/default
        |       |
        |       v
        |   unavailable Scheduler + unavailable Heartbeat
        |
        +-- enabled local
                |
                v
        AutonomyProviderFactory
                |
                +-- LocalSchedulerProvider
                +-- LocalHeartbeatProvider
                +-- AutonomySupervisor
                        |
                        v
                ServiceRuntime dispatch
```

## Component Responsibilities

### AutonomyRuntimeConfig

Defines generic activation and safety controls:

- provider mode: unavailable or local
- supervisor enabled flag
- tick interval
- maximum leases per tick
- dispatch timeout
- shutdown grace
- heartbeat tick interval
- recovery wake enabled flag

The configuration MUST NOT contain application, workflow, model, driver,
gateway, chain, payment, or provider-business names.

### AutonomyProviderFactory

Runtime-host-owned Abstract Factory for autonomy providers. It constructs
either unavailable providers or local providers and returns a bootstrap bundle
with started service ids and an optional supervisor handle.

SDK, Web, CLI, applications, plugins, and the microkernel must not construct
these providers.

### AutonomySupervisor

Lifecycle-managed runtime-host daemon. It owns the timer loop and shutdown
coordination, but not cron parsing, heartbeat gate semantics, or application
business behavior.

On each scheduler tick it:

1. Requests bounded due-run materialization from Scheduler.
2. Acquires leases for queued runs.
3. Dispatches each run through a generic dispatch strategy.
4. Transitions the run to succeeded, failed, skipped, expired, or retry queued.
5. Emits sanitized logs, trace evidence, and audit ids for each key node.

On heartbeat ticks or recovery it:

1. Sends typed Heartbeat wake commands.
2. Lets Heartbeat coalesce, gate, accept, delay, or skip the wake.
3. Dispatches only accepted generic targets.
4. Records sanitized state transitions and run evidence.

### AutonomyDispatchStrategy

Strategy objects handle provider-neutral target categories:

- `ServiceCommand`
- `ApplicationCommand`
- `AgentExecutionCommand`
- `HeartbeatWakeCommand`
- `PluginCommand`

Strategies must route through existing service, application, task/execution, or
plugin boundaries. They must not inspect raw payloads for business semantics.

## Data Flow

```text
Application / shell / plugin
        |
        v
SystemFacade scheduler or heartbeat client
        |
        v
ServiceRuntime decorators
        |
        v
LocalSchedulerProvider / LocalHeartbeatProvider
        |
        v
AutonomySupervisor tick
        |
        v
lease run -> dispatch generic command -> transition run -> audit snapshot
```

## Error Handling

Every failure path returns structured unavailable, unsupported, denied,
invalid-request, conflict, timeout, provider-failure, or cancelled states. The
supervisor must never panic for optional-provider absence, malformed target
metadata, dispatch failure, shutdown, or policy denial.

Disabled mode is a valid operational state. It must return structured
unavailable results and must not start the background loop.

## Trace, Audit, and Logging

Every mutating command and every supervisor dispatch node must carry trace
context. Logs and snapshots must include safe ids and reason codes only.

Key log nodes:

- autonomy config resolved
- provider mode selected
- local provider registration started and completed
- supervisor started, ticked, idled, and stopped
- due-run materialization requested
- run lease acquired or denied
- dispatch strategy selected
- dispatch succeeded, failed, skipped, or timed out
- heartbeat wake accepted, coalesced, gated, dispatched, or skipped
- shutdown cancellation and grace completion

Logs, snapshots, and audit records must not include raw secrets, prompts,
manifests, WASM bytes, package bytes, credentials, private keys, raw provider
payloads, or unbounded output.

## Design Patterns

- Abstract Factory: runtime-host creates unavailable or local autonomy bundles.
- Facade: applications and shells use SystemFacade scheduler/heartbeat clients.
- Command: all cross-boundary operations use typed command/result DTOs.
- Strategy: dispatch target categories are replaceable dispatch strategies.
- State: Scheduler and Heartbeat retain explicit lifecycle state machines.
- Observer: supervisor emits trace, audit, and service events.
- Memento: snapshots and run summaries remain replayable diagnostics.
- Specification: boundary gates enforce allowed ownership and dependency rules.

## Testing Strategy

The proposal should add tests for:

- disabled mode registers unavailable providers and no supervisor loop
- enabled local mode registers local providers and starts the supervisor
- application-facing facade calls reach active local providers when enabled
- scheduler tick leases and dispatches generic service commands
- heartbeat scheduled and recovery wakes pass through Heartbeat gates
- shutdown cancels the loop cleanly and records safe evidence
- boundary gates reject provider construction or loops outside runtime-host
- logs and snapshots remain sanitized

## Approval Gate

This design is approved for OpenSpec proposal authoring only. Rust
implementation must wait until the OpenSpec proposal, design, tasks, and spec
deltas are reviewed and approved.
