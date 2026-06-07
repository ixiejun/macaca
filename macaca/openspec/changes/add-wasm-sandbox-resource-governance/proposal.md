# Change: Add WASM sandbox resource governance

## Why
Macaca can create a default in-process WASM session, but long-running infrastructure needs deterministic resource boundaries before guest code is treated as a safe 24/7 application substrate.

## What Changes
- Add provider-neutral resource policy, sandbox policy, WASI policy, and sanitized resource audit contracts.
- Enforce payload size and in-process session concurrency in the runtime host with traceable reason codes.
- Preserve deny-by-default WASI semantics: raw environment, filesystem, and network access remain unavailable unless a future policy explicitly grants scoped virtual resources.
- Map resource exhaustion and policy denial into the existing provider-neutral runtime error taxonomy.

## Impact
- Affected specs: `wasm-resource-policy`, `wasm-sandbox-policy`, `wasm-wasi-policy`, `wasm-resource-audit`
- Affected code: `macaca-proto` WASM runtime provider DTOs and `macaca-runtime-host` WASM provider modules
