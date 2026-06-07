## 1. Implementation

- [x] 1.1 Read the three required governance documents and confirm observability remains a runtime-host Observer boundary.
- [x] 1.2 Run GitNexus impact analysis for `DefaultInProcessWasmRuntimeProvider`, `UnavailableWasmRuntimeProvider`, `WasmHostImportBridge`, and `WasmCertificationHarness`.
- [x] 1.3 Add failing runtime-host tests for telemetry event emission and redaction across admission, compile, instantiate, invoke, resource, host import, lifecycle, daemon, certification, and supply-chain paths.
- [x] 1.4 Add `telemetry.rs` event DTOs, sink trait, in-memory sink, tracing sink, and sanitizer helpers.
- [x] 1.5 Inject telemetry sink into provider constructors using optional `Arc` dependencies.
- [x] 1.6 Emit events from unavailable, default in-process, Component Model, hardened, sandbox guard, host import bridge, lifecycle support, guest harness, and certification paths.
- [x] 1.7 Add English comments explaining Observer fan-out, sanitization, and non-fatal sink behavior.
- [x] 1.8 Update Route C regression matrix with observability readiness rows.
- [x] 1.9 Run `cargo test -p macaca-runtime-host wasm_telemetry --manifest-path macaca/Cargo.toml`.
- [x] 1.10 Run `openspec validate add-wasm-production-observability-sinks --strict`.
- [x] 1.11 Run GitNexus detect changes before commit.
