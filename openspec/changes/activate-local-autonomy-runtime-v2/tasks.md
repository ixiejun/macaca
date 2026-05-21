# Tasks: Activate Local Autonomy Runtime V2

## 1. OpenSpec and Governance

- [x] 1.1 Validate this proposal against `macaca-os-architecture-governance.md`,
  `macaca-os-microkernel-boundaries.md`, and
  `macaca-os-serviceization-allowlist.md`.
- [x] 1.2 Re-check existing scheduler/heartbeat V1 contracts and avoid
  duplicating or weakening provider-neutral boundaries.
- [x] 1.3 Run `openspec validate activate-local-autonomy-runtime-v2 --strict`.

## 2. Runtime-Host Configuration and Factory

- [x] 2.1 Add provider-neutral autonomy runtime configuration with disabled
  unavailable mode as the default.
- [x] 2.2 Add runtime-host Abstract Factory wiring for unavailable and local
  provider bundles.
- [x] 2.3 Ensure configuration contains no application, workflow, model,
  driver, gateway, chain, payment, provider-business, or domain-specific names.
- [x] 2.4 Add detailed English comments explaining activation mode, optional
  module behavior, and fail-closed defaults.
- [x] 2.5 Add structured logs for config resolution and provider mode selection.

## 3. Local Provider Activation

- [x] 3.1 Register `LocalSchedulerProvider` through runtime-host only when
  local autonomy is explicitly enabled.
- [x] 3.2 Register `LocalHeartbeatProvider` through runtime-host only when
  local autonomy is explicitly enabled.
- [x] 3.3 Preserve unavailable provider registration when local autonomy is
  disabled.
- [x] 3.4 Prove SDK/SystemFacade calls reach active local providers only in
  enabled mode.
- [x] 3.5 Add structured logs for provider registration, start, stop, and
  cleanup.

## 4. Autonomy Supervisor Lifecycle

- [x] 4.1 Add lifecycle-managed `AutonomySupervisor` owned by runtime-host.
- [x] 4.2 Add start, stop, shutdown-grace, and cancellation handling.
- [x] 4.3 Ensure no supervisor loop starts in disabled unavailable mode.
- [x] 4.4 Add bounded tick configuration, max leases per tick, dispatch timeout,
  and safe retention bounds.
- [x] 4.5 Add detailed English comments explaining daemon lifecycle and safety
  invariants.
- [x] 4.6 Add structured logs for supervisor start, tick, idle, cancellation,
  timeout, and stop.

## 5. Scheduler Dispatch Loop

- [x] 5.1 Materialize or refresh due scheduler runs through Scheduler service
  boundaries.
- [x] 5.2 Acquire bounded leases before dispatching any run.
- [x] 5.3 Dispatch only provider-neutral target command categories through
  approved service/application/task/plugin boundaries.
- [x] 5.4 Transition runs to succeeded, failed, skipped, expired, or retry
  queued with sanitized evidence.
- [x] 5.5 Add Strategy-based dispatch modules so the supervisor does not become
  a god object.
- [x] 5.6 Add structured logs for due calculation, lease acquisition, strategy
  selection, dispatch outcome, retry, skip, failure, and completion.

## 6. Heartbeat Runtime Integration

- [x] 6.1 Add heartbeat scheduled tick lane through `HeartbeatWakeCommand`.
- [x] 6.2 Add optional recovery wake on runtime-host startup when local autonomy
  is enabled and recovery wakes are configured.
- [x] 6.3 Let Heartbeat own coalescing, gates, and wake lifecycle; supervisor
  only dispatches accepted generic targets.
- [x] 6.4 Add structured logs for recovery wake, scheduled wake, coalesced wake,
  gated wake, accepted wake, dispatch, skip, and completion.

## 7. Boundary Gates and Security

- [x] 7.1 Extend serviceization escape-hatch tests to reject background
  autonomy loops outside runtime-host.
- [x] 7.2 Extend dependency-boundary tests to reject local provider construction
  in kernel, SDK, Web, CLI, frontend, and application-specific code.
- [x] 7.3 Add checks that logs, snapshots, and audit DTOs do not expose raw
  secrets, prompts, manifests, package bytes, WASM bytes, private keys,
  credentials, raw signatures, raw provider payloads, or unbounded output.
- [x] 7.4 Run GitNexus impact analysis before editing implementation symbols.

## 8. Tests and Validation

- [x] 8.1 Add tests proving disabled mode registers unavailable providers and
  starts no supervisor.
- [x] 8.2 Add tests proving enabled local mode registers active local providers.
- [x] 8.3 Add tests proving application-facing facade calls become
  production-active in enabled mode.
- [x] 8.4 Add tests proving scheduler tick leases and dispatches generic service
  commands.
- [x] 8.5 Add tests proving heartbeat scheduled and recovery wakes pass through
  Heartbeat gates.
- [x] 8.6 Add tests proving shutdown cancels loops cleanly.
- [x] 8.7 Run targeted Rust tests, boundary tests, and OpenSpec validation.

## 9. Documentation

- [x] 9.1 Update architecture docs with local autonomy activation ownership and
  disabled/default behavior.
- [x] 9.2 Document safe extension points for future plugin, remote, mock, and
  distributed autonomy runners.
- [x] 9.3 Document application usage through declared capabilities and
  SystemFacade only, with no application-specific OS branches.

## 10. Web Host Activation and Application Monitoring

- [x] 10.1 Explicitly enable local autonomy runtime in local web host
  configuration while preserving fail-closed Rust defaults.
- [x] 10.2 Bootstrap autonomy services from the web host through runtime-host
  configuration and retain the supervisor bundle for lifecycle ownership.
- [x] 10.3 Add application-scoped serviceized Scheduler registration and run
  monitoring routes backed by SDK clients.
