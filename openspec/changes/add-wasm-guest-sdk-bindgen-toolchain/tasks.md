## 1. Implementation

- [x] 1.1 Read the three required governance documents and confirm SDK remains provider-neutral and does not construct runtime-host providers.
- [x] 1.2 Run GitNexus impact analysis for `WasmComponentApplicationScaffold` and `WasmComponentApplicationDescriptor`.
- [x] 1.3 Add failing SDK tests for WIT input validation, binding plan generation, Rust scaffold generation, mock host import registration, fixture generation, and local certification report.
- [x] 1.4 Add `wasm_bindgen.rs` with bindgen input, output, diagnostic, backend trait, and Rust scaffold builder.
- [x] 1.5 Integrate generated package descriptor output with existing `WasmComponentApplicationScaffold`.
- [x] 1.6 Reuse runtime guest harness fixture shapes for local mock host imports without introducing SDK runtime-host dependency.
- [x] 1.7 Add English comments explaining provider neutrality, generated source boundaries, and bindgen backend extensibility.
- [x] 1.8 Add sanitized logs for bindgen planning, scaffold generation, fixture emission, and local certification.
- [x] 1.9 Run `cargo test -p macaca-sdk wasm_bindgen --manifest-path macaca/Cargo.toml`.
- [x] 1.10 Run `openspec validate add-wasm-guest-sdk-bindgen-toolchain --strict`.
- [x] 1.11 Run GitNexus detect changes before commit.
