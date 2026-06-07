# Change: Add WASM guest SDK toolchain test harness

## Why
Third-party WASM application developers need a runtime-owned local harness that proves guest SDK commands, WIT labels, host import routing, and fixture metadata match Macaca's real provider-neutral runtime contracts before publishing or executing inside Macaca.

## What Changes
- Add a runtime-scoped local WASM guest harness that models guest SDK facade calls as provider-neutral Application ABI host commands.
- Add mock host import outcomes for success, denied, unavailable, and unsupported behavior using the same DTO/error vocabulary as the runtime host import bridge.
- Add deterministic toolchain fixture generation for WIT labels, package artifact descriptors, service dependencies, permissions, and example application shapes.
- Add runtime contract tests that keep mock host import behavior aligned with the real host import portal without adding business-specific names.

## Impact
- Affected specs: `wasm-guest-sdk`, `wasm-toolchain`, `wasm-local-test-harness`, `wasm-example-apps`
- Affected code: `macaca-runtime-host` WASM runtime provider test harness modules
- Scope note: this implementation is intentionally runtime-scoped per request; it does not add full multi-language SDKs, IDE plugins, or Store publish pipeline behavior.
