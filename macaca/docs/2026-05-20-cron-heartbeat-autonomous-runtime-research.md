# Cron and Heartbeat Research for Macaca Autonomous Runtime

> Date: 2026-05-20
>
> Scope: Research `/Users/quantum/Code/dev/agent/openclaw` and
> `/Users/quantum/Code/dev/agent/hermes-agent` cron/heartbeat mechanisms, then
> brainstorm how Macaca should introduce scheduler and heartbeat capabilities
> without violating the microkernel, serviceization, or allowlist constitutions.

## Executive Summary

Macaca needs scheduler and heartbeat capabilities to become a true 24/7
autonomous Agent OS. The important architectural point is that cron and
heartbeat are not kernel business logic. They are replaceable autonomy
capabilities that should enter through system service contracts, service
runtime lifecycle, policy, trace, audit, health, snapshots, and SDK facades.

Hermes demonstrates a pragmatic cron implementation with strong operational
details: file locks, secure JSON storage, schedule parsing, at-most-once
recurring runs, workdir isolation, output archives, delivery targets, silent
responses, timeout hardening, and a background curator piggybacking on the cron
ticker. Its main weakness for Macaca is ownership: the gateway process owns the
ticker and execution path, which would violate Macaca's shell-thinness goal if
copied directly.

OpenClaw provides a better architectural reference. Its cron capability is
represented by a `CronServiceContract` with `start`, `stop`, `status`, `list`,
`add`, `update`, `remove`, `run`, `enqueueRun`, and `wake`. It separates typed
job config from runtime state, uses timer-based next wake scheduling, tracks
running jobs, emits cron events, bridges cron to heartbeat, supports isolated
agent turns, records run logs, applies backoff, auto-disables repeatedly broken
schedules, and avoids heartbeat execution while cron lanes are busy. Its main
weakness for Macaca is that much composition still lives in the gateway layer;
Macaca should move this composition into runtime-host providers and expose only
focused service/SDK clients to shells.

Recommended Macaca direction: introduce a serviceized autonomy scheduling
family with two cooperating services:

- `service.scheduler`: owns schedules, jobs, due calculation, run leasing,
  retries, missed-run policy, job snapshots, and run history.
- `service.heartbeat`: owns autonomous heartbeat wakes, active-hour windows,
  busy gates, coalesced wake requests, liveness events, and agent/application
  check-in runs.

The kernel should only own generic scheduler primitives such as identity,
capability registry names, policy facade, trace/audit evidence ids, service
call dispatch, resource quotas, and wakeup fairness. It must not parse cron
expressions, run agent jobs, deliver messages, inspect application workflows,
or hardcode heartbeat behavior.

## Constitutional Constraints

The three Macaca governance documents imply the following hard constraints.

### Microkernel Boundary

The kernel may own identity, policy facade, service registry, typed call
dispatch, trace/audit buses, scheduler primitives, session/task primitives, and
resource primitives. It must not own concrete LLM providers, planner loops,
driver runtimes, skill runtimes, MCP runtimes, gateway delivery, application
workflows, or shell behavior.

For cron/heartbeat this means:

- Kernel can define `ScheduleId`, `WakeId`, `ExecutionId`, `LeaseId`,
  `TraceId`, capability names, and fairness/resource primitives.
- Kernel must not own cron expression parsing, job payload execution, delivery
  routing, heartbeat prompts, or app-specific automation behavior.
- Cron/heartbeat must call through typed service commands with trace and policy
  before side effects.

### Serviceization Admission

Task planning, task execution, review, recovery, retry, execution control,
driver/skill/MCP, gateways, and third-party adapters must be services. Cron and
heartbeat touch all of these areas, so they must be serviceized from the start.

Every scheduling service must expose:

- Stable descriptor, command surface, lifecycle, health, snapshot, and
  structured errors.
- Trace-required calls and policy checks before side effects.
- Sanitized audit events and bounded snapshots.
- Unavailable, unsupported, denied, and failure states.
- Built-in, plugin, remote, mock, and unavailable provider replacement.

### Shell Boundary

Web, CLI, and frontend may create/list/update jobs by calling SDK/SystemFacade
clients. They may render schedule state, run logs, health, and audit evidence.
They must not own the scheduler loop, cron expression semantics, job execution
semantics, heartbeat prompts, retries, or delivery policy.

## Research: Hermes Agent

### High-Signal Files

- `hermes-agent/cron/jobs.py`
- `hermes-agent/cron/scheduler.py`
- `hermes-agent/gateway/run.py`
- `hermes-agent/agent/curator.py`
- `hermes-agent/hermes_cli/cron.py`

### Storage Model

Hermes stores jobs in `~/.hermes/cron/jobs.json` and job output under
`~/.hermes/cron/output/{job_id}/{timestamp}.md`.

Key traits:

- Secure directory/file permissions are applied to cron storage.
- Writes use temp files plus atomic replacement.
- An in-process lock protects `load_jobs -> modify -> save_jobs` because due
  jobs can run in parallel threads.
- Job output is always saved locally, even when delivery is suppressed.

Macaca lesson: run history should be a first-class audit/memento stream, not a
side-effect of a shell. The storage backend should be behind a persistence port
or service provider, not direct shell-owned JSON.

### Schedule Types

Hermes supports:

- One-shot relative durations like `30m`.
- Recurring intervals like `every 30m`.
- Cron expressions via `croniter`.
- ISO timestamps.

It computes next runs with timezone-aware normalization and a grace window for
one-shot jobs. Recurring jobs that missed the catch-up window are fast-forwarded
instead of burst-executed after downtime.

Macaca lesson: schedule DTOs should support at least `At`, `Every`, and `Cron`
with timezone and missed-run policy. Catch-up must be explicit, not accidental.

### Scheduler Tick

Hermes exposes `tick(verbose, adapters, loop)` in `cron/scheduler.py`.
The gateway starts a daemon thread that calls it every 60 seconds.

Reliability features:

- A cross-process file lock prevents concurrent ticks from gateway, daemon, or
  manual invocations.
- Recurring jobs advance `next_run_at` before execution, giving at-most-once
  behavior during crashes.
- Jobs with `workdir` run sequentially because they mutate process-global
  `TERMINAL_CWD`.
- Other jobs run through a thread pool with configurable max parallelism.
- After every tick, orphan MCP subprocess cleanup runs best-effort.

Macaca lesson: we need leases and active-run records. We should avoid
process-global mutation by making working directory part of a typed execution
command and by running jobs in isolated execution envelopes.

### Execution and Delivery

Hermes builds a cron-specific prompt with delivery guidance. The agent is told
that final output will be delivered automatically and that `[SILENT]` suppresses
delivery when there is nothing new to report.

It supports:

- Pre-run scripts that can return `wakeAgent=false` to skip the agent run.
- Per-job toolset overrides.
- Per-job working directories.
- ContextVars for delivery/session metadata during parallel runs.
- Delivery to origin or configured platform targets.
- MEDIA tag extraction for native attachments.
- Separate delivery errors from agent execution errors.
- Empty final responses are treated as soft failures.

Macaca lesson: job payloads should distinguish execution outcome from delivery
outcome. Delivery must be a gateway/service capability behind commands, not
agent self-delivery. A silent/no-op result should be explicit and auditable.

### Curator Piggyback

Hermes runs a background skill curator through the cron ticker. The ticker polls
hourly, while curator itself gates on `enabled`, `paused`, `last_run_at`, and
`interval_hours` defaults. First run is seeded and deferred to avoid surprising
mutation immediately after install.

Macaca lesson: higher-level autonomous maintenance loops should be application
or plugin jobs scheduled through the generic scheduler, not special code in the
gateway ticker.

### Hermes Strengths To Borrow

- Durable job/output storage.
- Cross-process tick lock or lease semantics.
- At-most-once recurring execution.
- Missed-run grace and fast-forwarding.
- Per-job workdir/context awareness.
- Delivery outcome separated from execution outcome.
- Script/preflight gate before expensive agent execution.
- Silent/no-op result contract.
- Best-effort cleanup after runs.

### Hermes Weaknesses To Avoid

- Gateway owns the cron ticker and too much scheduling behavior.
- Cron execution mutates process-wide environment for workdir.
- Delivery logic is embedded in scheduler code.
- Job execution directly constructs the agent.
- Cron is not represented as a provider-neutral system service descriptor.

## Research: OpenClaw

### High-Signal Files

- `openclaw/src/cron/service-contract.ts`
- `openclaw/src/cron/service.ts`
- `openclaw/src/cron/service/ops.ts`
- `openclaw/src/cron/service/timer.ts`
- `openclaw/src/cron/service/jobs.ts`
- `openclaw/src/cron/store.ts`
- `openclaw/src/cron/types.ts`
- `openclaw/src/gateway/server-cron.ts`
- `openclaw/src/gateway/server-runtime-services.ts`
- `openclaw/src/infra/heartbeat-runner.ts`
- `openclaw/src/infra/heartbeat-wake.ts`
- `openclaw/src/infra/heartbeat-events.ts`
- `openclaw/src/infra/heartbeat-schedule.ts`
- `openclaw/src/agents/tools/cron-tool.ts`

### Cron Service Contract

OpenClaw exposes cron through `CronServiceContract`:

- `start`
- `stop`
- `status`
- `list`
- `listPage`
- `add`
- `update`
- `remove`
- `run`
- `enqueueRun`
- `getJob`
- `getDefaultAgentId`
- `wake`

This is much closer to Macaca's serviceized target. The gateway composes the
service and supplies dependencies, while callers use a typed contract.

Macaca lesson: define `SchedulerService` and `HeartbeatService` command/result
DTOs in provider-neutral crates, then expose focused SDK clients and
SystemFacade methods for shells.

### Typed Job Model

OpenClaw models:

- `CronSchedule`: `at`, `every`, or `cron` with timezone and optional stagger.
- `CronSessionTarget`: `main`, `isolated`, `current`, or explicit session.
- `CronWakeMode`: `now` or `next-heartbeat`.
- `CronPayload`: `systemEvent` or `agentTurn`.
- `CronDelivery`: none, announce, or webhook.
- `CronRunOutcome`: ok, error, skipped, diagnostics, session ids, model/provider
  telemetry, delivery trace, and usage.

Macaca lesson: do not make "cron means agent prompt" the only payload. Use a
generic scheduled command envelope that can target service commands,
agent-execution commands, application lifecycle commands, heartbeat wakes, and
plugin-defined commands through declared capabilities.

### Store And State Split

OpenClaw stores cron job configuration separately from runtime state. Runtime
state is moved into a `jobs-state.json` style sidecar:

- Config can be versioned and migrated without constantly rewriting runtime
  fields.
- Runtime state tracks `nextRunAtMs`, `runningAtMs`, `lastRunAtMs`,
  `lastRunStatus`, diagnostics, consecutive errors, schedule errors, delivery
  state, and duration.

Macaca lesson: schedule definition and schedule runtime memento should be
separate. For Macaca, config belongs in package/application/service metadata or
the scheduler job store; runtime mementos belong in persistence/audit stores.

### Timer-Based Scheduler

OpenClaw does not poll every fixed minute for all work. It computes the next
wake time, arms a timer, and re-arms after each state change. It also clamps
long delays and maintains a periodic recheck to recover from wall-clock jumps
or stuck states.

Reliability features:

- `runningAtMs` prevents duplicate execution.
- `markCronJobActive` allows heartbeat to skip while cron is active.
- `applyJobResult` clears running state, updates outcome, computes next run,
  applies retry backoff, and emits failure alerts.
- Repeated schedule computation errors auto-disable jobs and enqueue a system
  event plus heartbeat wake so the user is notified.
- Missed jobs can be replayed on startup, with staggering to avoid gateway
  overload.
- Timer hot-loop guards avoid `setTimeout(0)` storms.

Macaca lesson: the scheduler service needs explicit job leases, active-run
tracking, backoff policy, schedule-error policy, missed-run policy, and
watchdog/recheck behavior.

### Gateway Composition

OpenClaw's gateway builds the cron service by injecting:

- Store path.
- Config and default agent id.
- System event enqueue function.
- Heartbeat request and direct heartbeat run functions.
- Isolated agent job runner.
- Timed-out agent cleanup.
- Failure alert sender.
- Event broadcaster and hook dispatch.
- Run-log appender.

Macaca lesson: this composition should move into `macaca-runtime-host` service
providers. Web/CLI should receive `SystemFacade` or focused clients only.

### Heartbeat Wake Layer

OpenClaw separates wake request coalescing from heartbeat execution:

- `requestHeartbeat` queues wake requests by agent/session target.
- Wake requests carry source, intent, reason, agent id, session key, and
  optional heartbeat overrides.
- The wake layer coalesces requests, prioritizes manual/immediate wakes, and
  retries retryable busy skips.
- `setHeartbeatWakeHandler` safely swaps lifecycle handlers during restart.

Macaca lesson: heartbeat should have a command-level wake queue with
coalescing, target normalization, retryable busy reasons, and lifecycle-safe
handler registration.

### Heartbeat Runner

OpenClaw heartbeat runner:

- Resolves heartbeat-enabled agents from config.
- Computes deterministic phase offsets from a stable scheduler seed to avoid
  thundering herds.
- Respects active-hours windows.
- Skips when main request lanes, cron lanes, or configured busy lanes are
  active.
- Supports isolated heartbeat sessions to avoid sending full conversation
  history every heartbeat.
- Reads heartbeat tasks from `HEARTBEAT.md`.
- Can inspect pending system events and cron events.
- Emits heartbeat events with status `sent`, `ok-empty`, `ok-token`,
  `skipped`, or `failed`.
- Separates "OK/no alert" from "send an alert" behavior.

Macaca lesson: heartbeat is not merely a health ping. It is an autonomous
attention loop that can inspect pending events, commitments, system state, and
application-declared heartbeat tasks while respecting policy and resource
limits.

### OpenClaw Strengths To Borrow

- Service contract around cron.
- Typed job model and protocol schemas.
- Store/state separation.
- Timer-based next-wake scheduling.
- Active-run tracking and heartbeat busy gates.
- Error backoff and schedule-error auto-disable.
- Startup missed-run handling with stagger.
- Wake modes: `now` and `next-heartbeat`.
- Heartbeat coalescing, retry, active hours, deterministic phase offsets.
- Event stream and hook integration.
- Isolated agent runs for cron and heartbeat.

### OpenClaw Weaknesses To Avoid

- Gateway still performs too much service composition.
- Cron payload names such as `agentTurn` and session modes should be generalized
  before entering Macaca OS contracts.
- Delivery and channel handling should be a gateway/capability service, not part
  of scheduler core.
- Heartbeat prompt/file conventions are useful but should remain application or
  agent policy, not kernel/service hardcoding.

## Superpowers Brainstorm

### Option A: Runtime-Host Timer Only

Add a simple runtime-host timer loop that scans stored schedules and calls
existing execution APIs.

Benefits:

- Fastest path.
- Minimal new contracts.
- Good for internal maintenance jobs.

Risks:

- Easy to recreate Hermes' gateway-owned semantics in another location.
- Hard to expose safely to applications.
- Hard to replace with remote/plugin scheduler providers later.
- Weak audit and policy boundaries unless extra work is added immediately.

Verdict: reject as the stable design. It may be acceptable only as a temporary
provider behind a real service contract.

### Option B: Serviceized Scheduler + Heartbeat Services

Create typed scheduler and heartbeat service contracts, with built-in providers
composed by runtime-host and consumed through SDK/SystemFacade clients.

Benefits:

- Matches Macaca microkernel and serviceization constitutions.
- Keeps Web/CLI thin.
- Gives applications a generic scheduled-work capability.
- Supports built-in, plugin, remote, mock, and unavailable providers.
- Makes trace, policy, health, snapshot, run history, and recovery auditable.

Risks:

- More up-front contract work.
- Requires careful DTO boundaries to avoid baking in application-specific job
  semantics.
- Needs migration gates to avoid shells calling provider internals directly.

Verdict: recommended.

### Option C: Plugin-First Scheduler

Make scheduling entirely plugin-owned. The base OS only exposes plugin lifecycle
and event bus hooks.

Benefits:

- Maximum extension flexibility.
- Keeps base OS small.
- External schedulers can be swapped in.

Risks:

- Macaca needs scheduling as a basic autonomy capability for upper-layer
  applications; making it optional-only weakens the platform contract.
- Harder to guarantee consistent trace, policy, audit, and resource controls.
- Applications would have inconsistent scheduling semantics across installs.

Verdict: keep as an extension path, not the base capability.

## Recommended Macaca Architecture

### Ownership Split

| Layer | Ownership |
| --- | --- |
| Kernel | Schedule identity, wake identity, lease identity, service registry, policy facade, trace/audit ids, resource/fairness primitives |
| Foundation/proto | Provider-neutral scheduler and heartbeat command/result DTOs |
| System services | Scheduler service and heartbeat service contracts |
| Runtime host | Built-in scheduler/heartbeat providers, persistence adapters, timer driver, service decorators |
| SDK/SystemFacade | Focused clients for schedule CRUD, run, wake, status, history, and snapshots |
| Application framework | Manifest-declared scheduled capabilities and app-scoped job permissions |
| Plugins/modules | Optional scheduler providers, delivery providers, calendar providers, remote runners |
| Shells | Render jobs, logs, health, and approvals; call SDK clients only |

### Service: `service.scheduler`

Responsibilities:

- Register schedule service descriptor and lifecycle.
- Validate job creation/update commands.
- Persist job definitions and runtime mementos.
- Compute next run for `At`, `Every`, and `Cron` schedule types.
- Support timezone, deterministic stagger, active windows, and missed-run policy.
- Lease due jobs before execution.
- Track `Scheduled`, `Leased`, `Running`, `Succeeded`, `Skipped`, `Failed`,
  `Disabled`, and `Expired` states.
- Emit sanitized events for job added, updated, removed, started, finished,
  skipped, failed, disabled, and lease expired.
- Apply retry/backoff and schedule-error policies.
- Expose health and snapshots.

Non-responsibilities:

- It must not construct LLM providers.
- It must not know application names or workflow names.
- It must not own delivery/channel implementations.
- It must not hardcode prompt text or business-specific agent roles.

### Service: `service.heartbeat`

Responsibilities:

- Register heartbeat service descriptor and lifecycle.
- Maintain per-agent or per-application heartbeat policies.
- Coalesce wake requests by target.
- Support scheduled, event, immediate, and manual wake intents.
- Respect active hours, cooldowns, busy lanes, resource policy, and cron-active
  gates.
- Run heartbeat checks through declared service/agent execution boundaries.
- Emit liveness, no-op, skipped, alert, failed, and delivered/undelivered
  events.
- Expose last heartbeat and deterministic snapshots.

Non-responsibilities:

- It must not own gateway/channel delivery implementations.
- It must not hardcode `HEARTBEAT.md` as an OS-layer requirement. A file-based
  heartbeat source can be an application/agent adapter strategy.
- It must not inspect raw prompts or leak raw session transcripts into logs.

### Scheduler Payload Model

Macaca should avoid an application-specific payload enum. A provider-neutral
payload model can be:

- `ServiceCommand`: call a typed service command.
- `AgentExecutionCommand`: call agent execution service with application,
  session, task, policy, and trace scope.
- `HeartbeatWakeCommand`: request heartbeat service wake.
- `ApplicationCommand`: call application service lifecycle or app-scoped command
  declared in manifest capabilities.
- `PluginCommand`: call a plugin-declared command through plugin capability
  registry.

Every payload must carry:

- application/session/task/tenant scope where applicable,
- trace context,
- policy context,
- resource budget,
- idempotency key,
- sanitized audit metadata,
- optional delivery intent as a separate capability, not embedded delivery code.

### Required Design Patterns

- **Facade:** SDK/SystemFacade clients expose scheduler and heartbeat operations
  to Web, CLI, applications, and plugins.
- **Command:** job payloads and service calls are typed commands/results.
- **Strategy:** schedule calculators, missed-run policy, retry policy, active
  windows, delivery intent resolution, and heartbeat prompt/context sources are
  replaceable.
- **Decorator:** trace, policy, resource, entitlement, budget, and metering run
  before scheduler/heartbeat side effects.
- **State:** job, run, lease, heartbeat, and delivery state machines are explicit.
- **Observer:** cron and heartbeat events flow through trace/audit/event buses.
- **Memento:** job definitions, run history, leases, snapshots, and checkpoints
  are replayable.
- **Abstract Factory:** built-in, plugin, remote, mock, and unavailable providers
  are composed only in runtime-host or plugin composition roots.
- **Null Object:** unavailable scheduler/heartbeat providers return structured
  unavailable results without crashing or faking success.
- **Specification:** dependency gates and static escape-hatch gates prevent
  shells/kernel from owning scheduling semantics.

## Proposed Command Surface

Initial scheduler commands:

- `scheduler.create_job`
- `scheduler.update_job`
- `scheduler.remove_job`
- `scheduler.get_job`
- `scheduler.list_jobs`
- `scheduler.run_job`
- `scheduler.enqueue_run`
- `scheduler.pause_job`
- `scheduler.resume_job`
- `scheduler.get_status`
- `scheduler.snapshot`
- `scheduler.list_runs`

Initial heartbeat commands:

- `heartbeat.register_policy`
- `heartbeat.update_policy`
- `heartbeat.request_wake`
- `heartbeat.run_once`
- `heartbeat.get_status`
- `heartbeat.get_last_event`
- `heartbeat.snapshot`
- `heartbeat.pause`
- `heartbeat.resume`

All commands must require trace context and return structured errors:

- `Unavailable`
- `Unsupported`
- `Denied`
- `InvalidCommand`
- `ScheduleError`
- `AlreadyRunning`
- `NotDue`
- `ExecutionFailed`
- `DeliveryFailed`
- `LeaseExpired`

## OpenSpec Direction

Recommended change id:

- `add-autonomy-scheduler-heartbeat-services-v1`

Affected future specs:

- `scheduler-service`
- `heartbeat-service`
- `autonomous-runtime`
- `sdk-system-facade`
- `serviceization-escape-hatches`
- `serviceization-dependency-gate`

The OpenSpec proposal should state that this is an additive service
capability. It must not migrate existing task execution behavior in the first
slice. The first slice should establish contracts, DTOs, Null Object behavior,
runtime-host provider shell, and executable boundary gates.

## Phased Delivery Recommendation

### Phase 1: Contracts And Governance

- Add OpenSpec proposal/design/tasks/specs.
- Add provider-neutral DTOs for scheduler and heartbeat commands/results.
- Add governance docs describing ownership and rejection rules.
- Extend serviceization escape-hatch gates to reject shell/kernel scheduler
  semantics.

### Phase 2: Built-In Unavailable Providers

- Register scheduler and heartbeat service descriptors.
- Provide Null Object/unavailable providers.
- Expose SDK clients and SystemFacade methods.
- Add health and snapshot commands.

### Phase 3: Local Built-In Scheduler Provider

- Implement local timer provider behind `service.scheduler`.
- Persist job config and runtime mementos through persistence ports.
- Support `At`, `Every`, `Cron`, timezone, stagger, missed-run policy, lease,
  status, run history, retry/backoff, and schedule-error auto-disable.

### Phase 4: Heartbeat Provider

- Implement coalesced wake requests and per-agent/app heartbeat policy.
- Add active hours, cooldown, busy gates, cron-active gate, and deterministic
  phase scheduling.
- Emit heartbeat events and snapshots.

### Phase 5: Application Capability Surface

- Allow application manifests to request scheduled-job and heartbeat
  capabilities.
- Validate permissions and resource budgets before app-owned scheduled work is
  admitted.
- Keep concrete job payload behavior application-owned or service-owned.

### Phase 6: Shell Integration

- Web/CLI render job lists, run logs, status, and health through SDK clients.
- Shells do not own the scheduler loop or heartbeat runner.

## Validation Gates

Targeted gates:

- OpenSpec strict validation for the new change.
- Service runtime tests for scheduler and heartbeat descriptors/lifecycle.
- SDK/SystemFacade tests for command validation and Null Object behavior.
- Dependency-boundary tests proving kernel and shells do not depend on provider
  implementations.
- Static escape-hatch tests rejecting scheduler/heartbeat semantics in Web/CLI
  outside command adapters.
- Persistence tests for job config/state split and crash recovery.
- Audit replay tests for job run and heartbeat event chains.

Manual validation:

- Existing YAML, WASM, and GenUI applications still run.
- `/api/chat/v2` session creation and recovery do not regress.
- Task boards remain session-scoped.
- Scheduler and heartbeat services return structured unavailable states when
  providers are absent.
- Logs and snapshots do not expose raw secrets, prompts, manifests, WASM bytes,
  package bytes, private keys, credentials, raw provider payloads, or unbounded
  output.

## Key Takeaways

1. Do not copy Hermes' gateway-owned ticker into Macaca shells.
2. Borrow Hermes' reliability details: locking/leases, output mementos,
   at-most-once recurring runs, preflight gates, delivery separation, and
   cleanup.
3. Borrow OpenClaw's service-shaped design: typed contract, store/state split,
   timer-based next wake, wake modes, active-run tracking, heartbeat busy gates,
   event streams, and backoff.
4. Improve on both systems by making runtime-host the composition root and
   exposing only service/SDK facades to Web/CLI.
5. Treat scheduler and heartbeat as autonomy services, not helper utilities.

