# Design: Activate Local Autonomy Runtime V2

## Context

Macaca V1 autonomy work introduced Scheduler and Heartbeat service contracts,
local providers, unavailable providers, runtime-host adapters, SDK clients,
sanitized audit snapshots, and boundary gates. That work makes the capability
surface visible, but not production-active in the default runtime-host path.

This change activates local autonomy through explicit configuration. The
microkernel still owns only system invariants. Runtime-host remains the
composition root. Applications and shells continue to use SDK/SystemFacade
clients. Concrete cron, wake, lease, and dispatch loops remain outside the
kernel and outside application-specific code.

## Goals

- Keep unavailable autonomy providers as the default fail-closed mode.
- Add an explicit local provider activation mode owned by runtime-host.
- Start and stop a bounded `AutonomySupervisor` with service lifecycle.
- Let Scheduler own due-run state, leases, retries, and run transitions.
- Let Heartbeat own wake coalescing, gate decisions, and wake-run evidence.
- Let the supervisor own only timer-loop coordination and generic dispatch.
- Preserve provider replacement for local, plugin, remote, mock, and
  unavailable providers.
- Make every key execution node traceable, auditable, logged, and sanitized.

## Non-Goals

- Do not add application-specific jobs, workflows, prompts, reminders,
  notifications, trading logic, scraping logic, provider routing, or business
  rules.
- Do not make the microkernel construct local providers or own timer loops.
- Do not make Web, CLI, frontend, SDK, or applications responsible for
  scheduler or heartbeat semantics.
- Do not make a specific cron library, database, queue, gateway, task provider,
  or application runtime mandatory for the base OS.
- Do not enable production-active background loops without explicit generic
  configuration.

## Decisions

### Decision: Runtime-host owns local autonomy activation

Runtime-host SHALL expose a generic autonomy bootstrap path with two initial
modes: unavailable and local. Unavailable remains the default. Local mode
constructs local Scheduler and Heartbeat providers plus a supervisor handle.

This follows the Abstract Factory pattern and preserves the approved
composition root. It also keeps SDK, shells, applications, and the kernel from
constructing providers.

### Decision: Supervisor owns loops, not business semantics

`AutonomySupervisor` SHALL own bounded scheduler and heartbeat tick loops,
shutdown cancellation, dispatch timeouts, and safe logs. It SHALL NOT parse
cron expressions, evaluate heartbeat gates, inspect application payloads, or
branch on application, workflow, model, driver, gateway, chain, payment,
provider, or business-domain names.

### Decision: Dispatch uses strategies over provider-neutral commands

Supervisor dispatch SHALL use a Strategy interface keyed by provider-neutral
target category. Initial categories may include service commands, heartbeat
wake commands, application commands, agent/task execution commands, and plugin
commands. Each strategy routes through the existing boundary for that category.

### Decision: Disabled mode is a first-class state

When local autonomy is disabled, runtime-host SHALL register unavailable
providers and SHALL NOT start the supervisor. Application-facing clients return
structured unavailable results. This protects optional-module semantics and
avoids hidden global side effects.

### Decision: Recovery uses heartbeat wake intent

Runtime recovery SHOULD issue provider-neutral Heartbeat `Recovery` wake
intents when local autonomy is enabled and recovery wakes are configured.
Heartbeat decides whether those wakes are accepted, coalesced, gated, delayed,
or skipped.

## Runtime Flow

```text
runtime-host startup
  -> resolve AutonomyRuntimeConfig
  -> unavailable mode:
       register unavailable scheduler and heartbeat providers
       do not start supervisor
  -> local mode:
       construct LocalSchedulerProvider and LocalHeartbeatProvider
       register and start both through ServiceRuntime
       construct AutonomySupervisor
       start supervisor loop with shutdown handle
```

## Scheduler Tick Flow

```text
supervisor tick
  -> request scheduler snapshot or due-run materialization
  -> acquire bounded run leases
  -> select dispatch strategy by target command category
  -> dispatch through ServiceRuntime or approved facade boundary
  -> transition run succeeded / failed / skipped / retry / expired
  -> emit sanitized trace, audit, and log evidence
```

## Heartbeat Flow

```text
heartbeat tick or recovery signal
  -> heartbeat.wake(ScheduledTick or Recovery)
  -> heartbeat evaluates coalescing and gates
  -> accepted wake creates run evidence
  -> optional generic target dispatch uses supervisor strategy
  -> result updates heartbeat run lifecycle
```

## Configuration Model

Configuration SHALL be generic and provider-neutral:

- provider mode: unavailable or local
- supervisor enabled
- scheduler tick interval
- heartbeat tick interval
- maximum leases per tick
- dispatch timeout
- shutdown grace
- recovery wake enabled
- safe snapshot/audit retention bounds

Configuration SHALL NOT include application, workflow, model, driver, gateway,
chain, payment, provider-business, or domain-specific names.

## Trace, Audit, and Logging

Every mutating service command, supervisor tick, lease attempt, dispatch
attempt, heartbeat wake, state transition, retry decision, skip decision,
failure, and shutdown event SHALL carry trace context or derive a bounded
system trace context. Logs SHALL include safe ids, service ids, command names,
state names, reason codes, and audit ids only.

Logs, traces, snapshots, and audit records SHALL NOT contain raw secrets, raw
prompts, raw manifests, package bytes, WASM bytes, credentials, private keys,
raw signatures, provider payloads, or unbounded application output.

## Code Quality Constraints

All new Rust code in this change MUST include detailed English comments for
non-obvious functionality, runtime principles, lifecycle behavior, and safety
invariants. Key execution nodes MUST include structured logging. Files must
remain small and cohesive; if implementation grows too large, split by
configuration, factory, supervisor, dispatch strategy, and lifecycle concerns.

## Risks and Mitigations

- Risk: hidden background side effects.
  Mitigation: disabled mode remains default and tests assert no supervisor
  starts unless explicitly enabled.
- Risk: supervisor becomes a god object.
  Mitigation: keep provider state in Scheduler/Heartbeat and split dispatch by
  Strategy.
- Risk: application-specific behavior leaks into OS code.
  Mitigation: require provider-neutral target commands and strengthen boundary
  gates for hardcoded names and shell-owned loops.
- Risk: duplicate or runaway autonomous execution.
  Mitigation: bounded tick intervals, bounded leases per tick, lease timeouts,
  heartbeat coalescing, busy gates, and structured shutdown.
- Risk: observability leaks sensitive data.
  Mitigation: sanitized DTOs, bounded snapshots, reason-code logs, and tests.

## Migration Plan

1. Add the OpenSpec and documentation for local autonomy activation.
2. Add runtime-host configuration and factory types without changing defaults.
3. Add local provider bootstrap path behind explicit configuration.
4. Add supervisor lifecycle with no-op/disabled tests first.
5. Add bounded scheduler tick and dispatch strategy wiring.
6. Add heartbeat scheduled/recovery wake lane.
7. Add integration tests and boundary gates.
8. Update docs and validate OpenSpec/code gates.

## Open Questions

- None for this proposal. Distributed, plugin-backed, and remote autonomy
  runtimes remain future provider implementations behind the same boundary.
