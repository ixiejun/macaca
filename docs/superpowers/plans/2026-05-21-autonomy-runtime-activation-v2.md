# Autonomy Runtime Activation V2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Activate local Scheduler and Heartbeat providers behind explicit runtime-host configuration and add a narrow lifecycle-managed supervisor that can dispatch provider-neutral work.

**Architecture:** Runtime-host remains the Abstract Factory composition root. Disabled mode keeps fail-closed unavailable providers. Enabled local mode registers local providers and starts an `AutonomySupervisor` that uses Strategy dispatch through ServiceRuntime, while Scheduler owns run state and Heartbeat owns wake gates.

**Tech Stack:** Rust, Tokio, `macaca-runtime-host`, `macaca-scheduler`, `macaca-heartbeat`, `macaca-proto`, OpenSpec.

---

### Task 1: Runtime-host activation configuration and factory

**Files:**
- Create: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_runtime_config.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`

- [ ] Add provider-neutral `AutonomyRuntimeConfig`, `AutonomyProviderMode`, and bounded defaults.
- [ ] Add local bootstrap that registers `LocalSchedulerProvider` and `LocalHeartbeatProvider` only when local mode is explicitly selected.
- [ ] Preserve existing unavailable bootstrap as the default.
- [ ] Export the new config, local bootstrap, and bundle status.
- [ ] Log config resolution, selected mode, provider registration, start, stop, and cleanup.

### Task 2: Scheduler lease target access for local supervisor

**Files:**
- Modify: `macaca/crates/services/macaca-scheduler/src/local_provider.rs`
- Modify: `macaca/crates/services/macaca-scheduler/src/local_provider/run_control.rs`

- [ ] Add `LocalSchedulerLeasedRun` containing the leased run summary and cloned provider-neutral target command.
- [ ] Add `acquire_next_run_lease_with_target` so runtime-host can dispatch without reading private store internals.
- [ ] Keep existing `acquire_next_run_lease` for compatibility.
- [ ] Add English comments and logs explaining that target access is payload-reference-only and does not execute application behavior.

### Task 3: Autonomy supervisor and dispatch strategies

**Files:**
- Create: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_dispatch.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`

- [ ] Add `AutonomySupervisor` lifecycle with `start`, `stop`, `run_scheduler_tick_once`, and `run_heartbeat_tick_once`.
- [ ] Add a Strategy dispatcher for provider-neutral target categories.
- [ ] Implement real dispatch for `ServiceCommand` and `HeartbeatWakeCommand`.
- [ ] Mark non-ready categories as structured skipped/unsupported through Scheduler run transitions without panics.
- [ ] Use bounded leases per tick and dispatch timeout.
- [ ] Add structured logs at each key node.

### Task 4: Integration tests and boundary gates

**Files:**
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/autonomy_scheduler_heartbeat_services.rs`
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/serviceization_escape_hatches.rs`
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries/gate.rs`

- [ ] Add disabled-mode test proving unavailable providers and no supervisor.
- [ ] Add enabled local-mode test proving active providers and supervisor handle.
- [ ] Add scheduler tick test dispatching a generic mock service command through ServiceRuntime.
- [ ] Add heartbeat recovery/scheduled wake test.
- [ ] Extend gates to reject autonomy loops and local provider construction outside runtime-host.

### Task 5: Docs, tasks, and validation

**Files:**
- Modify: `openspec/changes/activate-local-autonomy-runtime-v2/tasks.md`
- Modify: `macaca/docs/autonomy-scheduler-heartbeat-services.md`

- [ ] Update docs with explicit disabled/default and local activation ownership.
- [ ] Mark OpenSpec tasks complete only after implementation and tests pass.
- [ ] Run `openspec validate activate-local-autonomy-runtime-v2 --strict`.
- [ ] Run targeted Rust tests for runtime-host, scheduler, heartbeat, and integration gates.
