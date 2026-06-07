# Change: Add WASM host import service portal

## Why
WASM applications need a controlled way to call Macaca system services without linking guest code to concrete providers, backends, raw IO, or host internals.

## What Changes
- Add a provider-neutral host import bridge contract that converts guest imports into bounded typed commands.
- Route service imports through `ServiceRuntime` so trace, policy, capability, service availability, and payload limits remain centralized.
- Add sanitized host import audit/error vocabulary for allowed, denied, unavailable, and failed calls.
- Integrate the default in-process WASM provider with the host import bridge without adding service business logic to the provider.

## Impact
- Affected specs: `wasm-host-imports`, `wasm-service-portal`, `wasm-host-import-audit`, `wasm-host-import-error-taxonomy`
- Affected code: `macaca-proto` WASM host import DTOs, `macaca-runtime-host` WASM provider/bridge modules, runtime-host WASM tests, SDK WASM guest import contract tests
