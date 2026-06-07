## 1. Preparation

- [x] 1.1 Read the plugin enrichment plan and the first three plugin proposals.
- [x] 1.2 Read current `macaca-sdk`, `macaca-runtime-host` plugin facade, and IPC/service runtime contracts.
- [x] 1.3 Run GitNexus impact before editing existing symbols and report blast radius.

## 2. SDK Facade

- [x] 2.1 Add `PluginSdk` facade.
- [x] 2.2 Add `PluginContext`.
- [x] 2.3 Add manifest, registration, capability, hook, config, and secret builders.
- [x] 2.4 Ensure SDK uses proto DTOs and service clients only.
- [x] 2.5 Add detailed English comments explaining public SDK guarantees and internal boundaries.

## 3. Contract Test Kit

- [x] 3.1 Add manifest contract tests.
- [x] 3.2 Add capability contract tests.
- [x] 3.3 Add hook contract tests.
- [x] 3.4 Add config/secret validation tests.
- [x] 3.5 Add unavailable-safe behavior tests.
- [x] 3.6 Add boundary compliance fixtures.

## 4. Host Skeletons

- [x] 4.1 Add descriptor and built-in adapter host canonical APIs.
- [x] 4.2 Add WASM host skeleton with structured unavailable behavior.
- [x] 4.3 Add process host skeleton with structured unavailable behavior.
- [x] 4.4 Add remote proxy host skeleton with structured unavailable behavior.
- [x] 4.5 Add host lifecycle supervisor skeleton.
- [x] 4.6 Add trace, timeout, resource, health, and cleanup logging for host lifecycle.

## 5. Documentation

- [x] 5.1 Update plugin development guide with SDK examples.
- [x] 5.2 Add examples for descriptor plugin, built-in adapter plugin, hook plugin, capability plugin, WASM skeleton, process skeleton, and remote proxy skeleton.
- [x] 5.3 Document contract test commands and unavailable-safe semantics.

## 6. Verification

- [x] 6.1 Run `openspec validate add-plugin-sdk-and-hosts-v1 --strict`.
- [x] 6.2 Run `cargo fmt --all --check`.
- [x] 6.3 Run `cargo check --workspace`.
- [x] 6.4 Run `cargo test -p macaca-sdk plugin`.
- [x] 6.5 Run `cargo test -p macaca-runtime-host plugin_hosts`.
- [x] 6.6 Run `cargo test -p macaca-proto plugin`.
- [x] 6.7 Run `cargo test -p macaca-integration-tests plugin_contract`.
- [x] 6.8 Run `npx gitnexus detect-changes -r agent` before commit.

Note: GitNexus impact and detect-changes were attempted, but GitNexus reported read-only shadow-page/FTS replay errors while resolving graph/file-symbol metadata. Treat impact scope reporting as degraded until the GitNexus database is reopened or rebuilt in read-write mode.
