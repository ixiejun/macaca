# Change: Add WASM Package Admission and ABI Negotiation

## Why

Macaca now has a provider-neutral WASM runtime provider contract, but package admission still lacks structured artifact, ABI, import/export, resource, and compatibility checks. Without this control-plane layer, future WASM packages could bypass traceable fail-closed admission or leak raw artifacts and manifests into metadata surfaces.

## What Changes

- Add provider-neutral WASM artifact, digest, ABI requirement, import requirement, export declaration, and ABI negotiation DTOs.
- Add Application Framework admission specifications for artifact references, ABI compatibility, import permissions, runtime capability matching, and sanitized admission reports.
- Adapt the current metadata-only WASM skeleton to emit the new admission report while preserving runtime-unavailable behavior.
- Require ABI mismatch, missing artifact digest, missing import permission, and runtime capability mismatch to fail closed with traceable reason codes.
- Forbid raw WASM bytes, raw manifests, raw payloads, secrets, env values, API keys, private keys, prompts, and unbounded provider output in manifest metadata, trace, logs, and reports.

## Impact

- Affected specs: `wasm-package-admission`, `wasm-abi-negotiation`, `wasm-compatibility-report`
- Affected code:
  - `macaca/crates/foundation/macaca-proto/src/wasm_runtime_provider.rs`
  - `macaca/crates/application/macaca-app/src/certification/`
  - `macaca/crates/application/macaca-app/src/wasm/`
  - `macaca/crates/facade/macaca-sdk/src/application_testkit/`
- Depends on: `add-wasm-component-application-abi-skeleton`, `add-wasm-runtime-provider-contract`
