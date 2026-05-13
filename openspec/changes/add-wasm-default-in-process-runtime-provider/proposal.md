# Change: Add WASM Default In-Process Runtime Provider

## Why

Macaca now has provider-neutral WASM runtime contracts and package admission, but admitted WASM packages still cannot execute through a default runtime provider. A default in-process provider is needed to prove compile, instantiate, invoke, cache, and sanitized error behavior while keeping engine details out of public Application Framework, SDK, and protocol contracts.

## What Changes

- Add a default in-process WASM runtime provider inside `macaca-runtime-host`.
- Add provider-neutral runtime error taxonomy and compiled artifact cache DTOs.
- Add runtime-host modules for descriptor/factory, engine adapter, compile cache, instance/session, errors, and diagnostics.
- Compile and instantiate minimal WASM modules, invoke exported functions, and return provider-neutral host command results.
- Map engine-specific compile, instantiate, invoke, and trap failures into sanitized provider-neutral diagnostics.

## Impact

- Affected specs: `wasm-default-runtime-provider`, `wasm-runtime-error-taxonomy`, `wasm-compiled-artifact-cache`
- Affected code:
  - `macaca/crates/foundation/macaca-proto/src/wasm_runtime_provider.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/`
  - `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
  - `macaca/crates/runtime/macaca-runtime-host/Cargo.toml`
- Depends on: `add-wasm-runtime-provider-contract`, `add-wasm-package-admission-abi-negotiation`
