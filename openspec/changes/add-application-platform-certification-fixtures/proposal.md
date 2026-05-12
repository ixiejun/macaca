# Change: Add Application Platform Certification Fixtures

## Why

Application Platform must prove it supports real ecosystem shapes beyond YAML chat demos. Certification fixtures and tests should validate declarative, GenUI, headless, Store-entitled, Plugin-enhanced, and WASM skeleton applications against the platform contracts without requiring real network, Store, Payment, Plugin execution, Web3/EVM, or WASM runtime.

## What Changes

- Add generic Application Platform certification fixtures.
- Add CertificationKit checks over Manifest v1, Ability descriptors, SDK TestKit, YAML adapter, sanitized metadata views, WASM unavailable host, Store declarations, and Plugin dependencies.
- Add integration tests proving missing permissions/services/plugins/runtimes fail closed or return structured unavailable.
- Add redaction tests proving metadata, diagnostics, logs, and snapshots do not leak unsafe payloads.

## Impact

- Affected specs: `application-platform-certification`
- Affected code:
  - `macaca/crates/tests/macaca-integration-tests/tests/application_platform_contracts.rs`
  - `macaca/crates/tests/macaca-integration-tests/tests/application_platform_contracts/fixtures.rs`
  - `macaca/crates/facade/macaca-sdk/examples/genui_app_fixture.rs`
  - `macaca/crates/facade/macaca-sdk/examples/headless_app_fixture.rs`
  - `macaca/crates/facade/macaca-sdk/examples/plugin_enhanced_app_fixture.rs`
  - `macaca/crates/facade/macaca-sdk/examples/store_entitled_app_fixture.rs`
  - `macaca/crates/application/macaca-app/src/certification/`
- Depends on all previous five Application Platform proposals.
