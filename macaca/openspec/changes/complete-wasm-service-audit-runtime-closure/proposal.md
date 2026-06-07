# Change: Complete WASM Service-Audit Runtime Closure

## Why
The current implementation already has core pieces (router audit events, replay provider, and bridge sink injection), but production runtime wiring is still partial. This leaves a gap between "feature exists" and "industrial-grade closed loop" for traceability and auditability.

## What Changes
- Unify `ServiceCallAuditSink` ownership at host composition level and inject the same sink into:
  - generic `system.service_audit` provider
  - production WASM host-import bridge construction path
- Remove split-brain audit flows (test-only or local-only sink paths) in runtime wiring where applicable.
- Enforce command-surface replay as a first-class system capability:
  - `service.audit.replay.trace`
  - `service.audit.replay.session`
- Add end-to-end integration coverage for:
  - WASM host import -> `service.call` route audit emission
  - replay query through `system.service_audit`
- Add structured lifecycle logs on sink binding, provider startup, replay query, and replay failure nodes.

## Impact
- Affected specs: `service-runtime-audit` (new capability delta in this change)
- Affected code:
  - `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/*`
  - `macaca/crates/runtime/macaca-runtime-host/src/service_call_audit*`
  - `macaca/crates/shells/macaca-web/src/lib.rs`
  - related runtime/integration tests

## Non-Goals
- No application-specific service hardcoding.
- No business-level audit semantics (only infrastructure-level routing evidence chain).
- No new external dependency introduction.
