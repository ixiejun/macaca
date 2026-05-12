# Change: Add WASM Runtime Provider Contract

## Why

Macaca already has a metadata-only WASM Application ABI skeleton, but future real engines need a stable execution-plane contract before any engine dependency is introduced. Without this boundary, Application Framework, SDK, Runtime Host, and future providers could accidentally leak concrete runtime types or diverge on unavailable behavior.

## What Changes

- Add provider-neutral WASM runtime DTOs for provider descriptors, engine capabilities, execution profiles, availability, diagnostics, session requests, and resource envelopes.
- Add runtime-host provider/session traits plus an unavailable provider that preserves fail-closed behavior without executing guest code.
- Require traceable provider selection, session creation/rejection, structured unavailable results, and sanitized diagnostics.
- Forbid public contracts from exposing concrete WASM engine types, provider names, raw WASM bytes, raw payloads, raw manifests, secrets, env values, or API keys.

## Impact

- Affected specs: `wasm-runtime-provider`, `wasm-runtime-diagnostics`
- Affected code:
  - `macaca/crates/foundation/macaca-proto/src/wasm_runtime_provider.rs`
  - `macaca/crates/foundation/macaca-proto/src/lib.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
- Depends on: `add-wasm-component-application-abi-skeleton`
