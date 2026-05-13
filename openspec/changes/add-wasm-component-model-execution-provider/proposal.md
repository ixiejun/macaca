# Change: Add WASM Component Model execution provider

## Why

The current default provider executes only a narrow core-WASM nullary export
surface. Industrial WASM applications require Component Model validation,
WIT/canonical ABI import-export execution, engine-enforced resource controls,
and sanitized trap diagnostics behind the existing provider-neutral contract.

## What Changes

- Add a runtime-host-only Component Model provider strategy.
- Add private engine adapter boundaries for Component Model validation and
  invocation.
- Route Component Model host imports through the existing service portal bridge.
- Enforce memory, fuel/epoch, timeout, and payload limits at provider and engine
  layers.
- Emit sanitized diagnostics and telemetry for compile, instantiate, invoke,
  trap, timeout, host import, resource, and shutdown decisions.
- Preserve the microkernel boundary: no kernel ownership of WASM engine
  selection, guest lifecycle, or provider implementation.

## Governance Constraints

- Must follow `macaca/docs/agent-os-microkernel-boundaries.md`.
- Must not add or widen entries in
  `macaca/docs/route-c-serviceization-allowlist.md`.
- Must satisfy `macaca/docs/route-c-architecture-governance.md` review
  checklist: provider-neutral service ownership, trace-required execution,
  permission/policy enforcement, additive compatibility, and no presentation
  shell semantics.

## Impact

- Affected specs: `wasm-runtime`
- Affected code: `macaca-runtime-host/src/wasm_runtime_provider`,
  `macaca-runtime-host/Cargo.toml`
- Dependency boundary: any optional engine dependency must remain private to
  `macaca-runtime-host` and must not become a kernel, SDK, Web, CLI, app, or
  proto dependency.
