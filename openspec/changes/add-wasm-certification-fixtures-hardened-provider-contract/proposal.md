# Change: Add WASM certification fixtures and hardened provider contract

## Why
Macaca needs an executable, runtime-owned WASM certification and conformance surface before third-party WASM applications can be treated as industrial-ready ecosystem artifacts.

## What Changes
- Add runtime-scoped WASM certification profile and conformance harness contracts for dev, default, and hardened provider profiles.
- Add deterministic conformance fixtures and negative security cases covering ABI, resource, host import, lifecycle, observability, package compatibility, and report sanitization.
- Add a hardened out-of-process provider contract envelope and mock adapter that reuse the existing provider-neutral WASM runtime API.
- Update Route C regression expectations so WASM certification is a required gate rather than an optional happy-path test.

## Impact
- Affected specs: `wasm-certification`, `wasm-conformance-fixtures`, `wasm-security-negative-tests`, `wasm-hardened-provider-contract`, `wasm-regression-matrix`
- Affected code: `macaca-runtime-host` WASM runtime provider certification/conformance modules and runtime-scoped tests
- Scope note: this change does not implement a real out-of-process runtime daemon or Store commercial review flow; hardened execution is modeled as a provider-neutral contract and mock adapter.
