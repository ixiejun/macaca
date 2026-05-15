# Change: Add Execution Control Service v1

## Why

Macaca currently has pause/resume behavior embedded in specific execution paths, especially the chat main-thread goal flow. That fixes one coordinator bug, but it leaves pause/resume as web/runtime glue instead of a general execution capability that applications can opt into and define through policy.

Macaca OS treats session lifecycle, pause/resume, checkpoint identity, cancellation, trace, and audit as system invariants. Execution control therefore needs a provider-neutral contract that can start as a built-in runtime capability and then graduate into `service.execution_control` without hardcoding application names, agent names, workflow names, or driver names.

## What Changes

- Add an `ExecutionControl` capability for pause/resume, checkpoint identity, execution state transitions, and resume signals.
- Support both application-declared defaults and per-execution overrides:
  - Application manifest / application runtime metadata declares the default execution-control policy.
  - `AgentExecutionCommand` may provide a bounded override for a single run.
  - Runtime merges them deterministically, with command overrides constrained by declared app capabilities and policy.
- Stage 1: implement execution control as an internal runtime capability consumed by `service.agent_execution`.
- Stage 2: expose the same contract as `service.execution_control` with descriptor, lifecycle, health, snapshot, typed commands, trace, policy, audit, and structured unavailable behavior.
- Replace path-specific pause wiring with strategy-driven triggers and resume sources such as tool-call barriers, goal completion, fork validation, approval, workflow barriers, app lifecycle signals, or future plugin events.
- Require sanitized trace/audit events for pause requested, pause entered, checkpoint recorded, resume requested, resume accepted, resume delivered, resume timed out, and resume rejected.
- Preserve existing `/api/chat/v2`, goal/planner/worker/review, workflow, YAML, WASM, GenUI, and headless application behavior while moving ownership away from shell glue.

## Impact

- Affected specs: `execution-control-service`
- Affected code:
  - `macaca-proto`: provider-neutral execution-control DTOs and command/result types.
  - `macaca-runtime-host`: built-in capability implementation, service provider, decorators, snapshots, and unavailable provider.
  - `macaca-web`: adapter migration away from path-specific pause/resume ownership.
  - `macaca-app` / application manifest adapters: app-declared execution-control capability defaults.
  - `macaca-sdk`: focused client or facade entrypoint after service exposure.
  - `macaca-task` / planner-worker integration: resume-source adapters remain policy driven and traceable.
  - Tests and boundary gates for trace, audit, policy, dependency direction, and no application-specific branching.

## Governance

This change follows:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

The kernel may own only identity, session primitive contracts, trace identity, policy facade, and service-call routing. It must not own concrete pause trigger rules, app workflows, planner behavior, worker behavior, approval semantics, or provider construction. Application-specific policy belongs to application declarations and service-consumed strategy configuration.
