## 1. Discovery

- [x] 1.1 Trace the app UI bridge path for `service.file/file.write`.
- [x] 1.2 Determine whether an existing `service.file` provider is registered
  but not connected to the WASM host import service portal.
- [x] 1.3 Determine whether `service.file` commands are intentionally denied by
  policy, missing provider registration, or missing provider implementation.

## 2. Proposal-Gated Implementation

- [x] 2.1 If a provider exists, wire it into the approved runtime-host
  composition root without app-specific branches.
- [x] 2.2 If a provider is missing, implement a generic provider behind the
  existing `service.file` contract with trace, policy, approval, sandbox,
  memento, and audit behavior. Discovery found the provider exists; the required
  implementation fix is generic parent-directory path resolution.
- [x] 2.3 Preserve structured unavailable or denied results for disabled
  deployments.

## 3. Verification

- [x] 3.1 Verify direct `service.file/file.write` through app UI bridge.
- [x] 3.2 Verify direct `service.file/file.read` through app UI bridge.
- [x] 3.3 Verify CODEX-WASM-WORKBENCH can create a real frontend/backend sample
  only through model-requested file tool calls.
