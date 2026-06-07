# Change: Add WASM Component Application ABI Skeleton

## Why

Macaca's target application model requires multi-language applications that compile to WASM and call Macaca through a stable ABI. Current code has metadata-only WASM adapters, but there is no formal WIT/schema, guest SDK scaffold, host factory, or unavailable-safe host contract for application WASM components.

## What Changes

- Add a WASM Component Application ABI schema or WIT file aligned with existing `ApplicationImport` and `ApplicationExport`.
- Add WASM component descriptor and guest SDK scaffold helpers.
- Add unavailable-safe WASM application host factory and host result behavior.
- Add fixtures proving WASM packages can be admitted as metadata while execution returns structured unavailable until a real runtime is approved.
- Do not introduce a heavy real WASM runtime dependency in this proposal.

## Impact

- Affected specs: `application-wasm-abi-skeleton`
- Affected code:
  - `macaca/crates/foundation/macaca-proto/src/application_abi.rs`
  - `macaca/crates/application/macaca-app/src/abi.rs`
  - `macaca/crates/application/macaca-app/src/wasm/`
  - `macaca/crates/runtime/macaca-runtime-host/src/application_hosts/`
  - `macaca/crates/facade/macaca-sdk/src/application_kit/wasm.rs`
  - `macaca/application-wit/` or `macaca/resources/application-wit/`
  - `macaca/crates/facade/macaca-sdk/examples/wasm_component_app_fixture.rs`
- Depends on: `add-application-manifest-v1-ability-baseline`, `add-application-sdk-kits-v1`
