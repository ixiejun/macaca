# Change: Add Plugin Runtime v0

## Why

Route C Phase 07 needs a standard, auditable way for third parties and built-in adapters to extend Macaca OS capabilities without turning the kernel, web shell, gateway, driver, memory, skill, or MCP modules into hardcoded integration points.

Without Plugin Runtime v0, gateway/driver/memory/context/skill/MCP/payment/compliance extensions would either bypass the service registry or couple directly to concrete crates. That would violate the microkernel boundary, make plugin lifecycle untraceable, and block later Store, entitlement, paid package, external process plugin, WASM/native plugin, and marketplace phases.

## What Changes

- Add provider-neutral plugin protocol contracts in `macaca-proto` for plugin manifest v0, runtime kind, provided services, provided capabilities, required services, permissions, resources, entry declaration, signature metadata, lifecycle status, lifecycle events, health, and structured plugin errors.
- Add a `macaca-runtime-host` Plugin Runtime facade that validates plugin manifests, models host creation through an Abstract Factory boundary, and registers plugin-provided service descriptors without executing arbitrary third-party binaries in v0.
- Add a `macaca-kernel` plugin registry for plugin identity, service descriptors, lifecycle state, and uninstall cleanup while keeping capability implementation outside the kernel.
- Model built-in gateway/driver/memory/skill/MCP adapters as plugin-provided service descriptors through Adapter patterns without changing existing runtime paths in Phase 07.
- Add lifecycle state transitions for `installed -> registered -> starting -> running -> stopping -> stopped -> uninstalled` with structured trace/audit records for install, register, start, stop, failure, and uninstall.
- Require permission/resource declarations before plugin registration and reject plugins with missing permissions or unsupported runtime declarations.
- Add detailed English comments and structured logs in all new Phase 07 Rust code explaining plugin manifest invariants, lifecycle state transitions, registry ownership, trace/audit behavior, adapter boundaries, and explicit non-goals.

## Impact

- Affected specs: `plugin-runtime-v0`
- Affected crates: `macaca-proto`, `macaca-runtime-host`, `macaca-kernel`, `macaca-gateway`, `macaca-driver`, `macaca-memory`, `macaca-skill`
- Affected code: `macaca-proto/src/plugin.rs`, `macaca-runtime-host/src/plugin.rs`, `macaca-kernel/src/plugin_registry.rs`, built-in gateway/driver/memory/skill adapter descriptors, and plugin runtime tests
- Affected tests: plugin manifest serde/validation tests, runtime-host plugin runtime tests, kernel plugin registry tests, built-in adapter descriptor tests, Route C baseline tests
- Regression matrix references: `RC-DRIVER-001`, `RC-SKILL-001`, plus `RC-TRACE-001` for lifecycle trace integrity

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: plugins extend system surfaces; kernel only stores identity/registry/lifecycle invariants and never implements plugin business behavior.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 07 must preserve driver trace and skill/MCP smoke paths.
- Follows `macaca/docs/route-c-phase-template.md`: Superpowers brainstorm, OpenSpec proposal/design/tasks/spec, GitNexus impact before symbol edits, additive-first implementation, targeted tests, integration smoke, detect_changes before commit.
- Follows `macaca/docs/route-c-architecture-governance.md`: plugin lifecycle must be traceable; plugins must declare permissions/resources; optional gateways/drivers must be unavailable without breaking base OS; no app/provider/driver/gateway/chain hardcoding is allowed.

## Non-Goals

- Do not execute arbitrary third-party WASM, native binaries, remote code, shell scripts, or process plugins in Phase 07.
- Do not implement Store, entitlement, paid package installation, subscription metering, encrypted package distribution, Web3, EVM, or payment settlement.
- Do not replace existing gateway, driver, memory, skill, MCP, chat, task, trace, or session runtime paths; built-in adapter modeling is additive.
- Do not allow plugins to bypass service registry, permission/resource declarations, package runtime guard, trace/audit bus, or lifecycle state machine.
- Do not hardcode application names, workflow names, concrete provider names, driver names, gateway names, chain names, model names, or business routing into plugin runtime logic.
- Do not leave plugin-provided services registered after uninstall, failed start cleanup, or disabled plugin state.
