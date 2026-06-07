## 1. Runtime Wiring Closure
- [x] 1.1 Introduce one host-scoped shared `ServiceCallAuditSink` in production composition path.
- [x] 1.2 Wire `system.service_audit` provider to the shared sink in startup lifecycle.
- [x] 1.3 Wire production WASM host-import bridge/router path to the same shared sink.
- [x] 1.4 Remove or refactor any duplicate sink creation that causes split replay chains.

## 2. Contract and Behavior Validation
- [x] 2.1 Verify `service.audit.replay.trace` returns events emitted by WASM `service.call` routing path.
- [x] 2.2 Verify `service.audit.replay.session` returns events emitted by the same path.
- [x] 2.3 Verify replay failure paths are structured and traceable (missing sink, invalid query, empty result).

## 3. Test and Observability Hardening
- [x] 3.1 Add/extend runtime-host tests for startup wiring and shared-sink replay consistency.
- [x] 3.2 Add/extend integration tests for end-to-end audit chain (WASM host import -> replay service).
- [x] 3.3 Ensure key execution nodes emit structured logs without sensitive payload leakage.

## 4. Acceptance Gate
- [x] 4.1 Run targeted test suites for runtime-host and relevant integration tests.
- [x] 4.2 Run `openspec validate complete-wasm-service-audit-runtime-closure --strict`.
- [x] 4.3 Document completion status with explicit done/not-done matrix.

## Completion Matrix

| Area | Status | Evidence |
|---|---|---|
| Shared sink composition | Done | `service_audit_runtime_bundle.rs` introduces one shared sink and shared wiring helpers. |
| Replay provider startup wiring | Done | `macaca-web/src/lib.rs` registers `system.service_audit` from `ServiceAuditRuntimeBundle`. |
| WASM host import replay chain | Done | `host_import_tests.rs` validates bridge emits replayable `service.call` audit stages. |
| Trace replay contract | Done | `service_call_audit_provider_replays_shared_bridge_events` + bundle test assert replay by trace. |
| Session replay contract | Done | `service_call_audit_provider_replays_shared_bridge_events` + bundle test assert replay by session. |
| Structured failure behavior | Done | `service_call_audit_provider_rejects_invalid_replay_payload_with_structured_error` and empty replay test. |
| Observability and redaction | Done | Structured `tracing::warn!` on replay failures and host import metadata redaction tests. |
| OpenSpec validation | Done | `openspec validate complete-wasm-service-audit-runtime-closure --strict` |
