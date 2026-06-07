# Change: Add tool runtime environments and managed gateway

## Why

Industrial tools require controlled execution environments, process/runtime lifecycle, artifact roots, network and filesystem policy, secret injection policy, and optional managed gateway providers. Without this layer, Macaca can plan and route tools but cannot safely run complex real-world work across local, sandboxed, remote, browser, WASM, and managed execution contexts.

## What Changes

- Add runtime environment descriptors for local workspace, local sandbox, Docker, SSH/remote, WASM host import, browser sandbox, per-call environments, and session-scoped environments.
- Add environment health, cleanup, resource policy, artifact roots, network policy, secret injection policy, and process handle contracts.
- Add managed gateway provider interface for web, browser, media, document, remote sandbox, enterprise connector, and other optional tool providers.
- Add metering and audit hooks for environment and gateway routes.
- Keep provider names in descriptor/config data only.

## Impact

- Affected specs: `tool-runtime-environments`, `service-runtime`, `serviceization-dependency-gate`
- Affected code: `macaca-runtime-host`, `macaca-proto`, `macaca-sdk`, environment provider adapters
- Depends on: `add-tool-capability-contracts`, `route-tool-invocation-through-tool-service`

## Constraints

- No specific gateway provider may become mandatory.
- Runtime environment providers must enter through service/provider contracts.
- Optional environment and gateway modules must return structured unavailable, disabled, unsupported, or denied states when absent.
- Shells must not own environment lifecycle.
