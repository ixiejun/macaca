# Change: Add WASM artifact supply-chain verification

## Why

Industrial package admission must verify artifact identity, signature, signer,
provenance, origin, ABI, and certification compatibility before a WASM
application can be marked industrial-ready.

## What Changes

- Add provider-neutral signature and provenance DTOs.
- Add supply-chain verification rules to package admission and certification.
- Add deterministic signed and unsigned fixtures.
- Add sanitized reason codes for digest mismatch, missing signature, untrusted
  signer, stale provenance, origin mismatch, and incompatible certification.
- Keep Store/CI trust policy pluggable rather than hardcoding a vendor, tenant,
  application, workflow, or package name.

## Governance Constraints

- Must follow Application Framework ownership: `macaca-app` may own admission
  Specification, while Store/Entitlement and external trust services remain
  service boundaries.
- Must not introduce crypto/provider dependencies into kernel, Web, CLI, or SDK
  runtime composition.
- Must not add Route C allowlist exceptions without the required OpenSpec and
  dependency-test updates.

## Impact

- Affected specs: `wasm-package-admission`
- Affected code: `macaca-proto`, `macaca-app`, `macaca-sdk` fixtures
