## 1. Implementation

- [ ] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-serviceization-allowlist.md`, and `macaca/docs/route-c-architecture-governance.md`; verify the implementation does not add kernel/provider/presentation dependency violations.
- [ ] 1.2 Run GitNexus impact analysis for `WasmApplicationRuntimeProvider`, `WasmExecutionSession`, `DefaultInProcessWasmRuntimeProvider`, and `WasmRuntimeProviderRegistry`.
- [ ] 1.3 Add failing runtime-host tests for Component Model provider descriptor, missing trace, invalid component, missing WIT export, host import bridge dispatch, timeout, and sanitized trap diagnostics.
- [ ] 1.4 Add private `component_model_adapter.rs` with an engine-neutral Adapter trait and a production adapter implementation.
- [ ] 1.5 Add `component_model.rs` provider/session implementation using the existing `WasmApplicationRuntimeProvider` and `WasmExecutionSession` traits.
- [ ] 1.6 Wire the provider from `wasm_runtime_provider/mod.rs` without changing public proto/app/sdk/kernel/Web/CLI dependencies.
- [ ] 1.7 Add engine-enforced resource checks and map all failures to sanitized `WasmRuntimeErrorReport` and `ApplicationAbiError` values.
- [ ] 1.8 Add English comments explaining each public/private runtime boundary and the engine adapter isolation model.
- [ ] 1.9 Add logging at compile, instantiate, invoke, timeout, trap, host import, resource decision, and shutdown boundaries.
- [ ] 1.10 Run `cargo test -p macaca-runtime-host component_model --manifest-path macaca/Cargo.toml`.
- [ ] 1.11 Run `openspec validate add-wasm-component-model-execution-provider --strict`.
- [ ] 1.12 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries --manifest-path macaca/Cargo.toml` if Cargo dependencies change.
- [ ] 1.13 Run GitNexus detect changes before commit.
