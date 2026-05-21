# Design: Autonomy Scheduler and Heartbeat Services V1

## Context

Macaca needs true unattended operation: scheduled tasks, recurring autonomous
checks, recovery wakeups, and heartbeat loops must keep working after restarts
and without a human manually pressing "continue". The research comparison with
Hermes Agent and OpenClaw confirms that cron-style scheduling and heartbeat
wake mechanics are foundational primitives for autonomous agents.

The key architectural constraint is that Macaca is an Agent OS, not a single
application. Scheduler and heartbeat behavior must therefore be generic system
services with clear service runtime boundaries.

## Goals

- Provide a durable Scheduler Service for generic scheduled work.
- Provide a Heartbeat Service for generic wake-loop coordination.
- Keep all concrete timing loops and wake decisions outside the microkernel.
- Give upper applications basic scheduled and heartbeat capabilities through
  declared service/facade boundaries.
- Preserve replacement mechanics for built-in, plugin, remote, mock, and
  unavailable providers.
- Make every key execution step traceable, auditable, resumable, and safe to
  inspect.

## Non-Goals

- Do not implement application-specific reminder, notification, trading,
  scraping, chat, market, workflow, driver, chain, or provider logic.
- Do not make Web, CLI, frontend, or applications responsible for OS cron
  semantics.
- Do not make the microkernel construct concrete scheduler or heartbeat
  providers.
- Do not define one cron syntax library, database backend, queue backend, or
  file format as the only valid implementation.

## Architecture Overview

```text
Applications / Shells
        |
        v
SystemFacade focused clients
        |
        v
Service Runtime decorators
trace -> policy -> resource -> audit -> service call
        |
        v
Runtime-host provider registry
        |
        +-- service.scheduler provider
        |
        +-- service.heartbeat provider
```

## Ownership Boundaries

### Microkernel

The microkernel may own identity, policy handles, trace handles, audit handles,
service registry primitives, resource primitives, and scheduling primitives. It
must not own concrete timer loops, cron parsing, wake queues, job stores, retry
engines, or heartbeat execution policy.

### Service Runtime

The service runtime owns service descriptors, lifecycle, health, snapshots,
typed command dispatch, structured errors, and decorators for trace, policy,
resource, entitlement, and audit. It must treat scheduler and heartbeat as
replaceable service families.

### Runtime Host

Runtime-host is the Abstract Factory / composition root for service providers.
It may register built-in unavailable providers by default and later register
local, plugin, remote, or mock providers through configuration. It must not
route by application names, provider names, workflow names, or business domains.

### SDK / SystemFacade

The SDK exposes focused Scheduler and Heartbeat clients. These clients are
Facade objects over typed service calls. They may normalize request DTOs and
return structured results, but they must not construct providers, runtimes,
databases, queues, timers, or application-specific workflows.

### Shells and Applications

Shells and applications may create, update, pause, resume, trigger, and observe
scheduled work only through facade clients and declared capabilities. They may
render audit evidence or health snapshots, but must not become semantic owners
of scheduling or heartbeat state.

## Scheduler Service Design

The Scheduler Service owns durable scheduled job definitions and run state.

### Provider-Neutral Job Definition

A scheduler job definition contains generic metadata only:

- Stable job identity, tenant/application/session/task scope, and ownership
  references.
- Schedule spec: `At`, `Every`, or `CronExpression`.
- Time zone and clock policy.
- Optional deterministic stagger policy for fleet-safe distribution.
- Missed-run policy: `Skip`, `FireOnce`, or bounded catch-up.
- Concurrency policy and lease timeout.
- Retry/backoff policy with bounded attempts.
- Payload reference or typed command, never raw secrets or unbounded payloads.
- Trace and audit correlation metadata.

### Provider-Neutral Command Payload

The scheduler may enqueue only generic command categories:

- `ServiceCommand`: call a declared service capability by service identifier and
  command name.
- `AgentExecutionCommand`: request agent/task execution through the existing
  task or execution service boundary.
- `HeartbeatWakeCommand`: request a heartbeat wake through the heartbeat service.
- `ApplicationCommand`: call an application-declared capability without knowing
  application-specific business semantics.
- `PluginCommand`: call a plugin-declared capability through plugin boundaries.

The scheduler must not inspect command payloads to branch on application,
workflow, model, driver, chain, gateway, or provider names.

### Job Lifecycle

Jobs use an explicit State pattern:

- `Draft`: definition exists but is not eligible to run.
- `Active`: definition is eligible for due-work calculation.
- `Paused`: definition is intentionally suspended.
- `Disabled`: definition is blocked by policy or provider state.
- `Deleted`: definition is tombstoned for audit/history.

### Run Lifecycle

Runs use a separate state machine:

- `Queued`: the run was materialized and awaits a lease.
- `Leased`: a provider instance acquired the run.
- `Running`: execution was dispatched to the target service boundary.
- `Succeeded`: the target command completed successfully.
- `Failed`: the run exhausted execution or retry policy.
- `Cancelled`: the run was explicitly cancelled.
- `Skipped`: policy, missed-run handling, or concurrency rules skipped the run.
- `Expired`: the lease expired and the run needs recovery or failure handling.

### Leases and Concurrency

The scheduler uses leases instead of shell polling. A provider must acquire a
run lease before dispatching work. Lease expiry must be recoverable, and
concurrency policy must prevent duplicate active runs beyond the job's declared
limits. The contract does not promise exactly-once execution; it promises
auditable at-least-once dispatch with explicit idempotency and lease evidence.

### Persistence and Snapshots

The scheduler separates durable definitions from runtime mementos:

- Job definition store.
- Run history store.
- Lease state.
- Provider snapshot.
- Health snapshot.

This keeps storage replaceable and allows future local, remote, plugin, or mock
providers without changing service contracts.

## Heartbeat Service Design

The Heartbeat Service coordinates wake requests and heartbeat run evidence. It
is not a hidden task runner; it decides when a wake should be accepted,
coalesced, delayed, skipped, or dispatched through declared service commands.

### Wake Intents

The heartbeat service accepts provider-neutral wake intents:

- `ScheduledTick`: a scheduled tick generated by scheduler integration.
- `EventSignal`: a generic event requested a wake.
- `Immediate`: a caller requested immediate wake processing.
- `Manual`: a human or shell explicitly requested a wake.
- `Recovery`: restart or lease recovery requested a wake.

### Coalescing and Gates

Wake requests are coalesced by scope so repeated signals do not create runaway
loops. Gates are evaluated before side effects:

- Active-hours policy.
- Cooldown policy.
- Busy/concurrency policy.
- Resource or budget policy.
- Cron-active / scheduled-run activity gates.
- Provider health gates.

Rejected, delayed, or coalesced wakes must return structured results and audit
records rather than silently disappearing.

### Heartbeat Run Lifecycle

Heartbeat runs use explicit states:

- `Requested`: a wake intent was accepted.
- `Coalesced`: the wake merged into an existing pending run.
- `Gated`: policy delayed or denied the wake.
- `Running`: the provider dispatched generic work.
- `Succeeded`: the heartbeat cycle completed.
- `Failed`: the cycle failed with structured error evidence.
- `Skipped`: the wake was intentionally skipped.

### Scheduler Integration

Recurring heartbeat schedules are represented as scheduler jobs that enqueue
`HeartbeatWakeCommand`. This keeps cron semantics in `service.scheduler` and
wake semantics in `service.heartbeat`. The heartbeat service may expose
snapshot fields that explain whether scheduler-driven ticks are active, but it
must not implement a separate shell-owned cron loop.

## Trace, Audit, and Logs

Every scheduler and heartbeat command must carry or derive trace context. Key
execution nodes must be auditable:

- Provider selected.
- Command accepted or rejected.
- Policy gates evaluated.
- Job or wake state transition.
- Lease acquired, renewed, expired, or released.
- Target command dispatched.
- Retry/backoff scheduled.
- Run completed, failed, skipped, or cancelled.
- Snapshot or health state changed.

Audit records must use sanitized payload references and bounded summaries. They
must not expose raw secrets, prompts, manifests, WASM bytes, package bytes,
private keys, credentials, raw signatures, provider payloads, or unbounded
application output.

## Error Model

Both services return structured errors:

- `Unavailable`: no provider is installed or enabled.
- `Unsupported`: the provider does not support a requested capability.
- `Denied`: policy, entitlement, budget, or scope rejected the command.
- `InvalidRequest`: the command failed validation.
- `Conflict`: lifecycle or concurrency state prevented the command.
- `ProviderFailure`: the provider failed after accepting the command.
- `Timeout`: lease, dispatch, or provider call exceeded configured bounds.

Errors are part of the service contract and must be represented in snapshots
and audit records where appropriate.

## Alternatives Considered

### Put Scheduling in the Microkernel

Rejected. The microkernel may own primitives, but concrete cron parsing, stores,
timer loops, retries, and dispatch policies are replaceable service behavior.

### Put Cron in Web or CLI Shells

Rejected. Shell-owned cron would make presentation adapters semantic owners of
OS behavior and would break unattended autonomous operation after shell exit.

### Build One Large Autonomy Service

Rejected. Scheduler and heartbeat have distinct state machines, policies, and
replacement needs. Splitting them keeps contracts smaller and lets future task,
planner, review, recovery, and notification services evolve independently.

### Require a Specific Cron Library or Storage Backend

Rejected. The contract defines behavior, not implementation. Providers may use
different cron parsers, clocks, queues, databases, or remote scheduler systems as
long as the service contract remains stable.

## Future Implementation Notes

Future Rust work should be split into small slices:

- DTOs in `macaca-proto`.
- Service traits and Null Object providers in service crates.
- Runtime-host provider registration.
- Local provider implementation behind the service runtime.
- SDK focused clients.
- Boundary gates and integration tests.

All future Rust code must include detailed English comments for non-obvious
runtime behavior and log key execution nodes without leaking sensitive payloads.
