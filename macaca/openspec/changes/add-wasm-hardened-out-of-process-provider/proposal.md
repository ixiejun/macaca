# Change: Add WASM hardened out-of-process provider

## Why

Industrial WASM execution needs process isolation, health checks, cancellation,
timeout, backpressure, and crash recovery. The current hardened provider
envelope is a mock contract only.

## What Changes

- Add a runtime-host provider strategy that dispatches to a hardened daemon
  transport.
- Add provider-neutral daemon request/response validation and sanitized
  diagnostics.
- Add health, overload, malformed response, timeout, cancellation, and crash
  recovery handling.
- Preserve existing provider/session traits and host import command semantics.
- Emit logs and audit events at provider selection, daemon health, dispatch,
  cancellation, timeout, overload, crash recovery, and malformed response
  boundaries.

## Governance Constraints

- Must follow `macaca/docs/agent-os-microkernel-boundaries.md`.
- Must not introduce kernel-owned daemon lifecycle or presentation-shell daemon
  construction.
- Must not add a new Route C allowlist exception unless OpenSpec, allowlist
  docs, and dependency tests are updated first.

## Impact

- Affected specs: `wasm-runtime`
- Affected code: `macaca-runtime-host/src/wasm_runtime_provider`
- Dependency boundary: daemon transport is runtime-host-owned and must stay
  provider-neutral at public boundaries.
