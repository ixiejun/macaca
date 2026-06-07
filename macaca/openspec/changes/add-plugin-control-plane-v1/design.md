# Design: Plugin Control Plane v1

## Context

Macaca Plugin Runtime v0 is descriptor-first. It can validate manifests, select descriptor-safe hosts, and register plugin-owned service descriptors, but it does not manage where plugins come from, whether they are enabled, whether config/secrets are missing, how health is reported, or how Web/CLI inspect plugins.

This change introduces the Plugin Control Plane as a runtime-host-owned service facade. It does not execute third-party code. Execution hosts are handled by later plugin phases.

## Goals

- Provide an OS-level plugin management control plane.
- Keep Web/CLI as thin shells.
- Keep kernel limited to identity/lifecycle/ownership invariants.
- Make plugin state traceable, auditable, deterministic, and unavailable-safe.
- Define repository/install-source abstractions without committing to one marketplace or package manager.
- Keep the design ready for Store, Entitlement, Capability Registry, Hook Bus, and host execution phases.

## Non-Goals

- No real third-party WASM/process/native execution.
- No real network git clone in the first implementation unless separately approved.
- No Store marketplace implementation.
- No plugin capability call execution; that belongs to `add-plugin-capability-registry-v1`.
- No hook execution; that belongs to `add-plugin-hook-bus-v1`.

## Architecture

```text
Web / CLI
  -> macaca-sdk PluginControlClient
  -> ServiceRuntime / PluginControlService
  -> PluginRepository + ManifestLoader + AdmissionChain
  -> PluginRuntimeFacade / PluginRegistry
  -> Trace / Audit / Health Snapshot
```

## Design Patterns

- **Facade**: `PluginControlService` hides repository, loader, admission, activation, lifecycle, health, and diagnostics internals.
- **Strategy**: install source, activation policy, compatibility policy, and config source are replaceable strategies.
- **Chain of Responsibility**: install admission validates manifest shape, signature metadata, compatibility, permissions, resources, config, secrets, and entitlement placeholders in order.
- **Command**: every control-plane operation is a typed command and returns a typed result.
- **State**: enabled/disabled/starting/running/stopped/failed states are explicit and auditable.
- **Observer**: control-plane operations emit trace/audit events and structured logs.
- **Null Object**: unsupported install sources and missing optional plugins return structured unavailable results.
- **Memento**: deterministic health and diagnostics snapshots support Web/CLI and tests.

## Boundary Decisions

- `macaca-proto` owns data-only DTOs.
- `macaca-runtime-host` owns repository strategies, manifest loader orchestration, admission chain, control facade, and health snapshots.
- `macaca-kernel` continues to own only registry invariants and must not know install-source details.
- `macaca-sdk` owns shell-facing clients.
- `macaca-web` and `macaca-cli` must not directly read plugin directories or start plugin hosts.

## Trace And Logging

Every state-changing operation must log and emit trace/audit data containing plugin id when known, install source kind, operation, activation state, health state, trace id, result status, and structured error code.

Logs and trace/audit records must not include secrets, raw config values, private keys, unbounded manifest content, API keys, package bytes, or provider credentials.

## Security

- Project-local plugins are disabled by default and require explicit opt-in.
- Path-based install sources must reject traversal and paths outside their declared root.
- Secret/env requirements are reported as names and status only, never values.
- Unsupported install sources return structured unavailable.
- Admission must fail closed when required permissions/resources/config/secrets are missing.

## Risks

- **Risk: Control plane grows into execution plane.** Mitigation: execution remains behind Plugin Runtime/Host phases.
- **Risk: Web/CLI duplicate repository logic.** Mitigation: only SDK/control service exposes management commands.
- **Risk: Config/secret leaks in diagnostics.** Mitigation: DTOs carry presence/status, not values.
- **Risk: Install source ambiguity.** Mitigation: install source is a Strategy with explicit source kinds and structured unsupported results.
