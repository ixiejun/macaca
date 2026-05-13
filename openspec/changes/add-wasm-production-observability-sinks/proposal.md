# Change: Add WASM production observability sinks

## Why

Industrial WASM operation requires sanitized telemetry for admission, provider
selection, compile, instantiate, invoke, host imports, resource decisions,
lifecycle transitions, daemon health, certification, and supply-chain checks.

## What Changes

- Add runtime-host telemetry event DTOs and sink traits.
- Add in-memory test sink and tracing-compatible sink.
- Emit sanitized events from key runtime provider paths.
- Add tests proving raw payloads and secrets never enter telemetry.
- Keep observability as an Observer boundary, not a vendor-specific dashboard or
  presentation-shell semantic layer.

## Governance Constraints

- Must follow Trace/Audit Bus and ServiceRuntime governance rules.
- Every WASM runtime decision point must have traceable, sanitized audit data.
- Telemetry must not add provider construction duties to Web/CLI or concrete
  backend dependencies to kernel/proto/app/sdk.

## Impact

- Affected specs: `wasm-observability`
- Affected code: `macaca-runtime-host/src/wasm_runtime_provider`,
  `macaca/docs/route-c-regression-matrix.md`
