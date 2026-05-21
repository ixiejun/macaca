# Decouple Heartbeat From Scheduler Runtime Design

## Context

Macaca OS currently exposes `service.scheduler` and `service.heartbeat` as
separate autonomy services, but the recent scheduler-management UI made a
design tension visible: heartbeat wake commands can be created as Scheduler job
targets. That is useful as a compatibility bridge, but it makes heartbeat look
like a user-created scheduled task. It also weakens the autonomy model because
agent heartbeat cadence should not depend on application schedule jobs.

The architecture constitutions require non-kernel capabilities to stay
serviceized, modular, provider-neutral, traceable, auditable, and free of
application-specific semantics. Heartbeat is an autonomy service that coordinates
agent and system wake cadence; Scheduler is an autonomy service that materializes
application/system schedule definitions into due runs. They should cooperate
through typed service commands, but neither should be the semantic owner of the
other.

## Problem Statement

Recurring heartbeat behavior is currently described as Scheduler jobs that
enqueue `HeartbeatWakeCommand`. This creates four problems:

- Heartbeat cadence appears to be a Scheduler job rather than an independent
  autonomy lane.
- Scheduler provider unavailability can block natural heartbeat operation.
- Schedule-management UI can expose heartbeat as if it were application-created
  recurring work.
- Agent-level autonomous behavior, such as memory consolidation or fixed
  recurring agent duties, lacks a clear heartbeat-owned profile model.

## Goals

- Decouple heartbeat cadence from scheduler due-run materialization.
- Keep Scheduler and Heartbeat as sibling system services behind typed service
  boundaries.
- Make runtime-host `AutonomySupervisor` own two lifecycle-managed lanes:
  `SchedulerLane` and `HeartbeatLane`.
- Support agent/system heartbeat profiles without hardcoded app, workflow,
  model, driver, provider, or business-domain names.
- Preserve trace, audit, policy, resource, health, snapshot, and structured
  unavailable behavior for both lanes.
- Keep frontend and Web shells as presentation/adapter surfaces only.

## Non-Goals

- Do not implement application-specific heartbeat tasks.
- Do not move heartbeat cadence into the microkernel.
- Do not make Web, CLI, SDK, or frontend own timers, loops, coalescing, or
  heartbeat lifecycle semantics.
- Do not remove Scheduler service or application schedule-management APIs.
- Do not design a full heartbeat-management UI in this change; the UI impact is
  limited to avoiding heartbeat-as-schedule confusion.

## Recommended Approach: Dual Autonomy Lanes

Use runtime-host `AutonomySupervisor` as the approved composition root and
lifecycle owner for two sibling lanes:

```text
runtime-host AutonomySupervisor
  |-- SchedulerLane
  |   |-- tick service.scheduler
  |   |-- materialize due scheduler runs
  |   |-- lease due runs
  |   `-- dispatch provider-neutral scheduled targets
  |
  `-- HeartbeatLane
      |-- tick service.heartbeat cadence
      |-- resolve provider-neutral heartbeat scopes
      |-- evaluate coalescing, gates, policy, and health
      |-- dispatch provider-neutral heartbeat actions
      `-- persist heartbeat mementos and audit evidence
```

Scheduler owns scheduled job definitions, due calculations, leases, and
scheduled run history. Heartbeat owns heartbeat profiles, cadence, wake
coalescing, gates, heartbeat run history, and heartbeat action dispatch.

## Design Patterns

- **Facade:** SDK/SystemFacade exposes focused scheduler and heartbeat clients
  without constructing providers or owning loops.
- **Command:** Scheduler and Heartbeat commands remain provider-neutral typed
  DTOs with trace context.
- **Strategy:** Heartbeat cadence and action dispatch use replaceable strategies
  such as fixed interval, adaptive interval, and load-aware interval.
- **Observer:** Heartbeat emits sanitized events and audit records that memory,
  task, execution, and diagnostic services can consume through service
  boundaries.
- **State:** Scheduler runs and heartbeat runs each have explicit lifecycle
  states, preventing hidden task execution.
- **Memento:** Heartbeat profiles, last ticks, gate outcomes, and run summaries
  are persisted as bounded mementos for restart recovery and audit replay.
- **Abstract Factory:** Runtime-host provider factories compose local,
  unavailable, remote, or plugin-backed Scheduler and Heartbeat providers.

## Service Boundary Decisions

### Scheduler Service

Scheduler SHALL manage application/system scheduled jobs only. It SHALL
materialize due runs and dispatch provider-neutral scheduled targets through
runtime-host supervision. Scheduler SHALL NOT be required for heartbeat cadence.

### Heartbeat Service

Heartbeat SHALL own native cadence, scope resolution, wake coalescing, gates,
and heartbeat run lifecycle. It SHALL accept immediate/manual/event/recovery
wakes, and it SHALL also tick itself through `HeartbeatLane` when local autonomy
is enabled.

### Runtime Host

Runtime-host SHALL be the only approved owner of the concrete local autonomy
supervisor. It SHALL start, stop, snapshot, and observe `SchedulerLane` and
`HeartbeatLane` independently. If one lane is unavailable or degraded, the other
lane SHALL return structured evidence and continue when policy permits.

### Web and Frontend

Web routes and frontend components SHALL remain adapters. Schedule-management UI
SHALL manage Scheduler jobs and SHALL NOT present Heartbeat as a normal schedule
target. A future heartbeat-management UI, if needed, must call heartbeat-focused
service routes and show heartbeat profiles/runs separately.

## Heartbeat Scope Model

Heartbeat scope identities SHALL be provider-neutral typed values rather than
hardcoded strings. Valid conceptual scopes include:

- System autonomy scope.
- Application autonomy scope.
- Agent autonomy scope.
- Session or task recovery scope when declared by policy.

Code must not special-case any application name, workflow name, agent role,
provider name, model name, driver name, gateway name, chain name, payment name,
or business-domain string.

## Heartbeat Actions

Heartbeat SHALL NOT implement memory consolidation, task execution, review, or
business work directly. A heartbeat action is a provider-neutral command routed
to another service boundary, for example:

- memory service command for memory consolidation.
- task service command for generic periodic task evaluation.
- execution service command for resumable work inspection.
- diagnostics service command for health probing.

The action catalog must be declared through capabilities and policy. The
heartbeat provider records safe action summaries, trace identifiers, audit
identifiers, lifecycle states, and gate outcomes, never raw prompts or raw
provider payloads.

## Migration Strategy

1. Update OpenSpec to mark Scheduler-driven heartbeat ticks as deprecated or
   compatibility-only.
2. Add Heartbeat native cadence/profile requirements.
3. Split runtime-host supervisor behavior into SchedulerLane and HeartbeatLane.
4. Remove HeartbeatWake from application-facing schedule creation UI.
5. Add focused tests proving Scheduler can run without Heartbeat cadence and
   Heartbeat cadence can run without Scheduler due-run materialization.
6. Preserve structured unavailable behavior for absent providers.

## Risks and Mitigations

- **Risk:** Heartbeat becomes a hidden task runner.
  **Mitigation:** Heartbeat dispatches only typed service commands through
  declared capabilities and records bounded mementos.
- **Risk:** Runtime-host supervisor becomes too broad.
  **Mitigation:** Split lane modules by responsibility and keep each lane behind
  a narrow interface.
- **Risk:** Existing tests or demos rely on Scheduler-targeted HeartbeatWake.
  **Mitigation:** Keep a compatibility path for internal/runtime-only wake
  intents during migration, but remove it from application-facing UI.
- **Risk:** UI loses the ability to manually test heartbeat.
  **Mitigation:** Add a future heartbeat diagnostic action surface rather than
  using schedule creation as a heartbeat test tool.

## Approval Recommendation

Proceed with an OpenSpec change named
`decouple-heartbeat-from-scheduler-runtime`. The change should modify
`heartbeat-service`, `autonomous-runtime`, `scheduler-service`,
`web-cli-thin-shell-v0`, and `serviceization-escape-hatches` so implementation
can follow the architecture without reintroducing scheduler-owned heartbeat
semantics.
