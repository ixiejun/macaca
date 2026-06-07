# Tasks: Add Autonomy Scheduler and Heartbeat Services V1

## 1. OpenSpec and Governance

- [x] 1.1 Capture Hermes/OpenClaw cron and heartbeat research in `docs/`.
- [x] 1.2 Create this OpenSpec proposal, design, task checklist, and delta specs.
- [x] 1.3 Validate the OpenSpec change with `openspec validate add-autonomy-scheduler-heartbeat-services-v1 --strict`.
- [x] 1.4 Re-check the three Macaca constitutions before Rust implementation begins.

## 2. Provider-Neutral DTOs

- [x] 2.1 Add scheduler DTOs for job identity, schedule spec, missed-run policy, lease policy, retry policy, run state, snapshots, and structured errors.
- [x] 2.2 Add heartbeat DTOs for wake intent, coalescing result, gate evaluation, heartbeat run state, snapshots, and structured errors.
- [x] 2.3 Ensure DTOs carry trace/audit correlation without exposing raw secrets or unbounded provider payloads.
- [x] 2.4 Add English comments explaining each non-obvious DTO and state-machine invariant.

## 3. Service Contracts and Null Object Providers

- [x] 3.1 Add Scheduler Service trait/contract with descriptor, lifecycle, health, snapshot, typed commands/results, and structured unavailable behavior.
- [x] 3.2 Add Heartbeat Service trait/contract with descriptor, lifecycle, health, snapshot, typed commands/results, and structured unavailable behavior.
- [x] 3.3 Implement fail-closed unavailable providers using the Null Object pattern.
- [x] 3.4 Log key command acceptance, denial, unavailable, lifecycle, and snapshot events without leaking payloads.

## 4. Runtime-Host Provider Registration

- [x] 4.1 Register scheduler and heartbeat providers through runtime-host composition, not kernel, SDK, Web, CLI, or application code.
- [x] 4.2 Preserve replacement mechanics for built-in, local, plugin, remote, mock, and unavailable providers.
- [x] 4.3 Ensure runtime-host registration does not branch on application, workflow, provider, driver, model, chain, gateway, or business names.

## 5. Scheduler Local Provider

- [x] 5.1 Implement generic job definition storage and runtime memento storage behind replaceable persistence boundaries.
- [x] 5.2 Implement schedule calculation for `At`, `Every`, and `CronExpression` with time-zone and deterministic stagger handling.
- [x] 5.3 Implement missed-run policy, bounded catch-up, concurrency policy, lease acquisition, lease expiry, and lease recovery.
- [x] 5.4 Implement run lifecycle transitions, retry/backoff, cancellation, run history, health snapshots, and sanitized audit records.
- [x] 5.5 Add focused logs for due-work calculation, lease acquisition, dispatch, retry, skip, failure, and completion.

## 6. Heartbeat Local Provider

- [x] 6.1 Implement wake intent validation for scheduled, event, immediate, manual, and recovery wakes.
- [x] 6.2 Implement coalescing by scope, active-hours gates, cooldown gates, busy gates, resource/budget gates, and provider-health gates.
- [x] 6.3 Integrate recurring heartbeat ticks through `service.scheduler` using generic `HeartbeatWakeCommand`.
- [x] 6.4 Implement heartbeat run lifecycle, last-event snapshots, health snapshots, and sanitized audit records.
- [x] 6.5 Add focused logs for wake accepted, wake coalesced, gate denied, gate delayed, dispatch, failure, skip, and completion.

## 7. SDK and Facade Clients

- [x] 7.1 Add focused Scheduler client methods to `SystemFacade` or SDK service clients.
- [x] 7.2 Add focused Heartbeat client methods to `SystemFacade` or SDK service clients.
- [x] 7.3 Ensure clients only build typed command DTOs and call the service runtime; they must not construct providers, timers, stores, or queues.
- [x] 7.4 Return structured unavailable/unsupported/denied/failure results instead of panics or silent fallbacks.

## 8. Boundary Gates and Validation

- [x] 8.1 Add serviceization escape-hatch gates that reject scheduler or heartbeat semantics in kernel, Web, CLI, frontend, and application-specific branches.
- [x] 8.2 Add dependency-boundary checks proving shells call facade clients rather than service providers or runtime internals.
- [x] 8.3 Add integration tests for unavailable providers, provider registration, typed commands, trace/audit evidence, and boundary violations.
- [x] 8.4 Run targeted Rust checks only after implementation slices are ready and approved for verification.

## 9. Documentation and Follow-Up

- [x] 9.1 Update architecture docs if implementation introduces new stable ownership diagrams.
- [x] 9.2 Document safe extension points for plugin/remote scheduler and heartbeat providers.
- [x] 9.3 Keep application examples outside OS-layer code and route them through declared service capabilities.
