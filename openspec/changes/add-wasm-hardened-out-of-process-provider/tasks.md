## 1. Implementation

- [ ] 1.1 Read the three required governance documents and confirm no kernel/provider/presentation boundary exception is needed.
- [ ] 1.2 Run GitNexus impact analysis for `WasmHardenedProviderEnvelope`, `WasmHardenedProviderResponse`, and `WasmApplicationRuntimeProvider`.
- [ ] 1.3 Add failing tests for daemon unavailable, unhealthy, overloaded, timeout, cancellation, malformed response, crash recovery, and sanitized diagnostics.
- [ ] 1.4 Add `hardened_transport.rs` transport trait with deterministic in-memory test transport.
- [ ] 1.5 Add `hardened_provider.rs` provider/session implementation.
- [ ] 1.6 Reuse existing hardened envelope and response DTOs where possible.
- [ ] 1.7 Add health and backpressure state handling.
- [ ] 1.8 Add English comments explaining daemon transport isolation, failure mapping, and why the provider remains runtime-host-owned.
- [ ] 1.9 Add logs for provider selection, daemon health, dispatch, cancellation, timeout, overload, malformed response, and crash recovery.
- [ ] 1.10 Run `cargo test -p macaca-runtime-host hardened_provider --manifest-path macaca/Cargo.toml`.
- [ ] 1.11 Run `openspec validate add-wasm-hardened-out-of-process-provider --strict`.
- [ ] 1.12 Run GitNexus detect changes before commit.
