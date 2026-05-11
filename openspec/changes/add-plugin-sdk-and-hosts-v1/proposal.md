# Change: Add Plugin SDK and Hosts v1

## Why

Macaca needs a stable plugin developer surface and execution-host boundary so plugins can become first-class OS capabilities without depending on internal Rust crates or bypassing service governance.

After Control Plane, Capability Registry, and Hook Bus exist, SDK/ABI/Hosts v1 provides the developer facade, contract test kit, and execution-plane skeleton required to support descriptor, built-in adapter, WASM, process, and remote proxy plugin models.

## What Changes

- Add a `macaca-sdk` Plugin SDK facade with manifest, registration, capability, hook, config, secret, health, and contract-test builders.
- Add protocol-level plugin ABI/host metadata for descriptor, built-in adapter, WASM, process, and remote proxy hosts.
- Add runtime-host host modules and Abstract Factory branches for descriptor, built-in, WASM skeleton, process skeleton, and remote proxy skeleton.
- Add host lifecycle supervisor skeleton with trace, timeout, resource, health, and unavailable-safe behavior.
- Add contract test kit for manifest, capability, hook, config/secret, unavailable-safe, and boundary compliance.
- Update developer documentation with minimal descriptor, built-in adapter, hook, capability, WASM skeleton, process skeleton, and remote proxy skeleton examples.

## Impact

- Affected specs: `plugin-sdk-and-hosts`
- Affected code: `macaca-proto`, `macaca-runtime-host`, `macaca-ipc`, `macaca-sdk`, `macaca-integration-tests`, plugin developer docs
- Affected tests: SDK plugin tests, runtime-host plugin host tests, proto plugin ABI tests, integration contract tests

## Required Governance

- SDK must not expose kernel/runtime-host internals.
- Host skeletons must be unavailable-safe unless a runtime is explicitly implemented.
- WASM/process/remote proxy must be behind Proxy and Abstract Factory boundaries.
- Host lifecycle must be traceable and auditable.
- All new Rust code must include detailed English comments and structured logs.
