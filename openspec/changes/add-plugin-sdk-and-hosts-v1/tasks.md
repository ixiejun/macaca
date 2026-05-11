## 1. Preparation

- [ ] 1.1 Read the plugin enrichment plan and the first three plugin proposals.
- [ ] 1.2 Read current `macaca-sdk`, `macaca-runtime-host` plugin facade, and IPC/service runtime contracts.
- [ ] 1.3 Run GitNexus impact before editing existing symbols and report blast radius.

## 2. SDK Facade

- [ ] 2.1 Add `PluginSdk` facade.
- [ ] 2.2 Add `PluginContext`.
- [ ] 2.3 Add manifest, registration, capability, hook, config, and secret builders.
- [ ] 2.4 Ensure SDK uses proto DTOs and service clients only.
- [ ] 2.5 Add detailed English comments explaining public SDK guarantees and internal boundaries.

## 3. Contract Test Kit

- [ ] 3.1 Add manifest contract tests.
- [ ] 3.2 Add capability contract tests.
- [ ] 3.3 Add hook contract tests.
- [ ] 3.4 Add config/secret validation tests.
- [ ] 3.5 Add unavailable-safe behavior tests.
- [ ] 3.6 Add boundary compliance fixtures.

## 4. Host Skeletons

- [ ] 4.1 Add descriptor and built-in adapter host canonical APIs.
- [ ] 4.2 Add WASM host skeleton with structured unavailable behavior.
- [ ] 4.3 Add process host skeleton with structured unavailable behavior.
- [ ] 4.4 Add remote proxy host skeleton with structured unavailable behavior.
- [ ] 4.5 Add host lifecycle supervisor skeleton.
- [ ] 4.6 Add trace, timeout, resource, health, and cleanup logging for host lifecycle.

## 5. Documentation

- [ ] 5.1 Update plugin development guide with SDK examples.
- [ ] 5.2 Add examples for descriptor plugin, built-in adapter plugin, hook plugin, capability plugin, WASM skeleton, process skeleton, and remote proxy skeleton.
- [ ] 5.3 Document contract test commands and unavailable-safe semantics.

## 6. Verification

- [ ] 6.1 Run `openspec validate add-plugin-sdk-and-hosts-v1 --strict`.
- [ ] 6.2 Run `cargo fmt --all --check`.
- [ ] 6.3 Run `cargo check --workspace`.
- [ ] 6.4 Run `cargo test -p macaca-sdk plugin`.
- [ ] 6.5 Run `cargo test -p macaca-runtime-host plugin_hosts`.
- [ ] 6.6 Run `cargo test -p macaca-proto plugin`.
- [ ] 6.7 Run `cargo test -p macaca-integration-tests plugin_contract`.
- [ ] 6.8 Run `npx gitnexus detect-changes -r agent` before commit.
